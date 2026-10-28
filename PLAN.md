# novabrowser-webkit — plan (rev 1)

Rev 1: 2026-10-28 (IST). Source of truth for the design. Feature-level detail
lives in `docs/feature-map.md`; module reuse in `docs/porting.md`.

Tags: [Certain] = checked this session (headers, docs, crates.io, the CEF
edition's code), [Likely] = strong inference, [Guessing] = needs a spike.

---

## 0. What was checked (2026-10-28)

- The CEF edition (`../novabrowser`, 0.1.8, `9574fca`): ~17k lines of Rust/JS in
  `src/`, 4 HTML UI pages, 92 unit tests, `tests/smoke.sh`. Every module was read
  for features and CEF coupling [Certain].
- On novarch: `webkit2gtk-4.1 2.54.2` (GTK 3) is installed; **`webkitgtk-6.0`
  (GTK 4) is not** (`pacman -Si webkitgtk-6.0` = 2.54.2, available). Installed:
  `gtk4 4.24.1`, `gst-libav`, `gst-plugins-bad`, `gst-plugin-va` [Certain].
  User must run `sudo pacman -S webkitgtk-6.0` before Phase 0.
- Rust bindings on crates.io [Certain]: `webkit6 0.6.1` (features up to `v2_52`),
  `webkit-web-process-extension6 0.6.0`, `javascriptcore6`, all from
  `gitlab.gnome.org/World/Rust/webkit6-rs`. `adblock 0.13.3` has a
  `content-blocking` feature (ABP rules → WebKit content-blocker JSON).
- WebKitGTK 6.0 API (docs at webkitgtk.org, "API Version 6.0, Library 2.54.2")
  [Certain]: `WebPage::send-request` (also fires on redirects, may edit the
  request and headers), `ScriptWorld::window-object-cleared`, `WebFormManager`
  (`will-submit-form`, `form-controls-associated`), `NetworkSession`
  (`new_ephemeral`, `allow_tls_certificate_for_host`, `download-started`,
  `set_itp_enabled`), `UserContentManager.register_script_message_handler(name,
  world_name)` + `…_with_reply`, `add_filter`, `WebView.get/restore_session_state`,
  `evaluate_javascript(…, world_name, …)`, `get_snapshot`, `set_background_color`,
  `terminate_web_process`, `is-web-process-responsive`, `web-process-terminated`,
  `permission-request`, `ClipboardPermissionRequest`,
  `execute_editing_command_with_argument`, `register_uri_scheme`,
  `SecurityManager.register_uri_scheme_as_secure`.
- The web-process extension API in 6.0 has **no DOM classes** (only
  `Frame`, `ScriptWorld`, `WebPage`, `WebFormManager`, `WebEditor`, context
  menu, URI request/response) [Certain]. DOM work is JavaScript.
- WebKit feature flags in 2.54 include `HTTPSByDefault`,
  `UpgradeKnownHostsToHTTPS`, `StorageAccessAPI`,
  `IsThirdPartyCookieBlockingDisabled`; none for forced dark mode [Certain,
  enumerated on webkit2gtk-4.1 2.54.2 via `Settings.get_all_features()`].

---

## 1. Goals and non-goals

### Goals (v1)
- Feature parity with the CEF edition where WebKitGTK allows it, with the same
  keys, commands, setting names, `config.ts` API, userscript protocol and IPC.
  A user of one edition can switch with no relearning.
- Same rules as the CEF edition: zero-config usable, never block the UI
  thread, a web-process crash never loses the session, atomic writes, no
  secrets at rest.
- Native Wayland (GTK 4 default), X11 via GTK's X11 backend.
- Video through GStreamer, including H.264/AAC when `gst-libav` is installed,
  VA-API through `gst-plugin-va`.

### Non-goals (v1)
- Widevine / DRM (WebKitGTK has no Widevine) [Likely].
- Chrome or Safari extensions, greasemonkey (same as the CEF edition).
- Features WebKit cannot give us (per-site cookie policy, page freezing,
  trusted synthetic input). Each one is listed in `docs/feature-map.md` with
  what we do instead.
- macOS / Windows.

---

## 2. Stack

| Area | Choice | Why |
|---|---|---|
| Engine | WebKitGTK **6.0** (GTK 4), crate `webkit6` | 4.1 is GTK 3; gtk3-rs bindings are no longer maintained [Likely]. 6.0 is where new WebKitGTK API lands [Likely]. |
| UI toolkit | GTK 4 via `gtk4` crate | Same toolkit as the engine; GTK CSS for theming. |
| Web-process extension | Rust `cdylib` on `webkit-web-process-extension6` | Per-request hooks (`send-request`) and form-submit signals only exist in the web process. |
| Concurrency | `std::thread` + channels; back to the UI thread with `glib::MainContext::default().invoke` | Replaces CEF `post_task(TID_UI)`. No tokio (same rule as the CEF edition). |
| Browser chrome | **Decision D2** (section 4): native GTK widgets (recommended) or the existing `ui/*.html` in WebViews | |
| Internal pages | HTML on a private `nova://` scheme (`register_uri_scheme`, marked secure) | Settings, history, bookmarks, downloads, welcome and crash pages reused from the CEF edition. Replaces `data:` URLs. |
| Page scripts | User scripts and `evaluate_javascript` in a private script world `novabrowser` | Replaces the CDP isolated world. Page scripts can't see it. |
| Config | Unchanged: `config.ts` → `oxc` → QuickJS (`rquickjs`) | Pure Rust, no CEF coupling. |
| Storage | Unchanged: SQLite (`rusqlite`), JSON/text files | Same file formats, so data can be copied over. |
| Adblock | `adblock` 0.13.3; delivery = **Decision D3** | |
| Media | GStreamer (WebKitGTK's backend) | H.264/AAC need `gst-libav` at runtime, not a custom build. |
| Sandbox | WebKitGTK's bubblewrap sandbox (dependency of `webkitgtk-6.0`) | The extension `.so` runs inside it: no process spawning there. |
| Packaging | AUR `novabrowser-webkit` (+ `-bin`) | Depends on `webkitgtk-6.0`, `gtk4`, `gst-plugins-good`; optdepends `gst-libav`, `gst-plugin-va`, `aria2`, `rbw`/`pass`. |

---

## 3. Architecture

```
novabrowser-webkit (UI process, one per profile — same as the CEF edition)
│
├─ GTK main thread              never blocks
│   window, tabs, keys/modes, command dispatch, WebView signals,
│   evaluate_javascript in world "novabrowser"
├─ history thread               rusqlite (unchanged)
├─ adblock thread               list download, engine/filter build (unchanged + filter compile)
├─ ipc thread                   unix socket <data>/ipc: --cmd, --query, single instance (unchanged)
├─ aria2 poller, password backends, spawn waiters (unchanged, std threads)
│
├─ WebKit network process       cookies, cache, downloads, content filters
└─ WebKit web processes (bubblewrap sandbox)
     └─ libnova_extension.so    (WebKitWebProcessExtension, Rust cdylib)
          - send-request: HTTPS upgrade of subresources + redirects, custom headers
            (Sec-GPC, DNT), optional adblock network check (D3)
          - window-object-cleared: scriptlets before page scripts (D3 option b)
          - WebFormManager will-submit-form: password save prompt
          - talks to the UI process with WebKitUserMessage
```

**Messaging** (replaces CEF process messages and `window.nova`):
- Page scripts in world `novabrowser` → UI: `webkit.messageHandlers.nova.postMessage(json)`;
  handler registered for that world only, so page scripts can't call it.
  Requests that need an answer later (password fill into the exact frame
  that asked) use `register_script_message_handler_with_reply` and keep the
  reply object until the user picks [Guessing, spike W6].
- UI → page: `evaluate_javascript(code, world="novabrowser")` (main frame only),
  or the reply object above for sub-frames.
- UI ↔ extension: `send_message_to_page` / `WebPage::user-message-received` and
  `web_page_send_message_to_view` (GVariant payloads).
- Internal `nova://` pages → UI: `fetch('nova://action/<token>/…')` handled by the
  scheme handler, which checks the page token (as `page_token()` does now) and
  that the requesting view shows a `nova://` page.

**Keys:** one `GtkEventControllerKey` in the **capture** phase on the window.
It sees every key before the focused WebView and returns `Propagation::Stop`
to consume it [Likely, spike W3]. Mode, keymap trie and counts stay in Rust
(`keys.rs`, `modes.rs` unchanged apart from a `key_from_gdk` adapter).

**Window layout:**
```
GtkApplicationWindow
└─ GtkOverlay
   ├─ GtkBox (vertical or horizontal, from tabs.position)
   │  ├─ tab bar (+ status: mode badge, pending keys, messages, downloads pill, load line)
   │  └─ content: GtkStack of WebKitWebView, one child per loaded tab
   └─ overlay children: palette, completion list, which-key, tab tooltip/menu,
      password suggestions (positioned with GtkOverlay get-child-position)
```
Switching tabs changes the visible stack child; hidden views stay alive (WebKit
throttles hidden pages itself [Likely]). Lazy tabs have no WebView until shown.

---

## 4. Decisions to make (user)

| # | Question | Recommendation | Why / risk |
|---|---|---|---|
| D1 | GTK 4 + WebKitGTK 6.0, or GTK 3 + webkit2gtk-4.1? | **6.0** | 4.1 allows synthetic GdkEvents (trusted clicks/keys, as luakit/vimb do) [Likely], but gtk3-rs is unmaintained and new WebKitGTK API targets 6.0 [Likely]. Risk of 6.0: no trusted input (see feature-map §Input). |
| D2 | Browser chrome as native GTK widgets or reuse `ui/*.html` in WebViews? | **Native GTK** | Overlays, transparency, focus and live blur (GTK CSS `filter: blur()` on the content [Guessing]) are native; no extra web processes; no focus juggling (the CEF edition needed posted focus tasks to avoid crashes). Cost: rewrite ~360 lines of HTML/JS (tabbar, palette, keyhint, popup) as GTK code. Reuse is possible: WebViews can be transparent (`set_background_color` alpha 0). |
| D3 | Adblock delivery | **(a) content filter + per-navigation scriptlet user script**, spike (b) if fidelity is short | (a) `adblock` `content-blocking` feature → `UserContentFilterStore` (compiled once, shared by all web processes). Lossy: no `$redirect`, `$removeparam`, some `$csp` [Likely]. (b) adblock engine inside the web-process extension (`send-request` + `window-object-cleared`): full fidelity, but one engine copy per web process (tens of MB each [Guessing]). |
| D4 | Data and config dirs shared with the CEF edition? | **Separate** (`~/.config/novabrowser-webkit`, `$XDG_DATA_HOME/novabrowser-webkit`) + `--import novabrowser` | Both editions can run at once (own IPC socket); `chromium.*` settings in a shared config.ts would error. Risk: two configs to keep in sync by hand. |
| D5 | Share code with the CEF edition as a crate? | **Copy now, extract `nova-core` later** | Pure modules (~7k lines) are tangled with `window::`/`post_task` calls. A shared crate first means refactoring the shipping CEF edition. Risk: drift; record the source commit in each copied file's header. |
| D6 | Dark mode | **`prefers-color-scheme` + per-site injected dark stylesheet**; Dark Reader port only if that's not enough | No WebKit force-dark. A generic dark CSS on every page needs a Speedometer A/B (rule from the CEF edition, 2026-10-12). |

---

## 5. Phase 0 — spikes (`spikes/wN-<name>/`, throwaway)

Each spike is a tiny `gtk4` + `webkit6` program. Pass conditions are checked
with real keys (`wtype` or the compositor's `send_shortcut`) and readback
through `evaluate_javascript` / `get_snapshot` — there is no CDP.

| # | Question | Pass condition |
|---|---|---|
| W1 | Wayland basics | WebKitGTK 6.0 window on Hyprland: HiDPI, clipboard both ways, `<select>` popup, IME passes through, VA-API decode of H.264 + AV1 (`GST_DEBUG=va*:4` shows `vah264dec`), H.264/AAC MSE `isTypeSupported` true with `gst-libav` |
| W2 | Layout | GtkStack of 2 WebViews + tab bar + overlay; switching keeps scroll and JS state; overlay takes keyboard focus and gives it back |
| W3 | Key interception | Capture-phase key controller consumes `j`/`k`/Escape: page gets no `keydown`; works with an input focused; repeats consumed; unconsumed keys reach the page |
| W4 | Script world bridge | User script in world `novabrowser` (all frames, document start) posts focus changes for input/password/textarea/contenteditable incl. same- and cross-origin iframes; page world can't see `webkit.messageHandlers.nova` |
| W5 | Input without CDP | Hints with JS `click()`/`focus()` follow links and buttons on GitHub, Jira, YouTube, HN, Gmail; `InsertText` editing command types into an input with an `input` event; JS scroll of the right inner scroller (Gmail, Slack-like layouts). Record every site where it fails |
| W6 | Frame-targeted reply | Message with reply from a cross-origin iframe; UI answers 10 s later; value lands in that frame only |
| W7 | Adblock | adblock-tester.com score with (a) content filter, and (b) extension engine; YouTube `adPlacements` gone with scriptlets injected by a per-navigation user script; RSS per web process for (b) |
| W8 | Session state | `get_session_state` → serialize → new view `restore_session_state` restores back/forward and scroll; size per tab |
| W9 | Baseline | Startup, idle RSS (5 heavy sites), Speedometer 3.1, YouTube scroll: spike vs CEF novabrowser vs Chrome (same method as CEF spike S9, `../novabrowser/spikes/s9-baseline`) |
| W10 | Sign-in and compat | Google sign-in, ChatGPT "Sign in with Google", Microsoft login, one bank, WhatsApp Web, Discord, Google Meet: works / degraded / blocked, with and without a Safari UA |
| W11 | Permissions + TLS + dialogs | `permission-request` fires for notifications, geolocation, mic/cam, clipboard, pointer lock; held requests answered later work; `load-failed-with-tls-errors` on badssl; `script-dialog` held then answered |

Exit: all pass, or each failure has a written fallback in PROGRESS.md
Decisions. **W9 and W10 decide whether the edition is worth building at all**:
if common sites break, stop and report.

---

## 6. Phases

Same order as the CEF edition. Each ends runnable, with an exit check.

### Phase 1 — Skeleton
GtkApplication, window, tab bar, GtkStack of WebViews, `novabrowser-webkit <url>`,
URL/title/progress/favicon in the tab bar, popups (`create` signal) and
middle/Ctrl-click (`decide-policy` NEW_WINDOW_ACTION / mouse button) as tabs,
crash page on `web-process-terminated`, `nova://` scheme.
**Exit:** browse with the mouse; `kill -9` of a web process shows the crash page and `r` reloads.

### Phase 2 — Modal core
Copy `keys.rs`, `modes.rs`, `bindings.rs`, `commands.rs` parser; `key_from_gdk`;
auto insert mode from the W4 focus messages; scroll/zoom/back/forward.
**Exit:** daily navigation without the mouse; `cargo test` green.

### Phase 3 — Palette, completion, history
Copy `history.rs`, `completion.rs`; palette (D2), readline keys, command history.
**Exit:** `:open` completion < 16 ms at 100k rows (existing test), real-key check.

### Phase 4 — Hints, find, yank
`hints.js` in world `novabrowser`; follow by JS (W5); `WebKitFindController`;
GdkClipboard for clipboard + primary.
**Exit:** `f`, `F`, `;y`, `hint links spawn mpv {hint-url}` on GitHub, Jira, YouTube.

### Phase 5 — config.ts and theme
Copy `config.rs`, `config.js`, `settings.rs` (schema edited per docs/porting.md),
`urlmatch.rs`, `theme.rs` → GTK CSS + page CSS vars; WebKitSettings mapping.
**Exit:** the user's config.ts (with `chromium.*` removed) loads; broken config keeps the old one.

### Phase 6 — External tools
`spawn.rs`, userscript env, `QUTE_FIFO`, IPC (`ipc.rs`), `--cmd`/`--query`,
`fake-key` / `insert-text` (JS + editing commands), `dark-toggle`.
**Exit:** as CEF Phase 6, with `fake-key` limits written down.

### Phase 7 — Privacy, blocking, prompts
Adblock (D3), HTTPS-only (UI-side main-frame upgrade + extension for
subresources/redirects), cookie policy + ITP, headers, permissions, JS
dialogs (with typed `prompt()` answers), own TLS-error page with "proceed".
**Exit:** adblock-tester ≥ CEF edition minus documented gaps; no ads on 3
YouTube videos; saved permission answers survive restart.

### Phase 8 — Sessions and daily driving
Sessions with native back/forward (W8), lazy restore, undo-close,
quickmarks/bookmarks, manager pages on `nova://`, downloads + aria2,
Web Inspector, fullscreen-in-window, spellcheck, hibernation, multiple
windows, live `tab-give`, profiles, `open -p` private windows, import
(incl. from the CEF edition).
**Exit:** one week as the only browser; problems logged.

### Phase 9 — Packaging
AUR PKGBUILD; extension installed under `/usr/lib/novabrowser-webkit/`
and set with `WebContext.set_web_process_extensions_directory`; `.desktop`, icons;
`tests/smoke.sh` ported.

### Phase 10 — Passwords
Copy the backend half of `passwords.rs` (rbw/bw/pass, generator, `$VISUAL`
helper); field detection in world `novabrowser`; fill through W6 replies;
save/update prompt from `WebFormManager::will-submit-form`.
**Exit:** same test set as the CEF edition (single-page, split, iframe, TOTP,
sign-up, cross-site iframe gets nothing).

---

## 7. Stability rules (carried over)

- Web-process crash → crash page; the session is untouched. Note: one web
  process can host several tabs of the same site, so they crash together [Likely].
- Unresponsive page (`notify::is-web-process-responsive` false) → tab-bar prompt
  "Kill it? y/n" → `terminate_web_process`.
- Atomic writes (`write_atomic`), 0700 data dirs, 0600 files and socket.
- Nothing blocking on the GTK main thread: SQLite, file IO, process waits
  stay on threads.
- Anything injected into every frame gets a Speedometer A/B (CEF decision 2026-10-12).

---

## 8. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Sites block or degrade WebKitGTK (Google, Meet, Teams, banks) | edition unusable as a daily browser | W10 first; UA quirks per site (`pattern()` + `user-agent`); stop if core sites fail |
| Untrusted JS clicks fail on some sites | hints don't work there | W5 site list; fallbacks: open `href`, `focus()` + Enter via `InsertNewline`; D1 (GTK 3) if it's widespread |
| Speed below Chromium | slower heavy apps | W9 measures it; nothing we can fix in the engine |
| Content-filter conversion loses rules | more ads/trackers | D3 option (b) |
| webkit6-rs bindings lag or miss an API | blocks a feature | raw `webkit6-sys` calls; upstream MR |
| No Widevine | DRM sites fail | documented non-goal; use the CEF edition for those |
| Two codebases drift | double maintenance | D5: extract `nova-core` once both editions are stable |
| Distro WebKitGTK update changes behaviour | surprise breakage | smoke test after each `webkitgtk-6.0` update |

---

## 9. Checks

- Unit tests: the ported tests from the CEF edition (keys, urlmatch, config,
  history incl. the 100k-row timing, completion, commands, marks, session,
  import, passwords, downloads, net, spawn, theme, hints) plus `key_from_gdk`.
- `tests/smoke.sh` over IPC, same steps as the CEF edition (no `Check failed`
  grep; instead: no `CRITICAL` / `WARNING **` from GLib on stderr).
- A debug-only IPC `query eval <js>` and `query snapshot <path>` replace
  `spikes/cdp.mjs` for automated checks.
