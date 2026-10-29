# Getting started

This is the guide for **novabrowser-webkit**, the WebKitGTK edition of
novabrowser. Keys, commands and `config.ts` are the same as in the CEF
(Chromium) edition; the differences are noted on each page.

## Install

The AUR packages are not published yet. On Arch Linux, build the package
from a checkout:

```sh
cd packaging
NOVA_SRC=file://$PWD/.. makepkg -si
xdg-settings set default-web-browser novabrowser-webkit.desktop   # optional
```

It runs on Arch's `webkitgtk-6.0`. The optional `webkitgtk-6.0-nova` package
is a faster WebKitGTK build with a better dark mode; it needs an AVX2
(x86-64-v3) CPU. For most web video install `gst-libav` (H.264/AAC), and
`gst-plugins-bad` for more codecs.

Other distributions: see [building](building.md).

The WebKit edition can run next to the CEF edition: its binary, config and
data directories are all named `novabrowser-webkit`.

## First start

Run `novabrowser-webkit` (or pick "novabrowser (WebKit)" from your app
launcher). The first start opens a welcome page with the basic keys and the
import commands for the browsers found on your machine. `:help` brings it
back.

## Bring your data

Quit the browser first, then:

```sh
novabrowser-webkit --import            # list the browser profiles found, with sizes
novabrowser-webkit --import firefox    # history + bookmarks from the biggest Firefox profile
novabrowser-webkit --import /path/to/profile
```

Supported: Chrome, Chromium, Brave, Helium, Vivaldi, Edge, Firefox, Zen,
LibreWolf, Floorp and qutebrowser (history, bookmarks, quickmarks, and the
session as `:session-load qutebrowser`). Importing twice doesn't create
duplicates. Passwords are not imported: novabrowser uses your
[password manager](passwords.md) directly.

Coming from the CEF edition: `novabrowser-webkit --import novabrowser`
(add `--profile=<name>` for a named profile). History, command history,
quickmarks and bookmarks are merged. `config.ts` and the files next to it,
sessions, saved permission answers and the downloads list are copied, but
only where this edition has none yet; nothing is overwritten. Cookies,
logins, site data and per-site zoom are not carried over: sign in again.

## The basic keys

The browser starts in normal mode: letters are commands, not text.

1. `o`, type `wikipedia.org`, `Enter`. Typing something that isn't a URL
   searches for it.
2. `j` / `k` scroll, `Ctrl-d` / `Ctrl-u` half a page, `gg` / `G` top/bottom.
3. `f` shows labels on every link; type a label to click it. `F` opens it in
   a new tab.
4. `J` / `K` switch tabs, `d` closes one, `u` brings it back.
5. `H` / `L` go back / forward.
6. Click a search box (or press `gi`): you're in insert mode and can
   type. `Escape` goes back to normal mode.
7. `/word` finds text, `n` / `N` jump between matches.
8. `yy` copies the URL, `pp` opens the URL in your clipboard.
9. `:` opens the command line; `Tab` completes.
10. `F1` lists every key. After a prefix key like `g`, a popup shows what
    can follow.

The mouse works everywhere too.

## Changing settings

- `Ss` opens the settings page: theme, search engine, start page, tab bar
  position, dark mode, ad blocking, cookies, downloads, password manager.
- `:set <setting> <value>` changes any setting ([reference](settings.md)).
- `~/.config/novabrowser-webkit/config.ts` holds key bindings, per-site rules
  and anything you want to keep in git ([configuration](configuration.md)).

Turn on session restore, so your tabs come back after a restart:

```
:set auto_save.session true
```

## How this edition differs

- Pages see **macOS Safari** by default (`content.user_agent`), so some sites
  offer Mac downloads or show Cmd shortcuts.
- **No DRM (Widevine).** Netflix and similar sites won't play; use the CEF
  edition for them.
- Hint clicks, `fake-key` and scrolling are JavaScript events, not real
  input; a few sites ignore them.
- Known site issue: HDFC NetBanking shows an "upgrade your browser"
  message. In testing it appeared with the stock WebKit user agent
  (`content.user_agent = "webkit"`), not with the default Safari one.

## Next

- [Key bindings](keybindings.md) and [commands](commands.md)
- [Tabs, sessions and profiles](tabs-sessions-profiles.md)
- [Passwords](passwords.md), [downloads](downloads.md), [privacy](privacy.md)
- Coming from qutebrowser? Read [migrating](migrating-from-qutebrowser.md).
