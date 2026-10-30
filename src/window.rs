//! Windows, the tab list, the HTML tab bar (mode, keys, messages, progress) and the
//! overlays (palette, which-key, tooltip/menu). All state lives on the GTK main thread.
//! Web views are created lazily (first show) and live in a GtkStack per window.
use crate::modes::{self, Mode};
use crate::theme::{self, TitleFields};
use crate::{config, history, tabview, web};
use gtk::prelude::*;
use gtk::glib;
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;
use webkit6::WebView;

pub use nova_core::host::js_str;

pub struct Tab {
    /// None: not loaded yet (lazy restore) or hibernated.
    pub view: Option<WebView>,
    pub url: String,
    pub title: String,
    pub progress: f64,
    pub loading: bool,
    /// Set while the crashed page shows; holds the URL `r` reloads.
    pub crashed: Option<String>,
    pub can_back: bool,
    pub can_fwd: bool,
    pub muted: bool,
    pub playing: bool,
    /// Serialized WebKit session state (back/forward + scroll) to restore on first show.
    pub state: Option<glib::Bytes>,
    pub pinned: bool,
    pub seen: std::time::Instant,
    pub keep: bool,
    pub asleep: bool,
    pub typed: bool,
    pub fresh: bool,
    pub icon: String,
    /// Stable id for the tab bar's element (animated open/close/move).
    pub key: u64,
}

#[derive(Default)]
pub struct State {
    pub window: Option<gtk::ApplicationWindow>,
    outer: Option<gtk::Box>,
    pub stack: Option<gtk::Stack>,
    content: Option<gtk::Overlay>,
    root: Option<gtk::Overlay>,
    pub tabbar: Option<WebView>,
    palette: Option<WebView>,
    /// the page, live and blurred, under the palette: a Box holding a Picture (CSS `.nova-backdrop`)
    backdrop: Option<gtk::Box>,
    keyhint: Option<WebView>,
    tip: Option<WebView>,
    menu: Option<WebView>,
    /// the open question (permissions.rs queue), spotlight-style under the tab bar
    ask: Option<WebView>,
    /// messages and the find count, a pill at the bottom centre (ui/toast.html)
    toast: Option<WebView>,
    keyhint_gen: u64,
    compact: bool,
    pub tabs: Vec<Tab>,
    pub active: usize,
    last: usize,
    closed: Vec<String>,
    message: Option<(String, String)>,
    prompt: Option<String>,
    message_id: u64,
    start_url: Option<String>,
    startup: Option<(String, String)>,
    claimed: bool,
    restore: Option<Value>,
    /// Private window: tabs on the ephemeral session, nothing saved.
    pub private: bool,
}

static BACKDROP_CSS: std::sync::Once = std::sync::Once::new();

thread_local! {
    static WINS: RefCell<Vec<State>> = const { RefCell::new(Vec::new()) };
    static CUR: Cell<usize> = const { Cell::new(0) };
    static STARTED: Cell<bool> = const { Cell::new(false) };
    static QUITTING: Cell<bool> = const { Cell::new(false) };
    static APP: RefCell<Option<gtk::Application>> = const { RefCell::new(None) };
    static TYPED_NEXT: Cell<bool> = const { Cell::new(false) };
    /// Tab a userscript's FIFO line runs in (see run_in_tab); None = the active tab.
    static TARGET: Cell<Option<usize>> = const { Cell::new(None) };
}

pub fn quitting() -> bool {
    QUITTING.get()
}

pub struct Cur;
pub static S: Cur = Cur;
impl Cur {
    pub fn with_borrow<R>(&self, f: impl FnOnce(&State) -> R) -> R {
        ensure_slot();
        WINS.with_borrow(|w| f(&w[CUR.get()]))
    }
    pub fn with_borrow_mut<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        ensure_slot();
        WINS.with_borrow_mut(|w| f(&mut w[CUR.get()]))
    }
}

fn ensure_slot() {
    WINS.with_borrow_mut(|w| {
        if w.is_empty() {
            w.push(State::default());
        }
    });
}

fn vid(v: &Option<WebView>) -> Option<usize> {
    v.as_ref().map(web::id)
}

/// Slot of the window owning view `id` (a tab or one of its chrome views).
fn owner(id: usize) -> Option<usize> {
    WINS.with_borrow(|w| {
        w.iter().position(|s| {
            s.tabs.iter().any(|t| vid(&t.view) == Some(id))
                || [&s.tabbar, &s.palette, &s.keyhint, &s.tip, &s.menu, &s.ask, &s.toast]
                    .iter()
                    .any(|v| vid(v) == Some(id))
        })
    })
}

pub fn in_slot<R>(slot: usize, f: impl FnOnce() -> R) -> R {
    let prev = CUR.replace(slot);
    let r = f();
    if WINS.with_borrow(|w| w.get(prev).is_some_and(|s| s.window.is_some())) {
        CUR.set(prev);
    }
    r
}

/// Run `f` acting on the window that owns `view` (events from background windows).
pub fn in_owner<R>(view: &WebView, f: impl FnOnce() -> R) -> R {
    match owner(web::id(view)) {
        Some(slot) => in_slot(slot, f),
        None => f(),
    }
}

pub fn set_app(app: &gtk::Application) {
    APP.set(Some(app.clone()));
}

pub fn create(start_url: Option<String>) {
    create_window(start_url, None, false);
}

fn create_window(start_url: Option<String>, restore: Option<Value>, private: bool) {
    let slot = WINS.with_borrow_mut(|w| {
        let slot = match w.iter().position(|s| !s.claimed) {
            Some(i) => i,
            None => {
                w.push(State::default());
                w.len() - 1
            }
        };
        let s = &mut w[slot];
        s.claimed = true;
        s.start_url = start_url;
        s.restore = restore;
        s.private = private;
        slot
    });
    in_slot(slot, || build_window(slot));
}

/// `open -w [url]`.
pub fn open_window(url: Option<String>) {
    create_window(
        Some(url.unwrap_or_else(|| config::get().str("url.default_page"))),
        None,
        false,
    );
}

/// `open -p [url]`: a private window.
pub fn open_private(url: String) {
    create_window(Some(url), None, true);
}

