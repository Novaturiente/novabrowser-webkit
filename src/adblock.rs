//! Brave's adblock engine (adblock-rust). Lists are fetched in the background, cached
//! on disk (`engine.dat`) and refreshed when older than a day. Network rules run in the
//! web process extension (ext/, D3), which loads the same `engine.dat`; this side keeps
//! an engine for cosmetics: per-site hide CSS + uBlock scriptlets as a per-tab user
//! script, and generic class/id hiding answered for `adblock-scan.js`.
use crate::{config, web, window, write_atomic};
use adblock::{
    lists::{FilterSet, ParseOptions},
    resources::Resource,
    Engine,
};
use nova_core::host;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};
use webkit6::prelude::*;
use webkit6::{
    UserContentInjectedFrames, UserContentManager, UserMessage, UserScript,
    UserScriptInjectionTime, WebView,
};

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);
/// One update at a time (daily timer vs `:adblock-update`).
static UPDATING: Mutex<()> = Mutex::new(());
const MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const SCAN_JS: &str = include_str!("adblock-scan.js");

thread_local! {
    /// view id -> its per-site cosmetic script.
    static SCRIPTS: RefCell<HashMap<usize, UserScript>> = RefCell::new(HashMap::new());
}

pub fn dir() -> PathBuf {
    // lists + engine are the same for every profile
    crate::xdg_root("XDG_DATA_HOME", ".local/share").join("adblock")
}

fn engine_file() -> PathBuf {
    dir().join("engine.dat")
}

/// uBlock Origin scriptlets, built by scripts/gen-adblock-resources.mjs (CEF repo).
/// shortcut: bundled at build time, so new scriptlets need a rebuild (rules in the lists update daily).
fn resources() -> Vec<Resource> {
    serde_json::from_str(include_str!("adblock-resources.json")).expect("bundled adblock resources")
}

fn engine() -> Option<Arc<Engine>> {
    ENGINE.read().unwrap().clone()
}

fn enabled(cfg: &config::Config, url: &str) -> bool {
    cfg.get_for("content.blocking.enabled", url).as_bool().unwrap_or(false)
        && matches!(
            cfg.str("content.blocking.method").as_str(),
            "auto" | "adblock" | "both"
        )
}

fn lists_key(cfg: &config::Config) -> String {
    cfg.list("content.blocking.adblock.lists").join("\n")
}

fn report(level: &'static str, text: String) {
    host::on_ui(move || window::message(level, &text));
}

/// What the extension needs besides per-page state (see ext/src/lib.rs).
pub fn ext_config(reload: bool) -> String {
    let cfg = config::get();
    json!({
        "engine": engine_file(),
        "whitelist": cfg.list("content.blocking.whitelist"),
        "excludes": crate::net::https_excludes(),
        "reload": reload,
    })
    .to_string()
}

/// After `:set`, `https-toggle`, `config-source`, or a new engine: tell every web process.
pub fn push_config() {
    push(false);
}

fn push(reload: bool) {
    let msg = UserMessage::new("cfg", Some(&ext_config(reload).to_variant()));
    web::page_context().send_message_to_all_extensions(&msg);
}

/// Load the cached engine, then keep the lists fresh (worker thread).
pub fn start() {
    std::thread::Builder::new()
        .name("adblock".into())
        .spawn(|| {
            let fresh = load_cache();
            if !fresh {
                update(false);
            }
            loop {
                std::thread::sleep(Duration::from_secs(3600));
                if !cache_fresh() {
                    update(false);
                }
            }
        })
        .expect("spawn adblock thread");
}

/// `:adblock-update`
pub fn update_now() {
    std::thread::spawn(|| update(true));
}

/// Install the cached engine if there is one. True if it is fresh (same lists, < 1 day old).
fn load_cache() -> bool {
    let Ok(bytes) = std::fs::read(engine_file()) else {
        return false;
    };
    let mut e = Engine::default();
    if e.deserialize(&bytes).is_err() {
        eprintln!("novabrowser-webkit: adblock cache unreadable, rebuilding");
        return false;
    }
    e.use_resources(resources());
    *ENGINE.write().unwrap() = Some(Arc::new(e));
    cache_fresh()
}

