//! Per-site dark mode (`colors.webpage.darkmode.enabled`) with WebKit's own colour filter
//! (`-apple-color-filter: apple-invert-lightness()`, the Apple Mail dark mode): it inverts
//! the lightness of CSS colours (text, backgrounds, borders, SVG fills) at paint time and
//! leaves images alone, close to Chromium's forced dark. The `ColorFilter` feature is on in
//! every page's settings (web.rs); the property does nothing until this user style sets it.
//! W13: no measurable Speedometer cost (Dark Reader −42%, CSS invert −12% and unreadable text).
//! Pages already dark on their own (GitHub under a dark system theme) are left alone, as in
//! Chromium: a document-end script in our world marks them and the style skips marked roots.
//! It re-checks when the page switches theme later (class/style/attribute change on html or
//! body, `load`, the system colour scheme, and once at 1 s and 3 s), both ways.
//! shortcut: `darkmode.algorithm` / `policy.images` are Chromium knobs and are ignored here;
//! text baked into images and CSS background-images stay as they are; a page that is light
//! until DOMContentLoaded and dark after shows inverted for that moment.
use crate::{config, web};
use std::cell::RefCell;
use std::collections::HashMap;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserScript, UserScriptInjectionTime, UserStyleLevel, UserStyleSheet, WebView};

const CSS: &str = ":root:not([data-nova-native-dark]) { -apple-color-filter: apple-invert-lightness() !important; }";
/// Marks the root while the page paints dark itself. Background = first opaque of html, body,
/// else the outermost opaque element under the view centre (SPA wrappers like `#root`); no
/// background anywhere -> the `color-scheme` the page picks. Computed colours are unfiltered.
/// Installed once per document; running it again (live toggle) just re-checks.
const NATIVE_DARK: &str = r#"(() => {
  if (globalThis.__novaDarkCheck) return globalThis.__novaDarkCheck();
  const A = 'data-nova-native-dark';
  const lum = e => { const m = (e && getComputedStyle(e).backgroundColor || '').match(/[\d.]+/g);
    return !m || (m[3] !== undefined && +m[3] === 0) ? null : (0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]) / 255; };
  const check = () => {
    const d = document.documentElement; if (!d) return;
    let bg = [d, document.body].map(lum).find(x => x != null);
    if (bg == null) bg = document.elementsFromPoint(innerWidth / 2, innerHeight / 2).map(lum).filter(x => x != null).pop();
    const cs = getComputedStyle(d).colorScheme || '';
    const dark = bg != null ? bg < 0.4 : /dark/.test(cs) && (!/light/.test(cs) || matchMedia('(prefers-color-scheme: dark)').matches);
    if (dark !== d.hasAttribute(A)) dark ? d.setAttribute(A, '') : d.removeAttribute(A);
  };
  globalThis.__novaDarkCheck = check;
  let t = 0;
  const soon = () => { if (!t) t = setTimeout(() => { t = 0; check(); }, 50); };
  // our own attribute is skipped, else marking would retrigger the check forever
  const mo = new MutationObserver(rs => { if (rs.some(r => r.attributeName !== A)) soon(); });
  const watch = () => [document.documentElement, document.body].forEach(e => e && mo.observe(e, { attributes: true }));
  watch();
  addEventListener('load', () => { watch(); check(); });
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', check);
  setTimeout(check, 1000);
  setTimeout(check, 3000);
  check();
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
