//! novabrowser-webkit: the keyboard-driven browser on WebKitGTK 6.0 (GTK 4). Logic
//! shared with the CEF edition lives in nova-core; this crate is the engine glue.
mod adblock;
mod anim;
mod clipboard;
mod commands;
mod darkmode;
mod downloads;
mod hints;
mod ipc;
mod modes;
mod passwords;
mod permissions;
mod spawn;
mod tabview;
mod web;
mod window;
mod zoom;

use gtk::prelude::*;
use gtk::{gio, glib};
// Engine-independent code from nova-core; re-exported so `crate::keys` etc. read like the CEF edition.
pub(crate) use nova_core::paths::{page_token, profile, valid_profile_name, write_atomic, xdg_dir, xdg_root};
pub(crate) use nova_core::{
    completion, config, history, import, keys, marks, net, profiles, session, settings_page, theme, welcome,
};

const APP: &str = "novabrowser-webkit";

/// nova-core's seam into this edition (see nova_core::host).
struct WebKitHost;
impl nova_core::host::Host for WebKitHost {
    fn message(&self, level: &str, text: &str) {
        window::message(level, text)
    }
    fn config_changed(&self) {
        window::config_changed()
    }
    fn open_window(&self, url: Option<String>) {
        window::open_window(url)
    }
    fn show_completion(&self, json: Option<&str>) {
        window::show_completion(json)
    }
    fn set_cmdline_text(&self, text: &str) {
        window::set_cmdline_text(text)
    }
    fn search(&self, text: &str, backward: bool) {
        let _ = commands::search(text, backward);
    }
    fn session_json(&self) -> Option<serde_json::Value> {
        window::session_json()
    }
    fn restore(&self, v: &serde_json::Value, clear: bool) {
        window::restore(v, clear)
    }
    fn later(&self, ms: u64, f: Box<dyn FnOnce()>) {
        glib::timeout_add_local_once(std::time::Duration::from_millis(ms), f);
    }
    fn run_in_tab(&self, tab: u64, line: &str) -> bool {
        let line = line.to_string();
        window::run_in_tab(tab as usize, move || commands::run(&line, None))
    }
    fn show_text(&self, text: &str) {
        // qutebrowser shows `spawn -o` output in a new tab as plain text
        let esc = text.replace('&', "&amp;").replace('<', "&lt;");
        let html = format!("<!doctype html><meta charset=utf-8><title>Output</title><pre style='white-space:pre-wrap'>{esc}</pre>");
        window::open(&web::internal_url(&html), true);
    }
}

/// Run `f` on the GTK main thread (results from worker threads).
fn post(f: Box<dyn FnOnce() + Send>) {
    glib::MainContext::default().invoke(f);
}

/// `novabrowser-webkit [switches] [url]`: first non-switch argument; bare hosts get https://.
fn start_url() -> Option<String> {
    let arg = std::env::args().skip(1).find(|a| !a.starts_with('-'))?;
    Some(
        if arg.contains("://") || arg.starts_with("about:") || arg.starts_with("data:") {
            arg
        } else {
            format!("https://{arg}")
        },
    )
}

/// `--profile <name>` / `-P <name>` → `--profile=<name>`; None = nothing to rewrite.
/// `Err` = the flag has no name: list the profiles instead.
fn normalize_profile_args(args: &[String]) -> Result<Option<Vec<String>>, ()> {
    let Some(i) = args
        .iter()
        .skip(1)
        .position(|a| a == "--profile" || a == "-P")
        .map(|i| i + 1)
    else {
        return Ok(None);
    };
    let name = args.get(i + 1).filter(|n| !n.starts_with('-')).ok_or(())?;
    let mut out = args.to_vec();
    out.splice(i..i + 2, [format!("--profile={name}")]);
    Ok(Some(out))
}

fn print_profiles() {
    let current = profile().unwrap_or("default");
    for n in profiles::list() {
        println!("{n}{}", if n == current { "  (this one)" } else { "" });
    }
    eprintln!("start one with: {APP} --profile=<name> [url]  (a new name creates it)");
}

