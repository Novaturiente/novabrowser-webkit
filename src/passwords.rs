//! Password manager UI: suggestions under the focused login field, fill, save/update
//! prompts. The vault CLIs (rbw, bw, pass) live in nova_core::passwords and run on
//! worker threads. `passwords.js` runs in our isolated world of the top frame.
//! shortcut: top frame only (WebKit's evaluate_javascript can't reach a subframe's world);
//! same-site login iframes need the web process extension's WebFormManager.
use crate::{web, window};
use nova_core::host::on_ui;
use nova_core::passwords::{self as pw, Backend, Entry};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserContentManager, UserScript, UserScriptInjectionTime, WebView};

const JS: &str = include_str!("passwords.js");

/// passwords.js wired to our message handler, kept as `__novaPw` in our world.
pub fn add_scripts(ucm: &UserContentManager) {
    let src = format!(
        "globalThis.__novaPw = ({JS})((s) => webkit.messageHandlers.nova.postMessage('pw submit ' + s));\
         addEventListener('focusin', () => setTimeout(() => {{ const f = __novaPw.focus(); webkit.messageHandlers.nova.postMessage('pw focus ' + f); }}, 0), true);"
    );
    ucm.add_script(&UserScript::for_world(
        &src,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::Start,
        web::WORLD,
        &["http://*/*", "https://*/*"],
        &[],
    ));
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    User,
    Password,
    New,
    Otp,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "user" => Kind::User,
            "password" => Kind::Password,
            "new" => Kind::New,
            "otp" => Kind::Otp,
            _ => return None,
        })
    }
    fn fill_what(self) -> &'static str {
        match self {
            Kind::Otp => "otp",
            Kind::New => "new",
            _ => "login",
        }
    }
}

#[derive(Clone)]
enum Item {
    Entry(Entry),
    Generate,
    /// vault locked: picking it unlocks, then the list fills in
    Unlock,
}

struct Field {
    seq: u64,
    view: glib::WeakRef<WebView>,
    view_id: usize,
    kind: Kind,
    url: String,
    rect: (i32, i32, i32, i32),
    items: Vec<Item>,
    sel: Option<usize>,
}

use gtk::glib;

thread_local! {
    static FIELD: RefCell<Option<Field>> = const { RefCell::new(None) };
    static GEN: Cell<u64> = const { Cell::new(0) };
    /// (view, site) → username from a split login's first page
    static LAST_USER: RefCell<Vec<(usize, String, String)>> = const { RefCell::new(Vec::new()) };
    /// our own fill moves focus; don't pop the list up again right after
    static FILLED_AT: Cell<Option<Instant>> = const { Cell::new(None) };
    /// last (site, user, password) offered for saving, to ignore repeats of one submit
    static LAST_SUBMIT: RefCell<Option<(Instant, String, String, String)>> = const { RefCell::new(None) };
}

fn next_seq() -> u64 {
    GEN.set(GEN.get() + 1);
    GEN.get()
}

fn url_ok(url: &str) -> Result<(), String> {
    if url.starts_with("http://") {
        return Err("not filling a login over plain http".into());
    }
    Ok(())
}

/// `pw focus <kind x y w h>` / `pw submit <json>` from passwords.js.
pub fn on_message(view: &WebView, arg: &str) {
    let (what, rest) = arg.split_once(' ').unwrap_or((arg, ""));
    match what {
        "focus" => on_focus(view, rest),
        "submit" => on_submit(view, rest),
        _ => {}
    }
}

