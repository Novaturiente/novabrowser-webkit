// w19 spike: passkeys in WebKitGTK through caBLE/hybrid (phone QR + Bluetooth) with
// webauthn-authenticator-rs. shim.js gives pages navigator.credentials / PublicKeyCredential
// and forwards requests here; this side takes the origin from WebKit (never from the page),
// webauthn-authenticator-rs checks rpId against it and builds clientDataJSON.
// usage: w19-passkey <data-dir> <url>
use gtk::{gdk, glib, prelude::*};
use serde_json::{json, Value};
use std::{cell::RefCell, rc::Rc, sync::mpsc};
use webauthn_authenticator_rs::{
    cable::connect_cable_authenticator,
    types::{CableRequestType, CableState, EnrollSampleStatus},
    ui::UiCallback,
    AuthenticatorBackendHashedClientData,
};
use webauthn_rs_proto::{CreationChallengeResponse, RequestChallengeResponse};
use webkit6::{javascriptcore, prelude::*};

const SHIM: &str = include_str!("shim.js");
// same as novabrowser-webkit's default (Google's sign-in accepts it)
const SAFARI_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Safari/605.1.15";

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
        let _ = self.0.send(Ev::Status("Processing…".into()));
    }
    fn fingerprint_enrollment_feedback(&self, _: u32, _: Option<EnrollSampleStatus>) {}
    fn cable_qr_code(&self, _: CableRequestType, url: String) {
        let _ = self.0.send(Ev::Qr(url));
    }
    fn dismiss_qr_code(&self) {}
    fn cable_status_update(&self, s: CableState) {
        let _ = self.0.send(Ev::Status(format!("{s:?}")));
    }
}

fn error(name: &str, message: &str) -> String {
    json!({ "error": name, "message": message }).to_string()
}

/// rpId must be the origin's host or a parent domain of it (WebAuthn §5.1.3 step 8).
/// shortcut: no public-suffix list, so `co.uk` passes; the phone still shows the rpId.
fn rp_id_ok(origin: &str, rp_id: &str) -> bool {
    let Some(host) = origin.strip_prefix("https://") else { return false };
    rp_id.contains('.') && (host == rp_id || host.ends_with(&format!(".{rp_id}")))
}

/// clientDataJSON as browsers write it. webauthn-authenticator-rs's own one serialises the
/// origin as a URL (`https://accounts.google.com/`, trailing slash) plus `tokenBinding: null`,
/// which Google's server rejects ("Something went wrong").
fn client_data(kind: &str, challenge: &[u8], origin: &str) -> Vec<u8> {
    use base64::Engine;
    let ch = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(challenge);
    format!(
        r#"{{"type":{},"challenge":{},"origin":{},"crossOrigin":false}}"#,
        json!(kind),
        json!(ch),
        json!(origin)
    )
    .into_bytes()
}