fn main() {
    nova_core::paths::set_app(APP);
    let raw: Vec<String> = std::env::args().collect();
    match normalize_profile_args(&raw) {
        Ok(Some(args)) => {
            // re-run ourselves so every later look at argv sees --profile=
            use std::os::unix::process::CommandExt;
            let exe = std::env::current_exe().unwrap_or_else(|_| raw[0].clone().into());
            let err = std::process::Command::new(exe).arg0(&raw[0]).args(&args[1..]).exec();
            eprintln!("{APP}: can't restart with {}: {err}", args.join(" "));
            std::process::exit(1);
        }
        Ok(None) => {}
        Err(()) => return print_profiles(),
    }
    if raw.iter().skip(1).any(|a| a == "--list-profiles") {
        return print_profiles();
    }
    let argv: Vec<String> = raw.iter().filter(|a| !a.starts_with("--profile=")).cloned().collect();
    if let Some(name) = profile() {
        if !valid_profile_name(name) {
            eprintln!("{APP}: bad profile name {name:?} (letters, digits, space, - and _)");
            std::process::exit(2);
        }
        let dir = xdg_root("XDG_DATA_HOME", ".local/share").join("profiles").join(name);
        if !dir.exists() {
            eprintln!("{APP}: new profile '{name}' ({})", dir.display());
        }
    }
    // rbw add/edit run us as $VISUAL (nova_core::passwords): put the secret in the file
    if std::env::var_os("NOVA_PW_SECRET").is_some() && argv.len() == 2 {
        std::process::exit(nova_core::passwords::editor_main(&argv[1]));
    }
    let data = xdg_dir("XDG_DATA_HOME", ".local/share");
    match argv.get(1).map(String::as_str) {
        Some("--import") => {
            std::fs::create_dir_all(&data).expect("create data dir");
            if let Err(e) = import::main(argv.get(2).map(String::as_str), &data.join("history.sqlite")) {
                eprintln!("{APP}: import: {e}");
                std::process::exit(1);
            }
            return;
        }
        Some("--check-config") => {
            match config::load() {
                Ok(()) => {
                    println!("{}: ok", config::path().display());
                    let ignored = config::get().ignored_per_site();
                    if !ignored.is_empty() {
                        println!("only global, ignored in pattern(): {}", ignored.join(", "));
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Some("--version") => return println!("{APP} {}", env!("CARGO_PKG_VERSION")),
        _ => {}
    }
    // Control socket: `--cmd` / `--query` talk to the running instance; a plain
    // start while one runs hands the URL over.
    let req = match argv.get(1).map(String::as_str) {
        Some("--cmd") => Some(format!("cmd {}", argv[2..].join(" "))),
        Some("--query") => Some(format!("query {}", argv.get(2).map_or("state", String::as_str))),
        Some("--key") => Some(format!("key {}", argv[2..].join(" "))),
        _ => None,
    };
    if let Some(req) = req {
        match ipc::request(&req) {
            Some(reply) => println!("{reply}"),
            None => {
                eprintln!("{APP}: no running instance ({})", ipc::socket_path().display());
                std::process::exit(1);
            }
        }
        return;
    }
    match argv.iter().skip(1).find(|a| !a.starts_with('-')) {
        // `url`, not `cmd open`: a URL from another app must never be parsed as commands
        Some(url) if ipc::request(&format!("url {url}")).is_some() => return,
        None if ipc::request("cmd open -w").is_some() => return,
        _ => {}
    }
    let config_error = config::load().err();
    net::load_https_excludes();
    let state = xdg_dir("XDG_STATE_HOME", ".local/state");
    let cache = xdg_dir("XDG_CACHE_HOME", ".cache");
    for d in [&data, &state, &cache] {
        std::fs::create_dir_all(d).expect("create data dirs");
    }
    // history, sessions, cookies, aria2's session file: owner only, whatever the umask
    for root in [xdg_root("XDG_DATA_HOME", ".local/share"), xdg_root("XDG_STATE_HOME", ".local/state")] {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700));
    }
    // Wayland app id / X11 WM_CLASS; no GApplication id: our own socket keeps one instance per profile
    glib::set_prgname(Some(APP));
    glib::set_application_name("novabrowser");
    let app = gtk::Application::new(None::<&str>, gio::ApplicationFlags::NON_UNIQUE);
    let start = start_url();
    app.connect_activate(move |app| {
        nova_core::host::set(Box::new(WebKitHost), post);
        window::set_app(app);
        web::init(&data, &cache);
        modes::init();
        history::start(data.join("history.sqlite"));
        adblock::start();
        downloads::init();
        ipc::start();
        match &config_error {
            Some(e) => window::startup_message("error", &format!("config: {e}")),
            None => {
                let ignored = config::get().ignored_per_site();
                if !ignored.is_empty() {
                    window::startup_message(
                        "warning",
                        &format!("config: only global, ignored in pattern(): {}", ignored.join(", ")),
                    );
                }
            }
        }
        window::create(start.clone());
        window::start_hibernation();
    });
    // GTK must not see our switches
    let code = app.run_with_args(&raw[..1]);
    nova_core::aria2::stop();
    std::process::exit(code.into());
}

#[cfg(test)]
mod tests {
    use super::normalize_profile_args as norm;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn profile_flag_forms() {
        assert_eq!(norm(&v(&["nb", "--profile", "work", "x.com"])), Ok(Some(v(&["nb", "--profile=work", "x.com"]))));
        assert_eq!(norm(&v(&["nb", "x.com", "-P", "my work"])), Ok(Some(v(&["nb", "x.com", "--profile=my work"]))));
        assert_eq!(norm(&v(&["nb", "--profile=work", "x.com"])), Ok(None));
        assert_eq!(norm(&v(&["nb", "--profile"])), Err(()));
    }
}
