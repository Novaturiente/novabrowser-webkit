//! W9 spike: bare WebKitGTK 6.0 window. `w9-baseline [url]` browses;
//! `w9-baseline --speedometer` runs Speedometer 3.1 once and prints the result JSON.
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};
use std::time::{Duration, Instant};
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebView};

const SPEEDOMETER: &str = "https://browserbench.org/Speedometer3.1/";
/// macOS Safari 27. Real Safari freezes the OS token at 10_15_7.
const SAFARI_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Safari/605.1.15";
/// navigator.platform must match the UA, else scripts see Linux behind a Mac UA.
const SAFARI_JS: &str = "Object.defineProperty(Navigator.prototype, 'platform', {get: () => 'MacIntel', configurable: true});";
const PROBE: &str = "JSON.stringify({ua: navigator.userAgent, platform: navigator.platform, vendor: navigator.vendor, uaData: typeof navigator.userAgentData, appVersion: navigator.appVersion})";

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bench = args.iter().any(|a| a == "--speedometer");
    let safari = args.iter().any(|a| a == "--safari");
    let probe = args.iter().any(|a| a == "--probe");
    let args: Vec<String> = args.into_iter().filter(|a| !a.starts_with("--")).collect();
    let url = if bench {
        SPEEDOMETER.to_string()
    } else {
        args.first().cloned().unwrap_or_else(|| "https://example.com".into())
    };
    let app = Application::builder()
        .application_id("site.novabrowser.w9")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        let view = WebView::new();
        if safari {
            if let Some(s) = WebViewExt::settings(&view) {
                s.set_user_agent(Some(SAFARI_UA));
            }
            let ucm = view.user_content_manager().unwrap();
            ucm.add_script(&webkit6::UserScript::new(
                SAFARI_JS,
                webkit6::UserContentInjectedFrames::AllFrames,
                webkit6::UserScriptInjectionTime::Start,
                &[],
                &[],
            ));
        }
        if probe {
            let done = std::cell::Cell::new(false);
            view.connect_load_changed(move |v, ev| {
                if ev == LoadEvent::Finished && !done.replace(true) {
                    // page JS view + what the server got (httpbin echoes request headers)
                    eval(v, PROBE, |r| println!("page: {r}"));
                    eval(v, "document.body.innerText", |r| {
                        println!("server: {}", r.replace('\n', " "));
                        std::process::exit(0);
                    });
                }
            });
        }
        let win = ApplicationWindow::builder()
            .application(app)
            .title("w9-baseline")
            .default_width(1400)
            .default_height(913)
            .child(&view)
            .build();
        win.present();
        if bench {
            let started = std::cell::Cell::new(false);
            view.connect_load_changed(move |v, ev| {
                if ev == LoadEvent::Finished && !started.replace(true) {
                    let v = v.clone();
                    // same 3 s settle as the CEF S9 script
                    glib::timeout_add_local_once(Duration::from_secs(3), move || run(v));
                }
            });
        }
        view.load_uri(&url);
    });
    app.run_with_args::<&str>(&[])
}

/// JS returning a JSON string, so results come back as one plain value.
fn eval(v: &WebView, js: &'static str, f: impl FnOnce(String) + 'static) {
    v.evaluate_javascript(js, None, None, None::<&gtk::gio::Cancellable>, move |r| {
        f(r.map(|x| x.to_str().to_string()).unwrap_or_else(|e| format!("error: {e}")))
    });
}

const BEFORE: &str = "JSON.stringify({w: innerWidth, h: innerHeight, dpr: devicePixelRatio, vis: document.visibilityState, focus: document.hasFocus(), ua: navigator.userAgent})";
const START: &str = "document.querySelector('.start-tests-button').click(); 'ok'";
const POLL: &str = "JSON.stringify({hash: location.hash, vis: document.visibilityState, focus: document.hasFocus(), res: document.getElementById('result-number').textContent.trim()})";
const RESULT: &str = "JSON.stringify({score: parseFloat(document.getElementById('result-number').textContent), confidence: document.getElementById('confidence-number').textContent.trim(), valid: document.getElementById('summary').className === 'valid', vis: document.visibilityState, focus: document.hasFocus()})";

fn run(v: WebView) {
    let v2 = v.clone();
    eval(&v, BEFORE, move |before| {
        eprintln!("before: {before}");
        eval(&v2.clone(), START, move |_| poll(v2, Instant::now()));
    });
}

fn poll(v: WebView, t0: Instant) {
    glib::timeout_add_local_once(Duration::from_secs(5), move || {
        let v2 = v.clone();
        eval(&v, POLL, move |st| {
            eprintln!("{:>4}s {st}", t0.elapsed().as_secs());
            if st.contains("\"hash\":\"#summary\"") && !st.contains("\"res\":\"\"") {
                eval(&v2, RESULT, move |res| {
                    println!("{{\"duration_s\":{},\"result\":{res}}}", t0.elapsed().as_secs());
                    std::process::exit(0);
                });
            } else if t0.elapsed() > Duration::from_secs(20 * 60) {
                println!("{{\"error\":\"timeout\"}}");
                std::process::exit(1);
            } else {
                poll(v2, t0);
            }
        });
    });
}
