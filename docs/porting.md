# Porting plan

How each part of the CEF edition (`../novabrowser/novabrowser`, 0.1.8, `9574fca`) moves
over. **Rev 2 (2026-10-28, D5):** "Copy" below now means "move into the shared
`nova-core` crate" (PLAN.md §5), used by both editions; nothing is duplicated.
D2 (chrome in HTML/CSS) means `ui/*.html` are reused, not rewritten as GTK
widgets; D3 means the adblock engine runs in `extension/`.

## 1. Modules

Lines = current size in the CEF edition.

### Copy as is (pure logic, no CEF types)

| File | Lines | Notes |
|---|---|---|
| `urlmatch.rs` | 1600 | 28 tests (qutebrowser port) |
| `keys.rs` | 1076 | replace `key_from_cef` (and its test) with `key_from_gdk(keyval, state)`; 22 tests |
| `config.rs` + `config.js` | 710 + 106 | no CEF; add QuickJS memory/interrupt limits (audit A2) |
| `settings.rs` | 155 | edit the schema per §3 below |
| `marks.rs` | 350 | fix audit C2 (unreadable file → wipe) while here |
| `import.rs` | 385 | add source `novabrowser` (§4) |
| `bindings.rs` | 141 | default keymap; test checks every command exists |
| `theme.rs` | 452 | keep `vars`, `format_title`, presets; add a GTK CSS writer for native chrome (D2) |
| `profiles.rs`, `welcome.rs` | 60 + 80 | swap data-URL output for `nova://` |
| `adblock-scan.js`, `adblock-resources.json`, `scripts/gen-adblock-resources.mjs` | | unchanged |

### Copy, then swap the CEF glue

| File | Lines | What changes |
|---|---|---|
| `history.rs` | 615 | `post_task(ThreadId::UI)` → `glib::MainContext::default().invoke` (2 places) |
| `session.rs` | 143 | `post_delayed_task` → `glib::timeout_add_local_once`; add the `state` blob |
| `completion.rs` | 378 | none directly; talks to the new window module |
| `modes.rs` | 267 | key entry point takes a `Key` from `key_from_gdk` |
| `commands.rs` | 1147 | parser, registry, aliases, placeholders stay; every handler using `Browser`/`CefString` is re-pointed to `WebView` calls (feature-map §2) |
| `hints.rs` + `hints.js` | 322 + 123 | eval in world `novabrowser`; follow = JS click (feature-map §4); fix audits A1, A5 |
| `spawn.rs` | 387 | `fake_key`/`insert_text` rewritten (feature-map §9); FIFO, env, temp dir unchanged |
| `downloads.rs` | 578 | `before`/`updated`/`start` on WebKitDownload; pure parts unchanged; fix audit A6 |
| `aria2.rs` | 316 | cookies from `CookieManager.get_cookies`, UA from WebKitSettings; fix audit A4 |
| `permissions.rs` | 416 | requests from `permission-request` / `script-dialog`; queue, prompt text, `permissions.json` unchanged; fix audit A3 (grace period) |
| `passwords.rs` + `passwords.js` | 1031 + 117 | backends/generator/editor helper unchanged; focus, fill and submit glue rewritten (feature-map §10) |
| `net.rs` | 327 | keep `is_local`, `https_upgrade`, `referrer`, `language_list`; cookie functions shrink to the global policy; header code moves to the extension |
| `adblock.rs` | 460 | update/cache/fetch unchanged; add filter compile (D3a) or engine-in-extension (D3b); drop CDP injection |
| `ipc.rs` | 81 | `post_task` → `glib` invoke |
| `settings_page.rs` | 148 | served on `nova://settings`; messages via `fetch('nova://action/…')` |
| `main.rs` | 555 | keep CLI (`--profile`, `-P`, `--list-profiles`, `--import`, `--check-config`, `--cmd`, `--query`, URL hand-over, `NOVA_PW_SECRET` editor mode), dirs, `write_atomic`, `page_token`; drop CEF init, switches and prefs; add GtkApplication + WebContext/NetworkSession setup |

### Rewrite

