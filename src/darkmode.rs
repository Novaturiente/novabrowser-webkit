//! Per-site dark mode (`colors.webpage.darkmode.enabled`) with Dark Reader (D6;
//! `vendor/darkreader`, MIT) in our isolated world. Pages that should be dark get it
//! as a per-tab user script at document start (swapped at main-frame decide-policy);
//! a page already loaded is switched live.
//! shortcut: `darkmode.algorithm` / `policy.images` are Chromium knobs; Dark Reader's own image handling is used.
use crate::{config, web};
use std::cell::RefCell;
use std::collections::HashMap;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserScript, UserScriptInjectionTime, WebView};

const DARK_READER: &str = include_str!("../vendor/darkreader/darkreader.js");
const ENABLE: &str = "DarkReader.setFetchMethod(window.fetch);\
 DarkReader.enable({mode: 1, brightness: 100, contrast: 100, sepia: 0});";

thread_local! {
    /// view id -> its Dark Reader script, while the tab is dark.
    static ON: RefCell<HashMap<usize, UserScript>> = RefCell::new(HashMap::new());
}

fn dark_at(url: &str) -> bool {
    url.starts_with("http")
        && config::get()
            .get_for("colors.webpage.darkmode.enabled", url)
            .as_bool()
            .unwrap_or(false)
}

/// The current page after a config change or `dark-toggle`: switch it live.
pub fn apply(view: &WebView) {
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    let was = ON.with_borrow(|o| o.contains_key(&web::id(view)));
    let want = dark_at(&url);
    apply_for(view, &url);
    if want == was {
        return;
    }
    let js = if want {
        format!("if (!globalThis.DarkReader) {{ {DARK_READER} }} {ENABLE}")
    } else {
        "globalThis.DarkReader && DarkReader.disable()".to_string()
    };
    web::eval_quiet(view, &js);
}

/// A main-frame navigation to `url` is about to go ahead: install or drop the
/// document-start script so the page is dark from its first paint.
pub fn apply_for(view: &WebView, url: &str) {
    let id = web::id(view);
    let want = dark_at(url);
    let have = ON.with_borrow(|o| o.contains_key(&id));
    if want == have {
        return;
    }
    let Some(ucm) = view.user_content_manager() else { return };
    if let Some(old) = ON.with_borrow_mut(|o| o.remove(&id)) {
        ucm.remove_script(&old);
    }
    if want {
        let s = UserScript::for_world(
            &format!("{DARK_READER}\n{ENABLE}"),
            UserContentInjectedFrames::AllFrames,
            UserScriptInjectionTime::Start,
            web::WORLD,
            &[],
            &[],
        );
        ucm.add_script(&s);
        ON.with_borrow_mut(|o| o.insert(id, s));
    }
}

/// Tab closed: forget its entry.
pub fn forget(view: &WebView) {
    ON.with_borrow_mut(|o| o.remove(&web::id(view)));
}