fn build_window(slot: usize) {
    let app = APP.with_borrow(|a| a.clone()).expect("app");
    let window = gtk::ApplicationWindow::builder()
        .application(&app)
        .default_width(1200)
        .default_height(800)
        .title("novabrowser")
        .build();
    let root = gtk::Overlay::new();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let content = gtk::Overlay::new();
    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::None);
    content.set_child(Some(&stack));
    content.set_hexpand(true);
    content.set_vexpand(true);
    root.set_child(Some(&outer));
    window.set_child(Some(&root));
    for o in [&root, &content] {
        o.connect_get_child_position(fixed_overlay_position);
    }
    let ui = |name: &str| {
        web::chrome_view(name, move |text| {
            in_slot(slot, || ui_message(text));
        })
    };
    let tabbar = ui("tabbar");
    let palette = ui("palette");
    let keyhint = ui("keyhint");
    let tip = ui("popup");
    let menu = ui("popup");
    let ask = ui("prompt");
    let toast = ui("toast");
    let backdrop = gtk::Picture::new();
    backdrop.add_css_class("nova-backdrop");
    backdrop.set_content_fit(gtk::ContentFit::Fill);
    backdrop.set_hexpand(true);
    // on black: the blur's soft edges would show the sharp page under them
    let backdrop_bg = gtk::Box::new(gtk::Orientation::Vertical, 0);
    backdrop_bg.add_css_class("nova-backdrop-bg");
    backdrop_bg.append(&backdrop);
    backdrop_bg.set_can_target(false);
    backdrop_bg.set_visible(false);
    content.add_overlay(&backdrop_bg); // added first: under the palette
    BACKDROP_CSS.call_once(|| {
        let css = gtk::CssProvider::new();
        css.load_from_string(
            ".nova-backdrop { filter: blur(10px) brightness(.45); } .nova-backdrop-bg { background: black; }",
        );
        gtk::style_context_add_provider_for_display(
            &WidgetExt::display(&window),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    for (v, parent) in [(&palette, &content), (&keyhint, &content), (&ask, &content), (&toast, &content), (&tip, &root), (&menu, &root)] {
        v.set_visible(false);
        parent.add_overlay(v);
    }
    // the tooltip and suggestions never take focus or clicks away from the page
    tip.set_can_target(false);
    tip.set_can_focus(false);
    keyhint.set_can_target(false);
    keyhint.set_can_focus(false);
    keyhint.set_halign(gtk::Align::End);
    keyhint.set_valign(gtk::Align::End);
    keyhint.set_margin_end(12);
    keyhint.set_margin_bottom(12);
    // keys answer through prompt mode, so the page keeps focus; clicks still reach the buttons
    ask.set_can_focus(false);
    ask.set_halign(gtk::Align::Center);
    ask.set_valign(gtk::Align::Start);
    ask.set_margin_top(12);
    toast.set_can_target(false);
    toast.set_can_focus(false);
    toast.set_halign(gtk::Align::Center);
    toast.set_valign(gtk::Align::End);
    toast.set_margin_bottom(18);
    ask.set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    palette.set_halign(gtk::Align::Fill);
    for v in [&tip, &menu] {
        v.set_halign(gtk::Align::Start);
        v.set_valign(gtk::Align::Start);
    }
    crate::modes::attach_keys(&window, slot);
    window.connect_is_active_notify(move |w| {
        if w.is_active() {
            CUR.set(slot);
        }
    });
    window.connect_default_width_notify(move |_| {
        glib::idle_add_local_once(move || in_slot(slot, apply_layout));
    });
    window.connect_close_request(move |_| {
        in_slot(slot, window_closing);
        glib::Propagation::Proceed
    });
    let (start, startup, private) = S.with_borrow_mut(|s| {
        s.window = Some(window.clone());
        s.outer = Some(outer);
        s.stack = Some(stack);
        s.content = Some(content);
        s.root = Some(root);
        s.tabbar = Some(tabbar);
        s.palette = Some(palette);
        s.backdrop = Some(backdrop_bg);
        s.keyhint = Some(keyhint);
        s.tip = Some(tip);
        s.menu = Some(menu);
        s.ask = Some(ask);
        s.toast = Some(toast);
        (s.start_url.clone(), s.startup.take(), s.private)
    });
    apply_layout();
    window.present();
    // once the first page is up: draw the palette and its blur once, unseen (anim::warm)
    glib::timeout_add_local_once(Duration::from_millis(1500), move || {
        in_slot(slot, || {
            let Some((p, b, page)) = S.with_borrow(|s| {
                Some((s.palette.clone()?, s.backdrop.clone()?, s.tabs.get(s.active)?.view.clone()?))
            }) else {
                return;
            };
            if modes::mode() == Mode::Command {
                return;
            }
            set_backdrop(&b, Some(&page));
            let b2 = b.clone();
            crate::anim::warm(&[p.upcast(), b.upcast()], move || set_backdrop(&b2, None));
        })
    });
    let session = if private || STARTED.replace(true) {
        None
    } else {
        crate::session::startup()
    };
    match (session, S.with_borrow_mut(|s| s.restore.take()), start) {
        (Some(v), _, url) => {
            restore(&v, false);
            if let Some(u) = url {
                open(&u, true)
            }
        }
        (None, Some(w), _) => restore_tabs(&w, false),
        (None, None, url) => open(&url.unwrap_or_else(start_page), true),
    }
    if !private && crate::welcome::first_run() {
        open(&web::internal_url(&crate::welcome::page()), true);
    }
    apply_layout();
    if let Some((level, text)) = startup {
        message(&level, &text);
    }
}

/// The window is closing: save the session if it is the last one, drop its tabs.
fn window_closing() {
    let last = WINS.with_borrow(|w| w.iter().filter(|s| s.window.is_some()).count() <= 1);
    if last && !QUITTING.get() {
        crate::session::save_now();
    }
    S.with_borrow_mut(|s| {
        *s = State {
            claimed: true,
            ..State::default()
        }
    });
    if let Some(i) = WINS.with_borrow(|w| w.iter().position(|s| s.window.is_some())) {
        CUR.set(i);
        if !QUITTING.get() {
            crate::session::save_now();
        }
    }
}

/// Run `f` (an `open` from the address bar) so the page it loads counts as typed.
pub fn typed_open(current: bool, f: impl FnOnce()) {
    if current {
        S.with_borrow_mut(|s| {
            let i = s.active;
            if let Some(t) = s.tabs.get_mut(i) {
                t.typed = true;
            }
        });
    }
    TYPED_NEXT.set(!current);
    f();
    TYPED_NEXT.set(false);
}

fn new_tab(view: Option<WebView>, url: String, title: String) -> Tab {
    Tab {
        view,
        url,
        title,
        progress: 0.0,
        loading: false,
        crashed: None,
        can_back: false,
        can_fwd: false,
        muted: false,
        playing: false,
        state: None,
        pinned: false,
        seen: std::time::Instant::now(),
        keep: false,
        asleep: false,
        typed: TYPED_NEXT.get(),
        fresh: true,
        icon: String::new(),
        key: {
            TAB_KEY.set(TAB_KEY.get() + 1);
            TAB_KEY.get()
        },
    }
}

thread_local! {
    static TAB_KEY: Cell<u64> = const { Cell::new(0) };
}

fn add_tab(tab: Tab, foreground: bool) {
    let n = S.with_borrow_mut(|s| {
        s.tabs.push(tab);
        s.tabs.len() - 1
    });
    if foreground || n == 0 {
        show(n);
    } else {
        refresh();
    }
    apply_tabs_show();
}

/// A background tab loads only when first shown (lazy, like session restore).
pub fn open(url: &str, foreground: bool) {
    add_tab(new_tab(None, url.to_string(), String::new()), foreground);
}

/// A popup the page opened (`create` signal): already a view, keeps window.opener.
pub fn adopt(view: WebView, foreground: bool) {
    tabview::connect(&view);
    let stack = S.with_borrow(|s| s.stack.clone());
    if let Some(st) = stack {
        st.add_child(&view);
    }
    add_tab(new_tab(Some(view), String::new(), String::new()), foreground);
}

/// The tab's view, created (and loaded / restored) on first need.
pub fn ensure_view(n: usize) -> Option<WebView> {
    if let Some(v) = S.with_borrow(|s| s.tabs.get(n)?.view.clone()) {
        return Some(v);
    }
    let (url, state, private, stack) = S.with_borrow_mut(|s| {
        let private = s.private;
        let stack = s.stack.clone();
        let t = s.tabs.get_mut(n)?;
        Some((t.url.clone(), t.state.take(), private, stack))
    })?;
    let view = web::page_view(None, private);
    tabview::connect(&view);
    if let Some(st) = stack {
        st.add_child(&view);
    }
    S.with_borrow_mut(|s| {
        if let Some(t) = s.tabs.get_mut(n) {
            t.view = Some(view.clone());
            t.asleep = false;
        }
    });
    let restored = state
        .and_then(|b| {
            let st = webkit6::WebViewSessionState::new(&b);
            view.restore_session_state(&st);
            let item = view.back_forward_list().and_then(|l| l.current_item())?;
            view.go_to_back_forward_list_item(&item);
            Some(())
        })
        .is_some();
    if !restored {
        tabview::load(&view, if url.is_empty() { "about:blank" } else { &url });
    }
    Some(view)
}

/// Every open window, in session.rs's shape (private windows left out).
pub fn session_json() -> Option<Value> {
    let slots: Vec<usize> = WINS.with_borrow(|w| {
        (0..w.len())
            .filter(|i| w[*i].window.is_some() && !w[*i].private)
            .collect()
    });
    let wins: Vec<Value> = slots.iter().filter_map(|i| in_slot(*i, window_json)).collect();
    if wins.is_empty() {
        return None;
    }
    let closed = S.with_borrow(|s| s.closed[s.closed.len().saturating_sub(100)..].to_vec());
    Some(crate::session::shape(wins, &closed))
}

fn window_json() -> Option<Value> {
    S.with_borrow(|s| {
        s.window.as_ref()?;
        // nova://page/ tabs are generated pages: stale after a restart
        let tabs = s
            .tabs
            .iter()
            .filter(|t| !web::is_internal(&t.url))
            .map(tab_json)
            .collect();
        Some(crate::session::window(tabs, s.active))
    })
}

/// One tab: URL, title, WebKit session state (back/forward, scroll) as base64, pinned.
fn tab_json(t: &Tab) -> Value {
    let state = t
        .view
        .as_ref()
        .and_then(|v| v.session_state())
        .and_then(|st| st.serialize())
        .or_else(|| t.state.clone());
    json!({
        "url": t.crashed.clone().unwrap_or_else(|| t.url.clone()),
        "title": t.title,
        "state": state.map(|b| glib::base64_encode(&b).to_string()),
        "pinned": t.pinned,
        "keep": t.keep,
        "icon": t.icon,
    })
}

/// Open a saved session: its first window's tabs go into this window (`clear`
/// closes the tabs that were there), every other window opens as a new window.
pub fn restore(v: &Value, clear: bool) {
    let wins = v["windows"].as_array().cloned().unwrap_or_else(|| vec![v.clone()]);
    let closed: Vec<String> = v["closed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|u| u.as_str().map(str::to_string))
        .collect();
    S.with_borrow_mut(|s| {
        let mut c = closed;
        c.append(&mut s.closed);
        s.closed = c;
    });
    for (i, w) in wins.iter().enumerate() {
        if i == 0 {
            restore_tabs(w, clear);
        } else {
            create_window(None, Some(w.clone()), false);
        }
    }
}

fn restore_tabs(v: &Value, clear: bool) {
    let before = S.with_borrow(|s| s.tabs.len());
    let tabs = v["tabs"].as_array().cloned().unwrap_or_default();
    for t in &tabs {
        let url = t["url"].as_str().unwrap_or("about:blank").to_string();
        let mut tab = new_tab(None, url, t["title"].as_str().unwrap_or("").to_string());
        tab.fresh = false;
        tab.typed = false;
        tab.state = t["state"]
            .as_str()
            .map(|b| glib::Bytes::from_owned(glib::base64_decode(b)));
        tab.pinned = t["pinned"].as_bool().unwrap_or(false);
        tab.keep = t["keep"].as_bool().unwrap_or(false);
        tab.icon = t["icon"].as_str().unwrap_or("").to_string();
        S.with_borrow_mut(|s| s.tabs.push(tab));
    }
    if !tabs.is_empty() {
        show(before + (v["active"].as_u64().unwrap_or(0) as usize).min(tabs.len() - 1));
    }
    if clear {
        for _ in 0..before {
            remove(0);
        }
    }
}

pub fn start_page() -> String {
    config::get()
        .list("url.start_pages")
        .into_iter()
        .next()
        .unwrap_or_else(|| "about:blank".into())
}

/// Bring tab `n` to the front (creating its view on first show). Hidden stack pages
/// are unmapped, so WebKit reports them hidden and throttles them.
pub fn show(n: usize) {
    let ok = S.with_borrow_mut(|s| {
        if n >= s.tabs.len() {
            return false;
        }
        if s.active != n && s.active < s.tabs.len() {
            s.last = s.active;
            s.tabs[s.active].seen = std::time::Instant::now();
        }
        s.active = n;
        true
    });
    if !ok {
        return;
    }
    if let Some(view) = ensure_view(n) {
        if let Some(st) = S.with_borrow(|s| s.stack.clone()) {
            if view.parent().is_none() {
                st.add_child(&view);
            }
            st.set_visible_child(&view);
        }
    }
    // qutebrowser leaves insert/passthrough when the tab changes
    if modes::mode() != Mode::Command {
        modes::enter(Mode::Normal);
        focus_active();
    }
    refresh();
}

/// `tab-close` / `d` / middle click: the page runs beforeunload first (`try_close`);
/// the tab goes away on its `close` signal. Unloaded and crashed tabs close at once.
pub fn close(n: usize) {
    let view = S.with_borrow(|s| {
        let t = s.tabs.get(n)?;
        t.crashed.is_none().then(|| t.view.clone()).flatten()
    });
    match view {
        Some(v) => v.try_close(),
        None => remove(n),
    }
}

/// The page agreed to close (`close` signal).
pub fn did_close(view: &WebView) {
    let id = web::id(view);
    in_owner(view, || {
        let n = S.with_borrow(|s| s.tabs.iter().position(|t| vid(&t.view) == Some(id)));
        if let Some(n) = n {
            remove(n);
        }
    });
}

/// Take tab `n` out now. Its view leaves the widget tree and is dropped, which ends
/// the page (unload handlers, audio) in the web process.
pub fn remove(n: usize) {
    let Some((tab, was_active, left, stack, window)) = S.with_borrow_mut(|s| {
        if n >= s.tabs.len() {
            return None;
        }
        let tab = s.tabs.remove(n);
        if !web::is_internal(&tab.url) {
            s.closed
                .push(tab.crashed.clone().unwrap_or_else(|| tab.url.clone()));
        }
        s.last = s.last.min(s.tabs.len().saturating_sub(1));
        let was_active = n == s.active;
        if n < s.active || (was_active && s.active == s.tabs.len() && s.active > 0) {
            s.active -= 1;
        }
        Some((tab, was_active, s.tabs.len(), s.stack.clone(), s.window.clone()))
    }) else {
        return;
    };
    if let (Some(v), Some(st)) = (&tab.view, &stack) {
        if v.parent().is_some() {
            st.remove(v);
        }
    }
    crate::permissions::forget(tab.view.as_ref());
    if let Some(v) = &tab.view {
        crate::adblock::forget(v);
        crate::darkmode::forget(v);
        crate::passwords::forget(v);
    }
    drop(tab);
    if left == 0 {
        let cfg = config::get();
        let reopen = match cfg.str("tabs.last_close").as_str() {
            "close" => None,
            "blank" => Some("about:blank".to_string()),
            "startpage" => Some(start_page()),
            "default-page" => Some(cfg.str("url.default_page")),
            _ => Some("about:blank".to_string()), // "ignore": keep a tab
        };
        match reopen {
            Some(url) => open(&url, true),
            None => {
                if let Some(w) = window {
                    w.close();
                }
            }
        }
        return;
    }
    if was_active {
        let active = S.with_borrow(|s| s.active);
        show(active);
    } else {
        refresh();
    }
    apply_tabs_show();
}

/// Apply `f` to the tab owning `view`, then redraw the bars.
pub fn update(view: &WebView, f: impl FnOnce(&mut Tab)) {
    let id = web::id(view);
    in_owner(view, || {
        let found = S.with_borrow_mut(|s| {
            s.tabs
                .iter_mut()
                .find(|t| vid(&t.view) == Some(id))
                .map(f)
                .is_some()
        });
        if found {
            refresh();
        }
    });
}

/// Is `view` in a private window?
pub fn is_private(view: &WebView) -> bool {
    in_owner(view, || S.with_borrow(|s| s.private))
}

pub fn set_url(view: &WebView, url: String) {
    let private = is_private(view);
    update(view, |t| {
        let typed = std::mem::take(&mut t.typed);
        let fresh = std::mem::take(&mut t.fresh);
        if t.crashed.is_none() && (t.url != url || typed || fresh) {
            if !private {
                history::visit(&url, typed);
            }
            t.url = url;
        }
    });
}

pub fn set_title(view: &WebView, title: String) {
    let private = is_private(view);
    update(view, |t| {
        if t.crashed.is_none() {
            if !private {
                history::title(&t.url, &title);
            }
            t.title = title
        }
    });
}

pub fn set_favicon(view: &WebView, icon: String) {
    update(view, |t| t.icon = icon);
}
pub fn set_progress(view: &WebView, p: f64) {
    update(view, |t| t.progress = p);
}
pub fn set_loading(view: &WebView, loading: bool) {
    let (back, fwd) = (view.can_go_back(), view.can_go_forward());
    update(view, |t| {
        t.loading = loading;
        t.can_back = back;
        t.can_fwd = fwd;
    });
}
pub fn set_audio(view: &WebView) {
    let (muted, playing) = (view.is_muted(), view.is_playing_audio());
    update(view, |t| {
        t.muted = muted;
        t.playing = playing;
    });
}

/// `back`/`forward` (n < 0 = back) through the back/forward list; the page can't trap them.
pub fn go(view: &WebView, n: i32) {
    let list = view.back_forward_list();
    let item = list.and_then(|l| l.nth_item(n));
    match item {
        Some(i) => view.go_to_back_forward_list_item(&i),
        None => message(
            "warning",
            if n < 0 {
                "At beginning of history."
            } else {
                "At end of history."
            },
        ),
    }
}

/// Web process died: keep the tab, show a local page with the URL; `r` reloads.
pub fn crashed(view: &WebView, why: &str) {
    let mut url = String::new();
    update(view, |t| {
        url = t.url.clone();
        t.crashed = Some(t.url.clone());
        t.loading = false;
    });
    notice_page(view, &url, "crashed", &format!("This tab crashed ({why})"), &[], &[]);
}

/// A local page in place of `url`: icon, heading, `lines` as text, `mono` lines (URL,
/// error code) muted, then the `r` hint. Text goes in as JSON, never as HTML.
fn notice_page(view: &WebView, url: &str, title: &str, heading: &str, lines: &[&str], mono: &[&str]) {
    let html = theme::themed_page(format!(
        "<!doctype html><meta charset=utf-8>\
         <style>.c{{max-width:34em;margin:18vh auto 0;text-align:center}}\
         .i{{width:3.2em;height:3.2em;margin:0 auto 1em;border-radius:50%;display:grid;place-items:center;\
         font-size:1.6em;font-weight:700;color:var(--red);background:color-mix(in srgb,var(--red) 18%,transparent)}}\
         .m{{font:.9em ui-monospace,monospace;color:var(--muted);word-break:break-all}}</style>\
         <div class=c><div class=i>!</div><h2 id=h style='text-transform:none;letter-spacing:0;font-size:1.3em;color:var(--fg)'>\
         </h2><div id=b></div><p>Press <kbd>r</kbd> to reload.</p></div>\
         <script>document.title = {};\
         document.getElementById('h').textContent = {};\
         const b = document.getElementById('b');\
         for (const [t, m] of {}) b.append(Object.assign(document.createElement('p'), {{textContent: t, className: m ? 'm' : ''}}));</script>",
        js_str(title),
        js_str(heading),
        json!(lines.iter().map(|l| (l, false)).chain(std::iter::once((&url, true))).chain(mono.iter().map(|l| (l, true))).collect::<Vec<_>>()),
    ));
    view.load_alternate_html(&html, url, None);
}

/// `load-failed` on a web page: a Chrome-style "can't be reached" page instead of
/// WebKit's one-line error. Returns whether it showed one.
pub fn load_error(view: &WebView, uri: &str, err: &glib::Error) -> bool {
    if !(uri.starts_with("http://") || uri.starts_with("https://")) {
        return false;
    }
    let host = crate::commands::host(uri);
    let (what, code) = load_error_text(host, err.message());
    notice_page(
        view,
        uri,
        host,
        "This site can't be reached",
        &[&what, "Check the address, your connection, and that the server is running."],
        &[&format!("{code} · {}", err.message())],
    );
    true
}

/// Chrome's wording for a network error message (libsoup / GIO text): (what happened, code).
fn load_error_text(host: &str, msg: &str) -> (String, &'static str) {
    let m = msg.to_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| m.contains(k));
    if has(&["connection refused"]) {
        (format!("{host} refused to connect."), "ERR_CONNECTION_REFUSED")
    } else if has(&["resolv", "name or service not known", "no address associated"]) {
        (format!("{host}'s server IP address could not be found."), "ERR_NAME_NOT_RESOLVED")
    } else if has(&["timed out"]) {
        (format!("{host} took too long to respond."), "ERR_CONNECTION_TIMED_OUT")
    } else if has(&["network is unreachable", "no route to host", "host is unreachable"]) {
        (format!("{host} is unreachable from this network."), "ERR_ADDRESS_UNREACHABLE")
    } else if has(&["connection reset", "connection closed", "unexpectedly closed"]) {
        ("The connection was reset.".into(), "ERR_CONNECTION_RESET")
    } else {
        (format!("{host} can't be reached."), "ERR_FAILED")
    }
}

/// `r` on a crashed tab: load its URL again. Returns whether it was crashed.
pub fn reload_crashed(view: &WebView) -> bool {
    let mut url = None;
    update(view, |t| url = t.crashed.take());
    let Some(url) = url else { return false };
    view.load_uri(&url);
    true
}

fn fields(i: usize, t: &Tab) -> TitleFields<'_> {
    TitleFields {
        index: i + 1,
        title: &t.title,
        url: &t.url,
        progress: t.loading.then_some(t.progress),
        muted: t.muted,
    }
}

/// Push the current state into the tab bar, status bar and window title.
pub fn refresh() {
    update_keyhint();
    let cfg = config::get();
    let tab_fmt = cfg.str("tabs.title.format");
    let pin_fmt = cfg.str("tabs.title.format_pinned");
    let Some((tabbar, window, bar_json, title)) = S.with_borrow(|s| {
        let tabs: Vec<String> = s
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let pinned = t.pinned && i != s.active;
                let label = theme::format_title(if pinned { &pin_fmt } else { &tab_fmt }, &fields(i, t));
                format!(
                    "{{\"k\":{},\"t\":{},\"i\":{},\"a\":{},\"l\":{},\"p\":{},\"e\":{},\"n\":{},\"z\":{}}}",
                    t.key,
                    js_str(&label),
                    js_str(&t.icon),
                    i == s.active,
                    t.loading,
                    t.progress,
                    t.crashed.is_some(),
                    t.pinned,
                    t.asleep
                )
            })
            .collect();
        let t = s.tabs.get(s.active)?;
        // the open question has its own popup (set_prompt), not the tab bar
        let (level, msg) = s.message.clone().unwrap_or_default();
        let profile = if s.private {
            "private".to_string()
        } else {
            crate::profile().unwrap_or("").to_string()
        };
        let status = json!({
            "profile": profile,
            "url": t.url,
            "p": t.progress,
            "loading": t.loading,
            "crashed": t.crashed.is_some(),
            "mode": modes::mode_label(),
            "cmd": modes::mode() == Mode::Command,
            "keys": if modes::mode() == Mode::Hint { crate::hints::typed() } else { modes::pending() },
            "level": level,
            "msg": msg,
            "tab": format!("[{}/{}]", s.active + 1, s.tabs.len()),
            "hist": match (t.can_back, t.can_fwd) {
                (true, true) => "[<>]",
                (true, false) => "[<]",
                (false, true) => "[>]",
                _ => "",
            },
            "modename": modes::mode_name(),
            "dl": crate::downloads::summary().0,
            "dlc": crate::downloads::summary().1,
        });
        let mut title = theme::format_title(&cfg.str("window.title_format"), &fields(s.active, t));
        if let Some(p) = crate::profile() {
            title = format!("[{p}] {title}");
        }
        // serde escapes "</script>"-like text fine for a JS expression
        let status = status.to_string().replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029");
        Some((
            s.tabbar.clone()?,
            s.window.clone()?,
            format!("{{\"tabs\":[{}],\"status\":{status}}}", tabs.join(",")),
            title,
        ))
    }) else {
        return;
    };
    crate::session::dirty();
    web::eval_page(&tabbar, &format!("window.nova && nova.apply && nova.apply({bar_json})"));
    window.set_title(Some(&title));
}

