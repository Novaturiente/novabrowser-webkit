# novabrowser-webkit — progress

Single source of truth for where the work stands (rules in `AGENTS.md`).
Every feature of the CEF edition (novabrowser 0.1.8) is listed below with its
pass condition. An item is `[x]` only when that condition was checked in a
session; the evidence goes on the item.

Status: `[ ]` todo · `[~]` partial · `[x]` done · `[-]` dropped (reason given) ·
`[m]` built, needs a human to check (listed in `docs/manual-test.md`).
"CEF ref" = where the CEF edition does it (`../novabrowser/novabrowser/src/`).
Evidence: **S** = `tests/smoke.sh` (43 checks), **F** = `tests/features.sh` (39 checks,
Phase 8), **P** = `tests/passwords.sh` (17 checks, Phase 10; `pass` backend, local
HTTPS); all pass 2026-10-29 on the release build pinned to nova-core `aec30e5`.
**U** = unit tests (`cargo test --workspace` here, `cargo test` in nova-core: 90).
Tests drive the browser over IPC: `--cmd`, `--query state`, `--key <keys>`.

## Now

- **Phase:** 1–8 and 10 built and checked by S/F/P where IPC can reach; user's manual pass (`docs/manual-test.md`) next.
- **Next action:** user runs `docs/manual-test.md`. Released 0.1.8 (2026-10-30). D7 duty now live: rebuild `webkitgtk-6.0-nova-bin` within 3 days of each WebKitGTK release/advisory. C5 (aria2) and C6 done in the CEF repo (`main` 650078a, not pushed).
- **Blockers:** none.

---

## Phase 0 — spikes (PLAN.md §6)

- [x] W9 Speedometer 3.1 — installed `webkitgtk-6.0-nova` 27.25 vs CEF 0.1.8 26.3 (2026-10-28, `spikes/w9-baseline/results/ab-cef.jsonl`); Arch build 18.95 vs own PGO 23.95 in the tool sandbox (`ab-build.jsonl`); env knobs no gain (`ab-env.jsonl`).
- [~] Own WebKitGTK build — PGO + v3 + ThinLTO, `webkitgtk-6.0-nova` 2.54.2-2 (with `patches/0001-color-filter-roles.patch`) installed on novarch 2026-10-29; smoke all passed with the sandbox on (`spikes/wk-build/`). Left: video/site check (W1, W10), introspection decision (Phase W).
- [x] W10 sign-in + site compat — **go** (user, 2026-10-29: Google sign-in and ChatGPT "Sign in with Google" work in the spike with `--safari`). HDFC NetBanking shows an "Upgrade browser … older browser version" modal with the stock WebKit UA and **none with the Safari identity** (correction, user 2026-10-29: the browser shows it with **both** UAs; HDFC is an open WebKit-edition issue) (`results/w10/hdfc-{webkit,safari}.png`; found by the user, missed by the text check, which had no "upgrade browser" pattern and only 600 chars). Screenshot diff safari vs stock: only hdfc (59.5%) is UA-dependent; reddit/youtube/maps/whatsapp diffs are live content. Not tried: Microsoft/Teams, WhatsApp link, Discord, bank login, Meet call — checked again in Phase 8 daily driving. Part 1 done 2026-10-29 (`spikes/w9-baseline/compat.sh`, `results/w10/compat.jsonl` + PNGs, ephemeral session, Safari and stock UA): Google sign-in, Gmail, YouTube, Maps, Meet, ChatGPT, Microsoft login, Outlook, Teams, WhatsApp Web (QR shown, "Download for Mac"), Discord, GitHub, Slack, Figma, HDFC, SBI YONO, Reddit (JS challenge passed), X all load their normal pages; no "unsupported browser" text on any; identical for both UAs. YouTube video loaded but didn't autoplay (no gesture; playback is W1). Left (needs user): real sign-ins — Google, ChatGPT via Google, Microsoft/Teams, WhatsApp QR link, Discord, a bank; Meet call.
- [m] W1 Wayland + media (HiDPI, clipboard, `<select>`, IME, VA-API, H.264/AAC, WebRTC)
- [x] W2 layout + transparent chrome views + focus hand-off — tab bar, palette, keyhint, popups are transparent `nova://ui/*` views over a GtkStack (screenshot 2026-10-29); focus hand-off [m]
- [x] W3 key capture — capture-phase `EventControllerKey` on the window; hint mode driven from it (screenshot of labels 2026-10-29)
- [~] W4 world `novabrowser` bridge — hints, focus reports, passwords, adblock scan run there (S: hints, adblock); all frames only for focus reports; passwords top frame only
- [-] W5 input without CDP — no trusted input in WebKit; hints use synthetic pointer+mouse events + `click()`, `fake-key` JS events + editing commands; site list → [m]
- [-] W6 frame-targeted reply — not available from the UI process; passwords fill the top frame only (shortcut in `passwords.rs`)
- [~] W7 adblock engine in extension — works (S: listed request blocked, others pass); score, YouTube, RSS per process [m]
- [x] W8 session state restore — WebKit session-state blob per tab (S: restore at start)
- [x] W11 permissions, TLS errors, JS dialogs — S: geolocation prompt, alert prompt, both dropped on tab close; TLS [m]
- [ ] W12 headless PGO training on novahome
- [x] W13 Dark Reader overhead, measured 2026-10-29 (`spikes/w9-baseline/ab-darkreader.sh`, Safari identity, 3 interleaved rounds, AC): off 26.7/26.8/26.8, Dark Reader 16.0/15.5/15.6 (**−42%**), inverting CSS 23.3/23.6/23.5 (**−12%**). Both fail the 3% rule (CSS invert also left text unreadable, user). WebKit's own colour filter (`-apple-color-filter`, feature `ColorFilter`, no WebKit rebuild needed): off 26.9/26.6/26.8 vs on 27.1/25.0/27.0, no measurable cost → shipped (D6). Dark Reader removed.
- [~] W14 real-site perf vs CEF (`spikes/w14/compare.py`, results/w14.json, 2026-10-29): startup to first local page median 0.312 s vs CEF 0.298 s; 5 real sites, PSS of the process tree 1556 MB (other runs 1433, 1924) vs CEF ~940 MB. Without the adblock extension 1564 MB: the gap is WebKit's page processes, not adblock (closes W7's RSS question). Fixed: chrome views now share one web process (was 5 per window). [m] scrolling feel
- [x] W15 overlay flash (user: URL bar "jumps/flashes" on light pages) — `spikes/w15-ui-smooth/flash.sh` records the window (wf-recorder, deleted after) and prints per-frame luma. Before (2026-10-29): white page 177 → **38 for 4 frames** (opaque `#111` palette body before the snapshot loaded) → 86; dark page 44 → **89 for 2 frames** (stale blurred white page: a hidden WebView keeps its last frame) → 35. After: 231 → 192 → 113 → 106 and 43 → 35 → 29, no dip or spike; close and which-key fade the same way. Fix: `src/anim.rs` shows overlays transparent, fills them, fades in once their page painted two rAFs (250 ms fallback) and fades out on close; palette body transparent over a GTK `WidgetPaintable` of the page (blurred, on black), card pop-in 160 ms; prompt, tooltip, menu, password list, which-key use the same path. S/F/P all pass. First open after start still drops ~2 frames (50 ms gap, likely GSK blur shader warm-up); later opens 17 ms per frame.