fn on_focus(view: &WebView, field: &str) {
    let Some(b) = pw::backend() else { return };
    let mut parts = field.split(' ');
    let Some(kind) = parts.next().and_then(Kind::parse) else {
        return hide();
    };
    let n: Vec<i32> = parts.filter_map(|p| p.parse().ok()).collect();
    let rect = match n[..] {
        [x, y, w, h] => (x, y, w, h),
        _ => (24, 0, 300, 8),
    };
    if FILLED_AT.get().is_some_and(|t| t.elapsed() < Duration::from_millis(800)) {
        return;
    }
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    if url_ok(&url).is_err() || window::active_view().is_none_or(|a| web::id(&a) != web::id(view)) {
        return hide();
    }
    let seq = next_seq();
    FIELD.set(Some(Field {
        seq,
        view: view.downgrade(),
        view_id: web::id(view),
        kind,
        url: url.clone(),
        rect,
        items: Vec::new(),
        sel: None,
    }));
    if kind == Kind::New {
        return show(seq, Some(Vec::new()));
    }
    std::thread::spawn(move || {
        let r = if !pw::locked(b) {
            pw::matching(b, &url).map(Some)
        } else if kind == Kind::Password {
            Ok(None)
        } else {
            return; // locked: the unlock row only on password fields, not every email box
        };
        on_ui(move || match r {
            Ok(v) => show(seq, v),
            Err(e) => window::message("error", &e),
        });
    });
}

/// `None` = vault locked: one "unlock" row (sign-up fields still get "generate").
fn show(seq: u64, entries: Option<Vec<Entry>>) {
    let rows = FIELD.with_borrow_mut(|f| {
        let f = f.as_mut().filter(|f| f.seq == seq)?;
        let mut items: Vec<Item> = match entries {
            Some(v) => v.into_iter().map(Item::Entry).collect(),
            None if f.kind == Kind::New => Vec::new(),
            None => vec![Item::Unlock],
        };
        if f.kind == Kind::New {
            items.push(Item::Generate);
        }
        // never preselected: Enter in a field must keep submitting the form
        f.sel = None;
        f.items = items;
        Some(rows(f))
    });
    match rows {
        Some(r) if r.1["rows"].as_array().is_some_and(|a| !a.is_empty()) => window::suggest(r.0, &r.1),
        _ => hide(),
    }
}

fn rows(f: &Field) -> ((i32, i32, i32, i32), Value) {
    let b = pw::backend().map(pw::label).unwrap_or("");
    let rows: Vec<Value> = f
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let (a, c) = match it {
                Item::Entry(e) if f.kind == Kind::Otp => (format!("{} code", e.name), e.user.clone()),
                Item::Entry(e) => (
                    if e.user.is_empty() { e.name.clone() } else { e.user.clone() },
                    e.name.clone(),
                ),
                Item::Generate => ("Generate a strong password".into(), format!("saved to {b} after you submit")),
                Item::Unlock => (format!("{b} is locked"), "Down, Enter: unlock".into()),
            };
            serde_json::json!({ "a": a, "b": c, "sel": f.sel == Some(i) })
        })
        .collect();
    (f.rect, serde_json::json!({ "title": b, "rows": rows }))
}

/// Safe from any signal handler: the overlay is hidden from an idle callback.
pub fn hide() {
    if visible() {
        glib::idle_add_local_once(window::suggest_hide);
    }
    FIELD.with_borrow_mut(|f| {
        if let Some(f) = f {
            f.items.clear();
        }
    });
}

fn visible() -> bool {
    FIELD.with_borrow(|f| f.as_ref().is_some_and(|f| !f.items.is_empty()))
}

/// Keys while the list is shown: Down/Up select, Enter fills, Escape hides.
/// Returns true when the key was ours.
pub fn on_key(name: &str) -> bool {
    if !visible() {
        return false;
    }
    let pick = FIELD.with_borrow_mut(|f| {
        let f = f.as_mut()?;
        let n = f.items.len();
        match name {
            "Down" => f.sel = Some(f.sel.map_or(0, |s| (s + 1) % n)),
            "Up" => f.sel = Some(f.sel.map_or(n - 1, |s| (s + n - 1) % n)),
            "Return" | "Enter" if f.sel.is_some() => return Some(f.sel),
            "Escape" => return Some(None),
            _ => return None,
        }
        let (r, j) = rows(f);
        window::suggest(r, &j);
        Some(None)
    });
    match (name, pick) {
        (_, Some(Some(i))) => pick_item(i),
        ("Escape", _) => hide(),
        ("Down" | "Up", _) => {}
        _ => return false,
    }
    true
}

/// A click on a row in the popup.
pub fn on_click(i: usize) {
    pick_item(i);
}

