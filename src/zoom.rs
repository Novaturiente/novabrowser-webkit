//! Per-host zoom, like Chromium: `zoom` / `zoom-in` / `zoom-out` set it for the page's
//! host, every tab on that host follows, and it persists in `<data>/zoom.json`.
use crate::{commands, window, xdg_dir};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::path::PathBuf;
use webkit6::prelude::*;
use webkit6::WebView;

thread_local! {
    static LEVELS: RefCell<Option<Map<String, Value>>> = const { RefCell::new(None) };
}

fn file() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", ".local/share").join("zoom.json")
}

fn with<R>(f: impl FnOnce(&mut Map<String, Value>) -> R) -> R {
    LEVELS.with_borrow_mut(|l| {
        f(l.get_or_insert_with(|| {
            std::fs::read_to_string(file())
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default()
        }))
    })
}

fn host_of(v: &WebView) -> String {
    commands::host(&v.uri().unwrap_or_default()).to_string()
}

/// Set `pct` for `v`'s host (private windows: that view only, nothing saved).
pub fn set(v: &WebView, pct: f64) {
    let pct = pct.clamp(10.0, 1000.0);
    let host = host_of(v);
    if host.is_empty() || window::is_private(v) {
        v.set_zoom_level(pct / 100.0);
        return;
    }
    let text = with(|m| {
        if (pct - 100.0).abs() < 0.01 {
            m.remove(&host);
        } else {
            m.insert(host.clone(), Value::from(pct));
        }
        serde_json::to_string(m).unwrap()
    });
    if let Err(e) = crate::write_atomic(&file(), text.as_bytes()) {
        window::message("error", &format!("{}: {e}", file().display()));
    }
    for t in window::all_views() {
        if host_of(&t) == host && !window::is_private(&t) {
            t.set_zoom_level(pct / 100.0);
        }
    }
}

/// A page committed: take its host's zoom.
pub fn apply(v: &WebView) {
    let host = host_of(v);
    let pct = with(|m| m.get(&host).and_then(Value::as_f64)).unwrap_or(100.0);
    if (v.zoom_level() * 100.0 - pct).abs() > 0.01 && !window::is_private(v) {
        v.set_zoom_level(pct / 100.0);
    }
}
