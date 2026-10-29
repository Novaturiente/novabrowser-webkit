//! WebKit setup. Web pages: one persistent NetworkSession (profile data dir) + WebContext;
//! private windows: an ephemeral session. Browser chrome (tab bar, palette, popups):
//! its own ephemeral session + WebContext serving `nova://ui/*`, and the only views
//! with the `nova` message handler in the page's main world (D2, PLAN 1.4).
//! Internal pages (history, bookmarks, settings…) are `nova://page/<n>` in tabs; their
//! actions come back as `nova://page/act?…` requests carrying this run's page token.
use crate::config;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use webkit6::prelude::*;
use webkit6::{
    CookieAcceptPolicy, CookiePersistentStorage, NetworkSession, Settings, URISchemeRequest,
    UserContentInjectedFrames, UserContentManager, UserScript, UserScriptInjectionTime,
    WebContext, WebView,
};

/// Isolated script world for our page scripts (hints, focus reports, passwords): the page
/// can't see or call into it.
pub const WORLD: &str = "novabrowser";

/// macOS Safari 27 (D8). Real Safari freezes the OS token at 10_15_7.
pub const SAFARI_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Safari/605.1.15";
/// navigator.platform must match a Mac UA, else scripts see Linux behind it.
const SAFARI_JS: &str = "if (/Macintosh/.test(navigator.userAgent)) Object.defineProperty(Navigator.prototype, 'platform', {get: () => 'MacIntel', configurable: true});";

/// Media that can't play (format unsupported / not decodable): one report per document.
/// WebKitGTK 6 has no missing-plugin signal, so the page's media `error` events stand in.
const MEDIA_JS: &str = r#"(() => {
  let sent = false;
  addEventListener('error', (e) => {
    const m = e.target instanceof HTMLSourceElement ? e.target.parentElement : e.target;
    if (sent || !(m instanceof HTMLMediaElement)) return;
    const code = m.error ? m.error.code : (m.networkState === 3 ? 4 : 0);
    if (code !== 3 && code !== 4) return; // 3 decode, 4 format/source not supported
    sent = true;
    try { webkit.messageHandlers.nova.postMessage('media ' + code + ' ' + (m.currentSrc || m.src || '').slice(0, 300)); } catch (_) {}
  }, true);
})()"#;

/// Focus reports for auto insert mode, all frames, in our world.
const FOCUS_JS: &str = r#"(() => {
  const ed = (e) => !!e && (e.isContentEditable || (e.tagName === 'INPUT' && !/^(button|checkbox|color|file|hidden|image|radio|range|reset|submit)$/i.test(e.type)) || e.tagName === 'TEXTAREA' || e.tagName === 'SELECT');
  const send = (s) => { try { webkit.messageHandlers.nova.postMessage(s); } catch (_) {} };
  let last = null;
  const report = () => { let a = document.activeElement; while (a && a.shadowRoot && a.shadowRoot.activeElement) a = a.shadowRoot.activeElement; const v = ed(a); if (v !== last) { last = v; send('focus ' + (v ? 1 : 0)); } };
  addEventListener('focusin', report, true);
  addEventListener('focusout', () => setTimeout(report, 0), true);
  addEventListener('pageshow', () => { last = null; report(); });
})();"#;

struct Ctx {
    pages: WebContext,
    session: NetworkSession,
    private: NetworkSession,
    chrome: WebContext,
    chrome_session: NetworkSession,
}

thread_local! {
    static CTX: OnceCell<Ctx> = const { OnceCell::new() };
    /// Internal pages by number (`nova://page/<n>`), newest kept: a reload re-serves it.
    static PAGES: RefCell<(u64, HashMap<u64, String>)> = RefCell::new((0, HashMap::new()));
}

fn ctx<R>(f: impl FnOnce(&Ctx) -> R) -> R {
    CTX.with(|c| f(c.get().expect("web::init not called")))
}

const TABBAR_HTML: &str = include_str!("../ui/tabbar.html");
const PALETTE_HTML: &str = include_str!("../ui/palette.html");
const KEYHINT_HTML: &str = include_str!("../ui/keyhint.html");
const POPUP_HTML: &str = include_str!("../ui/popup.html");
const PROMPT_HTML: &str = include_str!("../ui/prompt.html");
const TOAST_HTML: &str = include_str!("../ui/toast.html");

