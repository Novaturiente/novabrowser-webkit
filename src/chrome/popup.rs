//! Tooltip, tab menu, password list (STUB: replaced by the full native popups).
use super::motion::{Origin, Pop};
use super::{Ev, Sink};
use gtk::prelude::*;
use serde_json::Value;
use std::cell::Cell;
use std::rc::Rc;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct Tip(Pop, gtk::Label);

impl Tip {
    pub fn new() -> Self {
        let l = gtk::Label::new(None);
        Tip(Pop::new(&l, Origin::Top, false), l)
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }
    pub fn show(&self, title: &str, url: &str) {
        self.1.set_text(&format!("{title}\n{url}"));
        self.0.reveal();
    }
    pub fn hide(&self) {
        self.0.conceal(|| {});
    }
    pub fn shown_size(&self) -> Option<(i32, i32)> {
        self.0.is_shown().then(|| (self.0.width(), self.0.height()))
    }
}

#[derive(Clone)]
pub struct Menu(Rc<MenuInner>);

struct MenuInner {
    pop: Pop,
    list: gtk::Box,
    tab: Cell<usize>,
    sink: Sink,
}

impl Menu {
    pub fn new(sink: Sink) -> Self {
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let pop = Pop::new(&list, Origin::Top, true);
        Menu(Rc::new(MenuInner { pop, list, tab: Cell::new(0), sink }))
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.pop.clone().upcast()
    }
    /// items: (action, label, key hint)
    pub fn show(&self, tab: usize, items: &'static [(&'static str, &'static str, &'static str)]) {
        self.0.tab.set(tab);
        while let Some(c) = self.0.list.first_child() {
            self.0.list.remove(&c);
        }
        for (a, l, _) in items {
            let b = gtk::Button::with_label(l);
            let (s, a) = (self.0.sink.clone(), *a);
            b.connect_clicked(move |_| s(Ev::MenuAct(tab, a)));
            self.0.list.append(&b);
        }
        self.0.pop.reveal();
    }
    pub fn hide(&self) {
        self.0.pop.conceal(|| {});
    }
    pub fn is_shown(&self) -> bool {
        self.0.pop.is_shown()
    }
    /// A key while the menu is open (j/k/Up/Down/Tab/Return/Escape); true = handled.
    pub fn key(&self, name: &str) -> bool {
        if !self.is_shown() {
            return false;
        }
        if name == "Escape" {
            (self.0.sink)(Ev::MenuClose);
        }
        true
    }
}

#[derive(Clone)]
pub struct Suggest(Pop, gtk::Label, Sink);

impl Suggest {
    pub fn new(sink: Sink) -> Self {
        let l = gtk::Label::new(None);
        Suggest(Pop::new(&l, Origin::Top, true), l, sink)
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }
    /// data: {title, rows: [{a, b, sel}]}
    pub fn show(&self, data: &Value) {
        self.1.set_text(&data.to_string());
        self.0.reveal();
    }
    pub fn hide(&self) {
        self.0.conceal(|| {});
    }
    pub fn shown_size(&self) -> Option<(i32, i32)> {
        self.0.is_shown().then(|| (self.0.width(), self.0.height()))
    }
}
