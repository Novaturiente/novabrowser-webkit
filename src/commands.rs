//! Running commands on WebKit. Parsing, URL-or-search, zoom steps and the name list are
//! shared in `nova_core::commands`.
use crate::{clipboard, config, hints, modes, tabview, web, window};
use nova_core::commands::tab_index;
pub use nova_core::commands::{host, set_user, url_or_search, zoom_step, Cmd};
use std::cell::{Cell, RefCell};
use webkit6::prelude::*;
use webkit6::{FindOptions, WebView};

/// `a ;; b` → commands, with {url}/{title}/{clipboard}/{primary} filled per token.
pub fn parse_line(line: &str) -> Result<Vec<Cmd>, String> {
    nova_core::commands::parse_line(line, &expand)
}

thread_local! {
    /// {clipboard}, {primary} for the command line being run (read before running).
    static CLIP: RefCell<(String, String)> = const { RefCell::new((String::new(), String::new())) };
    /// True while a line typed into the command box/address bar runs.
    static FROM_BAR: Cell<bool> = const { Cell::new(false) };
    /// Last search (text, backward), for `n` / `N`.
    static SEARCH: RefCell<(String, bool)> = const { RefCell::new((String::new(), false)) };
}

fn expand(text: &str) -> String {
    let (url, title) = window::active_url_title();
    let (clip, primary) = CLIP.with_borrow(|c| c.clone());
    // shortcut: {url:pretty} and {url:yank} = {url}
    text.replace("{url:pretty}", &url)
        .replace("{url:yank}", &url)
        .replace("{url:host}", host(&url))
        .replace("{url}", &url)
        .replace("{title}", &title)
        .replace("{clipboard}", &clip)
        .replace("{primary}", &primary)
}

/// A line the user entered in the command box: an `open` there counts as typed.
pub fn run_from_bar(line: &str) {
    FROM_BAR.set(true);
    run(line, None);
    FROM_BAR.set(false);
}

/// Run a command line (`a ;; b`), reporting errors in the status bar.
/// `/text` and `?text` search forward / backward.
pub fn run(line: &str, count: Option<u32>) {
    let line = line.trim();
    if let Some(text) = line.strip_prefix('/') {
        return report("search", search(text, false));
    }
    if let Some(text) = line.strip_prefix('?') {
        return report("search", search(text, true));
    }
    if line.contains("{clipboard}") || line.contains("{primary}") {
        return clipboard::expand_then_run(line, count);
    }
    run_parsed(line, count);
}

/// Run with `{clipboard}`/`{primary}` already read (clipboard.rs).
pub fn run_with_clipboard(line: &str, count: Option<u32>, clip: String, primary: String) {
    CLIP.set((clip, primary));
    run_parsed(line, count);
    CLIP.set((String::new(), String::new()));
}

fn report(name: &str, r: Result<(), String>) {
    if let Err(e) = r {
        window::message("error", &format!("{name}: {e}"));
    }
}

fn run_parsed(line: &str, count: Option<u32>) {
    let cmds = match parse_line(line) {
        Ok(c) => c,
        Err(e) => return window::message("error", &e),
    };
    for cmd in cmds {
        if let Err(e) = run_one(&cmd, count) {
            window::message("error", &format!("{}: {e}", cmd.name));
            return;
        }
    }
}

/// Find the element that scrolls under the viewport centre (or the document).
const SCROLLER_JS: &str = "((x) => { let e = document.elementFromPoint(innerWidth / 2, innerHeight / 2);\
 const ok = (e) => x ? e.scrollWidth > e.clientWidth && /auto|scroll/.test(getComputedStyle(e).overflowX)\
 : e.scrollHeight > e.clientHeight && /auto|scroll/.test(getComputedStyle(e).overflowY);\
 while (e && e !== document.body && e !== document.documentElement && !ok(e)) e = e.parentElement;\
 if (!e || e === document.body || e === document.documentElement) e = document.scrollingElement || document.documentElement;\
 return e; })";

