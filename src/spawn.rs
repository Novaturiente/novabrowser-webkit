//! `spawn` (programs and userscripts, shared in nova-core), `fake-key` and `insert-text`.
//! WebKit has no trusted input injection from the UI process (spike W5): text goes in
//! through editing commands, other keys as page KeyboardEvents (scripts see
//! `isTrusted == false`; documented in docs/scripting.md).
use crate::keys::{self, Mods};
use crate::{web, window};
use nova_core::spawn::{self as core, Page};
use webkit6::prelude::*;

/// `spawn [-u] [-o] <cmd> [args]` (flags already parsed off).
pub fn spawn(argv: &[String], userscript: bool, output: bool, count: Option<u32>) -> Result<(), String> {
    if !userscript {
        return core::program(argv, output);
    }
    core::find_userscript(argv.first().ok_or("usage: spawn [-u] [-o] <command> [args…]")?)?;
    let (url, title) = window::active_url_title();
    let argv = argv.to_vec();
    let Some(view) = window::active_view() else {
        return core::userscript(&argv, output, count, Page { url, title, ..Page::default() });
    };
    let tab = web::id(&view) as u64;
    // selection, HTML and text are read before the script starts (qutebrowser does too)
    web::eval_world(
        &view,
        "JSON.stringify([String(getSelection()), document.documentElement.outerHTML, document.body ? document.body.innerText : ''])",
        move |r| {
            let v = r.unwrap_or_default();
            let s = |i: usize| v[i].as_str().unwrap_or("").to_string();
            let page = Page { url, title, selection: s(0), html: s(1), text: s(2), tab };
            if let Err(e) = core::userscript(&argv, output, count, page) {
                window::message("error", &format!("spawn: {e}"));
            }
        },
    );
    Ok(())
}

/// WebKit editing command for a named key in an editable, if any.
fn editing_command(name: &str, mods: Mods) -> Option<&'static str> {
    if mods.contains(Mods::CTRL) {
        return match name {
            "a" => Some("SelectAll"),
            "c" => Some("Copy"),
            "x" => Some("Cut"),
            "v" => Some("Paste"),
            "z" => Some("Undo"),
            _ => None,
        };
    }
    let shift = mods.contains(Mods::SHIFT);
    Some(match (name, shift) {
        ("Backspace", _) => "DeleteBackward",
        ("Delete", _) => "DeleteForward",
        ("Return", _) => "InsertNewline",
        ("Left", false) => "MoveLeft",
        ("Right", false) => "MoveRight",
        ("Up", false) => "MoveUp",
        ("Down", false) => "MoveDown",
        ("Left", true) => "MoveLeftAndModifySelection",
        ("Right", true) => "MoveRightAndModifySelection",
        ("Home", _) => "MoveToBeginningOfLine",
        ("End", _) => "MoveToEndOfLine",
        _ => return None,
    })
}

/// DOM `key` for a notation key name.
fn dom_key(name: &str) -> String {
    match name {
        "Return" => "Enter",
        "Space" => " ",
        "Left" => "ArrowLeft",
        "Right" => "ArrowRight",
        "Up" => "ArrowUp",
        "Down" => "ArrowDown",
        "PgUp" => "PageUp",
        "PgDown" => "PageDown",
        n => n,
    }
    .to_string()
}

/// `fake-key <keystring>`: send keys to the page (qutebrowser notation).
pub fn fake_key(keystring: &str) -> Result<(), String> {
    let view = window::active_view().ok_or("no active tab")?;
    for key in keys::parse_keystring(keystring)? {
        let k = dom_key(&key.name);
        let js = format!(
            "(() => {{ let t = document.activeElement || document.body; while (t && t.shadowRoot && t.shadowRoot.activeElement) t = t.shadowRoot.activeElement;\
             const o = {{key: {k}, bubbles: true, cancelable: true, composed: true, ctrlKey: {c}, altKey: {a}, shiftKey: {s}, metaKey: {m}}};\
             const go = t.dispatchEvent(new KeyboardEvent('keydown', o)); t.dispatchEvent(new KeyboardEvent('keyup', o));\
             if (go && {k} === 'Tab' && !o.ctrlKey) {{ const f = [...document.querySelectorAll('a[href],button,input,select,textarea,[tabindex]:not([tabindex=\"-1\"])')].filter(e => !e.disabled && e.offsetParent); const i = f.indexOf(t); const n = f[(i + ({s} ? -1 : 1) + f.length) % f.length]; n && n.focus(); }}\
             if (go && {k} === 'Enter' && t.form && t.tagName === 'INPUT') t.form.requestSubmit ? t.form.requestSubmit() : t.form.submit();\
             return go; }})()",
            k = serde_json::to_string(&k).unwrap(),
            c = key.mods.contains(Mods::CTRL),
            a = key.mods.contains(Mods::ALT),
            s = key.mods.contains(Mods::SHIFT),
            m = key.mods.contains(Mods::META),
        );
        web::eval_page(&view, &js);
        let plain = !key.mods.contains(Mods::CTRL) && !key.mods.contains(Mods::ALT) && !key.mods.contains(Mods::META);
        if plain && key.name.chars().count() == 1 {
            view.execute_editing_command_with_argument("InsertText", &key.name);
        } else if plain && key.name == "Space" {
            view.execute_editing_command_with_argument("InsertText", " ");
        } else if let Some(c) = editing_command(&key.name, key.mods) {
            if key.name != "Return" {
                view.execute_editing_command(c);
            }
        }
    }
    Ok(())
}

/// `insert-text <text>`: insert at the focused element (no key events).
pub fn insert_text(text: &str) -> Result<(), String> {
    let view = window::active_view().ok_or("no active tab")?;
    view.execute_editing_command_with_argument("InsertText", text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_editing_commands() {
        assert_eq!(editing_command("Backspace", Mods::NONE), Some("DeleteBackward"));
        assert_eq!(editing_command("a", Mods::CTRL), Some("SelectAll"));
        assert_eq!(editing_command("Left", Mods::SHIFT), Some("MoveLeftAndModifySelection"));
        assert_eq!(editing_command("F5", Mods::NONE), None);
        assert_eq!(dom_key("Return"), "Enter");
        assert_eq!(dom_key("x"), "x");
    }
}
