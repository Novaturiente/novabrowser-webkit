//! Which-key (STUB: replaced by the full native which-key).
use super::motion::{Origin, Pop};
use gtk::prelude::*;

pub const CSS: &str = "";

#[derive(Clone)]
pub struct KeyHint(Pop, gtk::Label);

impl KeyHint {
    pub fn new() -> Self {
        let l = gtk::Label::new(None);
        KeyHint(Pop::new(&l, Origin::Bottom, false), l)
    }
    pub fn widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }
    /// rows: (rest of the keys, command)
    pub fn show(&self, prefix: &str, rows: &[(String, String)]) {
        let t: Vec<String> = rows.iter().map(|(k, c)| format!("{prefix}{k}  {c}")).collect();
        self.1.set_text(&t.join("\n"));
        self.0.reveal();
    }
    pub fn hide(&self) {
        self.0.conceal(|| {});
    }
}
