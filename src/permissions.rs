//! Site permissions: qutebrowser's `content.*` true/false/ask settings, a y/n/A/N prompt
//! (a popup under the tab bar, ui/prompt.html) for "ask", and A/N answers saved per origin in <data>/permissions.json
//! (they win over config.ts). JS dialogs, TLS errors and hung pages share the queue.
use crate::{config, modes, web, window};
use gtk::{gio, glib};
use serde_json::{Map, Value};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::{PermissionRequest, ScriptDialog, ScriptDialogType, WebView};

enum Callback {
    Request(PermissionRequest),
    /// alert / confirm / prompt() / before-unload; the String is what "yes" returns to prompt().
    Dialog(ScriptDialog, String),
    /// our own yes/no questions (passwords save/update, TLS exceptions); id for expiry
    Ask(u64, Rc<dyn Fn(bool)>),
    /// hung web process: yes = kill it (crash page, `r` reloads), no = keep waiting
    Hang,
}

struct Pending {
    view: usize,
    origin: String,
    settings: Vec<&'static str>,
    text: String,
    cb: Callback,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<Pending>> = const { RefCell::new(VecDeque::new()) };
    static SAVED: RefCell<Option<Map<String, Value>>> = const { RefCell::new(None) };
    static ASK_ID: Cell<u64> = const { Cell::new(0) };
    /// When the front prompt last changed: keys typed just before it appeared can't answer it (A3).
    static SHOWN: RefCell<(Option<String>, Option<std::time::Instant>)> = const { RefCell::new((None, None)) };
}

fn file() -> PathBuf {
    crate::xdg_dir("XDG_DATA_HOME", ".local/share").join("permissions.json")
}

fn saved(origin: &str, setting: &str) -> Option<bool> {
    SAVED.with_borrow_mut(|s| {
        let s = s.get_or_insert_with(|| {
            std::fs::read_to_string(file())
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default()
        });
        s.get(origin)?.get(setting)?.as_bool()
    })
}

fn save(origin: &str, settings: &[&str], allow: bool) {
    let text = SAVED.with_borrow_mut(|s| {
        let s = s.get_or_insert_with(Map::new);
        let site = s.entry(origin).or_insert_with(|| Value::Object(Map::new()));
        for n in settings {
            site[*n] = Value::Bool(allow);
        }
        serde_json::to_string_pretty(s).unwrap()
    });
    let f = file();
    if let Err(e) = crate::write_atomic(&f, text.as_bytes()) {
        window::message("error", &format!("{}: {e}", f.display()));
    }
}

#[derive(Debug, PartialEq)]
enum Decision {
    Allow,
    Deny,
    Ask,
}

/// Saved answer, else the setting for this origin: true / false / "ask".
fn decide(origin: &str, setting: &str) -> Decision {
    if let Some(b) = saved(origin, setting) {
        return if b { Decision::Allow } else { Decision::Deny };
    }
    let v = config::get().get_for(setting, origin);
    match (setting, v.as_str(), v.as_bool()) {
        (_, _, Some(true)) | ("content.javascript.clipboard", Some("access-paste"), _) => {
            Decision::Allow
        }
        (_, Some("ask"), _) => Decision::Ask,
        _ => Decision::Deny, // false, and clipboard none/access (write-only)
    }
}

fn combine(ds: &[Decision]) -> Decision {
    if ds.contains(&Decision::Deny) {
        Decision::Deny
    } else if ds.contains(&Decision::Ask) {
        Decision::Ask
    } else {
        Decision::Allow
    }
}

/// `scheme://host[:port]` of the page.
fn origin_of(view: &WebView) -> String {
    let u = view.uri().map(|u| u.to_string()).unwrap_or_default();
    match glib::Uri::parse(&u, glib::UriFlags::NONE) {
        Ok(p) => {
            let port = if p.port() > 0 { format!(":{}", p.port()) } else { String::new() };
            format!("{}://{}{port}", p.scheme(), p.host().unwrap_or_default())
        }
        Err(_) => u,
    }
}

