# Native GTK chrome (branch `gtk-ui`, experiment)

Test rewrite of the browser chrome from HTML views (`ui/*.html` in transparent
WebViews, decision D2) to native GTK 4 widgets. **Not merged into `master`.**
Started 2026-11-17 (IST).

## What changed
- `src/chrome/`: one file per part, same look and CSS variables as the HTML pages.
  - `mod.rs`: typed contract with `window.rs`: `Ev` (user actions, was `nova.send`
    strings), `TabItem`, `Status`, `Chrome` (all widgets of one window).
  - `motion.rs`: libadwaita spring presets (`snappy`, `smooth`, `layout`), `fade`, and
    `Pop` (a container that springs its child in/out; never takes clicks while hidden).
  - `style.rs`: one static GTK CSS sheet + the theme as custom properties on
    `window.nova` (`theme::vars`, unchanged names).
  - `tabbar.rs`: custom strip widget: spring layout (open/close/reorder), **live drag**
    (tab follows the pointer, others slide aside), hover ×, middle-click close, wheel
    scroll on overflow, vertical mode, status pills, progress line.
  - `palette.rs`, `popup.rs` (tooltip, tab menu, password list), `prompt.rs`,
    `toast.rs`, `keyhint.rs`.
- `window.rs` calls widgets directly; `anim.rs`, `ui/{tabbar,palette,keyhint,popup,
  prompt,toast}.html`, the chrome WebContext and `web::chrome_view` are gone (no chrome
  web process any more).
- New dependency: `libadwaita` 0.9 (`v1_6`, springs + spinner); `adw::init()` at start.
- IPC: `--query "snapshot <file.png>"` renders the window (works while covered);
  `--query state` gained `"menu"`.
- Tests: `tests/shot.sh` (screenshot helper), `tests/pointer.sh` (real mouse via
  ydotool: click, middle-click, hover ×, live drag, tooltip, menu, click-outside,
  prompt button).

## Checked (2026-11-17, debug build)
- `cargo test --workspace`: 22 + 4 passed (+ `palette::tests::live` passed on a display).
- `tests/pointer.sh`, `smoke.sh`, `passwords.sh`, `caret.sh`, `features.sh`,
  `passkey.sh`: all passed.
- Screenshots against the HTML version (`tests/shot.sh`): tab bar, insert pill,
  pinned, vertical, palette, search pill, which-key, toast, prompt, menu, tooltip.
- Springs run per frame (temporary frame log: ~30 frames per palette open/close).

## Not done / known gaps
- Release (LTO) build not built: it would overwrite the main checkout's
  `target/release` binary.
- Escape doesn't cancel a tab drag (the window's key handler eats Escape first);
  dropping outside still works, a cancelled gesture springs back.
- `Ev::Move` moves the *active* tab (the press made it active); a key that changes the
  active tab mid-drag would move the wrong one.
- Menu and password-list rows assume 28 px rows (highlight drifts with big fonts).
- Progress line width follows a window resize only on its next tick.
- Two sliding-highlight implementations (palette vs popup `List`) could be one.
- First open of the blurred backdrop may still drop frames (GSK blur warm-up; the
  HTML edition pre-warmed it, this one doesn't).
