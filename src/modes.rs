//! Modes and key dispatch: a capture-phase key controller on each window sees every
//! key before the focused web view (PLAN 2.1).
use crate::keys::{self, Key, Keymap, Mods, Outcome, Parser};
use crate::{commands, config, hints, window};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Passthrough,
    Command,
    Hint,
    Prompt,
}

struct State {
    mode: Mode,
    parser: Parser,
    normal: Keymap,
    insert: Keymap,
    passthrough: Keymap,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State {
        mode: Mode::Normal,
        parser: Parser::default(),
        normal: Keymap::new(),
        insert: Keymap::new(),
        passthrough: Keymap::new(),
    });
}

/// (Re)build the keymaps: `bindings.default`, then `bindings.commands`, then
/// config.ts bind()/unbind() in order. Bad key strings are reported, not fatal.
pub fn init() {
    let cfg = config::get();
    let binds = nova_core::bindings::all(&cfg);
    S.with_borrow_mut(|s| {
        s.parser.clear();
        s.normal = Keymap::new();
        s.insert = Keymap::new();
        s.passthrough = Keymap::new();
        for (mode, keystr, cmd, _) in &binds {
            let map = match mode.as_str() {
                "normal" => &mut s.normal,
                "insert" => &mut s.insert,
                "passthrough" => &mut s.passthrough,
                _ => continue,
            };
            match (keys::parse_keystring(keystr), cmd) {
                (Ok(seq), Some(cmd)) => map.bind(&seq, cmd),
                (Ok(seq), None) => {
                    map.unbind(&seq);
                }
                (Err(e), _) => eprintln!("novabrowser-webkit: bad binding {keystr:?}: {e}"),
            }
        }
    });
}

pub fn mode() -> Mode {
    S.with_borrow(|s| s.mode)
}

/// Which-key rows for the pending keychain in normal mode: (prefix, [(rest, command)]).
pub fn keyhint() -> (String, Vec<(String, String)>) {
    S.with_borrow(|s| {
        let seq = s.parser.sequence();
        if s.mode != Mode::Normal || seq.is_empty() {
            return (String::new(), vec![]);
        }
        (
            seq.iter().map(|k| k.to_string()).collect(),
            s.normal.continuations(seq),
        )
    })
}

pub fn pending() -> String {
    S.with_borrow(|s| s.parser.pending())
}

pub fn enter(m: Mode) {
    let old = S.with_borrow_mut(|s| {
        s.parser.clear();
        std::mem::replace(&mut s.mode, m)
    });
    if old == Mode::Hint && m != Mode::Hint {
        hints::on_leave();
    }
    if old != m {
        window::refresh();
    }
}

pub fn leave() {
    if mode() == Mode::Command {
        window::close_cmdline();
    } else {
        enter(Mode::Normal);
    }
}

pub fn clear_keychain() {
    S.with_borrow_mut(|s| s.parser.clear());
    window::refresh();
}

/// Focus moved inside the active tab (our focus script).
pub fn on_focus(editable: bool) {
    let cfg = config::get();
    match (mode(), editable) {
        (Mode::Normal, true) if cfg.bool("input.insert_mode.auto_enter") => enter(Mode::Insert),
        (Mode::Insert, false) if cfg.bool("input.insert_mode.auto_leave") => enter(Mode::Normal),
        _ => {}
    }
}