/// Saved notification answers, for `initialize-notification-permissions` (WebKit asks
/// at start so pages see `Notification.permission` right away).
pub fn init_notifications(ctx: &webkit6::WebContext) {
    saved("", ""); // load the file
    let (mut yes, mut no) = (Vec::new(), Vec::new());
    SAVED.with_borrow(|s| {
        for (origin, v) in s.iter().flat_map(|m| m.iter()) {
            let Some(b) = v["content.notifications.enabled"].as_bool() else { continue };
            let Ok(u) = glib::Uri::parse(origin, glib::UriFlags::NONE) else { continue };
            let port = u16::try_from(u.port()).unwrap_or(0);
            let o = webkit6::SecurityOrigin::new(&u.scheme(), &u.host().unwrap_or_default(), port);
            if b { yes.push(o) } else { no.push(o) }
        }
    });
    let (y, n): (Vec<_>, Vec<_>) = (yes.iter().collect(), no.iter().collect());
    ctx.initialize_notification_permissions(&y, &n);
}

/// (settings, words for the prompt) for a WebKit permission request; None = deny silently.
fn wanted(req: &PermissionRequest) -> Option<Vec<(&'static str, &'static str)>> {
    use webkit6::*;
    if let Some(m) = req.downcast_ref::<UserMediaPermissionRequest>() {
        let (audio, video) = (m.is_for_audio_device(), m.is_for_video_device());
        let display = webkit6::functions::user_media_permission_is_for_display_device(m);
        let v = media_wanted(audio, video, display);
        let to: Vec<_> = v.iter().map(|w| w.0).collect();
        eprintln!(
            "novabrowser: media request audio={} video={} display={} -> {}",
            audio as u8,
            video as u8,
            display as u8,
            if to.is_empty() { "deny".to_string() } else { to.join(",") }
        );
        return (!v.is_empty()).then_some(v);
    }
    let one = if req.is::<GeolocationPermissionRequest>() {
        ("content.geolocation", "see your location")
    } else if req.is::<NotificationPermissionRequest>() {
        ("content.notifications.enabled", "show notifications")
    } else if req.is::<PointerLockPermissionRequest>() {
        ("content.mouse_lock", "lock your mouse pointer")
    } else if req.is::<ClipboardPermissionRequest>() {
        ("content.javascript.clipboard", "read your clipboard")
    } else if req.is::<WebsiteDataAccessPermissionRequest>() {
        ("content.persistent_storage", "use its cookies on this site")
    } else if req.is::<DeviceInfoPermissionRequest>() {
        // device labels for already allowed media: follows the camera/mic setting
        ("content.media.video_capture", "list your cameras and microphones")
    } else {
        return None; // media keys (D9: no DRM), XR
    };
    Some(vec![one])
}

/// Media request flags -> (setting, prompt words), as the CEF edition's `MEDIA_BITS`.
/// A screen request is asked only under `content.desktop_capture`, never under a saved
/// microphone/camera answer; no flags at all = unknown request, denied.
fn media_wanted(audio: bool, video: bool, display: bool) -> Vec<(&'static str, &'static str)> {
    let mut v = Vec::new();
    if display {
        v.push(("content.desktop_capture", "capture your screen"));
        if audio {
            v.push(("content.desktop_capture", "capture your screen audio"));
        }
        return v;
    }
    if audio {
        v.push(("content.media.audio_capture", "use your microphone"));
    }
    if video {
        v.push(("content.media.video_capture", "use your camera"));
    }
    v
}

fn answer(view: usize, cb: &Callback, allow: bool) {
    match cb {
        Callback::Request(r) if allow => r.allow(),
        Callback::Request(r) => r.deny(),
        Callback::Dialog(d, input) => {
            match d.dialog_type() {
                ScriptDialogType::Confirm | ScriptDialogType::BeforeUnloadConfirm => {
                    d.confirm_set_confirmed(allow)
                }
                ScriptDialogType::Prompt if allow => d.prompt_set_text(input),
                _ => {}
            }
            d.close();
        }
        Callback::Ask(_, f) => f(allow),
        Callback::Hang if allow => {
            if let Some(v) = view_by_id(view) {
                v.terminate_web_process();
            }
        }
        Callback::Hang => {
            if let Some(v) = view_by_id(view) {
                hung_later(&v, 5); // still hung after "keep waiting": ask again
            }
        }
    }
}

