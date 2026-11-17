//! Command palette (STUB: replaced by the full native palette).
use super::motion::{Origin, Pop};
use super::{Ev, ListHeight, Sink};
use gtk::{gdk, prelude::*};
use serde_json::Value;
use std::cell::Cell;
use std::rc::Rc;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct Palette(Rc<Inner>);

struct Inner {
    pop: Pop,
    entry: gtk::Text,
    quiet: Cell<bool>,
}

impl Palette {
    pub fn new(sink: Sink) -> Self {
        let entry = gtk::Text::new();
        let pop = Pop::new(&entry, Origin::Center, true);
        pop.set_valign(gtk::Align::Start);
        let inner = Rc::new(Inner { pop, entry: entry.clone(), quiet: Cell::new(false) });
        let w = Rc::downgrade(&inner);
        let s = sink.clone();
        entry.connect_changed(move |e| {
            if w.upgrade().is_some_and(|i| !i.quiet.get()) {
                s(Ev::Text(e.text().to_string()));
            }
        });
        let s = sink.clone();
        entry.connect_activate(move |e| s(Ev::Run(e.text().to_string())));
        let k = gtk::EventControllerKey::new();
        k.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                sink(Ev::CmdLeave);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        entry.add_controller(k);
        Palette(inner)
    }

    /// The overlay child: fills the content area.
    pub fn widget(&self) -> gtk::Widget {
        self.0.pop.clone().upcast()
    }

    pub fn open(&self, text: &str, _compact: bool) {
        self.set_text(text);
        self.0.pop.reveal();
        self.0.entry.grab_focus();
        self.0.entry.set_position(-1);
    }

    pub fn close(&self) {
        self.0.pop.conceal(|| {});
    }

    pub fn is_shown(&self) -> bool {
        self.0.pop.is_shown()
    }

    pub fn set_text(&self, text: &str) {
        self.0.quiet.set(true);
        self.0.entry.set_text(text);
        self.0.entry.set_position(-1);
        self.0.quiet.set(false);
    }

    pub fn complete(&self, _data: Option<&Value>) {}

    pub fn icons(&self, _icons: &[(String, gdk::Texture)]) {}

    pub fn count(&self, _text: &str) {}

    pub fn set_list_height(&self, _h: ListHeight) {}
}