fn pick_item(i: usize) {
    let Some(b) = pw::backend() else { return };
    let Some((item, kind, seq, url)) = FIELD.with_borrow(|f| {
        f.as_ref().and_then(|f| Some((f.items.get(i)?.clone(), f.kind, f.seq, f.url.clone())))
    }) else {
        return;
    };
    hide();
    match item {
        Item::Generate => fill(seq, String::new(), pw::generate(), "new"),
        Item::Unlock => {
            std::thread::spawn(move || {
                let r = pw::unlock(b).and_then(|_| pw::matching(b, &url));
                on_ui(move || match r {
                    Ok(v) => {
                        let any = !v.is_empty();
                        show(seq, Some(v));
                        if any {
                            select_first(seq);
                        } else {
                            window::message("info", &format!("unlocked; no {} login for this site", pw::label(b)));
                        }
                    }
                    Err(e) => window::message("error", &e),
                });
            });
        }
        Item::Entry(e) => {
            std::thread::spawn(move || {
                let r = if kind == Kind::Otp {
                    pw::totp(b, &e).map(|c| (String::new(), c))
                } else {
                    pw::secret(b, &e)
                };
                on_ui(move || match r {
                    Ok((u, p)) => fill(seq, u, p, kind.fill_what()),
                    Err(e) => window::message("error", &e),
                });
            });
        }
    }
}

fn select_first(seq: u64) {
    FIELD.with_borrow_mut(|f| {
        if let Some(f) = f.as_mut().filter(|f| f.seq == seq && !f.items.is_empty()) {
            f.sel = Some(0);
            let (r, j) = rows(f);
            window::suggest(r, &j);
        }
    });
}

/// Send the secret to the field's page, if it is still the same site.
fn fill(seq: u64, user: String, pass: String, what: &str) {
    let Some((view, url)) = FIELD.with_borrow(|f| {
        f.as_ref().filter(|f| f.seq == seq).and_then(|f| Some((f.view.upgrade()?, f.url.clone())))
    }) else {
        return;
    };
    let now = view.uri().map(|u| u.to_string()).unwrap_or_default();
    if pw::site(&now) != pw::site(&url) {
        return window::message("warning", "page changed, nothing filled");
    }
    if let Err(e) = url_ok(&now) {
        return window::message("error", &e);
    }
    FILLED_AT.set(Some(Instant::now()));
    let js = format!(
        "globalThis.__novaPw && __novaPw.fill({}, {}, {})",
        serde_json::to_string(&user).unwrap(),
        serde_json::to_string(&pass).unwrap(),
        serde_json::to_string(what).unwrap()
    );
    web::eval_quiet(&view, &js);
}

/// `:password-fill [--otp]` (`Sp`): fill the best match into the page's login form.
/// Several matches → the list to pick from.
pub fn fill_command(otp: bool) -> Result<(), String> {
    let b = pw::backend().ok_or("passwords.backend is none")?;
    let view = window::active_view().ok_or("no tab")?;
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    url_ok(&url)?;
    let id = web::id(&view);
    let rect = FIELD
        .with_borrow(|f| f.as_ref().filter(|f| f.view_id == id).map(|f| f.rect))
        .unwrap_or((24, 0, 300, 8));
    let kind = if otp { Kind::Otp } else { Kind::Password };
    let seq = next_seq();
    FIELD.set(Some(Field {
        seq,
        view: view.downgrade(),
        view_id: id,
        kind,
        url: url.clone(),
        rect,
        items: Vec::new(),
        sel: None,
    }));
    std::thread::spawn(move || {
        // an explicit key press: unlocking (rbw's pinentry) is fine here
        let r = if pw::locked(b) { pw::unlock(b) } else { Ok(()) }.and_then(|_| pw::matching(b, &url));
        on_ui(move || match r {
            Ok(v) if v.is_empty() => {
                window::message("info", &format!("no {} login for {}", pw::label(b), pw::host(&url)))
            }
            Ok(v) if v.len() == 1 => {
                FIELD.with_borrow_mut(|f| {
                    if let Some(f) = f.as_mut().filter(|f| f.seq == seq) {
                        f.items = vec![Item::Entry(v[0].clone())];
                    }
                });
                pick_item(0);
            }
            Ok(v) => {
                show(seq, Some(v));
                select_first(seq);
            }
            Err(e) => window::message("error", &e),
        });
    });
    Ok(())
}