fn view_by_id(id: usize) -> Option<WebView> {
    window::all_views().into_iter().find(|v| web::id(v) == id)
}

fn push(view: &WebView, origin: String, settings: Vec<&'static str>, text: String, cb: Callback) {
    let id = web::id(view);
    QUEUE.with_borrow_mut(|q| q.push_back(Pending { view: id, origin, settings, text, cb }));
    show_later();
}

/// `permission-request`: always answered (now or after the prompt), never left hanging.
pub fn request(view: &WebView, req: &PermissionRequest) -> bool {
    let Some(wanted) = wanted(req) else {
        req.deny();
        return true;
    };
    let origin = origin_of(view);
    // device labels: allowed when the mic or the camera is (wanted() asks under camera)
    if req.is::<webkit6::DeviceInfoPermissionRequest>()
        && ["content.media.audio_capture", "content.media.video_capture"]
            .iter()
            .any(|s| decide(&origin, s) == Decision::Allow)
    {
        req.allow();
        return true;
    }
    let d = combine(&wanted.iter().map(|(s, _)| decide(&origin, s)).collect::<Vec<_>>());
    match d {
        Decision::Allow => req.allow(),
        Decision::Deny => req.deny(),
        Decision::Ask => {
            let what: Vec<_> = wanted.iter().map(|w| w.1).collect();
            let text = format!(
                "Allow {origin} to {}?",
                what.join(" and ")
            );
            let mut settings: Vec<&'static str> = wanted.iter().map(|w| w.0).collect();
            settings.dedup();
            push(view, origin, settings, text, Callback::Request(req.clone()));
        }
    }
    true
}

/// `script-dialog`: alert / confirm / prompt / before-unload in the prompt queue.
/// shortcut: prompt() can't take typed text; `y` returns its default value.
pub fn dialog(view: &WebView, d: &ScriptDialog) -> bool {
    let host = crate::commands::host(&view.uri().unwrap_or_default()).to_string();
    let msg = d.message().map(|m| m.to_string()).unwrap_or_default();
    let (text, input) = match d.dialog_type() {
        ScriptDialogType::Alert => (format!("{host} says: {msg}"), String::new()),
        ScriptDialogType::Prompt => {
            let def = d.prompt_get_default_text().map(|t| t.to_string()).unwrap_or_default();
            (format!("{host} says: {msg} (yes answers \"{def}\")"), def)
        }
        ScriptDialogType::BeforeUnloadConfirm => (
            "Leave this page? Changes you made may not be saved.".to_string(),
            String::new(),
        ),
        _ => (format!("{host} says: {msg}"), String::new()),
    };
    push(view, String::new(), Vec::new(), text, Callback::Dialog(d.clone(), input));
    true
}

/// A yes/no question in the prompt queue; `f` gets the answer (Escape = no).
/// The closure may hold a password: a newer question for the tab replaces it, and an
/// unanswered one is dropped (= no) after 5 minutes.
pub fn ask(view: &WebView, text: String, f: impl Fn(bool) + 'static) {
    let id = ASK_ID.replace(ASK_ID.get() + 1);
    let vid = web::id(view);
    glib::timeout_add_local_once(std::time::Duration::from_secs(300), move || {
        QUEUE.with_borrow_mut(|q| q.retain(|p| !matches!(p.cb, Callback::Ask(i, _) if i == id)));
        show();
    });
    QUEUE.with_borrow_mut(|q| q.retain(|p| !(p.view == vid && matches!(p.cb, Callback::Ask(..)))));
    push(view, String::new(), Vec::new(), text, Callback::Ask(id, Rc::new(f)));
}

