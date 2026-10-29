//! Control socket: one instance per profile. `novabrowser-webkit --cmd ':open -t x'`,
//! `--query state`, and `novabrowser-webkit <url>` while one runs (opens a tab).
//!
//! Protocol: one request line, one reply line. `cmd <line>` → `ok`; `url <text>` → `ok`
//! (new tab, text never parsed as commands); `query state` → JSON; `key <keys>` → `ok`
//! (qutebrowser key notation, e.g. `key y` or `key <Down><Return>`: fed to the mode
//! handler as if typed into the window; what a binding doesn't take is not sent to the page).
//! The socket lives in the profile dir, so test profiles don't talk to the real one.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc;

pub fn socket_path() -> PathBuf {
    crate::xdg_dir("XDG_DATA_HOME", ".local/share").join("ipc")
}

/// Send one request to a running instance; None if none is listening.
pub fn request(line: &str) -> Option<String> {
    let mut s = UnixStream::connect(socket_path()).ok()?;
    writeln!(s, "{line}").ok()?;
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).ok()?;
    Some(reply.trim_end().to_string())
}

/// Listen on a thread; each request runs on the UI thread, its reply comes back over a channel.
pub fn start() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path); // stale: request() already failed to connect
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => return eprintln!("novabrowser-webkit: ipc {}: {e}", path.display()),
    };
    use std::os::unix::fs::PermissionsExt; // any connecting process can run commands
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || serve(stream));
        }
    });
}

fn serve(stream: UnixStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let line = line.trim_end().to_string();
    let (tx, rx) = mpsc::channel();
    nova_core::host::on_ui(move || {
        let _ = tx.send(handle(&line));
    });
    let reply = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or_else(|_| "error: timeout".into());
    let _ = writeln!(&stream, "{reply}");
}

fn handle(line: &str) -> String {
    if crate::window::quitting() {
        return "error: quitting".into();
    }
    match line.split_once(' ') {
        Some(("url", u)) => {
            crate::window::open(&crate::commands::url_or_search(u), true);
            crate::window::raise();
            "ok".into()
        }
        Some(("cmd", c)) => {
            crate::commands::run(c, None);
            "ok".into()
        }
        Some(("query", "state")) => crate::window::state_json(),
        Some(("key", k)) => match nova_core::keys::parse_keystring(k) {
            Ok(keys) => {
                for key in keys {
                    crate::modes::on_key(key);
                }
                "ok".into()
            }
            Err(e) => format!("error: {e}"),
        },
        _ => format!("error: unknown request {line:?}"),
    }
}