/// Capture-phase key handling for window `slot`.
pub fn attach_keys(win: &gtk::ApplicationWindow, slot: usize) {
    let c = gtk::EventControllerKey::new();
    c.set_propagation_phase(gtk::PropagationPhase::Capture);
    c.connect_key_pressed(move |_, keyval, _code, state| {
        let Some(key) = key_from_event(keyval, state) else {
            return glib::Propagation::Proceed;
        };
        let t = std::time::Instant::now();
        let eaten = window::in_slot(slot, || on_key(key));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms > 2.0 && std::env::var_os("NOVA_KEY_TIMING").is_some() {
            eprintln!("novabrowser-webkit: key took {ms:.2} ms");
        }
        if eaten {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    win.add_controller(c);
}

pub fn key_from_event(keyval: gdk::Key, state: gdk::ModifierType) -> Option<Key> {
    let mut mods = Mods::NONE;
    for (bit, m) in [
        (gdk::ModifierType::SHIFT_MASK, Mods::SHIFT),
        (gdk::ModifierType::CONTROL_MASK, Mods::CTRL),
        (gdk::ModifierType::ALT_MASK, Mods::ALT),
        (gdk::ModifierType::SUPER_MASK, Mods::META),
        (gdk::ModifierType::META_MASK, Mods::META),
    ] {
        if state.contains(bit) {
            mods |= m;
        }
    }
    let name = keyval.name().map(|n| n.to_string()).unwrap_or_default();
    keys::key_from_gdk(&name, keyval.to_unicode(), mods)
}

enum Act {
    Pass,
    Eat,
    Run(String, Option<u32>),
    Hint(String),
    Prompt(String),
}

/// A key press in a window. Returns true to consume it (the page sees nothing).
pub fn on_key(key: Key) -> bool {
    // password suggestions open under a login field: Down/Up/Enter/Escape are theirs
    if key.mods == Mods::default() && crate::passwords::on_key(&key.name) {
        return true;
    }
    let act = S.with_borrow_mut(|s| match s.mode {
        Mode::Normal => match s.parser.handle(&key, &s.normal, true) {
            Outcome::Pending => Act::Eat,
            Outcome::Run { command, count } => Act::Run(command, count),
            // input.forward_unbound_keys = auto: unbound plain characters are eaten
            Outcome::NoMatch if is_plain_char(&key) => Act::Eat,
            Outcome::NoMatch => Act::Pass,
        },
        // only single-key bindings here, so typing "jk" in a form never triggers anything
        Mode::Insert => single(&s.insert, &key),
        Mode::Passthrough => single(&s.passthrough, &key),
        // the palette edits its own text and reports back
        Mode::Command => Act::Pass,
        Mode::Prompt => Act::Prompt(key.name.clone()),
        Mode::Hint => Act::Hint(key.name.clone()),
    });
    match act {
        Act::Pass => {
            window::refresh();
            false
        }
        Act::Eat => {
            window::refresh();
            true
        }
        Act::Hint(name) if name == "Escape" => {
            enter(Mode::Normal);
            true
        }
        Act::Hint(name) => {
            hints::on_key(&name);
            true
        }
        Act::Prompt(name) => {
            crate::permissions::on_key(&name);
            true
        }
        Act::Run(cmd, count) => {
            // the crashed page's `r` reloads it
            commands::run(&cmd, count);
            window::refresh();
            true
        }
    }
}

fn single(map: &Keymap, key: &Key) -> Act {
    match map.get(std::slice::from_ref(key)) {
        Some(cmd) => Act::Run(cmd.to_string(), None),
        None => Act::Pass,
    }
}

fn is_plain_char(key: &Key) -> bool {
    key.name.chars().count() == 1
        && !key.mods.contains(Mods::CTRL)
        && !key.mods.contains(Mods::ALT)
        && !key.mods.contains(Mods::META)
}

/// For the status bar colour (`colors.statusbar.<mode>`).
pub fn mode_name() -> &'static str {
    match mode() {
        Mode::Normal | Mode::Hint | Mode::Prompt => "normal",
        Mode::Insert => "insert",
        Mode::Passthrough => "passthrough",
        Mode::Command => "command",
    }
}

pub fn mode_label() -> &'static str {
    match mode() {
        Mode::Normal => "",
        Mode::Insert => "-- INSERT MODE --",
        Mode::Passthrough => "-- PASSTHROUGH MODE --",
        Mode::Command => "",
        Mode::Hint => "-- HINT MODE --",
        Mode::Prompt => "-- PROMPT --",
    }
}
