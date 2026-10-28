//! Clipboard and primary selection through GdkClipboard (no wl-copy / xclip).
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Reads larger than this are cut (a huge clipboard shouldn't land in a command line).
const MAX: usize = 1 << 20;

fn board(primary: bool) -> Option<gdk::Clipboard> {
    let d = gdk::Display::default()?;
    Some(if primary {
        d.primary_clipboard()
    } else {
        d.clipboard()
    })
}

pub fn write(text: &str, primary: bool) {
    if let Some(c) = board(primary) {
        c.set_text(text);
    }
}

/// Read one board; `f` gets "" on error or after 2 s.
fn read(primary: bool, f: impl FnOnce(String) + 'static) {
    let Some(c) = board(primary) else { return f(String::new()) };
    let f: Rc<RefCell<Option<Box<dyn FnOnce(String)>>>> = Rc::new(RefCell::new(Some(Box::new(f))));
    let cancel = gio::Cancellable::new();
    let (f2, c2) = (f.clone(), cancel.clone());
    glib::timeout_add_local_once(Duration::from_secs(2), move || {
        c2.cancel();
        if let Some(f) = f2.take() {
            f(String::new());
        }
    });
    c.read_text_async(Some(&cancel), move |r| {
        let mut t = r.ok().flatten().map(|s| s.to_string()).unwrap_or_default();
        if t.len() > MAX {
            let mut cut = MAX;
            while !t.is_char_boundary(cut) {
                cut -= 1;
            }
            t.truncate(cut);
        }
        if let Some(f) = f.take() {
            f(t);
        }
    });
}

/// Read clipboard and primary, then run `line` with {clipboard}/{primary} filled.
pub fn expand_then_run(line: &str, count: Option<u32>) {
    let line = line.to_string();
    read(false, move |clip| {
        read(true, move |primary| {
            crate::commands::run_with_clipboard(&line, count, clip, primary);
        })
    });
}