// ---- queries and actions for commands.rs

/// The view commands act on: the userscript's tab, else the active one (loaded on demand).
pub fn active_view() -> Option<WebView> {
    let n = S.with_borrow(|s| match TARGET.get() {
        Some(id) => s.tabs.iter().position(|t| vid(&t.view) == Some(id)),
        None => Some(s.active),
    })?;
    S.with_borrow(|s| s.tabs.get(n).and_then(|t| t.view.clone()))
}

/// Run `f` with commands aimed at the tab whose view id is `id`. False if it's gone.
pub fn run_in_tab(id: usize, f: impl FnOnce()) -> bool {
    let Some(slot) = owner(id) else { return false };
    if !in_slot(slot, || {
        S.with_borrow(|s| s.tabs.iter().any(|t| vid(&t.view) == Some(id)))
    }) {
        return false;
    }
    in_slot(slot, || {
        TARGET.set(Some(id));
        f();
        TARGET.set(None);
    });
    true
}

/// `--query state`: tabs, active index, mode (for scripts and the smoke test).
pub fn state_json() -> String {
    S.with_borrow(|s| {
        json!({
            "tabs": s.tabs.iter().map(|t| json!({"url": t.url, "title": t.title, "pinned": t.pinned, "loaded": t.view.is_some(), "crashed": t.crashed.is_some(), "asleep": t.asleep, "muted": t.muted, "keep": t.keep})).collect::<Vec<_>>(),
            "private": s.private,
            "suggestions": crate::passwords::suggestions(),
            // on-screen size of the tooltip/password popup while shown
            "popup": s.tip.as_ref().filter(|v| v.is_visible()).map(|v| [v.width(), v.height()]),
            "fullscreen": s.window.as_ref().is_some_and(|w| w.is_fullscreen()),
            "active": s.active,
            "url": s.tabs.get(s.active).map(|t| t.url.as_str()),
            "mode": modes::mode_name(),
            "label": modes::mode_label(),
            "windows": WINS.with(|w| w.try_borrow().map(|w| w.iter().filter(|s| s.window.is_some()).count()).unwrap_or(1)),
            "message": s.message.as_ref().map(|m| format!("{}: {}", m.0, m.1)),
            "prompt": s.prompt,
        })
        .to_string()
    })
}