/// One WebAuthn request over caBLE, on a worker thread. `origin` is `https://host`, from
/// WebKit. Returns the JSON reply for shim.js.
fn run(origin: String, req: Value, tx: mpsc::Sender<Ev>) -> String {
    let create = req["op"] == "create";
    let options = json!({ "publicKey": req["publicKey"] });
    let ui = Ui(tx);
    let out = (|| {
        let (rp_id, challenge, timeout, request) = if create {
            let o: CreationChallengeResponse = serde_json::from_value(options).map_err(|e| format!("options: {e}"))?;
            let pk = o.public_key;
            (pk.rp.id.clone(), pk.challenge.to_vec(), pk.timeout, Err(pk))
        } else {
            let o: RequestChallengeResponse = serde_json::from_value(options).map_err(|e| format!("options: {e}"))?;
            let pk = o.public_key;
            (pk.rp_id.clone(), pk.challenge.to_vec(), pk.timeout, Ok(pk))
        };
        if !rp_id_ok(&origin, &rp_id) {
            return Err(format!("SecurityError: rpId {rp_id} does not match {origin}"));
        }
        let cd = client_data(if create { "webauthn.create" } else { "webauthn.get" }, &challenge, &origin);
        let hash = openssl::sha::sha256(&cd).to_vec();
        let timeout = timeout.unwrap_or(120_000).clamp(30_000, 300_000);
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            let kind = if create { CableRequestType::DiscoverableMakeCredential } else { CableRequestType::GetAssertion };
            let mut a = connect_cable_authenticator(kind, &ui).await.map_err(|e| format!("caBLE connect: {e:?}"))?;
            match request {
                Ok(pk) => a.perform_auth(hash, pk, timeout).map(|mut c| {
                    c.response.client_data_json = cd;
                    json!(c)
                }),
                Err(pk) => a.perform_register(hash, pk, timeout).map(|mut c| {
                    c.response.client_data_json = cd;
                    json!(c)
                }),
            }
            .map_err(|e| format!("{e:?}"))
        })
    })();
    match out {
        Ok(c) => {
            eprintln!("w19: ok, extensions {} userHandle {}", c["extensions"], !c["response"]["userHandle"].is_null());
            json!({ "credential": c }).to_string()
        }
        Err(e) => {
            eprintln!("w19: {e}");
            error("NotAllowedError", &e)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rp_id_and_client_data() {
        assert!(super::rp_id_ok("https://accounts.google.com", "google.com"));
        assert!(super::rp_id_ok("https://google.com", "google.com"));
        assert!(!super::rp_id_ok("https://evilgoogle.com", "google.com"));
        assert!(!super::rp_id_ok("https://accounts.google.com", "com"));
        assert!(!super::rp_id_ok("http://accounts.google.com", "google.com"));
        let cd = super::client_data("webauthn.get", &[0xfb, 0xff], "https://accounts.google.com");
        assert_eq!(
            String::from_utf8(cd).unwrap(),
            r#"{"type":"webauthn.get","challenge":"-_8","origin":"https://accounts.google.com","crossOrigin":false}"#
        );
    }
}

/// QR code as a grey texture: 8 px modules, 4-module quiet zone.
fn qr_texture(data: &str) -> gdk::Texture {
    let code = qrcode::QrCode::new(data.as_bytes()).expect("caBLE URL fits a QR code");
    let (w, scale, quiet) = (code.width(), 8, 4);
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
    gdk::MemoryTexture::new(n as i32, n as i32, gdk::MemoryFormat::G8, &glib::Bytes::from_owned(px), n).upcast()
}

type Reply = Rc<RefCell<Option<Box<dyn FnOnce(String)>>>>;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dir, start) = (args[1].clone(), args[2].clone());
    let app = gtk::Application::builder().application_id("dev.nova.w19passkey").build();
    app.connect_activate(move |app| {
        let session = webkit6::NetworkSession::new(Some(&dir), Some(&format!("{dir}/cache")));
        session
            .cookie_manager()
            .unwrap()
            .set_persistent_storage(&format!("{dir}/cookies.sqlite"), webkit6::CookiePersistentStorage::Sqlite);
        let ucm = webkit6::UserContentManager::new();
        ucm.add_script(&webkit6::UserScript::new(
            SHIM,
            webkit6::UserContentInjectedFrames::TopFrame,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.register_script_message_handler_with_reply("nbPasskey", None);
        let view = webkit6::WebView::builder().network_session(&session).user_content_manager(&ucm).build();
        let s = WebViewExt::settings(&view).unwrap();
        s.set_user_agent(Some(SAFARI_UA));
        s.set_enable_developer_extras(true);
        s.set_enable_write_console_messages_to_stdout(std::env::var_os("W19_CONSOLE").is_some());
        let win = gtk::ApplicationWindow::builder()
            .application(app)
            .title("w19-passkey")
            .default_width(1200)
            .default_height(900)
            .child(&view)
            .build();

        let busy = Rc::new(std::cell::Cell::new(false));
        let (w, v) = (win.clone(), view.clone());
        ucm.connect_script_message_with_reply_received(Some("nbPasskey"), move |_, value, reply| {
            let ctx = value.context().unwrap();
            let r = reply.clone();
            let pending: Reply = Rc::new(RefCell::new(Some(Box::new(move |s: String| {
                r.return_value(&javascriptcore::Value::new_string(&ctx, Some(&s)))
            }))));
            let answer = |p: &Reply, s: String| {
                if let Some(f) = p.borrow_mut().take() {
                    f(s)
                }
            };
            if busy.get() {
                answer(&pending, error("NotAllowedError", "another passkey request is still open"));
                return true;
            }
            // the page's origin as WebKit sees it (main frame: shim.js is TopFrame only)
            // shortcut: iframes can post to the handler too; production needs the sender frame's origin
            let origin = v.uri().and_then(|u| url::Url::parse(&u).ok()).map(|u| u.origin().ascii_serialization());
            let (Some(origin), Ok(req)) = (origin, serde_json::from_str::<Value>(&value.to_str())) else {
                answer(&pending, error("SecurityError", "no page origin"));
                return true;
            };
            eprintln!("w19: {} for {origin}", req["op"]);
            // request shape for debugging (challenge and ids are one-time / public, no secrets)
            eprintln!("w19: request {}", req["publicKey"]);
            busy.set(true);

            let pic = gtk::Picture::builder().width_request(360).height_request(360).build();
            let label = gtk::Label::new(Some("Starting…"));
            let cancel = gtk::Button::with_label("Cancel");
            let bx = gtk::Box::new(gtk::Orientation::Vertical, 12);
            bx.set_margin_top(16);
            bx.set_margin_bottom(16);
            bx.set_margin_start(16);
            bx.set_margin_end(16);
            bx.append(&gtk::Label::new(Some(&format!("Passkey for {origin}\nScan with your phone's camera"))));
            bx.append(&pic);
            bx.append(&label);
            bx.append(&cancel);
            let dlg = gtk::Window::builder().transient_for(&w).modal(true).title("Passkey").child(&bx).build();
            {
                let (p, d) = (pending.clone(), dlg.clone());
                cancel.connect_clicked(move |_| {
                    answer(&p, error("NotAllowedError", "cancelled"));
                    d.close();
                });
            }
            {
                let p = pending.clone();
                dlg.connect_close_request(move |_| {
                    answer(&p, error("NotAllowedError", "cancelled"));
                    glib::Propagation::Proceed
                });
            }
            dlg.present();

            let (tx, rx) = mpsc::channel();
            let done = tx.clone();
            std::thread::spawn(move || {
                let out = run(origin, req, tx);
                let _ = done.send(Ev::Done(out));
            });
            let (b, p) = (busy.clone(), pending.clone());
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                while let Ok(ev) = rx.try_recv() {
                    match ev {
                        Ev::Qr(u) => {
                            eprintln!("w19: QR shown");
                            pic.set_paintable(Some(&qr_texture(&u)))
                        }
                        Ev::Status(s) => {
                            eprintln!("w19: {s}");
                            label.set_text(&s)
                        }
                        Ev::Done(s) => {
                            answer(&p, s);
                            dlg.close();
                            b.set(false);
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
            true
        });

        view.load_uri(&start);
        win.present();
    });
    app.run_with_args(&[args[0].clone()]);
}
