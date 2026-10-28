//! Web process extension (D3), loaded by WebKit into every web process of the page
//! context. On each `send-request` (also fired for redirects): adblock network rules,
//! HTTPS-only upgrade, `content.headers.custom` / DNT. The UI process tells it:
//! - init data / context message `cfg`: `{"engine": path, "whitelist": [..], "excludes": [..]}`
//! - page message `nav`: `{"url": main-frame URL, "block": bool, "headers": [[k, v]..]}`,
//!   sent before the navigation goes ahead, so it arrives first (same IPC connection).
use adblock::{request::Request, Engine};
use nova_core::urlmatch::UrlPattern;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use webkit_web_process_extension6::glib::translate::{FromGlibPtrNone, ToGlibPtr};
use webkit_web_process_extension6::{glib, URIRequest, URIResponse, UserMessage, WebPage, WebProcessExtension};

#[derive(Default)]
struct Cfg {
    engine_path: String,
    whitelist: Vec<UrlPattern>,
    excludes: Vec<String>,
}

#[derive(Default, Clone)]
struct PageState {
    main: String,
    block: bool,
    headers: Vec<(String, String)>,
}

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
    static CFG: RefCell<Cfg> = RefCell::new(Cfg::default());
    static PAGES: RefCell<HashMap<u64, PageState>> = RefCell::new(HashMap::new());
}

/// Entry point WebKit looks up in every module of the extensions directory.
///
/// # Safety
/// Called by WebKit with a valid extension and a (possibly NULL) GVariant.
#[no_mangle]
pub unsafe extern "C" fn webkit_web_process_extension_initialize_with_user_data(
    ext: *mut webkit_web_process_extension6::ffi::WebKitWebProcessExtension,
    data: *const glib::ffi::GVariant,
) {
    let ext = WebProcessExtension::from_glib_none(ext);
    let data: Option<glib::Variant> = if data.is_null() {
        None
    } else {
        Some(glib::Variant::from_glib_none(data))
    };
    if let Some(s) = data.as_ref().and_then(|d| d.str().map(str::to_string)) {
        set_cfg(&s);
    }
    ext.connect_user_message_received(|_, m| {
        if m.name().as_deref() == Some("cfg") {
            if let Some(s) = param(m) {
                set_cfg(&s);
            }
        }
    });
    ext.connect_page_created(|_, page| {
        page.connect_user_message_received(on_page_message);
        connect_send_request(page);
    });
}

fn param(m: &UserMessage) -> Option<String> {
    m.parameters().and_then(|p| p.str().map(str::to_string))
}

fn set_cfg(json: &str) {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return };
    let strs = |k: &str| -> Vec<String> {
        v[k].as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(str::to_string))
            .collect()
    };
    let path = v["engine"].as_str().unwrap_or("").to_string();
    let reload = CFG.with_borrow(|c| c.engine_path != path) || v["reload"].as_bool() == Some(true);
    CFG.with_borrow_mut(|c| {
        c.whitelist = strs("whitelist").iter().filter_map(|p| UrlPattern::parse(p).ok()).collect();
        c.excludes = strs("excludes");
        c.engine_path = path.clone();
    });
    if reload {
        let e = std::fs::read(&path).ok().and_then(|b| {
            let mut e = Engine::default();
            e.deserialize(&b).ok().map(|_| e)
        });
        ENGINE.set(e);
    }
}

fn on_page_message(page: &WebPage, m: &UserMessage) -> bool {
    if m.name().as_deref() != Some("nav") {
        return false;
    }
    let Some(v) = param(m).and_then(|s| serde_json::from_str::<Value>(&s).ok()) else {
        return true;
    };
    let headers = v["headers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| Some((h[0].as_str()?.to_string(), h[1].as_str()?.to_string())))
        .collect();
    let st = PageState {
        main: v["url"].as_str().unwrap_or("").to_string(),
        block: v["block"].as_bool().unwrap_or(false),
        headers,
    };
    PAGES.with_borrow_mut(|p| p.insert(page.id(), st));
    true
}

/// adblock request type from the Accept header WebCore set, else the file extension.
/// shortcut: guessed, WebKit doesn't say what loads a request; scripts without `.js` are "other".
pub fn guess_type(url: &str, accept: &str) -> &'static str {
    if accept.starts_with("text/html") {
        return "subdocument";
    }
    if accept.starts_with("image/") {
        return "image";
    }
    if accept.starts_with("text/css") {
        return "stylesheet";
    }
    let path = url.split(['?', '#']).next().unwrap_or("");
    let ext = path.rsplit_once('.').map_or("", |(_, e)| e).to_ascii_lowercase();
    match ext.as_str() {
        "js" | "mjs" => "script",
        "css" => "stylesheet",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg" | "ico" => "image",
        "woff" | "woff2" | "ttf" | "otf" => "font",
        "mp4" | "webm" | "m3u8" | "mp3" | "ogg" => "media",
        _ => "other",
    }
}