pub fn active_url_title() -> (String, String) {
    S.with_borrow(|s| {
        let i = match TARGET.get() {
            Some(id) => s
                .tabs
                .iter()
                .position(|t| vid(&t.view) == Some(id))
                .unwrap_or(usize::MAX),
            None => s.active,
        };
        s.tabs
            .get(i)
            .map(|t| (t.url.clone(), t.title.clone()))
            .unwrap_or_default()
    })
}

pub fn content_size() -> (f64, f64) {
    S.with_borrow(|s| {
        s.stack
            .as_ref()
            .map(|c| (c.width() as f64, c.height() as f64))
            .unwrap_or((0.0, 0.0))
    })
}

pub fn close_active(force: bool) {
    let n = S.with_borrow(|s| s.active);
    close_user(n, force);
}

/// `d`, middle click, tab menu: pinned tabs only with `force` (qutebrowser).
pub fn close_user(n: usize, force: bool) {
    if !force && S.with_borrow(|s| s.tabs.get(n).is_some_and(|t| t.pinned)) {
        return message("error", "Tab is pinned! Unpin it or use tab-close --force.");
    }
    close(n);
}

/// `tab-close` with a count: close that many tabs from the active one rightwards.
pub fn close_count(count: u32, force: bool) {
    for _ in 0..count.max(1) {
        close_active(force);
    }
}