## Phase C — nova-core (PLAN.md §5)

- [x] C1 repo `../novabrowser/nova-core` + GitHub `Novaturiente/nova-core`, crate builds
- [~] C2 moved: urlmatch, keys (+ GDK converter), config + config.js, settings, marks, import, theme, history, session, password backends, net policy, aria2 RPC, spawn/userscripts, hint labels, command parser, permission setting names, profiles, welcome, settings page, write_atomic, dirs (U: 89 in nova-core). Not moved (per-edition copies): downloads list/page, permission decide/queue, adblock list fetch/engine build, ipc server
- [x] C3 `Host` seam trait (`nova_core::host`); CEF `CefHost`, WebKit `WebKitHost`
- [~] C4 CEF edition on nova-core `0433ff0` (branch `nova-core` in `.worktrees/cef`): `cargo test` 16, `tests/smoke.sh` pass (2026-10-28); `makepkg --nodeps` of `packaging/PKGBUILD` from that branch built `novabrowser-git r135.89185c0` incl. its `check()` (2026-10-29). Left: `release.sh` dry run; move CEF to the newer nova-core (needs `run_in_tab`/`show_text` in `CefHost`)
- [~] C5 audit fixes: A1 (hint chars < 2, nova-core `hints`, U), A3 (prompt grace 400 ms, WebKit `permissions`, U), A5 (`{hint-url}` stays one token, U), A7 (popups need a gesture, WebKit). Left: A2, A4 (secret still on aria2c argv), A6, A9, A10, B5–B7, C2–C4, and porting A3/A5 into the CEF edition
- [ ] C6 CEF edition session log + release note for the refactor

## Phase 1 — Skeleton

- [x] 1.1 GtkApplication, Wayland app id + X11 WM_CLASS `novabrowser-webkit` (prgname, no GApplication id: our socket keeps one instance per profile), window 1200×800 — S starts/quits; [m] resize/maximise
- [x] 1.2 `<url>`: bare host → `https://`; `://`, `about:`, `data:` pass through — `main.rs start_url`; S launches with `data:` URLs
- [~] 1.3 Tabs in a GtkStack, one WebView per loaded tab (S: tabs, switching); [m] `visibilityState=hidden` for hidden tabs
- [x] 1.4 Chrome views on `nova://ui/*`, transparent, own WebContext + ephemeral session; `nova` handler only there; `ready` handshake; reloaded after their web process dies (S: works after kill -9)
- [m] 1.5 Tab bar: label, favicon, active, loading, progress line, crashed, pinned, asleep markers; click switches, middle-click closes
- [m] 1.6 `tabs.position`, `tabs.width`, `tabs.show`, re-layout on resize
- [m] 1.7 Window title from `window.title_format` (`[profile] ` prefix)
- [x] 1.8 URL, title, load progress, back/forward from WebView signals (S: titles/URLs in `--query state`, back/forward)
- [m] 1.9 Favicons in the tab bar
- [m] 1.10 `target=_blank` / `window.open` → tab with opener kept (`create` + `related-view`)
- [m] 1.11 Middle / Ctrl-click link → background tab
- [m] 1.12 Popups without a user gesture refused ("Blocked a popup…")
- [x] 1.13 `nova://` secure + local + CORS; page token checked on every action (`tabview::internal_action`); only `nova://` main frames may act (S: `:history` page opens)
- [x] 1.14 Crash page on `web-process-terminated` with URL, reload brings it back, other tabs keep running (S: kill -9 every web process, crash page, `reload`)
- [m] 1.15 Hang prompt on `is-web-process-responsive` false → `terminate_web_process`; dropped on recovery
- [m] 1.16 Tab close runs beforeunload (`try_close`); closed pages really stop (YouTube audio)
- [x] 1.17 Quit exits 0 (S, twice)
- [-] 1.18 Log file — WebKit has no log file of its own; we log to stderr (journal when started by the desktop). Add a file if daily driving needs one
- [m] 1.19 Safari identity (D8) per site; httpbin check