/// A certificate WebKit refused: offer a one-run exception for this host.
pub fn tls_error(view: &WebView, uri: &str, cert: &gio::TlsCertificate, errors: gio::TlsCertificateFlags) {
    let host = crate::commands::host(uri).to_string();
    let text = format!(
        "{host}: certificate not trusted ({}). Load it anyway this run?",
        tls_words(errors)
    );
    let (w, cert, uri) = (view.downgrade(), cert.clone(), uri.to_string());
    ask(view, text, move |yes| {
        let Some(v) = w.upgrade() else { return };
        if yes {
            if let Some(s) = v.network_session() {
                s.allow_tls_certificate_for_host(&cert, &host);
            }
            v.load_uri(&uri);
        }
    });
}

fn tls_words(e: gio::TlsCertificateFlags) -> String {
    use gio::TlsCertificateFlags as F;
    let names = [
        (F::UNKNOWN_CA, "unknown issuer"),
        (F::BAD_IDENTITY, "wrong host"),
        (F::NOT_ACTIVATED, "not valid yet"),
        (F::EXPIRED, "expired"),
        (F::REVOKED, "revoked"),
        (F::INSECURE, "insecure"),
    ];
    let w: Vec<&str> = names.iter().filter(|(f, _)| e.contains(*f)).map(|n| n.1).collect();
    if w.is_empty() { "invalid".into() } else { w.join(", ") }
}

/// The web process stopped answering.
pub fn hung(view: &WebView) {
    let id = web::id(view);
    if QUEUE.with_borrow(|q| q.iter().any(|p| p.view == id && matches!(p.cb, Callback::Hang))) {
        return;
    }
    push(
        view,
        String::new(),
        Vec::new(),
        "This page isn't responding. Kill it?".into(),
        Callback::Hang,
    );
}

/// Ask about a hung page only if it still hangs `secs` from now.
pub fn hung_later(view: &WebView, secs: u32) {
    let v = view.downgrade(); // a tab closed meanwhile isn't asked about
    glib::timeout_add_seconds_local_once(secs, move || {
        if let Some(v) = v.upgrade().filter(|v| !v.is_web_process_responsive()) {
            hung(&v);
        }
    });
}

/// The page recovered: its hang question is moot.
pub fn responsive(view: &WebView) {
    let id = web::id(view);
    QUEUE.with_borrow_mut(|q| q.retain(|p| !(p.view == id && matches!(p.cb, Callback::Hang))));
    show_later();
}

/// `show()` later: signals may fire from inside window code that holds its state.
fn show_later() {
    glib::idle_add_local_once(show);
}

/// The answer buttons for a prompt: [key, label]; keys are what `on_key` takes.
fn choices(p: &Pending) -> Vec<[&'static str; 2]> {
    match &p.cb {
        Callback::Request(_) if !can_always(&p.settings) => vec![["y", "Yes"], ["n", "No"], ["N", "Never"]],
        Callback::Request(_) => vec![["y", "Yes"], ["n", "No"], ["A", "Always"], ["N", "Never"]],
        Callback::Dialog(d, _) if d.dialog_type() == ScriptDialogType::Alert => vec![["y", "OK"]],
        Callback::Hang => vec![["y", "Kill page"], ["n", "Keep waiting"]],
        _ => vec![["y", "Yes"], ["n", "No"]],
    }
}

/// Screen capture can't be allowed for good (Screen Capture spec: never stored as granted).
fn can_always(settings: &[&str]) -> bool {
    !settings.contains(&"content.desktop_capture")
}

/// Show the oldest open prompt, or leave prompt mode when there is none.
fn show() {
    let front = QUEUE.with_borrow(|q| q.front().map(|p| (p.text.clone(), choices(p))));
    let text = front.as_ref().map(|f| f.0.clone());
    SHOWN.with_borrow_mut(|s| {
        if s.0 != text {
            *s = (text.clone(), Some(std::time::Instant::now()));
        }
    });
    let on = text.is_some();
    window::set_prompt(front);
    match (on, modes::mode() == modes::Mode::Prompt) {
        (true, false) => {
            if modes::mode() == modes::Mode::Command {
                window::close_cmdline();
            }
            modes::enter(modes::Mode::Prompt);
            window::focus_active();
        }
        (false, true) => {
            modes::enter(modes::Mode::Normal);
            window::focus_active();
        }
        _ => {}
    }
}