/// Profile data dir + cache dir: cookies, local storage, cache live there.
pub fn init(data: &std::path::Path, cache: &std::path::Path) {
    let session = NetworkSession::new(data.to_str(), cache.to_str());
    if let Some(cm) = session.cookie_manager() {
        cm.set_persistent_storage(
            data.join("cookies.sqlite").to_str().expect("utf-8 data dir"),
            CookiePersistentStorage::Sqlite,
        );
    }
    let pages = WebContext::new();
    let chrome = WebContext::new();
    for c in [&pages, &chrome] {
        let sm = c.security_manager().expect("security manager");
        // secure: no mixed-content warnings; local: web pages can't load or link it;
        // cors: our pages may fetch their own actions
        sm.register_uri_scheme_as_secure("nova");
        sm.register_uri_scheme_as_local("nova");
        sm.register_uri_scheme_as_cors_enabled("nova");
    }
    pages.register_uri_scheme("nova", serve_page);
    // D3: adblock network rules, HTTPS-only for subresources, headers (ext/)
    match ext_dir() {
        Some(d) => pages.set_web_process_extensions_directory(&d.to_string_lossy()),
        None => eprintln!("novabrowser-webkit: web process extension not found; no network adblock or subresource HTTPS-only"),
    }
    let adblock = crate::adblock::dir();
    let _ = std::fs::create_dir_all(&adblock);
    pages.add_path_to_sandbox(&adblock, true);
    pages.connect_initialize_notification_permissions(crate::permissions::init_notifications);
    pages.connect_initialize_web_process_extensions(|c| {
        c.set_web_process_extensions_initialization_user_data(
            &crate::adblock::ext_config(true).to_variant(),
        );
    });
    chrome.register_uri_scheme("nova", serve_ui);
    let private = NetworkSession::new_ephemeral();
    // favicons are off by default in WebKitGTK 6: the `favicon` property stays empty without this
    for s in [&session, &private] {
        if let Some(m) = s.website_data_manager() {
            m.set_favicons_enabled(true);
        }
    }
    let c = Ctx {
        pages,
        session,
        private,
        chrome,
        chrome_session: NetworkSession::new_ephemeral(),
    };
    CTX.with(|x| {
        let _ = x.set(c);
    });
    apply_config();
}

const EXT_SO: &str = "libnovabrowser_webkit_ext.so";

/// `$NOVA_WEBKIT_EXT_DIR`, next to the binary (cargo builds), or the packaged dir.
fn ext_dir() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf()));
    std::env::var_os("NOVA_WEBKIT_EXT_DIR")
        .map(std::path::PathBuf::from)
        .into_iter()
        .chain(exe_dir)
        .chain([std::path::PathBuf::from("/usr/lib/novabrowser-webkit")])
        .find(|d| d.join(EXT_SO).is_file())
}

pub fn session() -> NetworkSession {
    ctx(|c| c.session.clone())
}

pub fn private_session() -> NetworkSession {
    ctx(|c| c.private.clone())
}

pub fn page_context() -> WebContext {
    ctx(|c| c.pages.clone())
}

/// Session-wide settings from config: cookies, ITP, languages, spellcheck.
pub fn apply_config() {
    let cfg = config::get();
    ctx(|c| {
        for s in [&c.session, &c.private] {
            if let Some(cm) = s.cookie_manager() {
                cm.set_accept_policy(match cfg.str("content.cookies.accept").as_str() {
                    "never" => CookieAcceptPolicy::Never,
                    "no-3rdparty" | "no-unknown-3rdparty" => CookieAcceptPolicy::NoThirdParty,
                    _ => CookieAcceptPolicy::Always,
                });
            }
            s.set_itp_enabled(cfg.bool("webkit.itp"));
        }
        let langs = crate::net::language_list(&cfg.str("content.headers.accept_language"));
        let langs: Vec<&str> = langs.split(',').filter(|l| !l.is_empty()).collect();
        c.pages.set_preferred_languages(&langs);
        let spell = cfg.list("spellcheck.languages");
        c.pages.set_spell_checking_enabled(!spell.is_empty());
        let spell: Vec<&str> = spell.iter().map(String::as_str).collect();
        c.pages.set_spell_checking_languages(&spell);
    });
}

/// User agent for `url` from `content.user_agent` (safari / webkit / custom string).
pub fn user_agent_for(url: &str) -> Option<String> {
    match config::get().get_for("content.user_agent", url).as_str() {
        Some("webkit") | Some("") => None,
        Some("safari") | None => Some(SAFARI_UA.to_string()),
        Some(custom) => Some(custom.to_string()),
    }
}