## Phase 2 — Modal core

- [x] 2.1 Capture-phase key controller; `key_from_gdk` with tests (U: `keys::tests::key_from_gdk_cases`)
- [~] 2.2 Modes normal / insert / passthrough / command / hint / prompt (S: hint, prompt); [m] badge look
- [x] 2.3 Key notation, trie, counts (U: 20+ `keys::tests`); [m] pending keys shown
- [m] 2.4 `forward_unbound_keys=auto`
- [m] 2.5 Insert/passthrough single-key binds, `Ctrl-v`, `Shift-Escape`
- [m] 2.6 Auto insert mode on focusing an editable (all frames)
- [x] 2.7 Default keymap; no duplicates, every command exists (U: `bindings::tests::core_is_valid_and_has_no_duplicates`)
- [m] 2.8 Which-key popup after a prefix
- [m] 2.9 `F1` / `:bindings [filter]`
- [x] 2.10 Registry, flags, `;;`, aliases before the split (U: `commands::tests`); `{clipboard}` `{primary}` [m]
- [x] 2.11 Unknown command → error in the tab bar (S); [m] `messages.timeout`
- [~] 2.12 `open [-t|-w] [url]` (S); [m] `-b`, `-p`, search engines, start pages
- [x] 2.13 `back` / `forward` (S)
- [m] 2.14 `reload [-f]`, `stop`
- [m] 2.15 `scroll*`, inner scrollers, `scrolling.smooth`
- [~] 2.16 `zoom` (S); per-site `zoom.json` [m]
- [~] 2.17 Tab commands: `tab-close`, `tab-pin`, `undo` (S); [m] the rest
- [~] 2.18 `mode-leave`, `message-info` (S); [m] `cmd-set-text`, `paste`
- [m] 2.19 `tabs.last_close`
- [x] 2.20 Pinned tab refuses `tab-close` ("Tab is pinned!…", S); [m] order + title formats
- [m] 2.21 Key → command < 2 ms (`NOVA_KEY_TIMING=1`); HN without a mouse

## Phase 3 — Palette, completion, history

- [m] 3.1 Floating palette for `o` / `O` / `:`
- [m] 3.2 Blurred backdrop (live page through GTK `filter: blur() brightness()`, since W15; was a PNG snapshot); `/` `?` thin strip
- [m] 3.3 Readline keys
- [m] 3.4 Tab / Shift-Tab, Up/Down, Ctrl-p/n
- [x] 3.5 Completion sources + order (U: `completion::tests`); [m] look
- [m] 3.6 Ranking typed → visits → recency
- [x] 3.7 History db on its own thread (U: `history::tests`, migration test)
- [x] 3.8 `search_100k_rows_under_16ms` (U)
- [x] 3.9 Command history prefix navigation (U: `completion::tests::history_prefix_navigation`)
- [~] 3.10 `:history` page opens (S); [m] search, delete

## Phase 4 — Hints, find, yank, clipboard

- [x] 4.1 `hint [-r] [-f] [group] [target] [args]` enters hint mode (S)
- [m] 4.2 Collection rules (iframes, shadow roots, dialogs)
- [x] 4.3 Labels `_hint_scattered` (U: `hints::tests`); closed shadow root drawing (screenshot 2026-10-29); [m] colours/fonts settings
- [m] 4.4 Filtering, Backspace, auto-follow, `-r`, `-f` / `gi`
- [~] 4.5 Targets: `run` URL stays one token (U); [m] all targets
- [m] 4.6 Exit: `f`, `F`, `;y`, `;o`, spawn mpv on GitHub, Jira, YouTube
- [x] 4.7 Find `/` incremental, smart case (U: `find_options`), match count / not found messages (S); [m] `n` / `N`
- [m] 4.8 `yank …` (not in S: it would overwrite your clipboard)
- [m] 4.9 GdkClipboard, `pp` / `Pp`, 1 MiB / 2 s caps

## Phase 5 — Config, settings, themes

- [x] 5.1 `config.ts` → oxc → QuickJS (U: `config::tests::loads_settings_patterns_binds`)
- [x] 5.2 Errors name the config.ts line (U: `errors_name_the_config_line`); [m] broken file at start
- [m] 5.3 `--check-config`
- [x] 5.4 `:set` persists to `autoconfig.json` (S), validation (U); [m] precedence with config.ts
- [x] 5.5 Per-site `pattern()` (U); `content.user_agent` added per site
- [x] 5.6 Schema: added `webkit.hardware_acceleration`, `webkit.site_specific_quirks`, `webkit.itp`, `content.user_agent`, `downloads.backend=webkit`. Per edition: `settings::other_edition` — `:set chromium.*` refused in WebKit, `webkit.*` in CEF; config.ts may still set them (one file, ignored) (S, U)
- [x] 5.7 Every colour setting covered by every preset (U: `theme::preset_tests`); [m] reaches chrome views and pages
- [m] 5.8 `fonts.web.*`, `colors.webpage.bg`; `preferred_color_scheme` not wired yet
- [m] 5.9 `:settings` page
- [m] 5.10 Exit: your config.ts loads

