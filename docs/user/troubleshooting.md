# Troubleshooting

The browser has no log file of its own: start `novabrowser-webkit` from a
terminal to see its errors (or read the journal when your desktop started
it). Downloads via aria2 log to `~/.local/state/novabrowser-webkit/aria2.log`.

## Typing does nothing / letters trigger commands

You're in normal mode. Click the field or press `i` (or `gi` for the first
field) to type; `Escape` returns to normal mode. The mode is shown at the
right end of the tab bar.

## A site doesn't work right

1. Turn off ad blocking for it:
   `:set -u '*://*.example.com/*' content.blocking.enabled false`, reload.
   If that fixes it, make it permanent with `pattern()` in config.ts.
2. Try WebKit's own user agent instead of Safari's:
   `:set -u '*://*.example.com/*' content.user_agent webkit`, reload.
   Some sites behave differently for the two.
3. If it needs third-party cookies (some logins): `content.cookies.accept`
   must be `all`. In this edition that setting is global, not per site.
4. If it only works over plain HTTP: `<Space>th`.
5. If dark mode makes it unreadable: `<Space>td`.
6. If a hint click does nothing, click with the mouse: hint clicks are
   synthetic DOM clicks, and some pages ignore those.

If it needs DRM (Netflix and similar), it won't play in this edition; use
the CEF edition.

Known: HDFC NetBanking shows an "upgrade your browser" message with both
the default Safari user agent and the stock WebKit one. Use the CEF edition
for it.

## Video doesn't play

Web video is decoded by GStreamer. Install `gst-libav` (H.264/AAC, used by
most sites) and `gst-plugins-bad` (more codecs). When a video fails to
decode, the tab bar says *"Missing codec? Install gst-libav and
gst-plugins-bad."* `gst-plugin-va` adds hardware decoding.

## Paste doesn't work in a web terminal

Web terminals often turn `Ctrl-V` into a control character. Use
`Ctrl-Shift-V` (`:paste`), `Shift-Insert`, or right-click → Paste.

The clipboard and primary selection go through GTK, so no `wl-clipboard` or
`xclip` is needed.

## A page asks to read the clipboard

That's the `content.javascript.clipboard = "ask"` default. Answer `A` to
always allow that site.

## Google shows a captcha / "unusual traffic"

Usually your IP address or network is flagged, not the browser: a fresh
browser on the same network gets the same page. It goes away by itself.

## "no running instance"

`--cmd`, `--query` and `--key` need a running novabrowser-webkit with the
same profile. Pass `--profile=name` for named profiles.

## The browser won't start or exits at once

- Another instance on the same profile is still shutting down: wait a
  second and try again.
- Run it from a terminal to see the error.
- Check the config: `novabrowser-webkit --check-config`. A broken
  `config.ts` at startup is reported, and the defaults are used.
- Pages render wrong or blank: try `:set webkit.hardware_acceleration never`
  and open a new tab.

## No ad blocking

If the terminal shows `web process extension not found; no network adblock
or subresource HTTPS-only`, the browser can't find
`libnovabrowser_webkit_ext.so`. It looks in `$NOVA_WEBKIT_EXT_DIR`, next to
the binary, then in `/usr/lib/novabrowser-webkit`. See
[building](building.md).

## Password suggestions don't appear

- Check the backend works in a terminal: `rbw list`, `bw list items`, or
  `pass ls`.
- `bw` needs `BW_SESSION` in the browser's environment.
- Suggestions only show for logins whose saved URL shares the site's domain.
- Login forms inside frames get no suggestions in this edition (top frame
  only).

## A download goes through WebKit instead of aria2

Expected for form (POST) downloads, `blob:` and `data:` files, and when
`aria2c` isn't installed or `downloads.backend = "webkit"` (or
`"chromium"`). Restart after installing aria2.

## Known limits

- Linux only. X11 support exists but is less tested than Wayland.
- HiDPI (scaling above 100%) and input methods (IME, compose) are not
  verified in the WebKit edition.
- Hint labels don't appear inside frames from another site.
- No DRM (Widevine), printing, Chrome extensions, or sync.
- Hint clicks, `fake-key` and scrolling are JavaScript events, not real
  input; CSS-only hover menus don't open with the `hover` hint target.
- Memory use is about 1.6 times the CEF edition's with the same sites open.

## Reporting a bug

Open an issue on GitHub with: what you did, what happened, your
distribution, Wayland or X11, which WebKitGTK package you use
(`webkitgtk-6.0` or `webkitgtk-6.0-nova`), the output of
`novabrowser-webkit --check-config`, and the terminal output. Never paste
cookies, passwords or `aria2.session`.
