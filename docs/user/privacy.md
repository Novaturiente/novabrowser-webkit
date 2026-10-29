# Privacy and content blocking

## Ad and tracker blocking

Built in and on by default, using Brave's
[adblock-rust](https://github.com/brave/adblock-rust) engine with
EasyList/uBlock Origin filter syntax. In this edition the engine runs inside
WebKit's page processes, through the extension library
`libnovabrowser_webkit_ext.so` (see [building](building.md)):

- Network blocking of ads and trackers. The page itself (the main frame) is
  never blocked.
- Cosmetic filtering, which hides ad containers, including generic class/id
  rules.
- Scriptlets (uBlock Origin's `+js(...)` rules), injected before page
  scripts run. They remove YouTube ads.

Default lists: EasyList and EasyPrivacy. Add more:

```ts
c.content.blocking.adblock.lists = [
  "https://easylist.to/easylist/easylist.txt",
  "https://easylist.to/easylist/easyprivacy.txt",
  "https://secure.fanboy.co.nz/fanboy-annoyance.txt",
  "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt",
  `file://${configDir}/my-filters.txt`,   // your own rules
];
```

Lists are compiled once and cached, so startup never waits for them. They
update daily in the background; `:adblock-update` updates now.

Turn blocking off for one site:

```ts
pattern("*://*.example.com/*", (p) => { p.content.blocking.enabled = false; });
```

Limits:

- No element picker.
- No procedural cosmetic filters (`:has-text` and similar).
- Generic class/id hiding works in the top frame only.
- The scriptlet code is bundled into the browser when it is built. Filter
  list rules update daily, but a rule that needs a newer scriptlet only works
  after a browser update.
- Hiding can flash briefly on slow pages.
- If the extension library isn't found, the browser prints
  `web process extension not found; no network adblock or subresource HTTPS-only`
  on stderr, and network blocking is off.

## Cookies

`content.cookies.accept`:

| Value | Effect |
|---|---|
| `all` (default) | accept everything |
| `no-3rdparty` | block third-party cookies |
| `never` | block all cookies |

In this edition the setting is global: per-site values in `pattern()` have
no effect. On top of it, WebKit's Intelligent Tracking Prevention
(`webkit.itp`, on by default) limits cookies and storage of sites that track
you across other sites. When a site embedded in another page (a login or
comments frame, for example) asks to use its own cookies there, you get a
prompt; that answer is the `content.persistent_storage` setting.

## HTTPS-only

Every `http://` page load is upgraded to `https://`, including redirects and
the page's own requests. Exempt automatically: `localhost`, hostnames without
a dot, `.local`, `.lan`, `.internal`, `.home`, `.arpa`, `.ts.net`, private
IPv4/IPv6 ranges and Tailscale (`100.64.0.0/10`). For other sites that only
work over HTTP press `<Space>th` (`:https-toggle`); the host is saved in
`~/.config/novabrowser-webkit/https_excludes`.

The page's own requests (images, scripts) are upgraded by the extension
library, like ad blocking; without it only the page address is upgraded.

## Permissions

Camera, microphone, screen sharing, location, notifications, clipboard
reading, pointer lock and storage access (see Cookies above) all ask by
default. A request to list your cameras and microphones follows the camera
setting. DRM (media key) requests are always refused: this edition has no
DRM.

Answer in the tab bar with `y`, `n`, `A` (always for this site) or `N`
(never). Always/Never answers are stored in
`~/.local/share/novabrowser-webkit/permissions.json`; delete a line there to
be asked again. Set a policy instead of being asked:

```ts
c.content.notifications.enabled = false;              // never ask, deny
pattern("*://app.slack.com/*", (p) => {
  p.content.notifications.enabled = true;
  p.content.media.audio_capture = true;
});
```

## Certificate errors

When WebKit refuses a site's certificate, the tab bar asks
*"<host>: certificate not trusted (<reason>). Load it anyway this run?"*
`y` accepts that certificate for that host until the browser quits and
reloads the page; `n` keeps it blocked.

## Headers and fingerprinting

- `DNT: 1` is sent by default (`content.headers.do_not_track`).
- `content.headers.custom` adds headers, e.g. `{"Sec-GPC": "1"}`.
- By default pages see macOS Safari (`content.user_agent = "safari"`): the
  user agent and `navigator.platform` say Mac. Fingerprinting scripts can
  still tell it's Linux from other signals, such as fonts.
- `content.canvas_reading` and `content.webrtc_ip_handling_policy` have no
  effect in this edition.

## Dark mode

`colors.webpage.darkmode.enabled` (per site, `<Space>td` toggles it) uses
WebKit's colour filter: light pages have their colours turned dark while
they are drawn. Pages that are already dark are left alone. Images and
videos keep their colours.

It works best with the `webkitgtk-6.0-nova` package, whose patch treats text
and backgrounds separately: dark headers and code blocks on light pages stay
dark, and text never gets darker. With Arch's stock `webkitgtk-6.0` the
filter inverts lightness for everything.

## Background network calls

With no page open, the browser makes no network connections except fetching
the ad-blocking lists. In a 90-second capture on `about:blank`, the only
connection was the filter list download on a fresh profile; with the lists
cached there were none. Spell checking uses dictionaries already on your
system.

WebKitGTK has no Safe Browsing, so the browser does not warn about known
phishing or malware sites. Ad-blocking lists catch some of them.
