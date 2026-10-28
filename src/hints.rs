//! `hint` command and hint mode (qutebrowser browser/hints.py). Labels come from
//! nova-core; finding, drawing and picking elements runs in `hints.js` in our
//! isolated world of the page.
use crate::modes::{self, Mode};
use crate::{clipboard, commands, config, theme, web, window};
use serde_json::Value;
use std::cell::RefCell;
use webkit6::WebView;

#[cfg(test)]
mod tests {
    #[test]
    fn hint_url_stays_one_token() {
        let line = format!("open -t {}", super::quote_url("https://x/a;;quit'b c"));
        let cmds = crate::commands::parse_line(&line).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].args, ["https://x/a%3B%3Bquit%27b c"]);
    }
}

const JS: &str = include_str!("hints.js");

/// `hints.selectors[group]`, joined into one CSS selector.
fn selector(group: &str) -> Option<String> {
    let sels = config::get().get("hints.selectors");
    let list = sels.get(group)?.as_array()?;
    Some(
        list.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Label look from `colors.hints.*`, `fonts.hints`, `hints.border/radius/padding`.
fn style() -> String {
    let cfg = config::get();
    let pad = cfg.get("hints.padding");
    let p = |k: &str| pad[k].as_i64().unwrap_or(0);
    serde_json::json!({
        "font": theme::css_font(&cfg, &cfg.str("fonts.hints")),
        "fg": theme::css_color(&cfg.str("colors.hints.fg")),
        "bg": theme::css_color(&cfg.str("colors.hints.bg")),
        "match": theme::css_color(&cfg.str("colors.hints.match.fg")),
        "border": cfg.str("hints.border"),
        "radius": format!("{}px", cfg.int("hints.radius")),
        "padding": format!("{}px {}px {}px {}px", p("top"), p("right"), p("bottom"), p("left")),
    })
    .to_string()
}

struct Session {
    view: WebView,
    labels: Vec<String>,
    typed: String,
    target: String,
    args: Vec<String>,
    rapid: bool,
    chars: String,
    uppercase: bool,
}

thread_local! {
    static S: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// `hint [--rapid] [--first] [group] [target] [args...]`
pub fn start(cmd: &commands::Cmd) -> Result<(), String> {
    let is_flag = |a: &str, s: &str, l: &str| a == s || a == l;
    let mut flags = cmd.flags.clone();
    let mut pos: Vec<String> = Vec::new();
    let mut it = cmd.args.iter();
    for a in it.by_ref() {
        if is_flag(a, "-r", "--rapid") || is_flag(a, "-f", "--first") {
            flags.push(a.clone());
        } else {
            pos.push(a.clone());
            if pos.len() == 2 {
                break;
            }
        }
    }
    let args: Vec<String> = it.cloned().collect();
    let group = pos.first().map_or("all", String::as_str);
    let sel = selector(group).ok_or_else(|| format!("unknown hint group {group}"))?;
    let target = pos.get(1).cloned().unwrap_or_else(|| "normal".into());
    let rapid = flags.iter().any(|f| is_flag(f, "-r", "--rapid"));
    let first = flags.iter().any(|f| is_flag(f, "-f", "--first"));
    let cfg = config::get();
    let chars = cfg.str("hints.chars");
    if chars.chars().count() < 2 {
        return Err("hints.chars needs at least 2 characters".into());
    }
    let view = window::active_view().ok_or("no active tab")?;
    let js = format!(
        "{JS}; JSON.stringify(__nova.collect({}))",
        serde_json::to_string(&sel).unwrap()
    );
    let v = view.clone();
    web::eval_world(&view, &js, move |r| {
        let Some(n) = r.and_then(|r| r.as_u64()) else {
            return window::message("error", "hints: page script failed");
        };
        let n = n as usize;
        if n == 0 {
            return window::message("error", "No elements found.");
        }
        if first {
            web::eval_quiet(&v, "__nova.focusFirst()");
            return modes::enter(Mode::Insert);
        }
        let cfg = config::get();
        let uppercase = cfg.bool("hints.uppercase");
        let min_chars = cfg.int("hints.min_chars").max(1) as usize;
        let labels = nova_core::hints::scattered(n, &chars, min_chars);
        let draw = format!(
            "__nova.draw({}, {uppercase}, {})",
            serde_json::to_string(&labels).unwrap(),
            style()
        );
        web::eval_quiet(&v, &draw);
        S.set(Some(Session {
            view: v,
            labels,
            typed: String::new(),
            target,
            args,
            rapid,
            chars,
            uppercase,
        }));
        modes::enter(Mode::Hint);
    });
    Ok(())
}

/// Keys in hint mode: label chars filter, Backspace un-types, anything else is eaten.
pub fn on_key(name: &str) {
    let action = S.with_borrow_mut(|s| {
        let s = s.as_mut()?;
        match name {
            "Backspace" => {
                s.typed.pop();
            }
            n if n.chars().count() == 1 && s.chars.contains(&n.to_lowercase()) => {
                s.typed.push_str(&n.to_lowercase())
            }
            _ => return None,
        }
        let matching: Vec<usize> = (0..s.labels.len())
            .filter(|&i| s.labels[i].starts_with(&s.typed))
            .collect();
        Some((s.view.clone(), s.typed.clone(), matching, s.uppercase))
    });
    let Some((v, typed, matching, uppercase)) = action else {
        return;
    };
    match matching.as_slice() {
        [] => {
            window::message("info", "No matching hint.");
            modes::enter(Mode::Normal);
        }
        [i] => follow(&v, *i),
        _ => {
            let js = format!(
                "__nova.filter({}, {uppercase})",
                serde_json::to_string(&typed).unwrap()
            );
            web::eval_quiet(&v, &js);
        }
    }
    window::refresh();
}

pub fn typed() -> String {
    S.with_borrow(|s| s.as_ref().map(|s| s.typed.clone()).unwrap_or_default())
}

fn follow(v: &WebView, i: usize) {
    let v2 = v.clone();
    let js = format!("JSON.stringify(Object.assign(__nova.pick({i}), {{blank: __nova.blank({i})}}))");
    web::eval_world(v, &js, move |info| {
        let Some(info) = info else {
            return window::message("error", "hints: element is gone");
        };
        let (target, args, rapid, uppercase) = S
            .with_borrow(|s| {
                s.as_ref()
                    .map(|s| (s.target.clone(), s.args.clone(), s.rapid, s.uppercase))
            })
            .unwrap_or_default();
        if rapid {
            S.with_borrow_mut(|s| s.as_mut().map(|s| s.typed.clear()));
            web::eval_quiet(&v2, &format!("__nova.filter('', {uppercase})"));
        }
        if let Err(e) = act(&v2, i, &target, &args, &info) {
            window::message("error", &e);
        }
        if !rapid {
            modes::enter(if info["editable"].as_bool() == Some(true) && matches!(target.as_str(), "normal" | "current") {
                Mode::Insert
            } else {
                Mode::Normal
            });
            on_leave();
        }
    });
}

/// Run `target` on picked element `i` (`info` from hints.js `pick`).
fn act(v: &WebView, i: usize, target: &str, args: &[String], info: &Value) -> Result<(), String> {
    let href = info["href"].as_str();
    let url = href.or(info["src"].as_str());
    let need_url = || {
        url.map(str::to_string)
            .ok_or_else(|| "hint: element has no URL".to_string())
    };
    let click = || web::eval_quiet(v, &format!("__nova.click({i})"));
    match target {
        "normal" | "current" => match href {
            Some(u) if info["blank"].as_bool() == Some(true) && target == "normal" => {
                window::open(u, true)
            }
            Some(u) if target == "current" => crate::tabview::load(v, u),
            _ => click(),
        },
        // qutebrowser tabs.background = true: `tab` opens in the background
        "tab" | "tab-bg" | "tab-fg" => match href {
            Some(u) => window::open(u, target == "tab-fg"),
            None => click(),
        },
        "window" => match href {
            Some(u) => window::open_window(Some(u.to_string())),
            None => click(),
        },
        "hover" => web::eval_quiet(v, &format!("__nova.hover({i})")),
        "yank" | "yank-primary" => {
            let u = need_url()?;
            clipboard::write(&u, target == "yank-primary");
            window::message(
                "info",
                &format!(
                    "Yanked URL to {}: {u}",
                    if target == "yank" { "clipboard" } else { "primary selection" }
                ),
            );
        }
        "fill" => window::open_cmdline(&args.join(" ").replace("{hint-url}", &need_url()?)),
        "spawn" => {
            let u = need_url()?;
            // each arg replaced on its own: the URL can never split into more args (A5)
            let argv: Vec<String> = args.iter().map(|a| a.replace("{hint-url}", &u)).collect();
            crate::spawn::spawn(&argv, false, false, None)?;
        }
        "run" => {
            let u = need_url()?;
            commands::run(&args.join(" ").replace("{hint-url}", &quote_url(&u)), None)
        }
        t => return Err(format!("unknown hint target {t}")),
    }
    Ok(())
}

/// A page URL as one command-line token: `;` and `'` percent-encoded so it can't
/// end the quote or chain another command (CEF audit A5).
fn quote_url(u: &str) -> String {
    format!("'{}'", u.replace(';', "%3B").replace('\'', "%27"))
}

/// Hint mode ended (Escape, tab switch, follow): remove the labels.
pub fn on_leave() {
    if let Some(s) = S.take() {
        web::eval_quiet(&s.view, "__nova.clear()");
    }
}