| CEF file | Lines | Replacement |
|---|---|---|
| `window.rs` | 2233 | `window.rs` (GTK window, layout, overlays) + `tabs.rs` (WebView lifecycle, signals) |
| `client.rs` | 554 | signal handlers on each WebView (`load-changed`, `notify::uri/title/estimated-load-progress/favicon`, `decide-policy`, `create`, `permission-request`, `script-dialog`, `context-menu`, `enter-fullscreen`, `web-process-terminated`, `load-failed`, `load-failed-with-tls-errors`) |
| `renderer.rs` | 169 | user scripts in world `novabrowser` + `extension/` cdylib |
| `devtools.rs` | 137 | `js.rs`: `eval_world(view, code) -> Future<Value>` over `evaluate_javascript`; no CDP |
| `clipboard.rs` | 132 | ~30 lines on GdkClipboard |
| `ui/*.html` | 363 | GTK widgets (D2) or reused as-is in transparent WebViews |

### New

- `extension/` — `cdylib` on `webkit-web-process-extension6`: `send-request`
  (HTTPS upgrade, headers, optional adblock), `WebFormManager` submit hook,
  user-message channel to the UI process.
- `scheme.rs` — `nova://` handler: internal pages, token-checked actions.
- `zoom.rs` — per-host zoom store.
- `darkmode` user script (D6).

## 2. Repository layout

```
novabrowser-webkit/
├─ Cargo.toml          workspace: the app crate + extension/
├─ src/                app (files above)
├─ extension/          web-process extension cdylib
├─ js/                 hints.js, passwords.js, adblock-scan.js, scroll.js, focus.js, dark.js
├─ pages/              nova:// HTML (settings, history, marks, downloads, welcome, crash, tls-error)
├─ tests/smoke.sh
├─ spikes/wN-*/
└─ packaging/          PKGBUILD, .desktop, icons
```

## 3. Settings schema changes

Removed (no WebKit equivalent):
- `chromium.args`, `chromium.low_end_device_mode`
- `content.canvas_reading`, `content.webrtc_ip_handling_policy` (unless W1 finds one)
- `content.persistent_storage`, `content.register_protocol_handler`
- `tabs.freeze_after` (or keep as a no-op with a warning; user's call)

Changed:
- `content.cookies.accept`: global only. A `pattern()` override is reported by
  `--check-config` as "ignored per site" (the mechanism already exists for
  global-only settings).
- `colors.webpage.darkmode.*`: now drives the injected dark stylesheet (D6).

New `webkit.*` group (WebKitSettings, all global):
- `webkit.hardware_acceleration` = `always` | `never` (`hardware-acceleration-policy`)
- `webkit.site_specific_quirks` (bool, default true; `enable-site-specific-quirks`)
- `webkit.features` (dict: feature identifier → bool; `Settings.set_feature_enabled`) for experiments
- `webkit.itp` (bool, default true; `NetworkSession.set_itp_enabled`)
- `content.user_agent` (str, per site; `user-agent` on the view's WebKitSettings), for compat quirks (W10)
- `content.storage_access` (bool/ask; Storage Access API prompt)

Mapped one-to-one: `content.javascript.clipboard` → `javascript-can-access-clipboard`,
`content.autoplay` → WebsitePolicies, `scrolling.smooth` → `enable-smooth-scrolling`,
`fonts.web.*` → font settings, `spellcheck.languages` → WebContext spell checking,
`content.media.*`, `content.geolocation`, `content.notifications.enabled`,
`content.mouse_lock`, `content.desktop_capture` → permission request types.

## 4. Data migration from the CEF edition

`novabrowser-webkit --import novabrowser [profile]` reads
`$XDG_DATA_HOME/novabrowser[/profiles/<name>]` and `~/.config/novabrowser`:

| Data | How |
|---|---|
| `history.sqlite` (history, typed, cmd_history) | same schema: `history::import` merge |
| quickmarks, `bookmarks/urls` | same format: `marks::import` |
| `sessions/*.json` | same shape; old url/back/fwd lists still load (no `state` blob) |
| `permissions.json` | copy entries whose setting still exists |
| `downloads.json` | copy |
| `config.ts`, `autoconfig.json`, `https_excludes`, `darkmode_excludes` | copy; `--check-config` lists removed `chromium.*` keys |
| Chromium per-host zoom (`Preferences`) | read `partition.per_host_zoom_levels` into `zoom.json` [Likely path] |
| Cookies, logins, site data | not migrated (users sign in again) |
