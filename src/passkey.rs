//! Passkeys (WebAuthn) for pages. WebKitGTK is built without WebAuthn and has no Linux
//! transport, so `ui/passkey.js` gives the main frame `navigator.credentials` /
//! `PublicKeyCredential`, `ui/passkey-bridge.js` (our world, top frame) forwards requests
//! here, and the phone does the ceremony over caBLE/hybrid (QR code + Bluetooth) with
//! webauthn-authenticator-rs (vendored, see vendor/README.md). Spike: spikes/w19-passkey.
//!
//! This side owns the security checks: the origin from our world's `location` must match
//! the view's URI, rpId must be that host or a parent domain below a public suffix, and
//! clientDataJSON is written here, browser-style.
use gtk::{gdk, glib, prelude::*};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};
use webauthn_authenticator_rs::{
    cable::connect_cable_authenticator,
    types::{CableRequestType, CableState, EnrollSampleStatus},
    ui::UiCallback,
    AuthenticatorBackendHashedClientData,
};
use webauthn_rs_proto::{CreationChallengeResponse, PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions, RequestChallengeResponse};
use webkit6::{javascriptcore, prelude::*, UserContentInjectedFrames, UserContentManager, UserScript, UserScriptInjectionTime, WebView};

const HANDLER: &str = "novaPasskey";
const PAGE_JS: &str = include_str!("../ui/passkey.js");
const BRIDGE_JS: &str = include_str!("../ui/passkey-bridge.js");

/// Page scripts and the reply handler (in our world, so the page can't post to it).
pub fn add_scripts(ucm: &UserContentManager) {
    ucm.add_script(&UserScript::new(
        PAGE_JS,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::Start,
        &["https://*/*"],
        &[],
    ));
    ucm.add_script(&UserScript::for_world(
        BRIDGE_JS,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::Start,
        crate::web::WORLD,
        &["https://*/*"],
        &[],
    ));
    ucm.register_script_message_handler_with_reply(HANDLER, Some(crate::web::WORLD));
}

pub fn connect(ucm: &UserContentManager, view: &WebView) {
    let weak = view.downgrade();
    ucm.connect_script_message_with_reply_received(Some(HANDLER), move |_, value, reply| {
        let Some(view) = weak.upgrade() else { return false };
        let Some(ctx) = value.context() else { return false };
        let reply = reply.clone();
        let answer: Answer = Box::new(move |s: String| reply.return_value(&javascriptcore::Value::new_string(&ctx, Some(&s))));
        on_message(&view, &value.to_str(), answer);
        true
    });
}

type Answer = Box<dyn FnOnce(String)>;

/// The one open request: its answer, QR window and the page request id it serves.
/// Dropping it drops `_cancel`, which stops the worker's caBLE connect.
struct Open {
    gen: u64,
    view: glib::WeakRef<WebView>,
    id: i64,
    answer: Answer,
    dialog: gtk::Window,
    nav: Option<glib::SignalHandlerId>,
    _cancel: tokio::sync::oneshot::Sender<()>,
}

/// After the user cancels, the same tab can't ask again for this long (a page re-asking
/// in a loop would keep the modal window up and the tab unclosable).
const COOLDOWN: Duration = Duration::from_secs(3);

thread_local! {
    static OPEN: RefCell<Option<Open>> = const { RefCell::new(None) };
    static GEN: Cell<u64> = const { Cell::new(0) };
    static CANCELLED: RefCell<(glib::WeakRef<WebView>, Option<Instant>)> = RefCell::new((glib::WeakRef::new(), None));
}

/// One runtime for every request: caBLE's websocket and Bluetooth tasks run on it.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("passkey")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

fn error(name: &str, message: &str) -> String {
    json!({ "error": name, "message": message }).to_string()
}

/// Answer and close request `gen` if it is still the open one.
fn finish(gen: u64, reply: String) {
    let Some(o) = OPEN.with_borrow_mut(|o| if o.as_ref().is_some_and(|o| o.gen == gen) { o.take() } else { None }) else { return };
    if let (Some(v), Some(h)) = (o.view.upgrade(), o.nav) {
        v.disconnect(h);
    }
    (o.answer)(reply);
    o.dialog.close();
}

enum Request {
    Get(PublicKeyCredentialRequestOptions),
    Create(PublicKeyCredentialCreationOptions),
}

