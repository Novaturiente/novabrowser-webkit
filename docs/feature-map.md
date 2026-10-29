# Feature map: CEF edition → WebKitGTK edition

Every feature the CEF edition (novabrowser 0.1.8) has, how it does it, and how
the WebKitGTK 6.0 edition will. API names are WebKitGTK 6.0 (`webkit_…`
functions, `Signal::name` signals) unless marked.

Rev 2 (2026-10-28): decisions D1–D7 are in PLAN.md §2. Where a row below says
"D2" the chrome is HTML/CSS in WebViews; "D3" = adblock engine in the extension;
"D6" = WebKit colour filter (`-apple-color-filter`) dark mode.

Verdict: **same** (direct equivalent), **better** (WebKit does more),
**worse** (works with a known limit), **gap** (not possible; what we do instead),
**spike** (unknown until a Phase 0 spike, PLAN.md §5).

---

## 1. Window, tabs, layout

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Window + layout | CEF Views: box layout, tab bar BrowserView, content panel, overlays (`add_overlay_view`) | GtkApplicationWindow → GtkOverlay → GtkBox (tab bar + GtkStack of WebViews) | same |
| `tabs.position` top/bottom/left/right, `tabs.width` (px or %), `tabs.show` never/always/multiple | `apply_layout` re-lays Views | GtkBox orientation + child order; width request | same |
| One view per tab, hidden tabs throttled | swap content child + `was_hidden` | GtkStack visible child; WebKit throttles unmapped views [Likely] | same |
| Lazy restore (background tabs load on first show) | CEF create-on-attach | create the WebView on first show | same |
| Popups / `target=_blank` / `window.open` as tabs, opener kept | `on_before_popup` + `on_popup_browser_view_created` | `WebView::create` signal returns a new view built with `related-view` | same |
| Middle / Ctrl-click → background tab | `on_open_urlfrom_tab` | `decide-policy` NEW_WINDOW_ACTION / NAVIGATION_ACTION with `navigation_action_get_mouse_button` / `get_modifiers` | same |
| Popup flood guard (audit A7, open) | not done | `navigation_action_is_user_gesture` in `create` | better (easy to add) |
| Pinned tabs, pinned group kept on moves, `format_pinned` | Rust | unchanged Rust | same |
| Drag tabs to reorder | tab bar JS mousedown/up → `move N` | GTK drag gesture (D2) or the same JS | same |
| Tab tooltip (title + URL, 450 ms) | tip overlay | `GtkWidget` tooltip or overlay popover | same |
| Tab right-click menu (reload, duplicate, copy URL, mute, bookmark, pin, move to window, hibernate, keep awake, close, close others) | menu overlay | GtkPopoverMenu with GActions | same |
| `tab-give` (move tab to a new window) | reloads the page in a new window (CEF can't move a live browser) | reparent the live WebView into the new window [Likely] | better |
| Multiple windows per profile, per-window state | `WINS` slots | same structure, GtkWindows | same |
| Window title (`window.title_format`, `[profile]` prefix), Wayland app id `novabrowser` | `linux_window_properties` | `gtk_window_set_title`; app id from GtkApplication id (`site.novabrowser.webkit` or similar) | same |
| Favicons in tabs | `download_image` → PNG → base64 → HTML | `notify::favicon` → GdkTexture → GtkImage (favicons enabled on the network session's data manager) | better (no base64) |
| Tab mute + `{audio}` in titles | `tab-mute` | `set_is_muted`; `is-playing-audio` gives a real audio indicator | better |
| Last-tab close: close / blank / startpage / default-page / ignore | Rust | unchanged | same |
| Undo close (last 100, across restarts) | URL list | URL list + session state blob → back history comes back too | better |
| Crash page ("This tab crashed (why)", `r` reloads) | `on_render_process_terminated` | `web-process-terminated` (CRASHED / EXCEEDED_MEMORY_LIMIT / TERMINATED_BY_API) | same (tabs sharing a process crash together [Likely]) |
| Hang prompt ("isn't responding. Kill it?") | `on_render_process_unresponsive` callback | `notify::is-web-process-responsive` + `terminate_web_process` | same |
| Fullscreen inside the window (`content.fullscreen.window`), `:fullscreen` | `on_fullscreen_mode_change` | `enter-fullscreen` / `leave-fullscreen` (return TRUE = we handle it: hide chrome only) | same |
| Idle tab hibernation (`tabs.hibernate_after`, exemptions: media, typed forms, keep-awake, active, loading, crashed, internal pages) | swap view for an unloaded one; back/fwd via saved lists | save `get_session_state`, destroy the WebView, restore on show; busy check JS unchanged | better (real back history) |
| Freeze background tabs (`tabs.freeze_after`) | CDP `Page.setWebLifecycleState frozen` | no API | **gap**: rely on WebKit's hidden-page throttling; hibernation covers memory. Drop the setting or keep it as an alias of a shorter hibernate |
| Which-key popup after a prefix (350 ms) | keyhint overlay HTML | overlay widget (D2) | same |
| Tab bar status: mode badge, pending keys, messages with timeout, `[i/n]`, `[<>]` history arrows, downloads pill, 2 px load line, profile pill | `nova.apply(json)` into tabbar.html | widgets or the same HTML (D2) | same |

## 2. Keys, modes, commands

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Key interception before the page | `on_pre_key_event` RAWKEYDOWN, return 1 | `GtkEventControllerKey` capture phase on the window, `Propagation::Stop` | spike W3 |
| Key → notation | `key_from_cef(vk, char, mods)` | new `key_from_gdk(keyval, state)`; Ctrl+letter is a plain keyval in GDK, so the control-code workaround goes away | same |
| Modes normal / insert / passthrough / command / hint / prompt; counts; trie; `;;` chaining; aliases; `forward_unbound_keys=auto` | `keys.rs`, `modes.rs`, `commands.rs` | copied | same |
| Auto insert mode on focusing an editable | renderer `on_focused_node_changed` + `is_editable` | `focusin`/`focusout` listener in world `novabrowser`, all frames, → message handler | spike W4 |
| IME / compose in insert mode | not tested (none installed) | GTK IM context of the WebView; capture controller must pass keys while the IM is composing | spike W1/W3 |
| All commands (open, tab-*, back/forward, reload, stop, scroll*, zoom*, search*, hint, spawn, fake-key, insert-text, yank, download*, password-fill, adblock-update, mode-*, cmd-set-text, message-*, fullscreen, devtools, session-*, quickmark-*, bookmark-*, history, downloads, settings, help, bindings, profile-*, set, config-cycle, config-source, paste, dark-toggle, https-toggle, close, quit) | `commands.rs` | parser/registry copied; each handler re-pointed to WebKit calls below | same, except notes below |
| `back` / `forward` | page-world `history.go` (audit A8: page can trap it) + `location.replace` for restored entries | `go_back` / `go_forward` / `go_to_back_forward_list_item` | better (fixes A8) |
| `reload [-f]`, `stop` | CEF | `reload` / `reload_bypass_cache`, `stop_loading` | same |
| `scroll` / `scroll-page` / `scroll-to-perc` | trusted wheel events via CDP (scrolls the element under the cursor) | JS in world `novabrowser`: scroll the active scroller (port Vimium's "find the scrollable element" logic), `scrolling.smooth` → `behavior` | worse (spike W5 on inner scrollers) |
| `zoom`, `zoom-in/out`, levels, per-site memory | Chromium per-host zoom prefs | `set_zoom_level` per view + own per-host store (`zoom.json`) applied on `load-changed` COMMITTED | same (own code) |
| `paste` (`Ctrl-Shift-v`) | `Frame::paste` | `execute_editing_command(PASTE)` | same |
| `open -p` private | silently a normal tab (audit C3) | ephemeral `NetworkSession` window | better |
| `devtools` | DevTools in own window | `WebKitWebInspector` (`show`, attach/detach), needs `enable-developer-extras` | same |
| `view-source` | not built | `view-source:` isn't supported by WebKitGTK [Guessing]; fetch main resource (`get_main_resource` → `get_data`) into a text tab | same effort |

## 3. Command palette, completion, history

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Floating palette for `o`, `O`, `:`; hidden prefixes (`:open -t `…) with placeholders; `/` `?` as a thin strip | `palette.html` overlay, `can_activate=1`, posted focus | GtkOverlay child: GtkEntry + GtkListView (D2) | same |
| Blurred page behind the palette | CDP `Page.captureScreenshot` JPEG → CSS background | `WidgetPaintable` of the page in a `GtkPicture` with GTK CSS `filter: blur() brightness()`, faded in with the palette (W15) | same / better |
| Readline keys (Ctrl-a/e/b/f/u/k/w/y/h, Alt-b/f/d), Up/Down vs Ctrl-p/n rules | palette.html JS | key handler on the entry | same |
| Completion sources: commands, quickmarks, bookmarks, history (FTS5 trigram, typed → visits → recency), profiles, bindings | `completion.rs`, `history.rs` | copied | same |
| History DB (`history.sqlite`), command history, `:history` page, import | rusqlite thread, `post_task` back | same thread, `glib` invoke back | same |

## 4. Hints, find, yank, clipboard

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Hint collection (same-origin iframes, open shadow roots, modal `<dialog>`), labels in a closed shadow root, top-layer popover, `color-scheme: only light` | `hints.js` in CDP isolated world | same `hints.js` via `evaluate_javascript(world="novabrowser")` | same |
| Label generation (`_hint_scattered`), `hints.chars`, uppercase, min_chars, rapid, `--first`, groups from `hints.selectors` | `hints.rs` | copied (fix audit A1: < 2 chars loops) | same |
| Follow (normal/current) | trusted `Input.dispatchMouseEvent` at the element centre | JS: `focus()` + `click()`; links: navigate to `href` | **worse**: `isTrusted=false`; popups opened by the click are blocked [Likely]; spike W5 |
| `hover` target | trusted mouse move (CSS `:hover` works) | dispatch `mouseover`/`mouseenter` | **gap** for CSS-only hover menus |
| tab / tab-bg / tab-fg / window / yank / yank-primary / fill / spawn / run targets | Rust | unchanged | same (fix audit A5 while porting) |
| Find in page, smart case, incremental, `n`/`N` | `BrowserHost::find` | `WebKitFindController` (`search`, `search_next/previous`, CASE_INSENSITIVE when all-lowercase, WRAP_AROUND) | same; `[i/n]` shows total only (`counted-matches`) [Likely] |
| Clipboard write (clipboard + primary) | `execCommand('copy')` in the tab bar page, else `wl-copy`/`xclip` | GdkClipboard (`display.clipboard()`, `primary_clipboard()`) | better (no external tools) |
| Clipboard read (`{clipboard}`, `{primary}`, 1 MiB cap) | `wl-paste`/`xclip -o` on a thread | `read_text_future` on GdkClipboard | better |
| `yank` url/pretty-url/title/domain/selection/inline | Rust + CDP for selection | Rust + JS `getSelection()` | same |

## 5. Config, settings, theme

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| `config.ts` (oxc strip → QuickJS; `c`, `bind`, `unbind`, `alias`, `pattern`, `readLines`, `configDir`, `profile`; errors name the line; broken file keeps old config) | `config.rs` + `config.js` | copied (fix audit A2: QuickJS limits) | same |
| `:set` saved to `autoconfig.json`, `-t`, `-u pattern`, `config-cycle`, `config-source`, `--check-config` | Rust | copied | same |
| Precedence `:set -t` → config.ts → autoconfig → preset → default; per-site `get_for` | Rust | copied | same |
| `:settings` page (17 rows) | data: page + token | `nova://settings` + token | same |
| Themes dark/light/tokyo-night/catppuccin, all `colors.*` / `fonts.*` | `theme.rs` → CSS vars into HTML UI | `theme.rs` → GTK CSS (native chrome) + same CSS vars for `nova://` pages | same |
| `chromium.*` settings (`chromium.args`, low-end mode) | Chromium switches | removed; new `webkit.*` group (docs/porting.md §3) | changed |
| Fonts for web content (`fonts.web.family.*`, `fonts.web.size.*`) | BrowserSettings | WebKitSettings `*-font-family`, `default-font-size`, `minimum-font-size` | same |
| `colors.webpage.bg` | BrowserSettings background | `set_background_color` | same |

## 6. Privacy, content, network

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Adblock network rules (lists, 24 h cache, hourly check, `:adblock-update`, whitelist, per-site `content.blocking.enabled`, main frame never blocked) | `on_before_resource_load` on the IO thread | D3: (a) compiled content filter per profile; whitelist and per-site off by removing the filter from that tab's UserContentManager on navigation; (b) engine in extension `send-request` | spike W7 |
| Cosmetic per-site CSS + uBlock scriptlets before page scripts | CDP `Page.addScriptToEvaluateOnNewDocument` per main-frame navigation, hostname-guarded | per-tab UserContentManager: on main-frame `decide-policy`, replace the tab's user script (main world, document start, all frames, hostname guard) before `use()`; or extension `window-object-cleared` (D3 b) | spike W7 |
| Generic cosmetic hiding (class/id scan, 100 ms batches) | `adblock-scan.js` in isolated world + `Runtime.addBinding` | same JS as a user script in world `novabrowser`; message handler with reply returns `hidden_class_id_selectors` | same |
| Bundled uBO scriptlet resources (`gen-adblock-resources.mjs`) | `include_str!` | copied | same |
| HTTPS-only (local/RFC1918/Tailscale/`.ts.net` exempt, `https_excludes`, `:https-toggle`, upgrade redirect targets, failure hint) | `on_before_resource_load` + `on_resource_redirect`; Chromium `HttpsUpgrades` off | main frame: `decide-policy` → `ignore` + `load_uri(https)`; subresources and redirects: extension `send-request` (fires on redirects) rewrites the URI; keep WebKit's `HTTPSByDefault` feature off so excludes stick; failure hint from `load-failed` | same (spike: confirm `send-request` URI rewrite) |
| Cookie policy all / no-3rdparty / never | Chromium `cookie_controls_mode` + content settings + per-cookie filter | `CookieManager.set_accept_policy` (ALWAYS / NO_THIRD_PARTY / NEVER) per network session; ITP via `NetworkSession.set_itp_enabled` | same globally |
| Per-site cookie rules (`pattern()` → `content.cookies.accept`), third-party exception pairs (`cookie_exceptions.json`) | content-setting pairs | no per-site API | **gap**: global only; Storage Access API (`WebsiteDataAccessPermissionRequest`) prompt covers embedded logins |
| Top-level navigation is its own first party ("Sign in with Google" fix) | `first_party()` | WebKit's own logic | spike W10 |
| Custom headers (`content.headers.custom`, `Sec-GPC`), `DNT` | `set_header_by_name` | extension `send-request` → `uri_request_get_http_headers` | same |
| Accept-Language | `accept_language_list` | `WebContext.set_preferred_languages` | same |
| Autoplay off | `--autoplay-policy` | `WebsitePolicies` autoplay ALLOW / ALLOW_WITHOUT_SOUND / DENY per view | same |
| Canvas reading off (`content.canvas_reading`) | `--disable-reading-from-canvas` | none | **gap**: drop the setting |
| WebRTC IP policy | `--force-webrtc-ip-handling-policy` | none known; WebRTC itself is spike W1 | **gap** [Guessing] |
| Hyperlink auditing off | `--no-pings` | `enable-hyperlink-auditing` false | same |
| No calls to Google with no page open | disabled features, prefs, CEF privacy patch | WebKitGTK has no Safe Browsing, sign-in or component updater [Likely]; set `enable-dns-prefetching` false; verify with a network capture | better (no patch) |
| Per-site dark mode (`colors.webpage.darkmode.enabled`, excludes, `dark-toggle`, `darkmode_excludes` file) | `WebContentsForceDark` + CDP `Emulation.setAutoDarkModeOverride` per tab | D6: per-view user style `-apple-color-filter: apple-invert-lightness()` (feature `ColorFilter`), native-dark pages skipped | same idea (paint-time colour filter), images untouched |
| `colors.webpage.preferred_color_scheme` | `blink-settings=preferredColorScheme` | GTK dark/light preference for the app (`gtk-application-prefer-dark-theme`) [Likely]; global only | same globally |
| Spellcheck (`spellcheck.languages`) | Chromium prefs + dictionary download | `WebContext.set_spell_checking_enabled/languages` (Enchant, system dictionaries) | same |
| Autofill off (card-save crash) | prefs | not needed (WebKitGTK has no autofill UI) | n/a |

## 7. Prompts and permissions

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Prompt in the tab bar `y/n/A/N`, per-tab FIFO, saved answers in `permissions.json` (win over config.ts) | `permissions.rs` | copied; requests come from `permission-request` | same |
| Notifications | bit 32768 | `NotificationPermissionRequest`; restore saved answers at start with `WebContext.initialize_notification_permissions`; show via `show-notification` (or WebKit's default) | same |
| Geolocation | bit 256 | `GeolocationPermissionRequest` (needs a GeoClue-backed provider) | same |
| Mic / camera / screen share | media access bits 1/2/4/8 | `UserMediaPermissionRequest` (`is_for_audio_device`, `is_for_video_device`, `is_for_display_device`) | same |
| Clipboard read | bit 16 | `ClipboardPermissionRequest` (DOM paste) + `javascript-can-access-clipboard` | same |
| Pointer lock | bit 131072 | `PointerLockPermissionRequest` | same |
| Persistent storage, protocol handlers | bits 64, 524288 | none | **gap**: drop `content.persistent_storage`, `content.register_protocol_handler` |
| New in WebKit | — | `WebsiteDataAccessPermissionRequest` (Storage Access API), `DeviceInfoPermissionRequest` (device labels: follow the media answer), `MediaKeySystemPermissionRequest` (deny), `InstallMissingMediaPluginsPermissionRequest` (deny + message naming `gst-libav`) | new settings |
| JS alert / confirm / prompt / beforeunload in the tab bar | `on_jsdialog`, `on_before_unload_dialog` | `script-dialog` signal; keep a ref, answer later with `script_dialog_close`; prompt text with `script_dialog_prompt_set_text` | better (typed `prompt()` answers possible) |
| Certificate errors | Chrome interstitial (its Proceed bypasses our handler, CEF S7b) | `load-failed-with-tls-errors` → own `nova://` error page; "proceed" = `NetworkSession.allow_tls_certificate_for_host` + reload (subresources: `TLS_ERRORS_POLICY_FAIL`) | better |

## 8. Downloads

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| No dialog, `downloads.location.directory`, sanitized + unique names, messages, pill, `:downloads` page, `download-open/cancel/clear`, `downloads.json` | `DownloadHandler` | `NetworkSession::download-started` → `decide-destination` (`set_destination`), `received-data`, `finished`, `failed`, `cancel` | same |
| aria2 backend (8 connections, resume, cookies incl. HttpOnly, UA, referer, magnet, torrents, Chromium fallback) | `can_download` returns 0 + CDP `Network.getCookies` | `decide-policy` RESPONSE: not displayable (`is_mime_type_supported` false or attachment) → `ignore` and hand to aria2; cookies via `CookieManager.get_cookies(uri)`; `download` attribute links: cancel in `download-started` and hand over; fallback = `download_uri` | same (spike: HttpOnly cookies returned [Likely]) |
| `magnet:` links | `on_protocol_execution` | `decide-policy` navigation to `magnet:` → `ignore` + aria2 | same |
| Context menu "Save link/image" | `start_download` | `WebView.download_uri` | same |

## 9. External tools

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| `spawn [-u] [-o]`, userscripts with `QUTE_*` env, `QUTE_HTML`/`QUTE_TEXT`/selection, `QUTE_FIFO` lines run in the spawning tab, `-o` output tab | `spawn.rs` + CDP eval | copied; eval via world `novabrowser`; `-o` tab via `load_plain_text` | same |
| `fake-key` | trusted `Input.dispatchKeyEvent` | JS `KeyboardEvent` (untrusted, types nothing) + editing commands for text (`InsertText`), Tab = focus next focusable, Return = `InsertNewline` / `requestSubmit()` | **worse** (spike W5); built-in `password-fill` replaces the qute-pass use case |
| `insert-text` | `Input.insertText` | `execute_editing_command_with_argument("InsertText", text)` [Guessing] | spike W5 |
| IPC socket per profile, `--cmd`, `--query state`, URL hand-over to the running instance | `ipc.rs` + `post_task` | copied, `glib` invoke | same |
| `--import` (Chrome, Chromium, Helium, Brave, Vivaldi, Edge, Firefox, Zen, LibreWolf, Floorp, qutebrowser) | `import.rs` | copied + new source `novabrowser` (the CEF edition: history, marks, sessions, permissions, zoom) | same + migration |

## 10. Passwords

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Backends rbw / bw / pass, list / unlock / secret / TOTP / save / update, 60 s metadata cache, generator (urandom, 20 chars), `$VISUAL` helper for rbw | `passwords.rs` (pure half) | copied | same |
| Field detection (`kind`: user/password/new/otp), `arm()` only after a login field is focused | `passwords.js` in the page main world, native `report` fn | same JS in world `novabrowser`, all frames | same (world isolation is stronger) |
| Suggestion list under the field (Down/Up/Enter/Escape, click), unlock row, generate row, `Sp` | tip overlay, field rect from the renderer | overlay widget positioned from the reported rect + frame offset | same |
| Fill only into the frame that asked, same-site frames only, never over http | `frame.send_process_message("nova.fill")` | script-message reply object kept per field focus (W6); same-site check against the tab URL in Rust | spike W6 |
| Save / update prompt after submit | page-world listeners (`grab` on submit/Enter/click) | `WebFormManager::will-submit-form` in the extension (sees the values without page listeners) [Likely] + JS fallback | better |

## 11. Sessions, profiles, marks

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| Session autosave (1 s debounce, atomic), restore at start, named sessions, `quit --save`, multi-window | JSON with url/title/back/fwd (≤50)/scroll | same JSON + `state` = base64 of `WebViewSessionState.serialize()`; keep reading the old url/back/fwd shape | better (real back/forward, W8) |
| Scroll restore | passive listener reports `scrollY` after 800 ms | WebKit back-forward items keep scroll [Likely]; keep the reporter only if W8 says so | same |
| Profiles (`--profile`, `-P`, `--list-profiles`, `:profile-new/open`, own process + data dir) | own process per profile | same (one `NetworkSession` per profile dir) | same |
| Private windows | not supported | `NetworkSession.new_ephemeral` | better |
| Quickmarks, bookmarks (qutebrowser formats), manager pages | `marks.rs` + data: pages | copied + `nova://` pages | same |
| Welcome page once per profile, `:help` | `welcome.rs` | copied | same |

## 12. Media and DRM

| Feature | CEF edition | WebKitGTK edition | Verdict |
|---|---|---|---|
| H.264 / AAC | own CEF build with proprietary codecs | GStreamer `gst-libav` (optdepend) | better (no custom engine) |
| VA-API decode | Chromium VA-API flags | GStreamer `va` plugin | spike W1 |
| Widevine | own CEF + component updater | not available | **gap** |
| WebRTC (Meet, Discord calls) | Chromium | GStreamer WebRTC if the distro build enables it [Guessing] | spike W1/W10 |
