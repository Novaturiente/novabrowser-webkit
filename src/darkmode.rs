//! Per-site dark mode (`colors.webpage.darkmode.enabled`) with WebKit's own colour filter
//! (`-apple-color-filter: apple-invert-lightness()`, the Apple Mail dark mode): it inverts
//! the lightness of CSS colours (text, backgrounds, borders, SVG fills) at paint time and
//! leaves images alone, close to Chromium's forced dark. The `ColorFilter` feature is on in
//! every page's settings (web.rs); the property does nothing until this user style sets it.
//! W13: no measurable Speedometer cost (Dark Reader −42%, CSS invert −12% and unreadable text).
//! Pages already dark on their own (GitHub under a dark system theme) are left alone, as in
//! Chromium: a document-end script in our world marks them and the style skips marked roots.
//! shortcut: `darkmode.algorithm` / `policy.images` are Chromium knobs and are ignored here;
//! text baked into images and CSS background-images stay as they are; a page that turns
//! dark only after DOMContentLoaded gets inverted back to light.
use crate::{config, web};
use std::cell::RefCell;
use std::collections::HashMap;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserScript, UserScriptInjectionTime, UserStyleLevel, UserStyleSheet, WebView};

const CSS: &str = ":root:not([data-nova-native-dark]) { -apple-color-filter: apple-invert-lightness() !important; }";
/// Marks the root when the page paints dark itself: its root/body background is dark, or it has
/// no background and its `color-scheme` picks dark. Computed colours are unfiltered.
const NATIVE_DARK: &str = r#"(() => {
  const lum = c => { const m = (c || '').match(/[\d.]+/g);
    return !m || (m[3] !== undefined && +m[3] === 0) ? null : (0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]) / 255; };
  const d = document.documentElement; if (!d) return;
  const bg = [d, document.body].map(e => e && lum(getComputedStyle(e).backgroundColor)).find(x => x != null);
  const cs = getComputedStyle(d).colorScheme || '';
  const dark = bg != null ? bg < 0.4 : /dark/.test(cs) && (!/light/.test(cs) || matchMedia('(prefers-color-scheme: dark)').matches);
  if (dark) d.setAttribute('data-nova-native-dark', '');
})()"#;

thread_local! {
    /// view id -> its dark style sheet and native-dark check, while the tab is dark.
    static ON: RefCell<HashMap<usize, (UserStyleSheet, UserScript)>> = RefCell::new(HashMap::new());
}

fn dark_at(url: &str) -> bool {
    url.starts_with("http")
        && config::get()
            .get_for("colors.webpage.darkmode.enabled", url)
            .as_bool()
            .unwrap_or(false)
}

/// The current page after a load commit, a config change or `dark-toggle`.
pub fn apply(view: &WebView) {
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    apply_for(view, &url);
}

/// Make the view dark iff `url` should be. Adding or removing a user style sheet also
/// restyles the page already shown, so this switches live.
pub fn apply_for(view: &WebView, url: &str) {
    let id = web::id(view);
    let want = dark_at(url);
    if want == ON.with_borrow(|o| o.contains_key(&id)) {
        return;
    }
    let Some(ucm) = view.user_content_manager() else { return };
    if let Some((css, js)) = ON.with_borrow_mut(|o| o.remove(&id)) {
        ucm.remove_style_sheet(&css);
        ucm.remove_script(&js);
    }
    if want {
        let css = UserStyleSheet::new(CSS, UserContentInjectedFrames::AllFrames, UserStyleLevel::User, &[], &[]);
        let js = UserScript::for_world(NATIVE_DARK, UserContentInjectedFrames::AllFrames, UserScriptInjectionTime::End, web::WORLD, &[], &[]);
        ucm.add_style_sheet(&css);
        ucm.add_script(&js);
        // already-loaded page (live toggle): user scripts only run on the next load
        web::eval_quiet(view, NATIVE_DARK);
        ON.with_borrow_mut(|o| o.insert(id, (css, js)));
    }
}

/// Tab closed: forget its entry.
pub fn forget(view: &WebView) {
    ON.with_borrow_mut(|o| o.remove(&web::id(view)));
}