fn on_message(view: &WebView, msg: &str, answer: Answer) {
    let Ok(m) = serde_json::from_str::<Value>(msg) else { return answer(error("TypeError", "bad request")) };
    let id = m["id"].as_i64().unwrap_or(-1);
    if m["op"] == "cancel" {
        let target = m["target"].as_i64();
        let gen = OPEN.with_borrow(|o| o.as_ref().filter(|o| Some(o.id) == target && o.view.upgrade().as_ref() == Some(view)).map(|o| o.gen));
        if let Some(gen) = gen {
            finish(gen, error("AbortError", "aborted"));
        }
        return answer("{}".into());
    }
    if OPEN.with_borrow(|o| o.is_some()) {
        return answer(error("NotAllowedError", "Another passkey request is open."));
    }
    // only the tab the user is looking at, in the active window, may open the QR window
    let active = view.is_mapped() && view.root().and_downcast::<gtk::Window>().is_some_and(|w| w.is_active());
    let cooling = CANCELLED.with_borrow(|(v, t)| v.upgrade().as_ref() == Some(view) && t.is_some_and(|t| t.elapsed() < COOLDOWN));
    if !active || cooling {
        return answer(error("NotAllowedError", "The tab is not in the foreground."));
    }
    // origin: our world's self.origin (unfakeable by the page; the bridge refuses opaque
    // ones) must be the view's own; a mismatch means the page navigated meanwhile
    let origin = m["origin"].as_str().unwrap_or("");
    let view_origin = view.uri().and_then(|u| url::Url::parse(&u).ok()).map(|u| u.origin().ascii_serialization());
    if view_origin.as_deref() != Some(origin) {
        return answer(error("NotAllowedError", "The page changed."));
    }
    // WebIDL defaults the page may leave out (webauthn-rs-proto requires them)
    let host = url::Url::parse(origin).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default();
    let mut pk = m["publicKey"].clone();
    if let Some(o) = pk.as_object_mut() {
        if m["op"] == "get" {
            o.entry("rpId").or_insert(json!(host));
            o.entry("allowCredentials").or_insert(json!([]));
            o.entry("userVerification").or_insert(json!("preferred"));
        } else if let Some(rp) = o.get_mut("rp").and_then(Value::as_object_mut) {
            rp.entry("id").or_insert(json!(host));
        }
    }
    let options = json!({ "publicKey": pk });
    let request = match m["op"].as_str() {
        Some("get") => serde_json::from_value::<RequestChallengeResponse>(options).map(|o| Request::Get(o.public_key)),
        Some("create") => serde_json::from_value::<CreationChallengeResponse>(options).map(|o| Request::Create(o.public_key)),
        _ => return answer(error("NotSupportedError", "unknown operation")),
    };
    let request = match request {
        Ok(r) => r,
        Err(e) => return answer(error("NotSupportedError", &format!("options: {e}"))),
    };
    let rp_id = match &request {
        Request::Get(o) => o.rp_id.clone(),
        Request::Create(o) => o.rp.id.clone(),
    };
    let Some(host) = rp_host(origin, &rp_id) else {
        return answer(error("SecurityError", "The relying party ID is not a registrable domain suffix of the origin."));
    };

    let gen = GEN.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    let (dialog, qr, status) = qr_window(view, &host, matches!(request, Request::Create(_)));
    // the page navigating away ends its request
    let nav = view.connect_load_changed(move |_, e| {
        if matches!(e, webkit6::LoadEvent::Started | webkit6::LoadEvent::Committed) {
            finish(gen, error("AbortError", "The page changed."));
        }
    });
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    OPEN.with_borrow_mut(|o| *o = Some(Open { gen, view: view.downgrade(), id, answer, dialog: dialog.clone(), nav: Some(nav), _cancel: cancel }));
    let v = view.downgrade();
    dialog.connect_close_request(move |_| {
        if OPEN.with_borrow(|o| o.as_ref().is_some_and(|o| o.gen == gen)) {
            CANCELLED.set((v.clone(), Some(Instant::now())));
        }
        finish(gen, error("NotAllowedError", "Cancelled."));
        glib::Propagation::Proceed
    });
    dialog.present();

    let (tx, rx) = mpsc::channel();
    let origin = origin.to_string();
    let spawned = std::thread::Builder::new().name("passkey".into()).spawn(move || {
        let out = run(&origin, request, Ui(tx.clone()), cancelled);
        let _ = tx.send(Ev::Done(out));
    });
    if spawned.is_err() {
        return finish(gen, error("NotAllowedError", "The passkey request failed."));
    }
    let v = view.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if v.upgrade().is_none() {
            finish(gen, error("AbortError", "The tab closed."));
        }
        loop {
            match rx.try_recv() {
                Ok(Ev::Qr(u)) => qr.set_paintable(qr_texture(&u).as_ref()),
                Ok(Ev::Status(s)) => status.set_text(&s),
                Ok(Ev::Done(out)) => {
                    finish(gen, out);
                    return glib::ControlFlow::Break;
                }
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                // worker gone without an answer (panic)
                Err(mpsc::TryRecvError::Disconnected) => {
                    finish(gen, error("NotAllowedError", "The passkey request failed."));
                    return glib::ControlFlow::Break;
                }
            }
        }
    });
}

