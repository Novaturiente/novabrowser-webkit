# novabrowser-webkit — plan (rev 2)

Rev 1: 2026-10-28 (IST), research. Rev 2: 2026-10-28 (IST), decisions D1–D7
taken by the user, benchmark results in, build order fixed.
Feature-level detail: `docs/feature-map.md`. Module moves: `docs/porting.md`.
Status and the full feature checklist: `PROGRESS.md`.

Tags: [Certain] = checked this session, [Likely] = strong inference,
[Guessing] = needs a spike.

---

## 0. What is known

### 0.1 Checked (2026-10-28)
- The CEF edition (`../novabrowser`, 0.1.8, `9574fca`): ~17k lines in `src/`, 4
  HTML UI pages, 92 unit tests, `tests/smoke.sh`; every module read [Certain].
- Rust bindings: `webkit6 0.6.1` (needs `gtk4 0.11` with feature `v4_10`+, we use
  `v4_18`), `webkit-web-process-extension6 0.6.0`, `javascriptcore6` [Certain].
- WebKitGTK 6.0 API present (webkitgtk.org docs, 2.54.2): `WebPage::send-request`
  (also on redirects), `ScriptWorld::window-object-cleared`, `WebFormManager`,
  `NetworkSession` (`new_ephemeral`, `allow_tls_certificate_for_host`,
  `download-started`, `set_itp_enabled`), script message handlers per world (+ with
  reply), session state, `evaluate_javascript` in a world, snapshots, transparent
  views, permission requests incl. clipboard, editing commands, custom URI schemes
  [Certain]. The extension API has no DOM classes [Certain].
- No forced-dark feature flag in WebKit 2.54; `HTTPSByDefault`,
  `UpgradeKnownHostsToHTTPS`, `StorageAccessAPI` exist [Certain].

### 0.2 Speedometer 3.1 on novarch (spike W9, `spikes/w9-baseline/results/`)

| Build | Median | Setup |
|---|---|---|
| Arch `webkitgtk-6.0` 2.54.2 | 18.95 | inside the tool sandbox |
| Own build, PGO + `x86-64-v3` + ThinLTO | 23.95 (+26%) | inside the tool sandbox, same rounds |
| **`webkitgtk-6.0-nova` installed (same build, prefix `/usr`)** | **27.25** | outside the tool sandbox, WebKit sandbox on |
| CEF novabrowser-bin 0.1.8, bare profile | 26.3 | same rounds as the row above |

Runtime env knobs give nothing (`results/ab-env.jsonl`). Absolute numbers inside
novacode's tool sandbox are ~15% low; ratios within one setup hold [Certain].
Not measured yet: real-site speed, scrolling, memory, startup, video.

### 0.3 Own WebKitGTK build (`spikes/wk-build/`)
- novahome, clang 23, `-march=x86-64-v3 -O2`, IR PGO (profile from 3 Speedometer
  runs), `LTO_MODE=thin`, docs/introspection/MiniBrowser off.
- Capped with `systemd-run --user --scope -p MemoryMax=20G` (user rule: other
  services on novahome must keep running), `-j8`, ThinLTO jobs 4: peak 13.2 GB,
  ~45 min per build [Certain].
- PGO needs the same source path as the training build (static-function names
  include the path), so `build.sh pkg` reuses `~/wk-build/webkitgtk-<ver>`.
- Packaged as `webkitgtk-6.0-nova` (`provides` + `conflicts` `webkitgtk-6.0`),
  installed on novarch 2026-10-28.

---

## 1. Goals and non-goals

### Goals (v1)
- Every feature of the CEF edition that WebKitGTK allows, with the same keys,
  commands, setting names, `config.ts` API, userscript protocol and IPC
  (checklist in PROGRESS.md). Users switch editions with no relearning.
- Same rules: zero-config usable, never block the UI thread, a crash never loses
  the session, atomic writes, no secrets at rest.
- Browser chrome fully restylable in HTML/CSS (user plans a tab-bar redesign).
- Ship the fast WebKitGTK (`webkitgtk-6.0-nova`) to users too, and keep it
  patched.

