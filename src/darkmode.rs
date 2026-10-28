//! Per-site dark mode (`colors.webpage.darkmode.*`). WebKit has no Blink-style forced
//! dark: an inverting user style sheet in every frame, with media re-inverted so photos
//! and video keep their colours (`policy.images = never` inverts them too).
//! shortcut: a CSS filter, not Dark Reader (D6); swap in Dark Reader once it is vendored and W13 measures it.
use crate::config;
use std::cell::RefCell;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserStyleLevel, UserStyleSheet, WebView};

thread_local! {
    /// (view id, css applied) so a reload doesn't stack sheets.
    static ON: RefCell<Vec<(usize, String)>> = const { RefCell::new(Vec::new()) };
}

/// The sheet for the current settings at `url`, None = dark mode off there.
fn css_for(url: &str) -> Option<String> {
    let cfg = config::get();
    if !cfg
        .get_for("colors.webpage.darkmode.enabled", url)
        .as_bool()
        .unwrap_or(false)
    {
        return None;
    }
    // hue-rotate keeps hues after the invert; brightness-rgb is the plain invert
    let base = match cfg.str("colors.webpage.darkmode.algorithm").as_str() {
        "brightness-rgb" => "invert(1)",
        _ => "invert(0.9) hue-rotate(180deg)",
    };
    let media = match cfg.str("colors.webpage.darkmode.policy.images").as_str() {
        "always" | "never" => "",
        // smart: re-invert pictures, keep icons/SVG inverted like the text around them
        _ => "img:not([src$='.svg']), picture, video, canvas, iframe, embed, object, [style*='background-image'] { filter: invert(1) hue-rotate(180deg) !important; }",
    };
    Some(format!(
        ":root {{ filter: {base} !important; background: #fff !important; }} {media}"
    ))
}

/// Add, change or drop the sheet on `view` for its current URL.
pub fn apply(view: &WebView) {
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    apply_for(view, &url);
}

/// Same for `url` about to load (decide-policy: set before the page paints).
pub fn apply_for(view: &WebView, url: &str) {
    if crate::web::is_internal(url) {
        return;
    }
    let want = css_for(url);
    let id = crate::web::id(view);
    let have = ON.with_borrow(|o| o.iter().find(|e| e.0 == id).map(|e| e.1.clone()));
    if want == have {
        return;
    }
    let Some(ucm) = view.user_content_manager() else { return };
    ucm.remove_all_style_sheets();
    ON.with_borrow_mut(|o| o.retain(|e| e.0 != id));
    if let Some(css) = want {
        ucm.add_style_sheet(&UserStyleSheet::new(
            &css,
            UserContentInjectedFrames::AllFrames,
            UserStyleLevel::User,
            &[],
            &[],
        ));
        ON.with_borrow_mut(|o| o.push((id, css)));
    }
}

/// Tab closed: forget its entry.
pub fn forget(view: &WebView) {
    let id = crate::web::id(view);
    ON.with_borrow_mut(|o| o.retain(|e| e.0 != id));
}