/// `origin`'s host if `rp_id` may be used on it (WebAuthn §5.1.3 step 8): https, a domain
/// (not an IP), rpId equal to it or a parent domain, and rpId not a public suffix.
fn rp_host(origin: &str, rp_id: &str) -> Option<String> {
    let u = url::Url::parse(origin).ok()?;
    let Some(url::Host::Domain(host)) = u.host() else { return None };
    // browsers refuse non-canonical rpIds; lowercasing here would hash a different rpId
    let rp = rp_id;
    if rp != rp.to_ascii_lowercase() {
        return None;
    }
    let under = host == rp || host.ends_with(&format!(".{rp}"));
    let suffix = psl::suffix_str(rp) == Some(rp);
    (u.scheme() == "https" && under && !suffix && !rp.is_empty()).then(|| host.to_string())
}

/// clientDataJSON as browsers write it. webauthn-authenticator-rs's own one serialises the
/// origin as a URL (trailing slash) plus `tokenBinding: null`; Google rejects that.
fn client_data(kind: &str, challenge: &[u8], origin: &str) -> Vec<u8> {
    use base64::Engine;
    let ch = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(challenge);
    format!(r#"{{"type":{},"challenge":{},"origin":{},"crossOrigin":false}}"#, json!(kind), json!(ch), json!(origin)).into_bytes()
}

enum Ev {
    Qr(String),
    Status(String),
    Done(String),
}

#[derive(Debug)]
struct Ui(mpsc::Sender<Ev>);

impl UiCallback for Ui {
    fn request_pin(&self) -> Option<String> {
        None
    }
    fn request_touch(&self) {}
    fn processing(&self) {
        let _ = self.0.send(Ev::Status("Waiting for the site…".into()));
    }
    fn fingerprint_enrollment_feedback(&self, _: u32, _: Option<EnrollSampleStatus>) {}
    fn cable_qr_code(&self, _: CableRequestType, url: String) {
        let _ = self.0.send(Ev::Qr(url));
    }
    fn dismiss_qr_code(&self) {}
    fn cable_status_update(&self, s: CableState) {
        let text = match s {
            CableState::ConnectingToTunnelServer => "Connecting…",
            CableState::Handshaking => "Connected to your phone",
            CableState::WaitingForAuthenticatorResponse => "Approve on your phone",
            _ => return,
        };
        let _ = self.0.send(Ev::Status(text.into()));
    }
}

/// One ceremony over caBLE on a worker thread; the JSON reply for passkey.js. Ends on
/// `cancelled` (the request was closed) or the timeout while connecting; after the phone
/// connects, `timeout` bounds the exchange (vendored patch: upstream ignored it).
/// shortcut: cancel isn't watched once connected, so the thread lives up to `timeout`
/// (≤ 5 min) after the page got its answer; fine while one request is open at a time.
fn run(origin: &str, request: Request, ui: Ui, cancelled: tokio::sync::oneshot::Receiver<()>) -> String {
    let (kind, challenge, timeout) = match &request {
        Request::Get(o) => ("webauthn.get", o.challenge.to_vec(), o.timeout),
        Request::Create(o) => ("webauthn.create", o.challenge.to_vec(), o.timeout),
    };
    let cd = client_data(kind, &challenge, origin);
    let hash = openssl::sha::sha256(&cd).to_vec();
    let timeout = timeout.unwrap_or(120_000).clamp(30_000, 300_000);
    let out = runtime().block_on(async {
        let t = if matches!(request, Request::Get(_)) { CableRequestType::GetAssertion } else { CableRequestType::DiscoverableMakeCredential };
        let connect = tokio::time::timeout(Duration::from_millis(timeout.into()), connect_cable_authenticator(t, &ui));
        let mut a = tokio::select! {
            r = connect => r.map_err(|_| "caBLE: timed out".to_string())?.map_err(|e| format!("caBLE: {e:?}"))?,
            _ = cancelled => return Err("cancelled".to_string()),
        };
        {
            match request {
                Request::Get(o) => a.perform_auth(hash, o, timeout).map(|mut c| {
                    c.response.client_data_json = cd;
                    json!(c)
                }),
                Request::Create(o) => a.perform_register(hash, o, timeout).map(|mut c| {
                    c.response.client_data_json = cd;
                    json!(c)
                }),
            }
            .map_err(|e| format!("{e:?}"))
        }
    });
    match out {
        Ok(c) => json!({ "credential": c }).to_string(),
        Err(e) => {
            eprintln!("novabrowser-webkit: passkey: {e}");
            error("NotAllowedError", "The passkey request failed or was cancelled.")
        }
    }
}

/// Modal window over the tab's window: who asks, the QR code, status, Cancel.
fn qr_window(view: &WebView, host: &str, create: bool) -> (gtk::Window, gtk::Picture, gtk::Label) {
    let qr = gtk::Picture::builder().width_request(320).height_request(320).build();
    let status = gtk::Label::new(Some("Starting…"));
    let title = gtk::Label::new(None);
    let what = if create { "Create a passkey for" } else { "Sign in with a passkey to" };
    title.set_markup(&format!("{what} <b>{}</b>", glib::markup_escape_text(host)));
    let hint = gtk::Label::new(Some("Scan with your phone's camera. Bluetooth must be on."));
    let cancel = gtk::Button::with_label("Cancel");
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 12);
    bx.set_margin_top(20);
    bx.set_margin_bottom(20);
    bx.set_margin_start(20);
    bx.set_margin_end(20);
    for w in [title.upcast_ref::<gtk::Widget>(), hint.upcast_ref(), qr.upcast_ref(), status.upcast_ref(), cancel.upcast_ref()] {
        bx.append(w);
    }
    let parent = view.root().and_downcast::<gtk::Window>();
    let dialog = gtk::Window::builder().modal(true).title("Passkey").resizable(false).child(&bx).build();
    dialog.set_transient_for(parent.as_ref());
    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    (dialog, qr, status)
}