### Non-goals (v1)
- Widevine / DRM (not in WebKitGTK) [Likely].
- Chrome/Safari extensions, greasemonkey.
- Per-site cookie policy, page freezing, trusted synthetic input: not possible;
  replacements in `docs/feature-map.md`.
- macOS / Windows.

---

## 2. Decisions (user, 2026-10-28)

| # | Decision | Consequence / risk |
|---|---|---|
| D1 | **GTK 4 + WebKitGTK 6.0** | Hint clicks, `fake-key`, scrolling are JS events (`isTrusted=false`); spike W5 lists failing sites. |
| D2 | **Browser chrome in HTML/CSS** ("whichever allows customising the UI style"; tab bar redesign later) | Tab bar, palette, which-key, tooltip/menu and password list are transparent WebViews in a GtkOverlay, served from `nova://ui/…`, in their own WebContext/NetworkSession. Reuses `ui/*.html` + `theme.rs` CSS vars. Cost: extra web process (~50–80 MB [Guessing]), focus hand-offs between views (spike W2). |
| D3 | **adblock engine inside the web-process extension** | Full fidelity (network + cosmetic + scriptlets, incl. `$redirect`, `$removeparam`). One engine per web process: RSS cost measured in W7; engine shared as a read-only serialized file the extension deserializes. |
| D4 | **Separate dirs** `~/.config/novabrowser-webkit`, `$XDG_DATA_HOME/novabrowser-webkit` + `--import novabrowser` | Both editions run side by side; two configs to keep in sync by hand. |
| D5 | **Extract a shared `nova-core` crate first** | Phase C refactors the shipping CEF edition before WebKit work starts. Its tests + smoke must stay green and its release flow must still build. |
| D6 | **Dark Reader port first**, fall back to `prefers-color-scheme` + injected CSS if the overhead is too high | Spike W13: Speedometer A/B with the Dark Reader engine; budget ≤ 3% (rule from the CEF edition, 2026-10-12). |
| D8 | **Identify as macOS Safari** by default: UA `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Safari/605.1.15` + `navigator.platform = "MacIntel"` (user script, main world, all frames, document start). Setting `content.user_agent` = `safari` (default) / `webkit` (stock Linux UA) / custom string, per site via `pattern()` | Checked in spike: page and server both see Safari 27 on Mac; Speedometer unchanged (26.6/26.8 vs 26.7/26.7). Risks: sites offer Mac downloads and ⌘ key hints; `Version/27.0` must be bumped with Safari releases; fingerprint scripts can still spot Linux (fonts, WebGL renderer); WebKit's own site quirks may override the UA on some Google domains [Likely]. |
| D9 | **No DRM in v1** (user, after D8): no EME/Widevine build; DRM sites stay in the CEF edition | Sparkle-CDM + `sparkle-cdm-widevine` path exists (`ENABLE_ENCRYPTED_MEDIA` + `ENABLE_THUNDER`), but L3 only, fragile, Widevine blob not redistributable, and a Safari UA makes sites pick FairPlay. Revisit only if this edition replaces the CEF one (spike W15). |
| D7 | **Ship `webkitgtk-6.0-nova` on the AUR** | We own WebKitGTK security updates for every user of it (Phase W). x86-64-v3 only; `novabrowser-webkit` itself depends on `libwebkitgtk-6.0.so`, so stock `webkitgtk-6.0` also works. |

---

## 3. Stack