/// `tab-pin [index]`: toggle; pinned tabs move to the end of the pinned group.
pub fn pin(n: Option<usize>) {
    let pinned = S.with_borrow_mut(|s| {
        let n = n.unwrap_or(s.active);
        if n >= s.tabs.len() {
            return None;
        }
        let was_active = n == s.active;
        let active_id = vid(&s.tabs[s.active].view);
        let mut tab = s.tabs.remove(n);
        tab.pinned = !tab.pinned;
        let pinned = tab.pinned;
        let to = s.tabs.iter().take_while(|t| t.pinned).count();
        s.tabs.insert(to, tab);
        s.active = if was_active {
            to
        } else {
            s.tabs
                .iter()
                .position(|t| active_id.is_some() && vid(&t.view) == active_id)
                .unwrap_or(s.active)
        };
        Some(pinned)
    });
    refresh();
    if let Some(p) = pinned {
        message("info", if p { "Tab pinned." } else { "Tab unpinned." });
    }
}

/// `tab-give` / `gD`: move tab `n` with its live view into a new window (no reload).
pub fn give(n: Option<usize>) {
    let Some((tab, stack)) = S.with_borrow_mut(|s| {
        let n = n.unwrap_or(s.active);
        if n >= s.tabs.len() || s.tabs.len() < 2 {
            return None;
        }
        let tab = s.tabs.remove(n);
        if s.active >= s.tabs.len() || n < s.active {
            s.active = s.active.saturating_sub(1);
        }
        Some((tab, s.stack.clone()))
    }) else {
        return message("error", "Can't give away the only tab.");
    };
    if let (Some(v), Some(st)) = (&tab.view, stack) {
        st.remove(v);
    }
    let active = S.with_borrow(|s| s.active);
    show(active);
    let slot = WINS.with_borrow_mut(|w| {
        w.push(State {
            claimed: true,
            ..State::default()
        });
        w.len() - 1
    });
    let mut tab = tab;
    tab.pinned = false;
    // the new window starts with this tab only (no start page)
    WINS.with_borrow_mut(|w| {
        w[slot].restore = Some(json!({"tabs": []}));
        w[slot].tabs.push(tab);
    });
    in_slot(slot, || {
        build_window(slot);
        let view = S.with_borrow(|s| s.tabs.first().and_then(|t| t.view.clone()));
        if let (Some(v), Some(st)) = (view, S.with_borrow(|s| s.stack.clone())) {
            st.add_child(&v);
        }
        show(0);
    });
}

pub fn close_others() {
    close_others_than(S.with_borrow(|s| s.active));
}

/// Close every unpinned tab but `keep`.
fn close_others_than(mut keep: usize) {
    loop {
        let n = S.with_borrow(|s| {
            (keep < s.tabs.len())
                .then(|| s.tabs.iter().enumerate().position(|(j, t)| j != keep && !t.pinned))
                .flatten()
        });
        let Some(n) = n else { break };
        if n < keep {
            keep -= 1;
        }
        remove(n);
    }
}

/// A page entered/left fullscreen (video). `content.fullscreen.window`: fill our
/// window only (hide the tab bar); otherwise the window goes fullscreen too.
pub fn page_fullscreen(on: bool) {
    let Some((window, tabbar)) = S.with_borrow(|s| Some((s.window.clone()?, s.tabbar.clone()?)))
    else {
        return;
    };
    tabbar.set_visible(!on);
    if !config::get().bool("content.fullscreen.window") {
        if on {
            window.fullscreen()
        } else {
            window.unfullscreen()
        }
    }
    if !on {
        apply_tabs_show();
    }
}

pub fn toggle_fullscreen() {
    if let Some(w) = S.with_borrow(|s| s.window.clone()) {
        if w.is_fullscreen() {
            w.unfullscreen()
        } else {
            w.fullscreen()
        }
    }
}

pub fn close_window() {
    if let Some(w) = S.with_borrow(|s| s.window.clone()) {
        w.close();
    }
}

/// `quit`: save the session once, then close every window.
pub fn quit() {
    crate::session::save_now();
    QUITTING.set(true);
    let wins: Vec<gtk::ApplicationWindow> =
        WINS.with_borrow(|w| w.iter().filter_map(|s| s.window.clone()).collect());
    for w in wins {
        w.close();
    }
}

pub fn cycle(by: i64) {
    let n =
        S.with_borrow(|s| (s.active as i64 + by).rem_euclid(s.tabs.len().max(1) as i64) as usize);
    show(n);
}

pub fn focus_last() {
    let n = S.with_borrow(|s| s.last);
    show(n);
}

/// 1-based; negative counts from the end (qutebrowser `tab-focus`).
pub fn focus_index(i: i64) {
    let len = S.with_borrow(|s| s.tabs.len()) as i64;
    let n = if i < 0 { len + i } else { i - 1 };
    if (0..len).contains(&n) {
        show(n as usize)
    } else {
        message("error", &format!("There's no tab with index {i}!"))
    }
}

/// `tab-move`: relative (`+`/`-` with count) or to a 1-based index (negative from the end).
pub fn move_active(i: i64, relative: bool) {
    S.with_borrow_mut(|s| {
        if s.tabs.is_empty() {
            return;
        }
        let len = s.tabs.len() as i64;
        let to = if relative {
            s.active as i64 + i
        } else if i < 0 {
            len + i
        } else {
            i - 1
        };
        let pinned = s.tabs.iter().filter(|t| t.pinned).count();
        let to = move_target(to, len as usize, pinned, s.tabs[s.active].pinned);
        let tab = s.tabs.remove(s.active);
        s.tabs.insert(to, tab);
        s.active = to;
    });
    refresh();
}

/// Clamp a move so pinned tabs stay in the pinned group at the start, unpinned ones after it.
fn move_target(to: i64, len: usize, pinned: usize, is_pinned: bool) -> usize {
    let (lo, hi) = if is_pinned {
        (0, pinned.saturating_sub(1))
    } else {
        (pinned.min(len - 1), len - 1)
    };
    to.clamp(lo as i64, hi as i64) as usize
}

pub fn undo_close() {
    match S.with_borrow_mut(|s| s.closed.pop()) {
        Some(url) => open(&url, true),
        None => message("error", "Nothing to undo!"),
    }
}