fn cache_fresh() -> bool {
    let d = dir();
    let same_lists =
        std::fs::read_to_string(d.join("lists.txt")).is_ok_and(|k| k == lists_key(&config::get()));
    let age = std::fs::metadata(engine_file())
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok());
    same_lists && age.is_some_and(|a| a < MAX_AGE)
}

fn fetch(url: &str) -> Result<String, String> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read_to_string(path).map_err(|e| e.to_string());
    }
    // shortcut: curl instead of an HTTP crate; swap if curl ever isn't on the target systems.
    let out = Command::new("curl")
        // https/http only, redirects only to https, lists over 64 MB refused
        .args(["-fsSL", "--compressed", "--max-time", "120", "--proto", "=https,http", "--proto-redir", "=https", "--max-filesize", "64M", url])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

/// Download every list (falling back to its last copy), build, cache, install.
fn update(verbose: bool) {
    let Ok(_lock) = UPDATING.try_lock() else {
        if verbose {
            report("info", "adblock: update already running".into());
        }
        return;
    };
    let cfg = config::get();
    let urls = cfg.list("content.blocking.adblock.lists");
    let d = dir();
    if let Err(e) = std::fs::create_dir_all(&d) {
        return report("error", format!("adblock: {}: {e}", d.display()));
    }
    let mut set = FilterSet::new(false);
    let mut failed = Vec::new();
    let mut rules = 0;
    for url in &urls {
        let copy = d.join(format!("list-{:016x}.txt", fnv(url)));
        let text = match fetch(url) {
            Ok(t) => {
                let _ = write_atomic(&copy, t.as_bytes());
                t
            }
            Err(e) => {
                eprintln!("novabrowser-webkit: adblock: {url}: {e}");
                failed.push(url.as_str());
                match std::fs::read_to_string(&copy) {
                    Ok(t) => t,
                    Err(_) => continue,
                }
            }
        };
        rules += text.lines().count();
        set.add_filter_list(text, ParseOptions::default());
    }
    if !urls.is_empty() && failed.len() == urls.len() {
        // offline: keep the old cache and its age, so the next hourly check tries again (A10)
        return report("warning", format!("adblock: no list could be fetched; kept the old rules ({})", failed.join(", ")));
    }
    let mut e = Engine::new_with_filter_set(set);
    let ok = write_atomic(&engine_file(), &e.serialize()).is_ok()
        && write_atomic(&d.join("lists.txt"), lists_key(&cfg).as_bytes()).is_ok();
    if !ok {
        eprintln!("novabrowser-webkit: adblock: could not write cache in {}", d.display());
    }
    e.use_resources(resources());
    *ENGINE.write().unwrap() = Some(Arc::new(e));
    host::on_ui(|| push(true));
    let msg = format!("adblock: {} lists, {rules} lines", urls.len() - failed.len());
    if !failed.is_empty() {
        report("warning", format!("{msg}; failed (kept old copy): {}", failed.join(", ")));
    } else if verbose {
        report("info", msg);
    } else {
        eprintln!("novabrowser-webkit: {msg}");
    }
}