| Area | Choice |
|---|---|
| Engine | WebKitGTK 6.0 via `webkit6` (feature `v2_52`); system lib = `webkitgtk-6.0` or `webkitgtk-6.0-nova` |
| UI toolkit | GTK 4 (`gtk4` 0.11, feature `v4_18`): window, GtkOverlay, GtkStack, key controller |
| Browser chrome | HTML/CSS/JS in transparent WebViews (`set_background_color` alpha 0), `nova://ui/*` |
| Shared logic | `nova-core` crate (D5), used by both editions |
| Web-process extension | `cdylib` on `webkit-web-process-extension6`: adblock, HTTPS upgrade, headers, form submit, scriptlets |
| Concurrency | `std::thread` + channels; to the UI thread via `glib::MainContext::default().invoke` (no tokio) |
| Internal pages | `nova://` scheme (`register_uri_scheme`, secure + local), token-checked actions |
| Page scripts | world `novabrowser` (user scripts + `evaluate_javascript`), page can't see it |
| Config | `config.ts` → `oxc` → QuickJS (`rquickjs`), from `nova-core` |
| Storage | SQLite (`rusqlite`) + JSON/text, same formats as the CEF edition |
| Adblock | `adblock` 0.13.3 in the extension (D3) |
| Dark mode | Dark Reader engine (MIT) in world `novabrowser` (D6) |
| Media | GStreamer: `gst-libav` (H.264/AAC), `gst-plugin-va` (VA-API) |
| Packaging | AUR `novabrowser-webkit` (+ `-bin`), AUR `webkitgtk-6.0-nova-bin` |

---

## 4. Architecture

```
novabrowser-webkit  (UI process, one per profile, like the CEF edition)
│
├─ GTK main thread            never blocks: windows, tabs, keys/modes, commands,
│                             WebView signals, JS calls into world "novabrowser"
├─ threads (from nova-core)   history (sqlite), adblock lists, ipc socket, aria2 poller,
│                             password backends, spawn waiters
├─ UI WebContext              chrome views: nova://ui/tabbar, palette, keyhint, popup
│                             (one shared web process, no network, no extension)
├─ Web WebContext/NetworkSession per profile (ephemeral one for `open -p`)
│   ├─ network process        cookies (global policy + ITP), cache, downloads
│   └─ web processes (bubblewrap)
│        └─ libnova_extension.so
│             send-request: adblock network check, HTTPS upgrade (+redirects), headers
│             window-object-cleared: cosmetic CSS + uBO scriptlets before page scripts
│             WebFormManager will-submit-form: password save/update prompt
│             WebKitUserMessage <-> UI process (config, engine path, per-site rules)
```

**Window layout**
```
GtkApplicationWindow
└─ GtkOverlay
   ├─ GtkBox (orientation from tabs.position)
   │  ├─ tab bar WebView (nova://ui/tabbar, transparent)
   │  └─ GtkStack: one WebKitWebView per loaded tab
   └─ overlay WebViews: palette, keyhint, popup (tooltip/menu/password list),
      shown/hidden and placed by Rust
```

**Messaging**
- Chrome views → Rust: `webkit.messageHandlers.nova.postMessage("<cmd> <arg>")`, same
  message names as the CEF edition (`ready`, `tab N`, `close N`, `move N`, `run`,
  `text`, `complete ±N`, `hist ±N`, `cmdleave`, `tip`, `menu`, `menuact`,
  `menuclose`, `pwpick`). Rust → views: `evaluate_javascript("nova.apply(json)")`.
- Page scripts (world `novabrowser`) → Rust: message handler registered for that
  world only; frame-targeted answers via `…_with_reply` (W6).
- Rust ↔ extension: `send_message_to_page` / `user-message-received` (GVariant).
- `nova://` manager pages → Rust: `fetch('nova://action/<token>/…')`, token-checked.

**Keys:** `GtkEventControllerKey`, capture phase, on the window; `Propagation::Stop`
consumes (W3). Keys typed into the palette WebView go to its `<input>` (command
mode), as in the CEF edition.

---

## 5. nova-core (Phase C, D5)

New repo `../nova-core` (GitHub `Novaturiente/nova-core`), used by both editions as
a git dependency pinned by rev (`--frozen` AUR builds keep working); local work via
`[patch]` path override.