fn scroll_by(v: &WebView, dx: f64, dy: f64) {
    let smooth = if config::get().bool("scrolling.smooth") { "smooth" } else { "instant" };
    let js = format!(
        "{SCROLLER_JS}({}).scrollBy({{left: {dx}, top: {dy}, behavior: '{smooth}'}})",
        dy == 0.0
    );
    web::eval_quiet(v, &js);
}

fn scroll_to_perc(v: &WebView, p: f64, x: bool) {
    let js = format!(
        "(() => {{ const e = {SCROLLER_JS}({x}); if ({x}) e.scrollLeft = (e.scrollWidth - e.clientWidth) * {p} / 100; else e.scrollTop = (e.scrollHeight - e.clientHeight) * {p} / 100; }})()"
    );
    web::eval_quiet(v, &js);
}

/// Find `text` in the active tab; smart case (case-sensitive with an uppercase letter).
pub fn search(text: &str, backward: bool) -> Result<(), String> {
    let v = window::active_view().ok_or("no active tab")?;
    let fc = v.find_controller().ok_or("no find controller")?;
    SEARCH.set((text.to_string(), backward));
    if text.is_empty() {
        fc.search_finish();
        return Ok(());
    }
    fc.search(text, find_options(text, backward).bits(), u32::MAX);
    Ok(())
}

fn find_options(text: &str, backward: bool) -> FindOptions {
    let mut o = FindOptions::WRAP_AROUND;
    if !text.chars().any(char::is_uppercase) {
        o |= FindOptions::CASE_INSENSITIVE;
    }
    if backward {
        o |= FindOptions::BACKWARDS;
    }
    o
}

fn search_again(reverse: bool, n: u32) -> Result<(), String> {
    let (text, backward) = SEARCH.with_borrow(|s| s.clone());
    if text.is_empty() {
        return Err("no previous search".into());
    }
    let v = window::active_view().ok_or("no active tab")?;
    let fc = v.find_controller().ok_or("no find controller")?;
    for _ in 0..n {
        if backward == reverse {
            fc.search_next()
        } else {
            fc.search_previous()
        }
    }
    Ok(())
}

/// Open `url` here, or in a new tab with -t / background with -b / window with -w.
fn open_in(url: &str, cmd: &Cmd) -> Result<(), String> {
    if cmd.flag("-b", "--bg") {
        window::open(url, false);
    } else if cmd.flag("-w", "--window") {
        window::open_window(Some(url.to_string()));
    } else if cmd.flag("-t", "--tab") {
        window::open(url, true);
    } else {
        tabview::load(&window::active_view().ok_or("no active tab")?, url);
    }
    Ok(())
}

