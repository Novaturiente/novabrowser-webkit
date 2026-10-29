# Settings reference

Every setting novabrowser-webkit understands. Names follow qutebrowser's, so most
qutebrowser configs port over (see [migrating](migrating-from-qutebrowser.md)).
Unknown names and wrong types are errors.

Set them in any of these ways (later ones in this list are overridden by earlier ones):

1. `config.ts`: `c.tabs.position = "left";`. This always wins.
2. `:set tabs.position left` or the `:settings` page, saved to `autoconfig.json`.
3. The `theme` preset (colours only).
4. The defaults below.

"Per site" means the setting also works inside
[`pattern()`](configuration.md#per-site-settings) and `:set -u <pattern>`.
Permission settings (`content.geolocation`, `content.media.*`,
`content.notifications.enabled`, …) are per site too. Other settings inside
`pattern()` are ignored; `novabrowser-webkit --check-config` lists them.

Most settings apply at once. "New tabs" settings are WebKit page settings
fixed when a tab is created: they apply to tabs opened after the change.
"Restart needed" settings are read once at start.

Both editions share this list. In the WebKit edition `chromium.*` settings
are refused by `:set` and ignored in `config.ts`; settings marked "No effect
in the WebKit edition" are accepted but do nothing here.

Value types: bool is `true`/`false`; bool or "ask" also allows `"ask"` (prompt);
percent looks like `"40%"`; list and dict are JSON arrays and objects. On the
`:set` command line write lists and dicts as JSON:
`:set zoom.levels '["50%", "100%", "150%"]'`.

## General

### `theme`

Colour preset for the browser UI (tab bar, palette, popups, built-in pages). Any `colors.*` you set wins over the preset.

- Type: `dark` / `light` / `tokyo-night` / `catppuccin`
- Default: `"\"dark\""`

### `aliases`

Command aliases: `{name: command}`. Expanded before `;;` splitting, recursively (max depth 8).

- Type: dict
- Default: `"{\"w\": \"session-save\", \"q\": \"close\", \"qa\": \"qu…`

### `session.default_name`

Session name used by autosave and by `:session-save` / `:session-load` without a name. Empty = `default`.

- Type: string
- Default: `"null"`

### `auto_save.session`

Save open windows and tabs (with back/forward history and scroll position) and restore them on the next start.

- Type: bool
- Default: `"false"`

### `messages.timeout`

How long messages stay in the tab bar, in ms.

- Type: int
- Default: `"3000"`

### `scrolling.smooth`

Smooth scrolling.

- Type: bool
- Default: `"false"`

### `spellcheck.languages`

Spell-check languages, e.g. `["en-US", "de-DE"]`. Uses the spell-check dictionaries installed on your system; nothing is downloaded. Empty = off.

- Type: list
- Default: `"[]"`

### `window.title_format`

Window title. Same fields as `tabs.title.format`.

- Type: string
- Default: `"\"{perc}{current_title}{title_sep}novabrowser\""`

### `zoom.levels`

Zoom steps used by `+` / `-`.

- Type: list
- Default: `"[\"25%\", \"33%\", \"50%\", \"67%\", \"75%\", \"90%\", \…`


## WebKit engine

These only exist in the WebKit edition (the CEF edition ignores them).

### `content.user_agent`

Which browser pages think you use. `safari` sends macOS Safari's user agent and `navigator.platform` (so some sites offer Mac downloads or show Cmd shortcuts); `webkit` sends WebKitGTK's own Linux user agent; any other text is used as the user agent as is. Per site.

- Type: string (`safari`, `webkit` or your own)
- Default: `"\"safari\""`

### `webkit.itp`

WebKit's Intelligent Tracking Prevention: limits cookies and storage of sites that track you across other sites.

- Type: bool
- Default: `"true"`

### `webkit.site_specific_quirks`

WebKit's built-in fixes for known broken sites.

- Type: bool
- Default: `"true"`

### `webkit.hardware_acceleration`

GPU compositing of pages. `never` turns it off. New tabs.

- Type: `always` / `never`
- Default: `"\"always\""`

## Content and privacy

### `content.autoplay`

Allow media to autoplay. `false` requires a click first.

- Type: bool
- Default: `"true"`

### `content.canvas_reading`

No effect in the WebKit edition (WebKit has no switch for it). In the CEF edition `false` blocks canvas read-back.

- Type: bool
- Default: `"true"`

### `content.cookies.accept`

Which cookies to accept. `no-3rdparty` blocks third-party cookies; `no-unknown-3rdparty` currently behaves the same. Global only in the WebKit edition: a value inside `pattern()` has no effect.

- Type: `all` / `no-3rdparty` / `no-unknown-3rdparty` / `never`
- Default: `"\"all\""`

### `content.fullscreen.window`

Page fullscreen (videos) fills the browser window only, hiding the tab bar, instead of the whole screen.

- Type: bool
- Default: `"false"`

### `content.desktop_capture`

Screen-sharing requests. `true`, `false` or `"ask"`. Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.geolocation`

Location requests. Whether an allowed request gets a location depends on your system's location service (not verified in the WebKit edition). Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.mouse_lock`

Pointer lock requests (games). Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.headers.accept_language`

Languages sent in `Accept-Language`, also used as WebKit's preferred languages.

- Type: string
- Default: `"\"en-US,en;q=0.9\""`

### `content.headers.custom`

Extra HTTP headers sent with every request: `{Header: value}`. Per site: the page's address picks the value, and every request from that page gets it.

- Type: dict
- Default: `"{}"`

### `content.headers.do_not_track`

Send `DNT: 1`. Per site, like `content.headers.custom`.

- Type: bool
- Default: `"true"`

### `content.blocking.enabled`

Ad and tracker blocking (network, cosmetic, scriptlets). Per site, so `pattern()` can turn it off for one site.

- Type: bool
- Default: `"true"`

### `content.blocking.method`

`auto`, `adblock` and `both` use the adblock engine. `hosts` (hosts-file blocking) is not implemented and turns blocking off.

- Type: `auto` / `adblock` / `hosts` / `both`
- Default: `"\"auto\""`

### `content.blocking.adblock.lists`

Filter list URLs (EasyList/uBlock syntax). `file://` paths work. Updated daily in the background and by `:adblock-update`.

- Type: list
- Default: `"[\"https://easylist.to/easylist/easylist.txt\", \"https:…`

### `content.blocking.whitelist`

URL patterns that are never blocked.

- Type: list
- Default: `"[]"`

### `content.javascript.clipboard`

Page access to the clipboard API. Controls clipboard *reads*: `access-paste` allows them, `ask` prompts (answer `A`/`N` to remember per site), `access` and `none` deny them. Per site.

- Type: `none` / `access` / `access-paste` / `ask`
- Default: `"\"ask\""`

### `content.media.audio_capture`

Microphone requests. Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.media.video_capture`

Camera requests. Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.notifications.enabled`

Desktop notification requests. Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.persistent_storage`

In the WebKit edition: requests from a site embedded in another (for example a login or comments frame) to use its own cookies there (the Storage Access API). Per site.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.register_protocol_handler`

No effect in the WebKit edition: WebKit doesn't ask for protocol handlers.

- Type: bool or "ask"
- Default: `"\"ask\""`

### `content.webrtc_ip_handling_policy`

No effect in the WebKit edition. In the CEF edition: which local IP addresses WebRTC may expose.

- Type: `all-interfaces` / `default-public-and-private-interfaces` / `default-public-interface-only` / `disable-non-proxied-udp`
- Default: `"\"all-interfaces\""`


## URLs and search

### `url.auto_search`

When `:open` text is a search: `naive` searches anything with a space or without a dot; `never` always treats it as a URL. `dns` and `schemeless` currently behave like `naive`.

- Type: `naive` / `dns` / `never` / `schemeless`
- Default: `"\"naive\""`

### `url.default_page`

Page opened by `:open` with no argument and by new windows.

- Type: string
- Default: `"\"https://www.google.com/\""`

### `url.searchengines`

Search engines: `{keyword: url}` with `{}` for the query (`{quoted}`, `{semiquoted}`, `{unquoted}` also work). `DEFAULT` is used without a keyword: `:open gh rust` uses `gh`.

- Type: dict
- Default: `"{\"DEFAULT\": \"https://www.google.com/search?q={}\"}"`

### `url.start_pages`

Page(s) opened at start when no session is restored.

- Type: list or value
- Default: `"[\"https://www.google.com/\"]"`


## Tabs

### `tabs.last_close`

What closing the last tab does: `ignore` keeps it, `blank` / `startpage` / `default-page` replace it, `close` closes the window.

- Type: `ignore` / `blank` / `startpage` / `default-page` / `close`
- Default: `"\"ignore\""`

### `tabs.padding`

Padding of each tab in px.

- Type: dict
- Default: `"{\"top\": 0, \"bottom\": 0, \"left\": 5, \"right\": 5}"`

### `tabs.position`

Tab bar position. `left` / `right` give vertical tabs.

- Type: `top` / `bottom` / `left` / `right`
- Default: `"\"top\""`

### `tabs.show`

When to show the tab bar. `switching` currently behaves like `multiple`.

- Type: `always` / `never` / `multiple` / `switching`
- Default: `"\"always\""`

### `tabs.title.alignment`

Alignment of tab titles.

- Type: `left` / `right` / `center`
- Default: `"\"left\""`

### `tabs.title.format`

Tab title. Fields: `{index}`, `{aligned_index}`, `{current_title}`, `{title_sep}` (` - ` when there is a title), `{current_url}`, `{host}`, `{protocol}`, `{perc}` (`[42%] ` while loading), `{perc_raw}`, `{audio}`.

- Type: string
- Default: `"\"{audio}{index}: {current_title}\""`

### `tabs.hibernate_after`

Unload tabs not looked at for this many minutes (`0` = never). Per site. See [tabs](tabs-sessions-profiles.md#tab-hibernation).

- Type: int
- Default: `"30"`

### `tabs.freeze_after`

No effect in the WebKit edition (WebKit has no way to freeze a page). WebKit slows down hidden tabs by itself, and hibernation frees their memory.

- Type: int
- Default: `"5"`

### `tabs.title.format_pinned`

Title of pinned tabs that aren't focused (same fields); the focused one uses
`tabs.title.format`. `"{audio}"` shows just the icon.

- Type: string
- Default: `"\"{audio}{index}\""`

### `tabs.width`

Width of the vertical tab bar, as a percentage of the window.

- Type: percent
- Default: `"\"15%\""`

### `tabs.min_width`

Minimum tab width in px (horizontal tabs). `-1` = automatic.

- Type: int
- Default: `"-1"`

### `tabs.max_width`

Maximum tab width in px (horizontal tabs). `-1` = automatic.

- Type: int
- Default: `"-1"`

### `tabs.indicator.width`

Width of the loading indicator in px.

- Type: int
- Default: `"3"`

### `tabs.indicator.padding`

Padding around the loading indicator in px.

- Type: dict
- Default: `"{\"top\": 2, \"bottom\": 2, \"left\": 0, \"right\": 4}"`


## Hints

### `hints.border`

CSS border of hint labels.

- Type: string
- Default: `"\"1px solid #E3BE23\""`

### `hints.padding`

Padding of hint labels in px: `{top, bottom, left, right}`.

- Type: dict
- Default: `"{\"top\": 0, \"bottom\": 0, \"left\": 3, \"right\": 3}"`

### `hints.radius`

Corner radius of hint labels in px.

- Type: int
- Default: `"3"`

### `hints.chars`

Characters used for hint labels.

- Type: string
- Default: `"\"asdfghjkl\""`

### `hints.min_chars`

Minimum label length.

- Type: int
- Default: `"1"`

### `hints.selectors`

CSS selectors per hint group (`all`, `links`, `images`, `media`, `url`, `inputs`, or your own), used by `:hint <group>`.

- Type: dict
- Default: `"{\"all\": [\"a\", \"area\", \"textarea\", \"select\", \"…`

### `hints.uppercase`

Show labels in upper case (you still type lower case).

- Type: bool
- Default: `"false"`


## Input and key bindings

### `input.insert_mode.auto_enter`

Enter insert mode when a text field gets focus (click, `gi`, page autofocus).

- Type: bool
- Default: `"true"`

### `input.insert_mode.auto_leave`

Leave insert mode when focus leaves a text field.

- Type: bool
- Default: `"true"`

### `bindings.default`

The built-in key bindings (see [keybindings](keybindings.md)). Setting it replaces all of them; usually use `bindings.commands` or `bind()` instead.

- Type: dict
- Default: `"{}"`

### `bindings.commands`

Your key bindings: `{mode: {key: command}}`. Same effect as `bind()`; `null` as the command unbinds.

- Type: dict
- Default: `"{}"`


## Completion, downloads, passwords

### `passwords.backend`

Password manager CLI used for login suggestions and saving. See [passwords](passwords.md).

- Type: `rbw` / `bw` / `pass` / `none`
- Default: `"\"rbw\""`

### `completion.height`

Height of the completion list in the palette, as a percentage of the window.

- Type: percent
- Default: `"\"50%\""`

### `downloads.location.directory`

Download folder. Empty = `~/Downloads`. `~` is expanded.

- Type: string
- Default: `"null"`

### `completion.open_categories`

Completion sources for `:open`, in order: `searchengines`, `quickmarks`, `bookmarks`, `history` (`filesystem` is accepted but not built).

- Type: list
- Default: `"[\"searchengines\", \"quickmarks\", \"bookmarks\", \"his…`

### `downloads.open_dispatcher`

Program for `:download-open`, with `{}` for the file. Empty = `xdg-open`.

- Type: string
- Default: `"null"`

### `statusbar.padding`

Padding of the mode/message badge in px.

- Type: dict
- Default: `"{\"top\": 1, \"bottom\": 1, \"left\": 0, \"right\": 0}"`

### `downloads.backend`

`auto` uses aria2 when `aria2c` is installed, `aria2` asks for it explicitly (falls back with a warning), `webkit` never uses it (WebKit's own downloader). `chromium` is accepted and means the same as `webkit`. Restart needed. See [downloads](downloads.md).

- Type: `auto` / `aria2` / `chromium` / `webkit`
- Default: `"\"auto\""`


## Web page appearance

### `colors.webpage.bg`

Background shown before a page paints. New tabs.

- Type: string
- Default: `"\"white\""`

### `colors.webpage.preferred_color_scheme`

What `prefers-color-scheme` reports to pages. Not wired up in the WebKit edition yet: no effect.

- Type: `auto` / `light` / `dark`
- Default: `"\"auto\""`

### `colors.webpage.darkmode.enabled`

Dark mode for light pages, using WebKit's colour filter (see [privacy](privacy.md#dark-mode)). Pages that are already dark are left alone. Per site; `<Space>td` toggles it for a site.

- Type: bool
- Default: `"false"`

### `colors.webpage.darkmode.algorithm`

No effect in the WebKit edition (CEF only).

- Type: `lightness-cielab` / `lightness-hsl` / `brightness-rgb`
- Default: `"\"lightness-cielab\""`

### `colors.webpage.darkmode.policy.images`

No effect in the WebKit edition: images always keep their colours.

- Type: `always` / `never` / `smart` / `smart-simple`
- Default: `"\"smart\""`

### `fonts.default_family`

Font family used where a font setting says `default_family`.

- Type: list or value
- Default: `"[]"`

### `fonts.default_size`

Size used where a font setting says `default_size`.

- Type: string
- Default: `"\"10pt\""`


## Web page fonts

Defaults for pages that don't set their own font. Empty family = WebKit's default. Applies to tabs opened after the change. `fonts.web.size.minimum_logical` has no effect in the WebKit edition.

| Setting | Default |
|---|---|
| `fonts.web.family.standard` | `"\"\""` |
| `fonts.web.family.fixed` | `"\"\""` |
| `fonts.web.family.serif` | `"\"\""` |
| `fonts.web.family.sans_serif` | `"\"\""` |
| `fonts.web.family.cursive` | `"\"\""` |
| `fonts.web.family.fantasy` | `"\"\""` |
| `fonts.web.size.default` | `"16"` |
| `fonts.web.size.default_fixed` | `"13"` |
| `fonts.web.size.minimum` | `"0"` |
| `fonts.web.size.minimum_logical` | `"6"` |

## UI fonts

qutebrowser font syntax: `[weight] [size] family`, e.g. `"bold 11pt JetBrains Mono"`; `default_size` / `default_family` refer to the settings above.

| Setting | Default |
|---|---|
| `fonts.completion.entry` | `"\"default_size default_family\""` |
| `fonts.completion.category` | `"\"bold default_size default_family\""` |
| `fonts.tooltip` | `"null"` |
| `fonts.contextmenu` | `"null"` |
| `fonts.hints` | `"\"bold default_size default_family\""` |
| `fonts.keyhint` | `"\"default_size default_family\""` |
| `fonts.statusbar` | `"\"default_size default_family\""` |
| `fonts.tabs.selected` | `"\"default_size default_family\""` |
| `fonts.tabs.unselected` | `"\"default_size default_family\""` |

## UI colours

Any CSS colour (`#rrggbb`, `rgba(…)`, names). qutebrowser `qlineargradient(…)` values are turned into a top-to-bottom gradient. The defaults below are qutebrowser's; the active `theme` preset overrides them unless you set a colour yourself. Not every colour has a matching UI element: the bottom status bar is replaced by a badge in the tab bar, so `colors.statusbar.*` colour that badge.

| Setting | Default |
|---|---|
| `colors.completion.fg` | `"[\"white\", \"white\", \"white\"]"` |
| `colors.completion.even.bg` | `"\"#333333\""` |
| `colors.completion.category.fg` | `"\"white\""` |
| `colors.completion.category.border.top` | `"\"black\""` |
| `colors.completion.item.selected.fg` | `"\"black\""` |
| `colors.completion.item.selected.bg` | `"\"#e8c000\""` |
| `colors.tooltip.bg` | `"null"` |
| `colors.tooltip.fg` | `"null"` |
| `colors.contextmenu.menu.bg` | `"null"` |
| `colors.contextmenu.menu.fg` | `"null"` |
| `colors.contextmenu.selected.bg` | `"null"` |
| `colors.contextmenu.selected.fg` | `"null"` |
| `colors.downloads.start.fg` | `"\"white\""` |
| `colors.downloads.start.bg` | `"\"#0000aa\""` |
| `colors.downloads.stop.fg` | `"\"white\""` |
| `colors.downloads.stop.bg` | `"\"#00aa00\""` |
| `colors.downloads.error.fg` | `"\"white\""` |
| `colors.downloads.error.bg` | `"\"red\""` |
| `colors.hints.fg` | `"\"black\""` |
| `colors.hints.bg` | `"\"qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 rgba(25…` |
| `colors.hints.match.fg` | `"\"green\""` |
| `colors.keyhint.fg` | `"\"#FFFFFF\""` |
| `colors.keyhint.suffix.fg` | `"\"#FFFF00\""` |
| `colors.keyhint.bg` | `"\"rgba(0, 0, 0, 80%)\""` |
| `colors.messages.error.fg` | `"\"white\""` |
| `colors.messages.error.bg` | `"\"red\""` |
| `colors.messages.warning.fg` | `"\"black\""` |
| `colors.messages.warning.bg` | `"\"darkorange\""` |
| `colors.messages.info.fg` | `"\"white\""` |
| `colors.messages.info.bg` | `"\"black\""` |
| `colors.prompts.fg` | `"\"white\""` |
| `colors.prompts.border` | `"\"1px solid gray\""` |
| `colors.prompts.bg` | `"\"#444444\""` |
| `colors.statusbar.normal.fg` | `"\"white\""` |
| `colors.statusbar.normal.bg` | `"\"black\""` |
| `colors.statusbar.insert.fg` | `"\"white\""` |
| `colors.statusbar.insert.bg` | `"\"darkgreen\""` |
| `colors.statusbar.passthrough.fg` | `"\"white\""` |
| `colors.statusbar.passthrough.bg` | `"\"darkblue\""` |
| `colors.statusbar.command.fg` | `"\"white\""` |
| `colors.statusbar.command.bg` | `"\"black\""` |
| `colors.statusbar.progress.bg` | `"\"white\""` |
| `colors.tabs.bar.bg` | `"\"#555555\""` |
| `colors.tabs.indicator.start` | `"\"#0000aa\""` |
| `colors.tabs.indicator.stop` | `"\"#00aa00\""` |
| `colors.tabs.indicator.error` | `"\"#ff0000\""` |
| `colors.tabs.odd.fg` | `"\"white\""` |
| `colors.tabs.odd.bg` | `"\"grey\""` |
| `colors.tabs.even.fg` | `"\"white\""` |
| `colors.tabs.even.bg` | `"\"darkgrey\""` |
| `colors.tabs.selected.odd.fg` | `"\"white\""` |
| `colors.tabs.selected.odd.bg` | `"\"black\""` |
| `colors.tabs.selected.even.fg` | `"\"white\""` |
| `colors.tabs.selected.even.bg` | `"\"black\""` |