/// Show (or hide, with None) the open question and its [key, label] answers.
pub fn set_prompt(p: Option<(String, Vec<[&'static str; 2]>)>) {
    let Some((view, content)) = S.with_borrow_mut(|s| {
        s.prompt = p.as_ref().map(|p| p.0.clone());
        Some((s.ask.clone()?, s.content.clone()?))
    }) else {
        return;
    };
    refresh();
    let Some((q, opts)) = p else {
        return crate::anim::conceal(&view, || {});
    };
    let w = (content.width() - 24).clamp(240, 640);
    // shortcut: line count guessed at ~9 px per character; prompt.html clamps to 3 lines
    let lines = (q.chars().count() as i32 * 9 / (w - 100) + 1).min(3);
    // body padding 10+10, card (56 px or 20 px lines + 20), gap 14, buttons 40, shadow room
    view.set_size_request(w, 20 + (20 * lines + 22).max(56) + 14 + 40 + 6);
    crate::anim::reveal(&view, &format!("nova.ask({})", json!({"q": q, "opts": opts})), &[]);
}

/// Show a status bar message for `messages.timeout` ms (`info`, `warning`, `error`).
pub fn message(level: &str, text: &str) {
    eprintln!("novabrowser-webkit: {level}: {text}");
    let id = S.with_borrow_mut(|s| {
        s.message = Some((level.to_string(), text.to_string()));
        s.message_id += 1;
        s.message_id
    });
    refresh();
    // typing a search: the count goes in the search pill, which sits where the toast would
    if modes::mode() == Mode::Command && S.with_borrow(|s| s.compact) {
        let count = match level {
            "find" => text.split('\t').nth(1),
            "warning" if text.ends_with("not found on page!") => Some("no match"),
            _ => None,
        };
        if let Some(c) = count {
            return palette_js(&format!("nova.count({})", js_str(c)));
        }
    }
    show_toast(level, text);
    let ms = config::get().int("messages.timeout");
    if ms > 0 {
        let slot = CUR.get();
        glib::timeout_add_local_once(Duration::from_millis(ms as u64), move || {
            in_slot(slot, || {
                let cleared =
                    S.with_borrow_mut(|s| s.message_id == id && s.message.take().is_some());
                if cleared {
                    refresh();
                    if let Some(t) = S.with_borrow(|s| s.toast.clone()) {
                        crate::anim::conceal(&t, || {});
                    }
                }
            })
        });
    }
}

/// The message as a pill at the bottom centre. The view spans the content width (it is
/// transparent and takes no clicks); the pill sizes itself to the text (toast.html).
fn show_toast(level: &str, text: &str) {
    let Some((t, content)) = S.with_borrow(|s| Some((s.toast.clone()?, s.content.clone()?))) else {
        return;
    };
    t.set_size_request((content.width() - 40).max(160), 46);
    crate::anim::reveal(&t, &format!("nova.toast({}, {})", js_str(level), js_str(text)), &[]);
}

/// Show `text` once the window is up (startup config errors).
/// Bring the current window to the front (a URL handed over from another app).
pub fn raise() {
    if let Some(w) = S.with_borrow(|s| s.window.clone()) {
        w.present();
    }
}

pub fn startup_message(level: &str, text: &str) {
    S.with_borrow_mut(|s| s.startup = Some((level.to_string(), text.to_string())));
}

fn palette_js(js: &str) {
    if let Some(v) = S.with_borrow(|s| s.palette.clone()) {
        web::eval_page(&v, js);
    }
}

/// `cmd-set-text`: open the floating palette with `text`. Search (`/`, `?`) gets a
/// thin strip so the matches stay visible; everything else floats over the page,
/// blurred live by GTK (no screenshot: that cost a PNG round trip and showed stale).
pub fn open_cmdline(text: &str) {
    modes::enter(Mode::Command);
    let compact = text.starts_with('/') || text.starts_with('?');
    let Some((view, backdrop, page)) = S.with_borrow_mut(|s| {
        s.compact = compact;
        Some((
            s.palette.clone()?,
            s.backdrop.clone()?,
            s.tabs.get(s.active).and_then(|t| t.view.clone()),
        ))
    }) else {
        return;
    };
    let also = match page.filter(|_| !compact) {
        Some(page) => {
            set_backdrop(&backdrop, Some(&page));
            if view.is_visible() {
                backdrop.set_visible(true);
            }
            vec![backdrop.upcast()]
        }
        None => {
            crate::anim::conceal(&backdrop, || {});
            vec![]
        }
    };
    place_palette();
    view.grab_focus();
    crate::anim::reveal(&view, &format!("nova.cmdline({}, {compact})", js_str(text)), &also);
}

/// Point the palette's backdrop at `page` (None: let go of it).
fn set_backdrop(b: &gtk::Box, page: Option<&WebView>) {
    if let Some(p) = b.first_child().and_downcast::<gtk::Picture>() {
        p.set_paintable(page.map(|v| gtk::WidgetPaintable::new(Some(v))).as_ref());
    }
}

/// Leave command mode and give focus back to the page.
pub fn close_cmdline() {
    modes::enter(Mode::Normal);
    crate::completion::reset();
    if let Some((v, b)) = S.with_borrow(|s| Some((s.palette.clone()?, s.backdrop.clone()?))) {
        crate::anim::conceal(&v, || {});
        let b2 = b.clone();
        // let go of the page: a live paintable redraws with it
        crate::anim::conceal(&b, move || set_backdrop(&b2, None));
    }
    focus_active();
    refresh();
}

pub fn set_cmdline_text(text: &str) {
    palette_js(&format!("nova.setText({})", js_str(text)));
}

pub fn show_completion(json: Option<&str>) {
    // closing (completion::reset): keep the list while the palette fades out, or the
    // card snaps to one line first; nova.cmdline clears it on the next open (W15)
    if json.is_none() && modes::mode() != Mode::Command {
        return;
    }
    palette_js(&format!("nova.complete({})", json.unwrap_or("null")));
    if let Some(j) = json {
        completion_icons(j);
    }
}

thread_local! {
    /// page URL -> site icon as a data: URL ("" = none known), for completion rows
    static ICONS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// Site icons for the completion rows with a web address, from WebKit's icon store (no
/// network): cached ones at once, the rest as each lookup answers. Same URL rule as palette.html.
fn completion_icons(json: &str) {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return };
    let urls: Vec<String> = v["items"]
        .as_array()
        .into_iter()
        .flatten()
        .take(40)
        .filter_map(|it| {
            it.as_array()?.iter().filter_map(|x| x.as_str()).find(|x| x.starts_with("http://") || x.starts_with("https://")).map(str::to_string)
        })
        .collect();
    let known: Map<String, Value> = ICONS.with_borrow(|c| {
        urls.iter().filter_map(|u| Some((u.clone(), Value::from(c.get(u)?.clone())))).filter(|(_, d)| d != "").collect()
    });
    if !known.is_empty() {
        palette_js(&format!("nova.icons({})", Value::Object(known)));
    }
    let Some(db) = web::session().website_data_manager().and_then(|m| m.favicon_database()) else {
        return;
    };
    let slot = CUR.get();
    let rows = Rc::new(urls.clone());
    for u in urls {
        if ICONS.with_borrow(|c| c.contains_key(&u)) {
            continue;
        }
        ICONS.with_borrow_mut(|c| c.insert(u.clone(), String::new()));
        // the page itself, else the site's home page (most visited pages here were never
        // opened in this edition: imported history has no icons)
        let root = u.find("://").and_then(|i| u[i + 3..].find('/').map(|j| u[..i + 3 + j + 1].to_string()));
        let tries: Vec<String> = std::iter::once(u.clone()).chain(root.filter(|r| *r != u)).collect();
        icon_lookup(db.clone(), tries, u, rows.clone(), slot);
    }
}

fn site_of(u: &str) -> &str {
    u.split('/').nth(2).unwrap_or("")
}

/// Try each page URL in `tries` for `row`'s icon. A found icon also goes to the shown rows
/// of the same site that have none.
fn icon_lookup(db: webkit6::FaviconDatabase, mut tries: Vec<String>, row: String, rows: Rc<Vec<String>>, slot: usize) {
    if tries.is_empty() {
        return;
    }
    let page = tries.remove(0);
    let db2 = db.clone();
    db.favicon(&page, None::<&gtk::gio::Cancellable>, move |r| {
        let Ok(t) = r else {
            return icon_lookup(db2, tries, row, rows, slot);
        };
        let data = format!("data:image/png;base64,{}", glib::base64_encode(&t.save_to_png_bytes()));
        let site = site_of(&row).to_string();
        let mut out = Map::new();
        ICONS.with_borrow_mut(|c| {
            for u in rows.iter().filter(|u| **u == row || site_of(u) == site) {
                if c.get(u).is_none_or(|d| d.is_empty()) {
                    c.insert(u.clone(), data.clone());
                    out.insert(u.clone(), Value::from(data.clone()));
                }
            }
        });
        in_slot(slot, || palette_js(&format!("nova.icons({})", Value::Object(out))));
    });
}

/// Full content area, or a top strip one input tall when compact.
fn place_palette() {
    let Some((p, compact)) = S.with_borrow(|s| Some((s.palette.clone()?, s.compact))) else {
        return;
    };
    if compact {
        // search: a pill at the bottom centre, where its find toast shows after Enter
        let w = S.with_borrow(|s| s.content.as_ref().map_or(600, |c| c.width()));
        p.set_halign(gtk::Align::Center);
        p.set_valign(gtk::Align::End);
        p.set_margin_bottom(12);
        p.set_size_request((w - 40).clamp(240, 560), 64);
    } else {
        p.set_halign(gtk::Align::Fill);
        p.set_valign(gtk::Align::Fill);
        p.set_margin_bottom(0);
        p.set_size_request(-1, -1);
    }
}

/// Overlay views with a size request (popups, prompt, key hints) get exactly that size,
/// placed by their align + margins: a WebView's natural size is its page's content size,
/// so the default placement grew the password list to the window's edge.
fn fixed_overlay_position(o: &gtk::Overlay, child: &gtk::Widget) -> Option<gtk::gdk::Rectangle> {
    let (w, h) = child.size_request();
    if w <= 0 || h <= 0 {
        return None;
    }
    let bw = child.margin_start() + w + child.margin_end();
    let bh = child.margin_top() + h + child.margin_bottom();
    let at = |a: gtk::Align, outer: i32, inner: i32| match a {
        gtk::Align::End => outer - inner,
        gtk::Align::Center => (outer - inner) / 2,
        _ => 0,
    };
    Some(gtk::gdk::Rectangle::new(
        at(child.halign(), o.width(), bw),
        at(child.valign(), o.height(), bh),
        bw,
        bh,
    ))
}

/// Show a root-overlay popup at window coords (x, y), size (w, h), kept inside, and run `js`.
fn popup_at(view: &WebView, x: i32, y: i32, w: i32, h: i32, js: &str) {
    let (ww, wh) = S.with_borrow(|s| {
        s.root
            .as_ref()
            .map_or((0, 0), |r| (r.width(), r.height()))
    });
    let x = x.min(ww - w - 4).max(4);
    let y = y.min(wh - h - 4).max(0);
    view.set_margin_start(x);
    view.set_margin_top(y);
    view.set_size_request(w, h);
    crate::anim::reveal(view, js, &[]);
}

/// Tab bar's origin in root (window content) coordinates.
fn origin_of(w: Option<impl IsA<gtk::Widget>>) -> (i32, i32) {
    let root = S.with_borrow(|s| s.root.clone());
    match (w, root) {
        (Some(w), Some(r)) => w
            .compute_point(&r, &gtk::graphene::Point::new(0.0, 0.0))
            .map_or((0, 0), |p| (p.x() as i32, p.y() as i32)),
        _ => (0, 0),
    }
}

fn tab_rect(arg: &str) -> Option<(usize, i32, i32, i32)> {
    let v: Vec<i64> = arg.split(' ').filter_map(|x| x.parse().ok()).collect();
    match v[..] {
        [i, l, b, r] if i >= 0 => Some((i as usize, l as i32, b as i32, r as i32)),
        _ => None,
    }
}

/// "tip i left bottom right" from the tab bar: show title + URL under (or beside) tab i.
pub fn tab_tip(arg: &str) {
    let Some(view) = S.with_borrow(|s| s.tip.clone()) else {
        return;
    };
    let Some((i, left, bottom, right)) = tab_rect(arg) else {
        return crate::anim::conceal(&view, || {});
    };
    let Some((title, url)) =
        S.with_borrow(|s| s.tabs.get(i).map(|t| (t.title.clone(), t.url.clone())))
    else {
        return;
    };
    let (ox, oy) = origin_of(S.with_borrow(|s| s.tabbar.clone()));
    let vertical = matches!(config::get().str("tabs.position").as_str(), "left" | "right");
    let (w, h) = (480, 30 + 20 * (url.len() as i32 / 60 + 1).min(4));
    let (x, y) = if vertical {
        (ox + right + 6, oy + bottom - 24)
    } else {
        (ox + left, oy + bottom + 4)
    };
    popup_at(&view, x, y, w, h, &format!("nova.tip({}, {})", js_str(&title), js_str(&url)));
}

const MENU: &[(&str, &str, &str)] = &[
    ("reload", "Reload", "r"),
    ("clone", "Duplicate", "gC"),
    ("yank", "Copy URL", "yy"),
    ("mute", "Mute / unmute", "Alt-m"),
    ("bookmark", "Bookmark", "M"),
    ("pin", "Pin / unpin", "Ctrl-Shift-p"),
    ("give", "Move to new window", "gD"),
    ("sleep", "Hibernate now", ""),
    ("keep", "Keep awake on / off", ""),
    ("close", "Close", "d"),
    ("others", "Close other tabs", "co"),
];

/// "menu i x bottom right": the tab's right-click menu at the mouse.
pub fn tab_menu(arg: &str) {
    let Some(view) = S.with_borrow(|s| s.menu.clone()) else {
        return;
    };
    let Some((i, x, bottom, _)) = tab_rect(arg) else {
        return;
    };
    if let Some(t) = S.with_borrow(|s| s.tip.clone()) {
        crate::anim::conceal(&t, || {});
    }
    let (ox, oy) = origin_of(S.with_borrow(|s| s.tabbar.clone()));
    let items: Vec<_> = MENU.iter().map(|(a, l, k)| [a, l, k]).collect();
    popup_at(
        &view,
        ox + x,
        oy + bottom + 2,
        230,
        MENU.len() as i32 * 28 + 14,
        &format!("nova.menu({i}, {})", json!(items)),
    );
    view.grab_focus();
}

/// Password suggestions under a login field; `field` = (x, y, w, h) in page CSS px.
pub fn suggest(field: (i32, i32, i32, i32), data: &Value) {
    let Some((view, stack)) = S.with_borrow(|s| Some((s.tip.clone()?, s.stack.clone()?))) else {
        return;
    };
    let (cx, cy) = origin_of(Some(stack));
    let n = data["rows"].as_array().map_or(0, |a| a.len()) as i32;
    popup_at(
        &view,
        cx + field.0,
        cy + field.1 + field.3 + 4,
        360,
        // header row with the key badge (popup.html .h) + padding + rows
        44 + n.min(8) * 28,
        &format!("nova.suggest({data})"),
    );
}

pub fn suggest_hide() {
    if let Some(v) = S.with_borrow(|s| s.tip.clone()) {
        let v2 = v.clone();
        crate::anim::conceal(&v, move || web::eval_page(&v2, "nova.suggest(null)"));
    }
}

pub fn close_popup() {
    if let Some(v) = S.with_borrow(|s| s.menu.clone()) {
        crate::anim::conceal(&v, || {});
    }
    focus_active();
}

/// "i action" from the menu: run it on tab i (not necessarily the active one).
pub fn tab_menu_action(arg: &str) {
    close_popup();
    let Some((i, action)) = arg
        .split_once(' ')
        .and_then(|(i, a)| Some((i.parse::<usize>().ok()?, a)))
    else {
        return;
    };
    let cmd = match action {
        "close" => return close_user(i, false),
        "others" => return close_others_than(i),
        "pin" => return pin(Some(i)),
        "give" => return give(Some(i)),
        "sleep" => return hibernate(Some(i)),
        "keep" => return keep_awake(Some(i)),
        "reload" => "reload",
        "clone" => "tab-clone -b",
        "yank" => "yank",
        "mute" => "tab-mute",
        "bookmark" => "bookmark-add",
        _ => return,
    };
    let Some(view) = ensure_view(i) else { return };
    run_in_tab(web::id(&view), || crate::commands::run(cmd, None));
}

/// Which-key: after a pending prefix waits 350 ms, list what can follow.
fn update_keyhint() {
    let rows = modes::keyhint().1;
    let (generation, slot) = S.with_borrow_mut(|s| {
        s.keyhint_gen += 1;
        (s.keyhint_gen, CUR.get())
    });
    if rows.is_empty() {
        if let Some(v) = S.with_borrow(|s| s.keyhint.clone()) {
            crate::anim::conceal(&v, || {});
        }
    } else {
        glib::timeout_add_local_once(Duration::from_millis(350), move || {
            in_slot(slot, || show_keyhint(generation))
        });
    }
}

fn show_keyhint(generation: u64) {
    let Some((view, stack)) = S.with_borrow(|s| {
        (s.keyhint_gen == generation).then(|| Some((s.keyhint.clone()?, s.stack.clone()?)))?
    }) else {
        return;
    };
    let (prefix, rows) = modes::keyhint();
    if rows.is_empty() {
        return;
    }
    let cfg = config::get();
    let row_h = (theme::setting_px(&cfg, "fonts.keyhint") * 1.6).ceil() as i32;
    let h = (rows.len() as i32 * row_h + 18).min(stack.height() - 24).max(30);
    let w = 380.min(stack.width() - 24).max(100);
    view.set_size_request(w, h);
    crate::anim::reveal(
        &view,
        &format!("nova.keyhint({}, {})", js_str(&prefix), json!(rows)),
        &[],
    );
}

/// Focus whichever tab is active (or the tab bar while a prompt waits) once the
/// current command finishes.
pub fn focus_active() {
    let slot = CUR.get();
    glib::idle_add_local_once(move || {
        in_slot(slot, || {
            let v = S.with_borrow(|s| {
                if modes::mode() == Mode::Prompt {
                    s.tabbar.clone()
                } else {
                    s.tabs.get(s.active).and_then(|t| t.view.clone())
                }
            });
            if let Some(v) = v {
                if modes::mode() != Mode::Command {
                    v.grab_focus();
                }
            }
        })
    });
}

// ---- hibernation

/// True if the page plays media or has typed form text: never hibernate it.
const BUSY_JS: &str = "JSON.stringify([...document.querySelectorAll('video,audio')].some(m=>!m.paused&&!m.ended)||\
[...document.querySelectorAll('input:not([type=hidden]),textarea')].some(e=>e.value!==e.defaultValue))";

/// Once a minute: unload background tabs idle for `tabs.hibernate_after` minutes.
pub fn start_hibernation() {
    glib::timeout_add_seconds_local(60, || {
        hibernate_idle();
        glib::ControlFlow::Continue
    });
}

fn hibernate_idle() {
    let cfg = config::get();
    let due: Vec<(usize, WebView)> = WINS.with_borrow(|w| {
        w.iter()
            .enumerate()
            .filter(|(_, s)| s.window.is_some())
            .flat_map(|(slot, s)| {
                let cfg = &cfg;
                s.tabs.iter().enumerate().filter_map(move |(i, t)| {
                    let mins = cfg.get_for("tabs.hibernate_after", &t.url).as_i64().unwrap_or(0);
                    let ok = i != s.active
                        && !t.keep
                        && !t.loading
                        && !t.playing
                        && t.crashed.is_none()
                        && !web::is_internal(&t.url)
                        && mins > 0
                        && t.seen.elapsed() >= Duration::from_secs(mins as u64 * 60);
                    ok.then(|| Some((slot, t.view.clone()?)))?
                })
            })
            .collect()
    });
    for (slot, v) in due {
        let v2 = v.clone();
        web::eval_world(&v, BUSY_JS, move |r| {
            if r == Some(Value::Bool(false)) {
                in_slot(slot, || hibernate_view(&v2, false));
            }
        });
    }
}

/// `tab-hibernate [index]`: unload a background tab now (not the active one).
pub fn hibernate(n: Option<usize>) {
    let v = S.with_borrow(|s| {
        let n = n.unwrap_or(s.active);
        if n == s.active {
            return Err("Can't hibernate the tab you're looking at.");
        }
        let t = s.tabs.get(n).ok_or("No such tab.")?;
        t.view.clone().ok_or("Tab is already unloaded.")
    });
    match v {
        Ok(v) => hibernate_view(&v, true),
        Err(e) => message("error", e),
    }
}

/// Keep the session state, drop the view; it comes back on the next show.
fn hibernate_view(view: &WebView, manual: bool) {
    let id = web::id(view);
    let old = S.with_borrow_mut(|s| {
        let n = s.tabs.iter().position(|t| vid(&t.view) == Some(id))?;
        let active = s.active;
        let t = &mut s.tabs[n];
        if n == active || t.crashed.is_some() || (!manual && t.keep) {
            return None;
        }
        t.state = view.session_state().and_then(|st| st.serialize());
        t.asleep = true;
        t.loading = false;
        t.progress = 0.0;
        t.view.take()
    });
    if let (Some(v), Some(st)) = (old, S.with_borrow(|s| s.stack.clone())) {
        st.remove(&v);
        refresh();
    }
}

/// `tab-keep-awake [index]`: toggle the tab's exemption from hibernation.
pub fn keep_awake(n: Option<usize>) {
    let on = S.with_borrow_mut(|s| {
        let n = n.unwrap_or(s.active);
        let t = s.tabs.get_mut(n)?;
        t.keep = !t.keep;
        Some(t.keep)
    });
    match on {
        Some(true) => message("info", "This tab won't be hibernated."),
        Some(false) => message("info", "This tab can be hibernated again."),
        None => message("error", "No such tab."),
    }
    refresh();
}

// ---- layout and theme from config

/// Tab bar side and size from `tabs.position` / `tabs.width` / fonts + paddings.
fn apply_layout() {
    let Some((window, outer, content, tabbar)) = S.with_borrow(|s| {
        Some((s.window.clone()?, s.outer.clone()?, s.content.clone()?, s.tabbar.clone()?))
    }) else {
        return;
    };
    let cfg = config::get();
    let pos = cfg.str("tabs.position");
    let vertical = pos == "left" || pos == "right";
    for w in [tabbar.upcast_ref::<gtk::Widget>(), content.upcast_ref()] {
        if w.parent().is_some() {
            outer.remove(w);
        }
    }
    outer.set_orientation(if vertical {
        gtk::Orientation::Horizontal
    } else {
        gtk::Orientation::Vertical
    });
    if pos == "bottom" || pos == "right" {
        outer.append(&content);
        outer.append(&tabbar);
    } else {
        outer.append(&tabbar);
        outer.append(&content);
    }
    if vertical {
        let ww = window.width().max(window.default_width());
        let w = match cfg.get("tabs.width") {
            Value::String(p) => p
                .trim_end_matches('%')
                .parse::<f64>()
                .map_or(200, |p| (ww as f64 * p / 100.0) as i32),
            v => v.as_i64().unwrap_or(200) as i32,
        };
        tabbar.set_size_request(w.max(1), -1);
        tabbar.set_vexpand(true);
        tabbar.set_hexpand(false);
    } else {
        // + 8: room for the pill tabs' 4 px margin above and below (ui/tabbar.html)
        let h = theme::bar_height(&cfg, "fonts.tabs.selected", "tabs.padding") + 8;
        tabbar.set_size_request(-1, h);
        tabbar.set_hexpand(true);
        tabbar.set_vexpand(false);
    }
    apply_tabs_show();
    place_palette();
    push_theme();
    focus_active();
}

/// `tabs.show`: never / always / multiple.
fn apply_tabs_show() {
    let Some(tabbar) = S.with_borrow(|s| s.tabbar.clone()) else {
        return;
    };
    let show = match config::get().str("tabs.show").as_str() {
        "never" => false,
        // shortcut: "switching" behaves like "multiple"
        "multiple" | "switching" => S.with_borrow(|s| s.tabs.len() > 1),
        _ => true,
    };
    tabbar.set_visible(show);
}

/// Send the CSS variables and layout facts to every chrome view.
fn push_theme() {
    let cfg = config::get();
    let vertical = matches!(cfg.str("tabs.position").as_str(), "left" | "right");
    let json = json!({
        "vars": theme::vars(&cfg),
        "vertical": vertical,
        "listHeight": match cfg.get("completion.height") {
            Value::String(p) if p.ends_with('%') => format!("{}vh", p.trim_end_matches('%')),
            v => format!("{}px", v.as_i64().unwrap_or(300)),
        },
    })
    .to_string();
    let views = S.with_borrow(|s| {
        [&s.tabbar, &s.palette, &s.keyhint, &s.tip, &s.menu, &s.ask, &s.toast]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
    });
    for v in views {
        web::eval_page(&v, &format!("window.nova && nova.theme && nova.theme({json})"));
    }
}

/// After `:config-source` / `:set`: rebuild keys, layout, theme in every window.
pub fn config_changed() {
    modes::init();
    web::apply_config();
    crate::tabview::config_changed();
    let slots: Vec<usize> =
        WINS.with_borrow(|w| (0..w.len()).filter(|&i| w[i].window.is_some()).collect());
    for i in slots {
        in_slot(i, || {
            apply_layout();
            refresh();
        });
    }
}

/// Every loaded tab view in every window.
pub fn all_views() -> Vec<WebView> {
    WINS.with_borrow(|w| {
        w.iter()
            .flat_map(|s| s.tabs.iter().filter_map(|t| t.view.clone()))
            .collect()
    })
}

/// `nova.send` from a chrome view of the current window.
fn ui_message(text: &str) {
    let (cmd, arg) = text.split_once(' ').unwrap_or((text, ""));
    match (cmd, arg.parse::<i64>()) {
        ("ready", _) => {
            push_theme();
            refresh();
        }
        ("tab", Ok(n)) if n >= 0 => show(n as usize),
        ("close", Ok(n)) if n >= 0 => close_user(n as usize, false),
        ("move", Ok(n)) if n >= 0 => move_active(n + 1, false),
        ("run", _) => {
            crate::completion::record(arg);
            close_cmdline();
            crate::commands::run_from_bar(arg);
        }
        ("text", _) => crate::completion::on_text(arg),
        ("complete", Ok(by)) => crate::completion::select(by),
        ("hist", Ok(by)) => crate::completion::history_step(by),
        ("cmdleave", _) => close_cmdline(),
        ("tip", _) => tab_tip(arg),
        ("menu", _) => tab_menu(arg),
        ("menuact", _) => tab_menu_action(arg),
        ("menuclose", _) => close_popup(),
        ("pwpick", Ok(i)) if i >= 0 => crate::passwords::on_click(i as usize),
        ("answer", _) => crate::permissions::on_key(arg),
        _ => eprintln!("novabrowser-webkit: unknown UI message {text:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{load_error_text, move_target};

    #[test]
    fn load_error_wording() {
        let code = |m: &str| load_error_text("h", m).1;
        assert_eq!(code("Could not connect to 100.88.215.101: Connection refused"), "ERR_CONNECTION_REFUSED");
        assert_eq!(code("Error resolving “nope.invalid”: Name or service not known"), "ERR_NAME_NOT_RESOLVED");
        assert_eq!(code("Socket I/O timed out"), "ERR_CONNECTION_TIMED_OUT");
        assert_eq!(code("Could not connect to h: No route to host"), "ERR_ADDRESS_UNREACHABLE");
        assert_eq!(code("Connection terminated unexpectedly"), "ERR_FAILED");
        assert_eq!(load_error_text("h", "Connection refused").0, "h refused to connect.");
    }

    #[test]
    fn move_keeps_pinned_group() {
        assert_eq!(move_target(4, 5, 2, true), 1);
        assert_eq!(move_target(-3, 5, 2, true), 0);
        assert_eq!(move_target(0, 5, 2, false), 2);
        assert_eq!(move_target(3, 5, 2, false), 3);
        assert_eq!(move_target(9, 5, 0, false), 4);
    }
}