## Phase 6 — External tools

- [x] 6.1 `spawn [-u] [-o]`; output tab (S); [m] stderr line on failure
- [x] 6.2 Userscripts from `<config>/userscripts/` with `QUTE_*` env (S: `QUTE_TITLE` echoed back)
- [x] 6.3 `QUTE_FIFO` lines run in the starting tab (S); temp dir removal (U: fifo test)
- [x] 6.4 `spawn -o` output in a text tab (S)
- [m] 6.5 `fake-key` (JS events + editing commands; limits in docs)
- [m] 6.6 `insert-text`
- [x] 6.7 IPC socket 0600, `url`, `cmd`, `query state` (S)
- [x] 6.8 `--cmd`, `--query`, second launch hands its URL over (S); [m] no URL → new window; no instance → exit 1
- [x] 6.9 `dark-toggle` writes `darkmode_excludes` (S)

## Phase 7 — Privacy, blocking, prompts

Adblock (D3, engine in the extension)
- [x] 7.1 Extension `libnovabrowser_webkit_ext.so` (workspace member `ext/`) loaded from `$NOVA_WEBKIT_EXT_DIR`, next to the binary or `/usr/lib/novabrowser-webkit`; config + engine path via init data and `cfg` messages (S: "extension found", blocking works)
- [x] 7.2 Blocking in `send-request`, main frame never blocked (UI sends `nav` before each main-frame navigation), per-site `content.blocking.enabled`, whitelist (S + U: `engine_from_cfg_blocks`)
- [x] 7.3 Lists fetched with curl (`--proto =https,http`, `--proto-redir =https`, `--max-filesize 64M`), cached `engine.dat` + `lists.txt`, `:adblock-update`, "adblock: N lists" (S); all lists failing keeps the old cache and its age (A10, code review only)
- [x] 7.4 Cosmetic CSS + scriptlets: per-tab user script swapped at main-frame `decide-policy`, hostname-guarded, constructable stylesheet (U: `cosmetic_and_generic`); [m] on real sites
- [x] 7.5 Generic hiding: `adblock-scan.js` in our world → `hidden_class_id_selectors` (U); [m] on real sites
- [m] 7.6 Exit: adblock-tester, YouTube ads, Speedometer cost, RSS per web process

HTTPS-only
- [~] 7.7 Main frame upgrade in `decide-policy`, subresources + redirects in the extension (U: `https_upgrade_rules`); [m] on a real http site
- [x] 7.8 Local exemptions (U), `https_excludes` + `https-toggle` (S)
- [m] 7.9 Failed upgraded load message

Cookies, headers, network privacy
- [m] 7.10 `content.cookies.accept` via CookieManager, ITP
- [m] 7.11 Custom headers / DNT in `send-request` (page-level values: shortcut, see ext)
- [~] 7.12 Accept-Language, autoplay via `media_playback_requires_user_gesture`; hyperlink auditing / DNS prefetch setters are deprecated no-ops in 2.50+ [m]
- [x] 7.13 Idle network capture (`tests/idle-net.py`, 2026-10-29): fresh profile 90 s on about:blank = only `curl` fetching the adblock lists; second start with lists cached = 0 peers. Polls /proc every 0.1 s (no root); DNS via systemd-resolved not visible

Prompts and permissions
- [x] 7.14 Tab-bar prompt queue, Escape = no, grace period 400 ms (U), dropped when its tab closes (S)
- [~] 7.15 geolocation prompt (S); notifications, mic, camera, screen, clipboard, pointer lock, storage access, device info [m]; missing codec: media `error` events (decode / not supported) give "Missing codec? Install gst-libav and gst-plugins-bad." (S: broken .mp4); WebKitGTK 6 has no missing-plugin signal
- [x] 7.16 true/false/ask per origin, saved A/N in `permissions.json`, saved notification answers given to WebKit at start
- [~] 7.17 alert in the prompt (S); confirm / prompt / beforeunload [m]
- [~] 7.18 TLS error → a prompt (not a `nova://` page); yes = `allow_tls_certificate_for_host` + reload [m]

Appearance
- [~] 7.19 Dark mode per site (S: `dark-toggle`): WebKit colour filter as a per-view user style, live switch on toggle; pages already dark (GitHub on a dark system theme) skipped via a check in world `novabrowser` that re-runs on theme changes (html/body attributes, load, system scheme, 1 s and 3 s); smoke: `tests/pages/late-dark.html` is filtered while light, left alone once dark. Screenshot-checked: Wikipedia live + fresh load dark, GitHub stays dark. Images keep their colours (Rust logo black on dark). WebKit patch `spikes/wk-build/patches/0001-color-filter-roles.patch` (webkitgtk-6.0-nova 2.54.2-2): backgrounds never lightened, text never darkened → dark headers/code blocks stay dark (`tests/pages/mixed-dark.html`, before/after shots); pages without a background get a dark view background; Speedometer with patched libs off 26.9/26.7/26.8 vs on 26.6/26.7/27.0. Checked: Wikipedia, HN, Stack Overflow (+ Google sign-in frame), GitHub, old.reddit [m] look
- [m] 7.20 Spellcheck

## Phase 8 — Sessions and daily driving

