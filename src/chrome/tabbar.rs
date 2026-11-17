//! Tab bar (STUB: replaced by the full native tab strip).
use super::{Ev, Sink, Status, TabItem};
use gtk::prelude::*;
use std::rc::Rc;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct TabBar(Rc<Inner>);

struct Inner {
    root: gtk::Box,
    sink: Sink,
}

impl TabBar {
    pub fn new(sink: Sink) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        root.add_css_class("nova-tabbar");
        root.set_focusable(true);
        TabBar(Rc::new(Inner { root, sink }))
    }

    pub fn widget(&self) -> gtk::Widget {
        self.0.root.clone().upcast()
    }

    pub fn set_vertical(&self, vertical: bool) {
        self.0.root.set_orientation(if vertical { gtk::Orientation::Vertical } else { gtk::Orientation::Horizontal });
    }

    pub fn apply(&self, tabs: &[TabItem], status: &Status) {
        while let Some(c) = self.0.root.first_child() {
            self.0.root.remove(&c);
        }
        for (i, t) in tabs.iter().enumerate() {
            let b = gtk::Button::with_label(&t.label);
            let sink = self.0.sink.clone();
            b.connect_clicked(move |_| sink(Ev::Tab(i)));
            self.0.root.append(&b);
        }
        self.0.root.append(&gtk::Label::new(Some(&status.mode)));
    }
}