**Moves to nova-core** (from the CEF edition's `src/`):
- pure: `urlmatch`, `keys` (notation, parser, trie; not `key_from_cef`), `config` +
  `config.js` (engine-specific settings passed in as an extra schema), core part of
  `settings`, `marks`, `import`, `theme` (vars, fonts, colours, `format_title`,
  presets), `history` (db, search, import, page HTML), `session` (file format, names,
  debounce timing), `passwords` backends (rbw/bw/pass, generator, `$VISUAL` helper),
  `net` pure parts (`is_local`, `https_upgrade`, `referrer`, `language_list`),
  `downloads` pure parts (`unique`, `sanitize`, `summary`, history page), `aria2`
  RPC client, `spawn` (env, FIFO, userscript lookup, `run_child`), hints labels,
  `commands` tokenizer/aliases/placeholders/flag parser, `permissions`
  decide/queue/persistence, `adblock` list fetch/cache/engine build, `ipc` server,
  `profiles`, `welcome`, `settings_page`, `write_atomic`, dirs.
- One seam trait `Host` with the engine-specific calls these modules make today
  (`post_to_ui`, `message`, `show_completion`, `set_cmdline_text`, …). Two
  implementations (CEF, WebKit), so the trait is earned.

**Steps** (each ends with the CEF edition's `cargo test` + `tests/smoke.sh` green):
C1 repo + CI-less workspace; C2 move pure modules; C3 add `Host`, move modules with
UI calls; C4 CEF edition depends on nova-core, release flow (`packaging/PKGBUILD`,
`release.sh`, AUR `--frozen`) builds; C5 nova-core tests = the 92 existing tests.
The CEF edition gets a session-log entry + release note for the refactor.

---

## 6. Phase 0 — spikes left (`spikes/wN-<name>/`)

W9 (Speedometer) is done. Checks use real keys (`wtype` / compositor
`send_shortcut`), `evaluate_javascript` and `get_snapshot`; benchmarks run outside
the tool sandbox.

| # | Question | Pass condition |
|---|---|---|
| W1 | Wayland + media | HiDPI, clipboard both ways, `<select>`, IME passes through in insert mode; VA-API H.264 + AV1 (`GST_DEBUG=va*:4`); H.264/AAC `isTypeSupported` true; WebRTC call works |
| W2 | Layout + chrome views | GtkStack of 2 tabs keeps scroll/JS state; transparent overlay WebView over the page; focus moves palette ↔ page with no lost keys |
| W3 | Key capture | capture controller consumes `j`/`k`/Escape (page sees no keydown), with an input focused too; repeats consumed; unconsumed keys reach the page |
| W4 | World bridge | world `novabrowser` script in all frames reports focus of input/password/textarea/contenteditable (same- and cross-origin iframes); page world can't see the handler |
| W5 | Input without CDP | JS hint follow works on GitHub, Jira, YouTube, HN, Gmail, Wikipedia; `InsertText` editing command types with an `input` event; JS scroll moves the right inner scroller (Gmail, chat apps); list failures |
| W6 | Frame reply | reply from a cross-origin iframe answered 10 s later lands only in that frame |
| W7 | Adblock in extension | adblock-tester.com ≥ CEF edition (100/100); YouTube `adPlacements` gone; scriptlets run before page scripts; RSS per web process with/without engine (5 sites) |
| W8 | Session state | serialize → restore in a new view gives back/forward + scroll; size per tab |
| W10 | Sign-in + compat | Google, ChatGPT "Sign in with Google", Microsoft, a bank, WhatsApp Web, Discord, Meet, YouTube, Gmail, Maps: works / degraded / blocked, with and without a UA override. **Go/no-go for the project** |
| W11 | Permissions, TLS, dialogs | permission-request for notifications, geolocation, mic/cam, clipboard, pointer lock; held answers; `load-failed-with-tls-errors` on badssl; `script-dialog` held then answered |
| W12 | Headless PGO training | instrumented WebKitGTK runs Speedometer on novahome under a headless compositor (`cage` or `weston --backend=headless`), profile within 2% of the laptop-trained build |
| W13 | Dark Reader overhead | Speedometer A/B with the Dark Reader engine injected: ≤ 3% → port it (D6); else color-scheme fallback |
| W14 | Real-site perf | scroll FPS / jank on YouTube, GitHub, Maps, Reddit, Wikipedia, idle RSS (5 sites), startup to first paint: vs CEF edition (method `../novabrowser/spikes/s9-baseline`) |

---

## 7. Phases

Order: **W10 → C → remaining spikes → 1…10 → W (WebKit distribution) in
parallel from Phase 9.** Each phase ends runnable; exit checks are in PROGRESS.md
per item.

| Phase | Scope |
|---|---|
| C | nova-core extraction (§5) |
| 1 | Skeleton: GtkApplication, window, chrome views, tabs (GtkStack), `nova://`, URL/title/progress/favicon, popups + middle-click tabs, crash and hang handling |
| 2 | Modal core: keys, modes, counts, commands, bindings, auto insert mode, which-key, F1, navigation, scroll, zoom |
| 3 | Palette, completion, history |
| 4 | Hints, find, yank, clipboard |
| 5 | config.ts, settings schema (`webkit.*`), themes, `:settings` |
| 6 | External tools: spawn/userscripts/FIFO, fake-key, insert-text, IPC, `--cmd`/`--query` |
| 7 | Privacy: adblock extension, HTTPS-only, cookies/ITP, headers, permissions, dialogs, TLS page, dark mode, spellcheck |
| 8 | Sessions, marks, downloads + aria2, context menu, inspector, fullscreen, hibernation, windows, profiles, private windows, import, tab menu/tooltip/drag |
| 9 | Packaging `novabrowser-webkit`, docs, smoke test |
| 10 | Passwords |
| W | `webkitgtk-6.0-nova` distribution: headless PGO pipeline, GitHub release tarball, AUR `-bin`, update policy |

### Phase W — WebKitGTK distribution (D7)
- Build pipeline on novahome, one command per WebKitGTK release: fetch + verify
  tarball (sha256 + upstream PGP key), instrumented build, headless training (W12),
  PGO build, package, smoke (spike `w9-baseline --speedometer` + `tests/smoke.sh`),
  publish GitHub release, bump AUR `webkitgtk-6.0-nova-bin`. Always under the 20 GB cap.
- **Update policy:** rebuild within 3 days of any WebKitGTK stable release or
  WebKitGTK security advisory (WSA); watch `webkitgtk.org/security` and the Arch
  package version. Users are told in the AUR description that updates come from us.
- Package keeps `provides=(webkitgtk-6.0 libwebkitgtk-6.0.so libjavascriptcoregtk-6.0.so)`,
  `conflicts=(webkitgtk-6.0)`; post-install message: AVX2 only; how to go back.
- Decide before publishing: include GObject introspection (other GTK apps may need
  the typelibs) — default yes for the public package (needs `gobject-introspection`
  on novahome).

---

## 8. Rules (carried over + new)

- Web-process crash → crash page, session untouched; hang → "Kill it? y/n".
- Atomic writes, 0700 data dirs, 0600 files and IPC socket; no secrets at rest.
- Nothing blocking on the GTK main thread.
- Anything injected into every frame (incl. Dark Reader, adblock scriptlets,
  password detector) gets a Speedometer A/B; budget ≤ 3% each.
- Benchmarks and GUI checks run outside the novacode tool sandbox (nested bwrap
  kills the WebKit web process there; absolute scores are ~15% low inside).
- novahome builds always under `systemd-run --user --scope -p MemoryMax=20G`.
- Fix the CEF edition's open audit items (A1–A10, B1–B8, C1–C4) as code moves
  into nova-core, so both editions get them.

---

## 9. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Sites block or degrade WebKitGTK | not usable daily | W10 before anything else; per-site UA override (`content.user_agent`) |
| Untrusted JS input fails on some sites | hints/fake-key broken there | W5 list; `href` navigation, focus + editing commands |
| Adblock engine per web process costs too much RAM | memory blow-up with many sites | W7 numbers; shared mmap of the engine file; fall back to content filter for network rules |
| Dark Reader too slow | Speedometer regression | W13 gate, fallback in D6 |
| We miss a WebKitGTK security release | users run a vulnerable engine | Phase W policy, version watch, `-bin` description |
| nova-core refactor breaks the CEF edition | regression in the shipping browser | C steps each gated by tests + smoke + package build |
| HTML chrome adds memory/latency | slower UI | one shared UI web process; measure key → palette latency (< 16 ms) |
| webkit6-rs lags the C API | blocked feature | raw `webkit6-sys` calls; upstream MR |
| No Widevine | DRM sites fail | documented non-goal |