Sessions and tabs
- [x] 8.1 Autosave (1 s debounce, atomic, only when changed) to `<data>/sessions/default.json`; restore at start, command-line URL added as a tab (F: written without quitting; restart + CLI URL = 3 tabs)
- [~] 8.2 Per tab: url, title, session-state blob (back/forward + scroll), pinned, keep-awake, icon (S/F: restore keeps tabs + titles); [m] back/forward + scroll after restore; legacy shape not tested
- [x] 8.3 Lazy restore: background tabs get a WebView only on first show (F: `loaded:false` after restart, true after `tab-focus`)
- [x] 8.4 `session-save [name]`, `session-load [-c] name`, `session-delete`, `quit -s`, `close -s`; names can't leave the dir (S: save, `../evil`, `quit -s`; F: `load -c`, delete)
- [x] 8.5 `undo` across restarts (last 100 closed) (F); brings back the URL only, same as the CEF edition (its `closed` list is URLs too)
- [~] 8.6 Hibernation `tabs.hibernate_after` (min, per site): save state, destroy the view, restore on show; skip active, loading, crashed, internal, playing media, typed forms, keep-awake; `tab-hibernate [n]`, `tab-keep-awake [n]`; dimmed in the tab bar (F: `tab-hibernate N`, active refused, wakes on show, `tab-keep-awake`); [m] timer-driven `tabs.hibernate_after`, dimmed look
- [x] 8.7 Multiple windows (`open -w`, `Ctrl-n`), per-window state, `:close` one keeps the rest, session restores all windows (S: `open -w`, `:close`; F: 2 windows restored)
- [~] 8.8 `tab-give` (`gD`) moves the live WebView to a new window (F: window count +1); [m] no reload
- [m] 8.9 Tab tooltip (title + URL, 450 ms), right-click tab menu (Reload, Duplicate, Copy URL, Mute, Bookmark, Pin, Move to new window, Hibernate now, Keep awake, Close, Close others) with j/k/Enter/Esc (code: all 11 menu actions wired, `window.rs` MENU / `tab_menu_action`)
- [~] 8.10 Drag a tab to reorder; pinned group kept (F: `tab-move +`); [m] drag
- [~] 8.11 Audio indicator + `{audio}` in titles from `is-playing-audio` / mute (F: `tab-mute`); [m] indicator with real audio

Marks
- [x] 8.12 Quickmarks `<config>/quickmarks` and bookmarks `<config>/bookmarks/urls` (qutebrowser formats); `m`, `M`, `b`, `B`, `gb`, `gB`; `quickmark-save/add/load/del`, `bookmark-add [--toggle]/load/del/list [-j]` (S: add/load, `bookmark-add`; F: `quickmark-save` opens the command line, add/del, bookmark del)
- [~] 8.13 Manager pages `Sb` / `Sq` on `nova://bookmarks` (edit, delete, validation) (F: `:bookmarks` opens); [m] edit/delete in the page

Downloads
- [~] 8.14 No dialog, `downloads.location.directory` (`~` expanded), sanitized names, unique `(1)` suffixes, started/finished/failed messages, pill `↓ N · P%` (S: directory, `(1)` names); [m] messages, pill
- [~] 8.15 `:downloads` / `Sd` page, `download [url]`, `download-open [N] [cmd]` (`downloads.open_dispatcher` / xdg-open), `download-cancel [N]`, `download-clear`, `downloads.json` (last 500) (F: json written, page opens, `download-clear`); [m] `download-open`, `download-cancel`
- [~] 8.16 aria2 backend (`downloads.backend` auto/aria2/chromium→webkit): RPC on 127.0.0.1 with secret not on argv (CEF audit A4), 8 connections, session resume, cookies incl. HttpOnly + UA + referer, magnet links (`decide-policy`), torrents as one entry, cancel removes partial files safely (A6), fallback to WebKit download (S/F: aria2 download lands; F: secret not on argv, now a 0600 `aria2.conf`, nova-core `aec30e5`); [m] magnet, resume, cancel