/// Web page settings from config (`fonts.web.*`, `content.*`, `webkit.*`).
fn page_settings() -> Settings {
    let cfg = config::get();
    let s = Settings::new();
    let px = |n: &str| cfg.int(n).max(0) as u32;
    s.set_default_font_family(&cfg.str("fonts.web.family.standard"));
    s.set_monospace_font_family(&cfg.str("fonts.web.family.fixed"));
    s.set_serif_font_family(&cfg.str("fonts.web.family.serif"));
    s.set_sans_serif_font_family(&cfg.str("fonts.web.family.sans_serif"));
    s.set_cursive_font_family(&cfg.str("fonts.web.family.cursive"));
    s.set_fantasy_font_family(&cfg.str("fonts.web.family.fantasy"));
    if px("fonts.web.size.default") > 0 {
        s.set_default_font_size(px("fonts.web.size.default"));
    }
    if px("fonts.web.size.default_fixed") > 0 {
        s.set_default_monospace_font_size(px("fonts.web.size.default_fixed"));
    }
    s.set_minimum_font_size(px("fonts.web.size.minimum"));
    s.set_enable_smooth_scrolling(cfg.bool("scrolling.smooth"));
    s.set_enable_developer_extras(true);
    s.set_javascript_can_access_clipboard(false);
    s.set_enable_site_specific_quirks(cfg.bool("webkit.site_specific_quirks"));
    s.set_hardware_acceleration_policy(match cfg.str("webkit.hardware_acceleration").as_str() {
        "never" => webkit6::HardwareAccelerationPolicy::Never,
        _ => webkit6::HardwareAccelerationPolicy::Always,
    });
    s.set_media_playback_requires_user_gesture(!cfg.bool("content.autoplay"));
    s.set_enable_webrtc(true);
    s.set_enable_media_stream(true);
    s.set_enable_encrypted_media(false); // D9: no DRM
    // `-apple-color-filter` for dark mode (darkmode.rs); inert until a style sets it
    if let Some(all) = Settings::all_features() {
        if let Some(f) = (0..all.length()).filter_map(|i| all.get(i)).find(|f| f.identifier().as_deref() == Some("ColorFilter")) {
            s.set_feature_enabled(&f, true);
        }
    }
    if let Some(ua) = user_agent_for("") {
        s.set_user_agent(Some(&ua));
    }
    s
}

/// A web page view. `related` (popups) shares the opener's process and session.
pub fn page_view(related: Option<&WebView>, private: bool) -> WebView {
    let ucm = UserContentManager::new();
    ucm.add_script(&UserScript::new(
        SAFARI_JS,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    ));
    ucm.add_script(&UserScript::for_world(
        FOCUS_JS,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::End,
        WORLD,
        &[],
        &[],
    ));
    ucm.add_script(&UserScript::for_world(
        MEDIA_JS,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        WORLD,
        &[],
        &[],
    ));
    crate::passwords::add_scripts(&ucm);
    crate::adblock::add_scripts(&ucm);
    ucm.register_script_message_handler("nova", Some(WORLD));
    let b = WebView::builder()
        .user_content_manager(&ucm)
        .settings(&page_settings());
    let view = match related {
        Some(r) => b.related_view(r).build(),
        None => {
            let session = if private { private_session() } else { session() };
            b.web_context(&page_context()).network_session(&session).build()
        }
    };
    let bg = config::get().str("colors.webpage.bg");
    if let Ok(c) = gdk::RGBA::parse(bg.as_str()) {
        view.set_background_color(&c);
    }
    view.set_vexpand(true);
    view.set_hexpand(true);
    let weak = view.downgrade();
    ucm.connect_script_message_received(Some("nova"), move |_, v| {
        if let Some(view) = weak.upgrade() {
            crate::tabview::page_message(&view, &v.to_str());
        }
    });
    view
}

thread_local! {
    /// A live chrome view; new ones are created related to it (same web process).
    static CHROME_PROC: RefCell<glib::WeakRef<WebView>> = RefCell::new(glib::WeakRef::new());
}

/// A chrome view (`ui` = tabbar, palette, keyhint, popup): transparent, own context,
/// `nova.send(text)` → `on_message(text)`.
pub fn chrome_view(ui: &str, on_message: impl Fn(&str) + 'static) -> WebView {
    let ucm = UserContentManager::new();
    ucm.add_script(&UserScript::new(
        "window.nova = { send: (s) => webkit.messageHandlers.nova.postMessage(String(s)) };",
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::Start,
        &["nova://ui/*"],
        &[],
    ));
    ucm.register_script_message_handler("nova", None);
    ucm.connect_script_message_received(Some("nova"), move |_, v| on_message(&v.to_str()));
    let s = Settings::new();
    s.set_enable_developer_extras(true);
    let (session, context) = ctx(|c| (c.chrome_session.clone(), c.chrome.clone()));
    // all chrome views share one web process (W14: 5 per window cost ~25-40 MB each)
    let related = CHROME_PROC.with_borrow(|w| w.upgrade());
    let b = WebView::builder().user_content_manager(&ucm).settings(&s);
    let view = match &related {
        Some(r) => b.related_view(r).build(),
        None => b.web_context(&context).network_session(&session).build(),
    };
    if related.is_none() {
        CHROME_PROC.with_borrow_mut(|w| w.set(Some(&view)));
    }
    view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    // its web process died: load it again (it says `ready` and gets its state back)
    view.connect_web_process_terminated(|v, _| v.reload());
    view.load_uri(&format!("nova://ui/{ui}"));
    view
}

