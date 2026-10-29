//! Overlay fades (W15). A hidden WebView doesn't repaint, so one shown at once first
//! flashes the frame it drew before it was hidden. Overlays are shown transparent,
//! filled, and faded in once their page painted; closing fades them out.
use gtk::glib::{self, ControlFlow};
use gtk::{gio, prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;
use webkit6::WebView;

const IN_MS: f64 = 200.0;
const OUT_MS: f64 = 140.0;

thread_local! {
    /// Running fade per widget: its tick callback and target opacity.
    static FADES: RefCell<HashMap<usize, (gtk::TickCallbackId, f64)>> = RefCell::default();
}

fn key(w: &gtk::Widget) -> usize {
    w.as_ptr() as usize
}

fn animations() -> bool {
    gtk::Settings::default().is_none_or(|s| s.is_gtk_enable_animations())
}

fn cancel(w: &gtk::Widget) {
    if let Some((id, _)) = FADES.with_borrow_mut(|f| f.remove(&key(w))) {
        id.remove();
    }
}

fn fading_out(w: &gtk::Widget) -> bool {
    FADES.with_borrow(|f| f.get(&key(w)).is_some_and(|x| x.1 == 0.0))
}

/// Animate `w`'s opacity to `to` (ease-in-out: an ease-out dimmed a white page by half
/// in two frames, which reads as a flash), then run `done`. Replaces a running fade.
fn fade(w: &gtk::Widget, to: f64, ms: f64, done: impl Fn(&gtk::Widget) + 'static) {
    cancel(w);
    let from = w.opacity();
    if !animations() || (from - to).abs() < 0.01 {
        w.set_opacity(to);
        return done(w);
    }
    let start = Cell::new(None::<i64>);
    let id = w.add_tick_callback(move |w, clock| {
        let now = clock.frame_time();
        let t0 = start.get().unwrap_or(now);
        start.set(Some(t0));
        let t = ((now - t0) as f64 / 1000.0 / ms).min(1.0);
        w.set_opacity(from + (to - from) * (1.0 - (t * std::f64::consts::PI).cos()) / 2.0);
        if t < 1.0 {
            return ControlFlow::Continue;
        }
        // Break removes the callback: drop its id without remove()
        FADES.with_borrow_mut(|f| f.remove(&key(w)));
        done(w);
        ControlFlow::Break
    });
    FADES.with_borrow_mut(|f| f.insert(key(w), (id, to)));
}

/// Show `view` with `js` applied: transparent until its page painted that, then faded
/// in together with `also` (the palette's backdrop). Shown or showing: just run `js`.
pub fn reveal(view: &WebView, js: &str, also: &[gtk::Widget]) {
    let w: &gtk::Widget = view.upcast_ref();
    if w.is_visible() && !fading_out(w) {
        return crate::web::eval_page(view, js);
    }
    let all: Vec<gtk::Widget> = std::iter::once(w.clone()).chain(also.iter().cloned()).collect();
    for x in &all {
        cancel(x);
        if !x.is_visible() {
            x.set_opacity(0.0);
            x.set_visible(true);
        }
    }
    let started = Rc::new(Cell::new(false));
    let start = Rc::new(move || {
        // late (timeout after the paint, or closed meanwhile): leave it be
        if started.replace(true) || !all[0].is_visible() || fading_out(&all[0]) {
            return;
        }
        for x in &all {
            fade(x, 1.0, IN_MS, |_| {});
        }
    });
    // the first rAF runs before the frame with `js` applied is drawn; the second after
    let body = format!("{js};\nawait new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));");
    let s = start.clone();
    view.call_async_javascript_function(&body, None, None, None, None::<&gio::Cancellable>, move |_| s());
    // never left invisible when the page doesn't paint (busy or restarting web process)
    glib::timeout_add_local_once(Duration::from_millis(250), move || start());
}

/// Draw `ws` once, almost transparent, and hide them again: the first draw of a view
/// (and of GTK's blur) dropped 2-3 frames of the first fade-in. Counts as a fade-out,
/// so a `reveal` meanwhile takes over.
pub fn warm(ws: &[gtk::Widget], done: impl Fn() + 'static) {
    let done = Rc::new(done);
    // shortcut: a click in these 100 ms lands on the invisible palette (closes it, a no-op)
    for w in ws.iter().filter(|w| !w.is_visible()) {
        w.set_opacity(0.02);
        w.set_visible(true);
        let d = done.clone();
        fade(w, 0.0, 100.0, move |w| {
            w.set_visible(false);
            w.set_opacity(1.0);
            d();
        });
    }
}

/// Fade `w` out, then hide it and run `done`.
pub fn conceal(w: &impl IsA<gtk::Widget>, done: impl Fn() + 'static) {
    let w: &gtk::Widget = w.upcast_ref();
    if !w.is_visible() {
        cancel(w);
        return done();
    }
    fade(w, 0.0, OUT_MS, move |w| {
        w.set_visible(false);
        // a direct set_visible(true) elsewhere still shows it
        w.set_opacity(1.0);
        done();
    });
}