Other
- [m] 8.17 Context menu: open link in tab/background/window, copy link, save link; image/media open/save/copy; search selection (code: link tab/background/window, image/media tab, search selection; WebKit's stock copy/save items kept)
- [~] 8.18 `devtools` → Web Inspector (own window) (F: no error); [m] the window
- [~] 8.19 Page fullscreen fills the window only (`content.fullscreen.window`), `fullscreen [--leave]` (F: `fullscreen` toggles the window); [m] a video's fullscreen stays in the window
- [x] 8.20 Profiles: `--profile=x`, `--profile x`, `-P x`, `--list-profiles`, `:profile-new`, `:profile-open`, own process + data dir, `[name]` pill (F: `--list-profiles`, `profile-open` starts its own instance with its own IPC socket); [m] pill
- [~] 8.21 Private windows `open -p` on an ephemeral NetworkSession, marked in the tab bar (F: `open -p` window reports `private`); [m] mark, nothing saved
- [x] 8.22 Welcome page once per profile (`<data>/welcomed`), `:help`, `--import` offers per browser found (F: welcome tab on a fresh profile, `welcomed` written)
- [~] 8.23 `--import [name|dir|file]`: Chrome, Chromium, Helium, Brave, Vivaldi, Edge, Firefox, Zen, LibreWolf, Floorp, qutebrowser (history, bookmarks, quickmarks, session) + `novabrowser` (CEF edition, built 2026-10-29 in nova-core 7c1557b — earlier this line claimed it before it existed: history + cmd history + marks merged; config dir files, sessions, permissions.json, downloads.json copied where missing; zoom not carried: Chromium keeps it in `Preferences`); idempotent (F: Chromium `History` dir imports 2 rows; F: CEF import merges, copies, keeps existing files, second run changes nothing; copy of the real CEF profile: 556 history rows + 184 commands, its config.ts passes `--check-config`); [m] the other browsers
- [ ] 8.24 Exit: one week as the only browser, problems logged here

Phase 8 note (2026-10-29): every item has code; F checks what IPC can reach.
What needs eyes or a mouse (tab menu, drag, context menu, pills, inspector window,
video fullscreen) is `[m]` in `docs/manual-test.md`.

## Phase 9 — Packaging and docs

- [x] 9.1 Cargo workspace: app + `ext/` cdylib; `[profile.release]` thin LTO + strip; release build has no warnings; `Cargo.lock` matches the pinned nova-core git rev (`cargo metadata --locked` ok) (2026-10-29)
- [x] 9.2 Released 0.1.0 (2026-10-29): GitHub `Novaturiente/novabrowser-webkit` (public, branch `master`), tag `v0.1.0` + release tarball, release `webkitgtk-6.0-nova-2.54.2-2` (WebKit build asset); AUR `novabrowser-webkit`, `novabrowser-webkit-bin`, `webkitgtk-6.0-nova-bin` pushed (`git ls-remote` + package pages 200); both -bin PKGBUILDs built from the uploaded assets. `packaging/PKGBUILD` (-git; `NOVA_SRC=file://$PWD/.. makepkg`), `packaging/aur/novabrowser-webkit` (tag tarball) + `-bin`, `packaging/release.sh`: depends `libwebkitgtk-6.0.so`, `gtk4`, `gst-plugins-good`, `curl`; optdepends `webkitgtk-6.0-nova`, `gst-libav`, `gst-plugins-bad`, `gst-plugin-va`, `aria2`, `rbw`, `pass`, `bitwarden-cli`; extension in `/usr/lib/novabrowser-webkit/`. Checked: -git makepkg builds + tests; packaged binary + extension pass `tests/smoke.sh` 3/3 runs; tag PKGBUILD builds from a `git archive` tarball. Not done: GitHub repo, tag, AUR push, `webkitgtk-6.0-nova-bin` (needs user go)
- [x] 9.3 `packaging/novabrowser-webkit.desktop` (`desktop-file-validate` ok; no private-window action: no `--private` flag), icons copied from the CEF edition, `xdg-settings` line in README
- [x] 9.4 `tests/smoke.sh` + `tests/features.sh` + `tests/passwords.sh` over IPC (shared `tests/lib.sh`) — 43 + 39 + 17 checks (start, tabs, hand-over, hints, find, prompts, crash, spawn/userscripts, sessions, marks, windows, downloads, toggles, adblock, quit, restore), all pass 2026-10-29
- [x] 9.5 Docs ported to `docs/user/` (2026-10-29; facts checked against `src/`, unverified points listed in the session log) from `../novabrowser/novabrowser/docs/` (getting-started, keybindings, commands, configuration, settings, tabs-sessions-profiles, passwords, downloads, privacy, scripting, migrating, building, troubleshooting) with WebKit differences
- [x] 9.6 README with install, default browser, differences from the CEF edition

## Phase 10 — Passwords

- [~] 10.1 `passwords.backend` rbw (default) / bw / pass / none; list, locked check, unlock, secret, TOTP, save, update; 60 s metadata cache (P with `pass`: list, match by host, none for another host, save, update); [m] rbw, bw, TOTP, locked vault
- [~] 10.2 Field detection (`passwords.js` kinds user / password / new / otp) in world `novabrowser`, all frames; listeners armed only after a login field is focused (P: focused user field → 2 rows); top frame only (WebKit limit, see note)
- [x] 10.3 Suggestion list under the field (non-focusing popup view): Down/Up/Enter/Escape, click; rows for entries, "Generate a strong password", "<backend> is locked" (P: list, `<Down><Return>`, several matches → Enter fills); [m] click, locked row
- [~] 10.4 Fill only into the asking frame (reply object), same-site frames only, never on `http://`, never on load; `Sp` = `password-fill [--otp]` fills the focused field (P: `password-fill` fills user + password, refuses http); frames: top frame only
- [x] 10.5 Generated passwords (urandom, 20 chars) for sign-up fields (P: Generate row fills 20 characters)
- [x] 10.6 Save / update prompt after submit (`WebFormManager::will-submit-form` + JS fallback); split logins remember page-1 user; same password → no prompt; newer prompt replaces older; expiry 5 min (P: save prompt + `y` writes the entry, update prompt + `y`, same password no prompt, split login keeps page 1's user); JS submit detection only
- [m] 10.7 rbw add/edit through novabrowser as `$VISUAL` (secret in env, never argv); locked vault asks before unlocking
- [~] 10.8 Exit: single-page, split, iframe, TOTP, sign-up pages; save + update land in the vault; cross-site iframe gets nothing (P: single-page, split, sign-up, save, update); [m] TOTP, rbw; iframes get nothing (top frame only)

Phase 10 note (2026-10-29): backends in nova-core (U: 3 tests); WebKit UI in
`src/passwords.rs` — top frame only (W6), JS submit detection only (no
WebFormManager yet).

## Phase W — webkitgtk-6.0-nova distribution (D7)

- [ ] W.1 One-command pipeline on novahome under the 20 GB cap: fetch + verify (sha256 + PGP), instrumented build, headless training (W12), PGO build, package
- [ ] W.2 Public package includes GObject introspection (decide), post-install note: AVX2 only, how to go back
- [ ] W.3 GitHub release tarball + AUR `webkitgtk-6.0-nova-bin` (`provides`/`conflicts` `webkitgtk-6.0`)
- [ ] W.4 Update policy in place: rebuild within 3 days of a WebKitGTK stable release or WSA; version watch
- [ ] W.5 Release check: Speedometer ≥ previous build, smoke test, YouTube plays

---

## Decisions

Newest first. Settled — don't reopen without new evidence.

- 2026-10-29 — Overlays fade (in 140 ms after their page painted, out 90 ms; off when GTK animations are off) and the palette backdrop is the live page blurred by GTK, not a snapshot: the snapshot cost a PNG + base64 round trip on the main thread and showed a stale frame (W15). Device-info requests are allowed when the mic or the camera is.

- 2026-10-29 — Prompts over pages: TLS errors ask in the tab-bar prompt (like every other question) instead of a `nova://` error page. Reopen if the prompt proves too easy to answer by accident.
- 2026-10-29 — Main-frame detection for adblock: the UI sends a `nav` page message before letting each main-frame navigation go ahead; it reaches the web process before the load (same IPC connection). The extension follows the main document through redirects.
- 2026-10-29 — D9 (user): no DRM in the WebKit edition v1; DRM sites stay in the CEF edition.
- 2026-10-28 — D8 (user): appear as macOS Safari by default (UA + `navigator.platform`); `content.user_agent` keeps `webkit` and custom values, per site. Safari version token followed by hand (27.0 now).

- 2026-10-28 — D1–D7 (user; PLAN.md §2): GTK 4 + WebKitGTK 6.0; chrome in HTML/CSS (user wants full styling freedom, tab bar redesign later — assistant had recommended native GTK); adblock engine in the web-process extension (assistant had recommended content filters; RSS checked in W7); separate dirs + `--import novabrowser`; extract `nova-core` first (assistant had recommended copying first; risk: refactor of the shipping CEF edition); Dark Reader port if ≤ 3% Speedometer cost, else color scheme; ship `webkitgtk-6.0-nova` on the AUR (we own its security updates).
- 2026-10-28 — Go ahead with the build (user) after `webkitgtk-6.0-nova` matched CEF on Speedometer (27.25 vs 26.3). W10 (site compat) still gates it.
- 2026-10-28 — Benchmarks and GUI checks run outside the novacode tool sandbox (nested bwrap kills the web process; scores ~15% low inside).
- 2026-10-28 — novahome builds capped at 20 GB RAM (user: other services must keep running): `systemd-run --user --scope -p MemoryMax=20G`, `-j8`.
- 2026-10-28 — Project created as a separate repo next to `../novabrowser/novabrowser`; plan from novabrowser 0.1.8 (`9574fca`).

## Session log

Newest first.

- 2026-10-30 (IST, 19:02) — Released 0.1.8 with mouse Back/Forward support. Workspace tests passed (12 browser, 2 extension); release.sh clean source package build and checks passed; GitHub v0.1.8 asset published. The -bin package built from the uploaded asset. Both AUR updates pushed and remote hashes verified: novabrowser-webkit 8425f5a, novabrowser-webkit-bin 0bfc940. System installation unchanged.

- 2026-10-30 (IST, 15:59) — Mouse side buttons now navigate history: tab WebViews capture GTK buttons 8/9, navigate on press, consume release, and leave other buttons untouched. Release build and 12 tests passed. Independent verification with ydotool and an isolated profile traversed three local pages backward and forward exactly one entry per click. Ordinary clicks on empty content did not navigate; link/context-menu behavior was not checked. Test-focus incident: one initial back click reached WhatsApp, effect unknown; subsequent events were guarded by active-window checks. Not released.

- 2026-10-30 (IST, 14:45) — Prepare 0.1.7: dark grey (`#333333`) is now the default page background in nova-core `b91dec7` (pushed; 91 tests passed). Both manifests pin it; settings reference updated. Release workspace build and tests passed. Includes the committed unreachable-page UI. Published at 14:50 IST: release.sh clean source build and package tests passed; GitHub v0.1.7 asset uploaded; -bin package built from that asset. Both AUR updates pushed and remote hashes verified: novabrowser-webkit beef8cc, novabrowser-webkit-bin b7c7ea0. System installation not changed.

- 2026-10-30 (IST, 13:35) — Unreachable http(s) pages get a Chrome-style "This site can't be reached" page (`window::load_error`, shares `notice_page` with the crashed page): Chrome wording + `ERR_*` code from the libsoup/GIO message, tab title = host, `r` retries the real URL. Policy errors (blocked ports, downloads) keep WebKit's text. Checked: refused (127.0.0.1:59999), unresolved (nope.invalid) by screenshot; `r` after starting a server loaded it; unit test `load_error_wording`. Crashed page not re-checked live after the refactor.

- 2026-10-30 (IST, 13:15) — Released 0.1.6: nova-core `8314d79` pushed (tab-pin default `Ctrl-Shift-p`) and pinned; `cargo test --workspace` pass; tag `v0.1.6`, `release.sh` (clean makepkg build + check, GitHub release asset); `-bin` PKGBUILD built from the uploaded asset; AUR `novabrowser-webkit` `431defa`, `novabrowser-webkit-bin` `425d372` pushed (`git ls-remote` matches). The CEF edition still pins an older nova-core (default pin key there is still `Ctrl-p`).

- 2026-10-30 (IST, 12:55) — Toast pill cut short text ("Tab pinn…"): WebKit sized the flex pill from the icon's content width, not its `flex-basis: 22px`. Fix: `width: 22px` on `#i` in `ui/toast.html`. Checked by screenshot: "Tab pinned." / "Tab unpinned." whole, a message wider than the window still ends in ….

- 2026-10-30 (IST, 12:40) — `tab-pin` shows "Tab pinned." / "Tab unpinned." (checked via `--cmd tab-pin` + `--query state`, and real `wtype` Ctrl-Shift-p). Default pin key `Ctrl-p` → `Ctrl-Shift-p` in nova-core (committed locally, not pushed; Cargo.toml still pins d3866c7 until the push + rev bump). Tab menu label and docs updated.

- 2026-10-29 (IST, 18:50) — W15: overlay flash measured and fixed (anim.rs, GTK-blurred live backdrop, fades for palette/prompt/tooltip/menu/password list/which-key); `DeviceInfoPermissionRequest` follows mic or camera (was camera only). S/F/P pass. Next: user manual pass.

- 2026-10-29 (IST, 12:40) — 5.6, 7.13, 7.15 done; W14 measured (startup 0.312 vs 0.298 s; PSS 1556 vs ~940 MB on 5 sites, not adblock); chrome views share one web process. Phase 9: LICENSE, .desktop, icons, -git/tag/-bin PKGBUILDs, release.sh, README install, `docs/user/` (13 guides, ported by a subagent). `--import novabrowser` built (it was claimed in 8.23 but missing). smoke.sh: fixed media bytes + 20 s waits (two timing flakes under load). CEF repo: aria2 on nova-core (A4 reaches CEF), C6 log + changelog, `nova-core` branch fast-forwarded into `main` (650078a, then e68d7b3 pin 7c1557b; not pushed). Doc statements the porting subagent could not verify (left hedged or marked [m]): history/scroll after restore, `tab-give` without reload, private-window mark, inspector window, `hardware_acceleration`/fonts new tabs only, spell-check dictionaries, `colors.contextmenu.*` on the GTK menu, HiDPI, IME, `.torrent` via aria2. Next: user manual pass; publish on go.

- 2026-10-29 (IST) — Phases 8 and 10 checked: `tests/features.sh` (39) and `tests/passwords.sh` (17, `pass` + local HTTPS) added, helpers in `tests/lib.sh`; IPC `key` request (`--key`). Fixed: aria2 RPC secret off argv (nova-core `aec30e5`, A4; CEF still has its own `aria2.rs`), password list suppressed on a new page right after a fill. Dark mode: WebKit colour-filter patch, `webkitgtk-6.0-nova` 2.54.2-2 installed.
- 2026-10-29 (IST, 01:30) — The WebKit edition runs: `src/` (app, windows/tabs, modes, commands, hints, permissions, downloads + aria2, passwords UI, adblock, dark mode, zoom, IPC) + `ext/` (web process extension). nova-core gained hints, spawn, aria2, passwords, net, GDK keys (commit `5ab8102`, local; not pushed). `tests/smoke.sh` 42/42 on debug + release. CEF `makepkg` from branch `nova-core` OK. Gotcha: the webkit-web-process-extension6 binding asserts on the NULL `redirected_response` of `send-request`; `ext` connects the signal by hand. Next: user manual pass (`docs/manual-test.md`).
- 2026-10-29 (IST) — W10 go (user sign-ins). HDFC "Upgrade browser" modal only with the stock UA → first evidence for D8. Next: Phase C.

- 2026-10-29 (IST, 00:15) — D9 added (no DRM in v1). W10 part 1: 18 sites × {safari, webkit} load normally, no unsupported-browser pages. Next: user sign-ins in the spike with `--safari`.

- 2026-10-28 (IST, 23:55) — Safari identity spike (`w9-baseline --safari --probe`): httpbin and page JS both report `Macintosh; Intel Mac OS X 10_15_7 … Version/27.0 Safari/605.1.15`, `platform=MacIntel`, `vendor=Apple Computer, Inc.`, no `userAgentData` (like real Safari). Speedometer default 26.7/26.7 vs safari 26.6/26.8. Decision D8 added.

- 2026-10-28 (IST, 23:40) — Decisions D1–D7 taken; PLAN.md rev 2 (nova-core Phase C, Phase W, spikes W12–W14, rules); PROGRESS.md rebuilt as the full feature checklist from the CEF edition. Next: W10.
- 2026-10-28 (IST, 23:10) — Installed `webkitgtk-6.0-nova` vs CEF novabrowser-bin 0.1.8: 27.25 vs 26.3 (`ab-cef.jsonl`, 4 interleaved rounds, outside the tool sandbox). Earlier in-sandbox numbers ~15% low.
- 2026-10-28 (IST, 21:00) — Env knob A/B: no gain. Own build on novahome (clang 23, v3, PGO from Speedometer, ThinLTO, 20 GB cap, peak 12.5–13.2 GB, ~45 min): +26% over Arch's build in the same setup.
- 2026-10-28 (IST, 18:15) — Spike `spikes/w9-baseline` (gtk4 0.11 + `v4_18`, webkit6 0.6 + `v2_52`), Speedometer runner via `evaluate_javascript`.
- 2026-10-28 (IST) — Read the CEF edition, checked the WebKitGTK 6.0 API, wrote README, PLAN, docs/feature-map, docs/porting.