/// QR code as a grey texture: 6 px modules, 4-module quiet zone.
fn qr_texture(data: &str) -> Option<gdk::Texture> {
    let code = qrcode::QrCode::new(data.as_bytes()).ok()?;
    let (w, scale, quiet) = (code.width(), 6, 4);
    let n = (w + 2 * quiet) * scale;
    let mut px = vec![255u8; n * n];
    for (i, c) in code.to_colors().iter().enumerate() {
        if *c == qrcode::Color::Dark {
            let (x, y) = (i % w + quiet, i / w + quiet);
            for dy in 0..scale {
                let row = (y * scale + dy) * n + x * scale;
                px[row..row + scale].fill(0);
            }
        }
    }
    Some(gdk::MemoryTexture::new(n as i32, n as i32, gdk::MemoryFormat::G8, &glib::Bytes::from_owned(px), n).upcast())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rp_id_rules() {
        assert_eq!(rp_host("https://accounts.google.com", "google.com").as_deref(), Some("accounts.google.com"));
        assert!(rp_host("https://google.com", "google.com").is_some());
        assert!(rp_host("https://login.microsoft.com", "LOGIN.microsoft.com").is_none());
        assert!(rp_host("https://evilgoogle.com", "google.com").is_none());
        assert!(rp_host("https://accounts.google.com", "com").is_none());
        assert!(rp_host("https://a.example.co.uk", "co.uk").is_none());
        assert!(rp_host("https://x.github.io", "github.io").is_none());
        assert!(rp_host("http://accounts.google.com", "google.com").is_none());
        assert!(rp_host("https://1.2.3.4", "2.3.4").is_none());
        assert!(rp_host("https://google.com", "").is_none());
        assert!(rp_host("https://google.com", "accounts.google.com").is_none());
    }

    #[test]
    fn client_data_is_browser_style() {
        let cd = client_data("webauthn.get", &[0xfb, 0xff], "https://accounts.google.com");
        assert_eq!(
            String::from_utf8(cd).unwrap(),
            r#"{"type":"webauthn.get","challenge":"-_8","origin":"https://accounts.google.com","crossOrigin":false}"#
        );
    }
}