/// A form with a password was submitted: offer to save or update.
/// `json` = {"user": "...", "pass": "..."}; no "pass" = first page of a split login.
fn on_submit(view: &WebView, json: &str) {
    let Some(b) = pw::backend() else { return };
    let Ok(v) = serde_json::from_str::<Value>(json) else { return };
    let url = view.uri().map(|u| u.to_string()).unwrap_or_default();
    let Some(st) = pw::site(&url) else { return };
    let id = web::id(view);
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    let (mut user, pass) = (s("user"), s("pass"));
    if pass.is_empty() {
        if !user.is_empty() {
            LAST_USER.with_borrow_mut(|l| {
                l.retain(|(b, s, _)| !(*b == id && *s == st));
                l.push((id, st, user));
            });
        }
        return;
    }
    if user.is_empty() {
        user = LAST_USER
            .with_borrow(|l| l.iter().rev().find(|(b, s, _)| *b == id && *s == st).map(|x| x.2.clone()))
            .unwrap_or_default();
    }
    let repeat = LAST_SUBMIT.with_borrow(|l| {
        l.as_ref().is_some_and(|(t, s, u, p)| {
            t.elapsed() < Duration::from_secs(30) && *s == st && *u == user && *p == pass
        })
    });
    if repeat {
        return;
    }
    LAST_SUBMIT.set(Some((Instant::now(), st, user.clone(), pass.clone())));
    // GObjects can't cross threads: the worker comes back with the view's id
    std::thread::spawn(move || {
        let locked = pw::locked(b);
        let r = if locked { Ok(Some(None)) } else { pw::known(b, &url, &user, &pass) };
        on_ui(move || match (r, view_by_id(id)) {
            (Ok(Some(existing)), Some(v)) => ask(&v, b, &url, user, pass, existing, locked),
            (Err(e), _) => window::message("error", &e),
            _ => {}
        });
    });
}

/// Save/update question. `locked`: we can't tell save from update without unlocking,
/// and don't pop the unlock dialog unasked; yes = unlock, then decide.
fn ask(view: &WebView, b: Backend, url: &str, user: String, pass: String, existing: Option<Entry>, locked: bool) {
    let h = pw::host(url);
    let who = if user.is_empty() { String::new() } else { format!(" for {user}") };
    let text = match (&existing, locked) {
        (_, true) => format!("Save this login{who} on {h} to {}? (unlocks the vault) [y]es [n]o", pw::label(b)),
        (Some(_), _) => format!("Update the {} password{who} on {h}? [y]es [n]o", pw::label(b)),
        (None, _) => format!("Save this login{who} on {h} to {}? [y]es [n]o", pw::label(b)),
    };
    let url = url.to_string();
    crate::permissions::ask(view, text, move |yes| {
        if !yes {
            return;
        }
        let (url, user, pass, existing) = (url.clone(), user.clone(), pass.clone(), existing.clone());
        std::thread::spawn(move || {
            let r = if locked {
                pw::unlock(b).and_then(|_| match pw::known(b, &url, &user, &pass)? {
                    None => Ok("already saved"),
                    Some(Some(e)) => pw::update(b, &e, &pass).map(|_| "updated"),
                    Some(None) => pw::save(b, &url, &user, &pass).map(|_| "saved"),
                })
            } else {
                match &existing {
                    Some(e) => pw::update(b, e, &pass).map(|_| "updated"),
                    None => pw::save(b, &url, &user, &pass).map(|_| "saved"),
                }
            };
            on_ui(move || match r {
                Ok(what) => window::message("info", &format!("login {what} in {}", pw::label(b))),
                Err(e) => window::message("error", &e),
            });
        });
    });
}

fn view_by_id(id: usize) -> Option<WebView> {
    window::all_views().into_iter().find(|v| web::id(v) == id)
}

/// Tab closed: forget its field and split-login usernames.
pub fn forget(view: &WebView) {
    let id = web::id(view);
    if FIELD.with_borrow(|f| f.as_ref().is_some_and(|f| f.view_id == id)) {
        hide();
        FIELD.set(None);
    }
    LAST_USER.with_borrow_mut(|l| l.retain(|(b, ..)| *b != id));
}