fn respond(req: &URISchemeRequest, body: &str, mime: &str) {
    let bytes = glib::Bytes::from_owned(body.as_bytes().to_vec());
    let len = bytes.len() as i64;
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    req.finish(&stream, len, Some(mime));
}

fn not_found(req: &URISchemeRequest) {
    req.finish_error(&mut glib::Error::new(gio::IOErrorEnum::NotFound, "not found"));
}

fn serve_ui(req: &URISchemeRequest) {
    let html = match req.path().as_deref() {
        Some("/tabbar") | Some("tabbar") => TABBAR_HTML,
        Some("/palette") | Some("palette") => PALETTE_HTML,
        Some("/keyhint") | Some("keyhint") => KEYHINT_HTML,
        Some("/popup") | Some("popup") => POPUP_HTML,
        Some("/prompt") | Some("prompt") => PROMPT_HTML,
        Some("/toast") | Some("toast") => TOAST_HTML,
        _ => return not_found(req),
    };
    respond(req, html, "text/html");
}

/// Our internal pages call `nova.send("page <token> <json>")`; here that is a fetch of
/// `nova://page/act?<text>` answered by the page's own scheme handler.
const PAGE_SHIM: &str = "<script>window.nova={send:(s)=>fetch('nova://page/act?'+encodeURIComponent(s))};</script>";

/// Make `html` an internal page and return its URL.
pub fn internal_url(html: &str) -> String {
    let body = match html.find('>') {
        Some(i) if html[..i].to_ascii_lowercase().starts_with("<!doctype") => {
            format!("{}{PAGE_SHIM}{}", &html[..=i], &html[i + 1..])
        }
        _ => format!("{PAGE_SHIM}{html}"),
    };
    PAGES.with_borrow_mut(|(n, m)| {
        *n += 1;
        m.insert(*n, body);
        // keep the newest few: back/forward and reloads re-serve them
        let old: Vec<u64> = m.keys().copied().filter(|k| *k + 64 < *n).collect();
        for k in old {
            m.remove(&k);
        }
        format!("nova://page/{n}")
    })
}

pub fn is_internal(url: &str) -> bool {
    url.starts_with("nova://")
}

fn serve_page(req: &URISchemeRequest) {
    let uri = req.uri().map(|u| u.to_string()).unwrap_or_default();
    if let Some(q) = uri.strip_prefix("nova://page/act?") {
        let text = glib::Uri::unescape_string(q, None::<&str>)
            .map(|s| s.to_string())
            .unwrap_or_default();
        // only a nova:// page in the main frame of a tab can act, and only with the token
        let from = req.web_view();
        respond(req, "ok", "text/plain");
        if let Some(view) = from.filter(|v| v.uri().is_some_and(|u| is_internal(&u))) {
            crate::tabview::internal_action(&view, &text);
        }
        return;
    }
    let n = uri
        .strip_prefix("nova://page/")
        .and_then(|n| n.parse::<u64>().ok());
    match n.and_then(|n| PAGES.with_borrow(|(_, m)| m.get(&n).cloned())) {
        Some(html) => respond(req, &html, "text/html"),
        None => respond(
            req,
            "<!doctype html><meta charset=utf-8><title>Expired</title><body style='font:16px sans-serif;padding:2em'>This page was generated in an earlier run. Open it again (for example <code>:history</code>).",
            "text/html",
        ),
    }
}

/// Run `js` in our isolated world of the main frame; the result as a JSON value.
/// The script must evaluate to a JSON string (wrap with `JSON.stringify`).
pub fn eval_world(view: &WebView, js: &str, cb: impl FnOnce(Option<serde_json::Value>) + 'static) {
    view.evaluate_javascript(js, Some(WORLD), None, None::<&gio::Cancellable>, move |r| {
        if let Err(e) = &r {
            eprintln!("novabrowser-webkit: script: {e}");
        }
        cb(r.ok().and_then(|v| serde_json::from_str(&v.to_str()).ok()))
    });
}

/// Run `js` in the page's main world, result ignored.
pub fn eval_page(view: &WebView, js: &str) {
    view.evaluate_javascript(js, None, None, None::<&gio::Cancellable>, |_| {});
}

/// Run `js` in our world, result ignored.
pub fn eval_quiet(view: &WebView, js: &str) {
    view.evaluate_javascript(js, Some(WORLD), None, None::<&gio::Cancellable>, |_| {});
}

/// Stable id of a view for its lifetime.
pub fn id(v: &WebView) -> usize {
    use glib::translate::ToGlibPtr;
    let p: *mut webkit6::ffi::WebKitWebView = v.to_glib_none().0;
    p as usize
}