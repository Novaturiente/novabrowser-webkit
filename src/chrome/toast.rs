//! Message toast (STUB: replaced by the full native toast).
use super::motion::{Origin, Pop};
use gtk::prelude::*;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct Toast(Pop, gtk::Label);

impl Toast {
    pub fn new() -> Self {
        let l = gtk::Label::new(None);
        Toast(Pop::new(&l, Origin::Bottom, false), l)
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }
    /// level: info / warning / error / find ("query\tpos / total")
    pub fn show(&self, _level: &str, text: &str) {
        self.1.set_text(text);
        self.0.reveal();
    }
    pub fn hide(&self) {
        self.0.conceal(|| {});
    }
}
