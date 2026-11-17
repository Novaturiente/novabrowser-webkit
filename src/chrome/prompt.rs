//! Permission question (STUB: replaced by the full native prompt).
use super::motion::{Origin, Pop};
use super::{Ev, Sink};
use gtk::prelude::*;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct Prompt(Pop, gtk::Box, Sink);

impl Prompt {
    pub fn new(sink: Sink) -> Self {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
        Prompt(Pop::new(&b, Origin::Top, true), b, sink)
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }
    /// opts: [key, label]
    pub fn show(&self, question: &str, opts: &[[&'static str; 2]]) {
        while let Some(c) = self.1.first_child() {
            self.1.remove(&c);
        }
        self.1.append(&gtk::Label::new(Some(question)));
        for [k, l] in opts {
            let b = gtk::Button::with_label(l);
            let (s, k) = (self.2.clone(), k.to_string());
            b.connect_clicked(move |_| s(Ev::Answer(k.clone())));
            self.1.append(&b);
        }
        self.0.reveal();
    }
    pub fn hide(&self) {
        self.0.conceal(|| {});
    }
}