/// Prompt-mode key: y / n / A / N, Escape = no. Other keys are eaten.
pub fn on_key(name: &str) {
    let (allow, keep) = match name {
        "y" => (true, false),
        "n" | "Escape" => (false, false),
        "A" => (true, true),
        "N" => (false, true),
        _ => return,
    };
    if name != "Escape" && !grace_over(SHOWN.with_borrow(|s| s.1)) {
        return;
    }
    if name == "A" && QUEUE.with_borrow(|q| q.front().is_some_and(|p| !can_always(&p.settings))) {
        return;
    }
    let Some(p) = QUEUE.with_borrow_mut(|q| q.pop_front()) else {
        return show();
    };
    if keep && !p.settings.is_empty() {
        save(&p.origin, &p.settings, allow);
    }
    answer(p.view, &p.cb, allow);
    show();
}

/// 400 ms after a prompt appears before y/n/A/N count (Escape always does: it's "no").
fn grace_over(shown: Option<std::time::Instant>) -> bool {
    shown.is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(400))
}

/// Tab closed or hibernated: answer its open questions with no.
pub fn forget(view: Option<&WebView>) {
    let Some(view) = view else { return };
    let id = web::id(view);
    let gone: Vec<Pending> = QUEUE.with_borrow_mut(|q| {
        let (gone, keep): (Vec<Pending>, Vec<Pending>) = q.drain(..).partition(|p| p.view == id);
        *q = keep.into();
        gone
    });
    for p in gone {
        if !matches!(p.cb, Callback::Hang) {
            answer(id, &p.cb, false);
        }
    }
    show_later();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_any_deny_wins_then_ask() {
        use Decision::*;
        assert_eq!(combine(&[Allow, Ask]), Ask);
        assert_eq!(combine(&[Ask, Deny]), Deny);
        assert_eq!(combine(&[Allow, Allow]), Allow);
        assert_eq!(combine(&[]), Allow);
    }

    #[test]
    fn keys_right_after_a_prompt_dont_answer_it() {
        use std::time::{Duration, Instant};
        assert!(!grace_over(Some(Instant::now())));
        assert!(grace_over(Instant::now().checked_sub(Duration::from_secs(1))));
        assert!(grace_over(None));
    }

    #[test]
    fn media_wanted_cases() {
        let s = |v: Vec<(&'static str, &'static str)>| v.into_iter().map(|w| w.0).collect::<Vec<_>>();
        let screen = ["content.desktop_capture"];
        assert_eq!(s(media_wanted(false, false, true)), screen);
        assert_eq!(s(media_wanted(false, true, true)), screen);
        // display with audio: screen audio words, never the microphone setting
        let both = media_wanted(true, true, true);
        assert!(both.iter().all(|w| w.0 == "content.desktop_capture"));
        assert_eq!(both[1].1, "capture your screen audio");
        assert_eq!(s(media_wanted(true, false, false)), ["content.media.audio_capture"]);
        assert_eq!(s(media_wanted(false, true, false)), ["content.media.video_capture"]);
        assert_eq!(
            s(media_wanted(true, true, false)),
            ["content.media.audio_capture", "content.media.video_capture"]
        );
        assert!(media_wanted(false, false, false).is_empty());
    }

    #[test]
    fn screen_prompt_offers_no_always() {
        assert!(!can_always(&["content.desktop_capture"]));
        assert!(can_always(&["content.media.audio_capture", "content.media.video_capture"]));
    }

    #[test]
    fn tls_words_name_the_problems() {
        use gio::TlsCertificateFlags as F;
        assert_eq!(tls_words(F::EXPIRED | F::BAD_IDENTITY), "wrong host, expired");
        assert_eq!(tls_words(F::empty()), "invalid");
    }
}