/// `dark-toggle`: flip dark mode for this host now, and remember it in
/// `<config>/darkmode_excludes` (the file config.ts reads with `readLines`).
fn dark_toggle() -> Result<(), String> {
    const D: &str = "colors.webpage.darkmode.enabled";
    let (url, _) = window::active_url_title();
    let host = host(&url).to_string();
    if host.is_empty() {
        return Err("no host for this page".into());
    }
    let file = config::path().with_file_name("darkmode_excludes");
    let mut lines: Vec<String> = std::fs::read_to_string(&file)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let cfg = config::get();
    let dark = !cfg.get_for(D, &url).as_bool().unwrap_or(false);
    if dark {
        lines.retain(|l| *l != host);
    } else if !lines.contains(&host) {
        lines.push(host.clone());
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    crate::write_atomic(&file, (lines.join("\n") + "\n").as_bytes())
        .map_err(|e| format!("{}: {e}", file.display()))?;
    cfg.set_runtime_for(&format!("*://{host}/*"), D, &dark.to_string())?;
    window::config_changed();
    window::message(
        "info",
        &format!("Dark mode {} for {host}", if dark { "on" } else { "off" }),
    );
    Ok(())
}

/// `https-toggle`: add/remove this page's host in https_excludes, then reload over
/// the other scheme.
fn https_toggle() -> Result<(), String> {
    let url = window::active_view()
        .and_then(|v| v.uri())
        .map(|u| u.to_string())
        .unwrap_or_default();
    let (auth, _) = crate::net::authority(&url).ok_or("no host for this page")?;
    let file = crate::net::excludes_file();
    let mut lines = crate::net::parse_excludes(&std::fs::read_to_string(&file).unwrap_or_default());
    let excluded = lines.iter().any(|l| l == auth);
    if excluded {
        lines.retain(|l| l != auth);
    } else {
        lines.push(auth.to_string());
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    crate::write_atomic(&file, (lines.join("\n") + "\n").as_bytes())
        .map_err(|e| format!("{}: {e}", file.display()))?;
    crate::net::load_https_excludes();
    crate::adblock::push_config();
    let rest = url.split_once("://").map_or(url.as_str(), |(_, r)| r);
    let target = if excluded {
        format!("https://{rest}")
    } else {
        format!("http://{rest}")
    };
    window::message(
        "info",
        &format!("HTTPS-only {} for {auth}", if excluded { "on" } else { "off" }),
    );
    run(&format!("open {target}"), None);
    Ok(())
}

fn run_one(cmd: &Cmd, count: Option<u32>) -> Result<(), String> {
    let n = count.unwrap_or(1);
    let view = window::active_view();
    let need = || view.clone().ok_or_else(|| "no active tab".to_string());
    match cmd.name.as_str() {
        "nop" | "clear-keychain" => modes::clear_keychain(),
        "open" => {
            let url = if cmd.args.is_empty() {
                config::get().str("url.default_page")
            } else {
                url_or_search(&cmd.rest())
            };
            let typed = FROM_BAR.get() && !cmd.args.is_empty();
            let open = |url: String, current: bool, f: &dyn Fn(String)| {
                if typed {
                    window::typed_open(current, || f(url));
                } else {
                    f(url);
                }
            };
            if cmd.flag("-p", "--private") {
                window::open_private(url);
            } else if cmd.flag("-b", "--bg") {
                open(url, false, &|u| window::open(&u, false));
            } else if cmd.flag("-w", "--window") {
                open(url, false, &|u| window::open_window(Some(u)));
            } else if cmd.flag("-t", "--tab") {
                open(url, false, &|u| window::open(&u, true));
            } else {
                let v = need()?;
                open(url, true, &|u| tabview::load(&v, &u));
            }
        }
        "tab-close" => match count {
            Some(c) => window::close_count(c, cmd.flag("-f", "--force")),
            None => window::close_active(cmd.flag("-f", "--force")),
        },
        "tab-pin" => window::pin(tab_index(cmd, count)),
        "tab-give" => window::give(tab_index(cmd, count)),
        "tab-hibernate" => window::hibernate(tab_index(cmd, count)),
        "tab-keep-awake" => window::keep_awake(tab_index(cmd, count)),
        "tab-only" => window::close_others(),
        "tab-next" => window::cycle(n as i64),
        "tab-prev" => window::cycle(-(n as i64)),
        "tab-focus" => match cmd.args.first().map(String::as_str) {
            Some("last") => window::focus_last(),
            Some(i) => window::focus_index(i.parse::<i64>().map_err(|_| format!("bad index {i}"))?),
            None => window::focus_index(count.map(i64::from).unwrap_or(1)),
        },
        "tab-move" => match cmd.args.first().map(String::as_str) {
            Some("+") => window::move_active(n as i64, true),
            Some("-") => window::move_active(-(n as i64), true),
            Some(i) => window::move_active(
                i.parse::<i64>().map_err(|_| format!("bad index {i}"))?,
                false,
            ),
            None => window::move_active(count.map(i64::from).unwrap_or(1), false),
        },
        "tab-clone" => {
            let (url, _) = window::active_url_title();
            window::open(&url, !cmd.flag("-b", "--bg"));
        }
        "tab-mute" => {
            let v = need()?;
            let m = !v.is_muted();
            v.set_is_muted(m);
            window::set_audio(&v);
            window::message("info", if m { "muted" } else { "unmuted" });
        }
        "undo" => window::undo_close(),
        "paste" => need()?.execute_editing_command("Paste"),
        "back" => window::go(&need()?, -(n as i32)),
        "forward" => window::go(&need()?, n as i32),
        "dark-toggle" => dark_toggle()?,
        "https-toggle" => https_toggle()?,
        "reload" => {
            let v = need()?;
            if !window::reload_crashed(&v) {
                if cmd.flag("-f", "--force") {
                    v.reload_bypass_cache()
                } else {
                    v.reload()
                }
            }
        }
        "stop" => need()?.stop_loading(),
        "scroll" => {
            let step = 40.0 * n as f64;
            let v = need()?;
            match cmd.args.first().map(String::as_str) {
                Some("down") => scroll_by(&v, 0.0, step),
                Some("up") => scroll_by(&v, 0.0, -step),
                Some("left") => scroll_by(&v, -step, 0.0),
                Some("right") => scroll_by(&v, step, 0.0),
                Some("top") => scroll_to_perc(&v, 0.0, false),
                Some("bottom") => scroll_to_perc(&v, 100.0, false),
                d => return Err(format!("bad direction {d:?}")),
            }
        }
        "scroll-page" => {
            let f = |i: usize| {
                cmd.args
                    .get(i)
                    .ok_or("usage: scroll-page x y")?
                    .parse::<f64>()
                    .map_err(|e| e.to_string())
            };
            let (w, h) = window::content_size();
            scroll_by(&need()?, f(0)? * w * n as f64, f(1)? * h * n as f64);
        }
        "scroll-to-perc" => {
            let p: f64 = match (cmd.args.first(), count) {
                (Some(a), _) => a.parse().map_err(|_| format!("bad percent {a}"))?,
                (None, Some(c)) => c as f64,
                (None, None) => 100.0,
            };
            scroll_to_perc(&need()?, p.clamp(0.0, 100.0), cmd.flag("-x", "--horizontal"));
        }
        "zoom-in" | "zoom-out" | "zoom" => {
            let v = need()?;
            let cur = v.zoom_level() * 100.0;
            let pct = match cmd.name.as_str() {
                "zoom-in" => zoom_step(cur, n as i32),
                "zoom-out" => zoom_step(cur, -(n as i32)),
                _ => match (cmd.args.first(), count) {
                    (Some(a), _) => a
                        .trim_end_matches('%')
                        .parse()
                        .map_err(|_| format!("bad zoom {a}"))?,
                    (None, Some(c)) => c as f64,
                    (None, None) => 100.0,
                },
            };
            crate::zoom::set(&v, pct);
            window::message("info", &format!("Zoom level: {pct}%"));
        }
        "search" => search(&cmd.rest(), cmd.flag("-r", "--reverse"))?,
        "search-next" => search_again(false, n)?,
        "search-prev" => search_again(true, n)?,
        "hint" => hints::start(cmd)?,
        "spawn" => crate::spawn::spawn(
            &cmd.args,
            cmd.flag("-u", "--userscript"),
            cmd.flag("-o", "--output"),
            count,
        )?,
        "fake-key" => crate::spawn::fake_key(&cmd.rest())?,
        "insert-text" => crate::spawn::insert_text(&cmd.rest())?,
        "yank" => yank(cmd, &need()?)?,
        "fullscreen" if cmd.flag("", "--leave") => {
            if let Some(v) = view {
                web::eval_page(&v, "document.fullscreenElement && document.exitFullscreen()");
            }
        }
        "fullscreen" => window::toggle_fullscreen(),
        "devtools" => {
            let v = need()?;
            let i = v.inspector().ok_or("no inspector")?;
            // shortcut: WebKit has no is-shown query on the inspector; a second press re-shows
            i.show();
        }
        "mode-enter" => modes::enter(match cmd.args.first().map(String::as_str) {
            Some("insert") => modes::Mode::Insert,
            Some("passthrough") => modes::Mode::Passthrough,
            Some("normal") => modes::Mode::Normal,
            m => return Err(format!("unsupported mode {m:?}")),
        }),
        "mode-leave" => modes::leave(),
        "cmd-set-text" => {
            let mut text = cmd.rest();
            if cmd.flag("-s", "--space") {
                text.push(' ');
            }
            window::open_cmdline(&text);
        }
        "message-info" | "message-warning" | "message-error" => {
            window::message(cmd.name.trim_start_matches("message-"), &cmd.rest());
        }
        "close" | "quit" => {
            if cmd.flag("-s", "--save") {
                crate::session::save(None)?;
            }
            if cmd.name == "quit" {
                window::quit()
            } else {
                window::close_window()
            }
        }
        "quickmark-save" => {
            let (url, _) = window::active_url_title();
            window::open_cmdline(&format!(":quickmark-add {url} "));
        }
        "quickmark-add" => {
            let [url, name] = &cmd.args[..] else {
                return Err("quickmark-add: url and name required".into());
            };
            crate::marks::quickmark_add(url, name)?
        }
        "quickmark-load" => open_in(&crate::marks::quickmark_url(&cmd.rest())?, cmd)?,
        "quickmark-del" => {
            let (url, _) = window::active_url_title();
            crate::marks::quickmark_del(cmd.args.first().map(String::as_str), &url)?
        }
        "bookmark-add" => {
            let (cur_url, cur_title) = window::active_url_title();
            let url = cmd.args.first().cloned().unwrap_or(cur_url);
            let title = if cmd.args.len() > 1 {
                cmd.args[1..].join(" ")
            } else if cmd.args.is_empty() {
                cur_title
            } else {
                String::new()
            };
            crate::marks::bookmark_add(&url, &title, cmd.flag("", "--toggle"))?
        }
        "bookmark-load" => open_in(&url_or_search(&cmd.rest()), cmd)?,
        "bookmark-del" => {
            let url = if cmd.args.is_empty() {
                window::active_url_title().0
            } else {
                cmd.rest()
            };
            crate::marks::bookmark_del(&url)?
        }
        "bindings" => window::open_cmdline(&format!(":bindings {}", cmd.rest())),
        "profile-new" => match cmd.args.is_empty() {
            true => window::open_cmdline(":profile-new "),
            false => crate::profiles::create(&cmd.rest())?,
        },
        "profile-open" => match cmd.args.is_empty() {
            true => window::open_cmdline(":profile-open "),
            false => crate::profiles::open(&cmd.rest())?,
        },
        "settings" => window::open(&web::internal_url(&crate::settings_page::page()), true),
        "bookmarks" => window::open(&web::internal_url(&crate::marks::page()), true),
        "bookmark-list" => {
            let page = web::internal_url(&crate::marks::page());
            if cmd.flag("-j", "--jump") {
                open_in(&page, cmd)?
            } else {
                window::open(&page, true)
            }
        }
        "download-open" => crate::downloads::open(&cmd.args)?,
        "download-cancel" => {
            crate::downloads::cancel(cmd.args.first().and_then(|a| a.parse().ok()))?
        }
        "download-clear" => crate::downloads::clear(),
        "password-fill" => crate::passwords::fill_command(cmd.flag("-o", "--otp"))?,
        "history" => window::open(
            &web::internal_url(&crate::history::page(Some(&cmd.rest()))),
            true,
        ),
        "help" => window::open(&web::internal_url(&crate::welcome::page()), true),
        "downloads" => window::open(&web::internal_url(&crate::downloads::page()), true),
        "download" => {
            let url = match cmd.rest() {
                r if r.is_empty() => window::active_url_title().0,
                r if r.starts_with("magnet:") => r,
                r => url_or_search(&r),
            };
            if url.starts_with("magnet:") {
                if !crate::downloads::magnet(url) {
                    return Err("magnet: links need aria2 (install aria2)".into());
                }
            } else {
                crate::downloads::start(view.as_ref(), &url);
            }
        }
        "session-save" => crate::session::save(cmd.args.first().map(String::as_str))?,
        "session-load" => crate::session::load(
            cmd.args.first().ok_or("session-load: name required")?,
            cmd.flag("-c", "--clear"),
        )?,
        "session-delete" => {
            crate::session::delete(cmd.args.first().ok_or("session-delete: name required")?)?
        }
        "adblock-update" => crate::adblock::update_now(),
        "config-source" => {
            config::load().map_err(|e| format!("config unchanged: {e}"))?;
            crate::marks::reload();
            crate::net::load_https_excludes();
            crate::adblock::push_config();
            window::config_changed();
            let ignored = config::get().ignored_per_site();
            if ignored.is_empty() {
                window::message("info", "config.ts loaded");
            } else {
                window::message(
                    "warning",
                    &format!(
                        "config.ts loaded; only global, ignored in pattern(): {}",
                        ignored.join(", ")
                    ),
                );
            }
        }
        "set" => {
            let temp = cmd.flag("-t", "--temp");
            let pattern = cmd
                .flag("-u", "--pattern")
                .then(|| cmd.args.first().cloned())
                .flatten();
            let args = &cmd.args[pattern.is_some() as usize..];
            let (name, value) = match args {
                [n] => {
                    window::message("info", &format!("{n} = {}", config::get().get(n)));
                    return Ok(());
                }
                [n, rest @ ..] if !rest.is_empty() => (n, rest.join(" ")),
                _ => return Err("usage: set <setting> [value]".into()),
            };
            match &pattern {
                Some(p) => config::get().set_runtime_for(p, name, &value)?,
                None => set_user(name, &value, temp)?,
            }
            crate::adblock::push_config();
            window::config_changed();
        }
        "config-cycle" => {
            let [name, values @ ..] = cmd.args.as_slice() else {
                return Err("usage: config-cycle <setting> <values…>".into());
            };
            if values.len() < 2 {
                return Err("config-cycle needs at least two values".into());
            }
            let cfg = config::get();
            let cur = cfg.get(name);
            let cur = cur
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| cur.to_string());
            let i = values
                .iter()
                .position(|v| *v == cur)
                .map_or(0, |i| (i + 1) % values.len());
            set_user(name, &values[i], cmd.flag("-t", "--temp"))?;
            window::config_changed();
        }
        other => return Err(format!("unknown command ({other})")),
    }
    Ok(())
}

/// `yank [url|title|domain|pretty-url|selection|inline <text>] [-s]`
fn yank(cmd: &Cmd, view: &WebView) -> Result<(), String> {
    let primary = cmd.flag("-s", "--sel");
    let (url, title) = window::active_url_title();
    let what = cmd.args.first().map_or("url", String::as_str);
    let text = match what {
        "url" | "pretty-url" => url.clone(),
        "title" => title,
        "domain" => match url.split_once("://") {
            Some((scheme, _)) => format!(
                "{scheme}://{}",
                url[scheme.len() + 3..].split('/').next().unwrap_or("")
            ),
            None => url.clone(),
        },
        "inline" => cmd.args[1..].join(" "),
        "selection" => {
            web::eval_world(view, "JSON.stringify(getSelection().toString())", move |r| {
                match r
                    .and_then(|v| v.as_str().map(str::to_string))
                    .filter(|t| !t.is_empty())
                {
                    Some(t) => yanked(&t, "selection", primary),
                    None => window::message("error", "Nothing selected."),
                }
            });
            return Ok(());
        }
        w => return Err(format!("unknown yank target {w}")),
    };
    yanked(&text, what, primary);
    Ok(())
}

fn yanked(text: &str, what: &str, primary: bool) {
    clipboard::write(text, primary);
    let to = if primary { "primary selection" } else { "clipboard" };
    let what = if what == "url" { "URL" } else { what };
    window::message("info", &format!("Yanked {what} to {to}: {text}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_is_handled() {
        // run_one's match must know every command the palette offers
        let src = include_str!("commands.rs");
        for n in nova_core::commands::NAMES {
            assert!(src.contains(&format!("\"{n}\"")), "{n} not handled");
        }
    }

    #[test]
    fn smart_case_find() {
        assert!(find_options("abc", false).contains(FindOptions::CASE_INSENSITIVE));
        assert!(!find_options("Abc", false).contains(FindOptions::CASE_INSENSITIVE));
        assert!(find_options("x", true).contains(FindOptions::BACKWARDS));
    }
}