/// `send-request` by hand: the binding wraps a NULL redirected_response (every
/// non-redirect) as an object and asserts on it.
fn connect_send_request(page: &WebPage) {
    use webkit_web_process_extension6::ffi;
    unsafe extern "C" fn trampoline(
        this: *mut ffi::WebKitWebPage,
        request: *mut ffi::WebKitURIRequest,
        redirected: *mut ffi::WebKitURIResponse,
        _data: glib::ffi::gpointer,
    ) -> glib::ffi::gboolean {
        // SAFETY: WebKit passes a live page and request; redirected may be NULL
        let page = WebPage::from_glib_none(this);
        let req = URIRequest::from_glib_none(request);
        let redirected = (!redirected.is_null()).then(|| URIResponse::from_glib_none(redirected));
        on_send_request(&page, &req, redirected.as_ref()) as glib::ffi::gboolean
    }
    unsafe {
        glib::signal::connect_raw(
            ToGlibPtr::<*mut ffi::WebKitWebPage>::to_glib_none(page).0 as *mut _,
            c"send-request".as_ptr(),
            Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(trampoline as *const ())),
            Box::into_raw(Box::new(())),
        );
    }
}

fn on_send_request(page: &WebPage, req: &URIRequest, redirected: Option<&URIResponse>) -> bool {
    let Some(url) = req.uri().map(|u| u.to_string()) else { return false };
    let redirect_from = redirected.map(|r| r.uri().map(|u| u.to_string()).unwrap_or_default());
    let id = page.id();
    let mut st = PAGES.with_borrow(|p| p.get(&id).cloned()).unwrap_or_default();
    let is_main = url == st.main || redirect_from.as_deref().is_some_and(|f| f == st.main);
    if is_main && url != st.main {
        // the main document redirected: follow it
        st.main = url.clone();
        PAGES.with_borrow_mut(|p| p.insert(id, st.clone()));
    }
    let method = req.http_method().map(|m| m.to_string());
    let get = matches!(method.as_deref(), None | Some("GET") | Some("HEAD"));
    let headers = req.http_headers();
    let accept = headers
        .as_ref()
        .and_then(|h| h.one("Accept"))
        .map(|a| a.to_string())
        .unwrap_or_default();
    if !is_main && st.block && should_block(&url, &st.main, guess_type(&url, &accept)) {
        return true;
    }
    if get {
        let up = CFG.with_borrow(|c| nova_core::net::https_upgrade_with(&url, &c.excludes));
        if let Some(up) = up {
            req.set_uri(&up);
        }
    }
    if let Some(h) = headers {
        for (k, v) in &st.headers {
            h.replace(k, v);
        }
    }
    false
}

fn should_block(url: &str, source: &str, kind: &str) -> bool {
    if CFG.with_borrow(|c| c.whitelist.iter().any(|p| p.matches(url))) {
        return false;
    }
    ENGINE.with_borrow(|e| {
        let Some(e) = e else { return false };
        let Ok(req) = Request::new(url, source, kind, "get") else { return false };
        e.check_network_request(&req).should_block()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_types() {
        assert_eq!(guess_type("https://x/a.js?v=1", "*/*"), "script");
        assert_eq!(guess_type("https://x/p", "image/webp,*/*"), "image");
        assert_eq!(guess_type("https://x/f", "text/html,application/xhtml+xml"), "subdocument");
        assert_eq!(guess_type("https://x/api", "*/*"), "other");
    }

    #[test]
    fn engine_from_cfg_blocks() {
        let mut set = adblock::lists::FilterSet::new(false);
        set.add_filter_list("||ads.example^\n".into(), Default::default());
        let e = Engine::new_with_filter_set(set);
        let p = std::env::temp_dir().join(format!("nova-ext-engine-{}", std::process::id()));
        std::fs::write(&p, e.serialize()).unwrap();
        set_cfg(&serde_json::json!({"engine": p, "whitelist": ["*://ads.example/ok*"], "excludes": []}).to_string());
        assert!(should_block("https://ads.example/x.js", "https://site.test/", "script"));
        assert!(!should_block("https://ads.example/ok.js", "https://site.test/", "script"));
        assert!(!should_block("https://cdn.test/x.js", "https://site.test/", "script"));
        std::fs::remove_file(p).unwrap();
    }
}
