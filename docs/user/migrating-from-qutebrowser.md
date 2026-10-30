# Migrating from qutebrowser

novabrowser borrows qutebrowser's ideas, key notation, command names and
setting names, so most of what you know carries over. This page lists what
changes.

## How the two compare

qutebrowser renders pages with Chromium through QtWebEngine. novabrowser-webkit
uses WebKitGTK 6.0, the GTK port of the engine behind Safari. (The other
edition of novabrowser uses Chromium through CEF.) The differences come from
that layer and from the code around it:

| | qutebrowser | novabrowser-webkit |
|---|---|---|
| Engine | QtWebEngine: a Chromium release that is usually several versions behind, plus security backports | WebKitGTK 6.0 from your distribution (or the faster `webkitgtk-6.0-nova` build) |
| Browser code | Python + PyQt | Rust; one native binary plus a small library loaded into WebKit's page processes |
| Crashes | a renderer crash can take the whole browser down in some setups | pages run in separate WebKit processes; a crashed tab shows a reload page |
| Video codecs | depends on how your distribution builds QtWebEngine | from GStreamer: install `gst-libav` for H.264/AAC, `gst-plugins-bad` for more; no Widevine DRM |
| Ad blocking | Brave's engine, network rules only (plus hosts lists) | the same engine plus cosmetic filtering and uBlock scriptlets (YouTube ads) |
| Passwords | userscripts (qute-pass, qute-bitwarden) | built in: Bitwarden (rbw/bw) or pass, with suggestions under the field |
| Downloads | built in | built in, or aria2 with resume and torrents |
| Config | `config.py` (Python) | `config.ts` (TypeScript), almost line for line the same |
| Platforms | Linux, macOS, Windows | Linux only |

Site compatibility is Safari's, not Chrome's. By default pages are told the
browser is Safari on a Mac (`content.user_agent`). Sites that need DRM
(Netflix and similar) don't play; use the CEF edition for them.

What qutebrowser has and novabrowser-webkit doesn't (yet): caret mode, marks,
macros, `:view-source`, Greasemonkey scripts, the statusbar/completion
widgets as Qt UI, macOS and Windows. Hint clicks and `fake-key` are
JavaScript events here, not real input (see [scripting](scripting.md#typing-into-pages)).

## Importing your data

```sh
novabrowser-webkit --import qutebrowser
```

Brings over history, quickmarks, bookmarks, and your last session (open it
with `:session-load qutebrowser`). Quickmarks and bookmarks use the same file
formats as qutebrowser, so you can also copy `quickmarks` and
`bookmarks/urls` into `~/.config/novabrowser-webkit/`.

## Porting config.py

Most lines change only in syntax:

| qutebrowser `config.py` | novabrowser-webkit `config.ts` |
|---|---|
| `c.tabs.position = "left"` | `c.tabs.position = "left";` (same) |
| `config.bind("J", "tab-prev")` | `bind("J", "tab-prev");` |
| `config.bind("<Escape>", "mode-leave", mode="insert")` | `bind("<Escape>", "mode-leave", "insert");` |
| `config.unbind("d")` | `unbind("d");` |
| `with config.pattern("*://x.com/*") as p: p.content.javascript.clipboard = "access"` | `pattern("*://x.com/*", (p) => { p.content.javascript.clipboard = "access"; });` |
| `c.aliases["mpv"] = "spawn mpv {url}"` | `alias("mpv", "spawn mpv {url}");` |
| `config.source("theme.py")` | `import "./theme.ts";` |
| `config.load_autoconfig(False)` | delete it |
| `c.qt.args = [...]` | delete it (no engine switches in this edition) |
| `True` / `False` / `None` | `true` / `false` / `null` |
| `config.configdir / "file"` | `` `${configDir}/file` `` |

Start the file with:

```ts
import { c, bind, unbind, alias, pattern, configDir, readLines } from "nova";
```

Then run `novabrowser-webkit --check-config`. Settings it doesn't have
are reported as `unknown setting` with the line number; remove those lines.
The [settings reference](settings.md) lists everything that exists.

Colour themes port directly: every `c.colors.*` and `c.fonts.*` setting that
novabrowser has works the same way. Or use a built-in preset with
`c.theme`.

## Key bindings

The defaults are a trimmed version of qutebrowser's: one key per action, and
only commands that exist. Most daily keys are identical (`o O f F j k H L J K
d u gg G / n N yy pp go gO gi m b B M : ;y`). Notable differences:

| qutebrowser | novabrowser-webkit |
|---|---|
| `wo`, `wi`… open in a new window | `Ctrl-n`, or `:open -w` |
| `wp` / `:open -p` private window | `:open -p` |
| `v` caret mode, `` ` `` / `'` marks, `q` / `@` macros | not available |
| `Sb`, `Sh`, `Ss`, `Sq` open qute:// pages | `Sb` bookmarks, `Sh` history, `Ss` settings, `Sd` downloads |
| `sf`, `ss`, `sl` (`:set` shortcuts) | `:set`, `Ss` |
| `ad`, `cd` download bar keys | `:downloads` (`Sd`) |
| `Ctrl-p` (print) | nothing (no printing yet); `Ctrl-Shift-p` pins the tab |
| `tsh`, `tCH`, etc. (per-site toggles) | `<Space>td` dark mode, `<Space>th` HTTPS-only |

Your own `config.bind` lines keep working after the syntax change. `F1`
shows the result.

## Userscripts

novabrowser-webkit sets qutebrowser's userscript environment (`QUTE_URL`,
`QUTE_FIFO`, `QUTE_HTML`, …) and runs commands written to `$QUTE_FIFO`, so
many userscripts work if you copy them into
`~/.config/novabrowser-webkit/userscripts/` and run them with `:spawn -u`. Scripts
that call qutebrowser-only commands won't. For password managers you
probably don't need one anymore: see [passwords](passwords.md).

## Things that work differently

- One floating box replaces the bottom status bar and completion: `o`,
  `:` and `/` open a rounded palette at the top. Mode, keys and messages
  show at the right end of the tab bar.
- HTTPS-only is always on (local addresses excepted); `<Space>th` makes
  an exception.
- Tabs unload when idle (`tabs.hibernate_after`, 30 minutes). Set it to
  `0` if you don't want that.
- Downloads never ask where to save (`downloads.location.prompt` doesn't
  exist).
- `:set` saves to `autoconfig.json` immediately, like qutebrowser's
  autoconfig, and `config.ts` wins over it.