fn fnv(s: &str) -> u64 {
    s.bytes()
        .fold(0xcbf29ce484222325, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// Every page view: the generic-hiding scanner in our world (main frame).
pub fn add_scripts(ucm: &UserContentManager) {
    ucm.add_script(&UserScript::for_world(
        SCAN_JS,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::Start,
        web::WORLD,
        &["http://*/*", "https://*/*"],
        &[],
    ));
}

/// Main-frame navigation about to go ahead (decide-policy): tell the extension the page
/// and whether to block on it, and swap the tab's per-site cosmetic script.
pub fn on_navigate(view: &WebView, url: &str) {
    let cfg = config::get();
    let block = url.starts_with("http") && enabled(&cfg, url);
    let headers: Vec<[String; 2]> = crate::net::headers_for(url)
        .into_iter()
        .map(|(k, v)| [k, v])
        .collect();
    let nav = json!({ "url": url, "block": block, "headers": headers }).to_string();
    view.send_message_to_page(
        &UserMessage::new("nav", Some(&nav.to_variant())),
        None::<&gtk::gio::Cancellable>,
        |_| {},
    );
    let Some(ucm) = view.user_content_manager() else { return };
    let id = web::id(view);
    if let Some(old) = SCRIPTS.with_borrow_mut(|s| s.remove(&id)) {
        ucm.remove_script(&old);
    }
    if !block {
        return;
    }
    let Some(js) = engine().and_then(|e| page_script(&e, url)) else { return };
    let s = UserScript::new(
        &js,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    ucm.add_script(&s);
    SCRIPTS.with_borrow_mut(|m| m.insert(id, s));
}

pub fn forget(view: &WebView) {
    SCRIPTS.with_borrow_mut(|s| s.remove(&web::id(view)));
}

/// `adblock {u, c, i}` from adblock-scan.js: answer with the generic hide rules that
/// match the new class names / ids (adblock-rust's hidden_class_id_selectors, as Brave).
pub fn on_message(view: &WebView, payload: &str) {
    let Ok(p) = serde_json::from_str::<Value>(payload) else { return };
    let url = p["u"].as_str().unwrap_or("");
    let Some(e) = engine() else { return };
    if !url.starts_with("http") || !enabled(&config::get(), url) {
        return;
    }
    let Some(css) = generic_css(&e, url, &p) else { return };
    web::eval_quiet(
        view,
        &format!("globalThis.__novaHide && __novaHide({})", serde_json::to_string(&css).unwrap()),
    );
}

fn generic_css(e: &Engine, url: &str, p: &Value) -> Option<String> {
    let r = e.url_cosmetic_resources(url);
    if r.generichide {
        return None;
    }
    let strs = |k: &str| -> Vec<String> {
        p[k].as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default()
    };
    let sels = e.hidden_class_id_selectors(strs("c"), strs("i"), &r.exceptions);
    if sels.is_empty() {
        return None;
    }
    Some(
        sels.iter()
            .map(|s| format!("{s}{{display:none!important}}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// shortcut: no procedural filters (:has-text etc.); specific hides + scriptlets only.
fn page_script(e: &Engine, url: &str) -> Option<String> {
    let r = e.url_cosmetic_resources(url);
    if r.hide_selectors.is_empty() && r.injected_script.is_empty() {
        return None;
    }
    let host = crate::commands::host(url);
    // One rule per selector, so one invalid selector only drops itself.
    let css: String = r
        .hide_selectors
        .iter()
        .map(|s| format!("{s}{{display:none!important}}\n"))
        .collect();
    // Hostname guard: the script stays registered for same-tab navigations and runs in frames.
    // Constructable stylesheet: not subject to the page's CSP, unlike a <style> element.
    Some(format!(
        "if(location.hostname==={host}){{try{{{script}\n}}catch(e){{}}\
         try{{const s=new CSSStyleSheet();s.replaceSync({css});\
         document.adoptedStyleSheets=[...document.adoptedStyleSheets,s]}}catch(e){{}}}}",
        host = serde_json::to_string(&host).unwrap(),
        script = r.injected_script,
        css = serde_json::to_string(&css).unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(rules: &str) -> Engine {
        let mut set = FilterSet::new(false);
        set.add_filter_list(rules.to_string(), ParseOptions::default());
        let mut e = Engine::new_with_filter_set(set);
        e.use_resources(resources());
        e
    }

    #[test]
    fn cosmetic_and_generic() {
        let e = engine_with(
            "example.com##.banner\n##.ad-box\n\
             youtube.com##+js(set-constant, ytInitialPlayerResponse.adPlacements, undefined)\n",
        );
        let js = page_script(&e, "https://example.com/a").unwrap();
        assert!(js.contains(".banner{display:none!important}") && js.contains("\"example.com\""));
        let yt = page_script(&e, "https://www.youtube.com/watch?v=x").unwrap();
        assert!(yt.contains("adPlacements"), "scriptlet resolved from bundled resources");
        assert!(page_script(&e, "https://other.test/").is_none());
        let css = generic_css(&e, "https://other.test/", &json!({"c": ["ad-box", "x"], "i": []})).unwrap();
        assert_eq!(css, ".ad-box{display:none!important}");
    }
}
