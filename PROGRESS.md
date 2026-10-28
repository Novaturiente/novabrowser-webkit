# novabrowser-webkit — progress

Single source of truth for where the work stands (rules in `AGENTS.md`).
Every feature of the CEF edition (novabrowser 0.1.8) is listed below with its
pass condition. An item is `[x]` only when that condition was checked in a
session; the evidence goes on the item.

Status: `[ ]` todo · `[~]` partial · `[x]` done · `[-]` dropped (reason given).
"CEF ref" = where the CEF edition does it (`../novabrowser/novabrowser/src/`).

## Now

- **Phase:** C (nova-core extraction)
- **Next action:** C1 — map module dependencies in the CEF edition, create
  `../novabrowser/nova-core`, move the pure modules (C2).
- **Blockers:** none.

---

## Phase 0 — spikes (PLAN.md §6)

- [x] W9 Speedometer 3.1 — installed `webkitgtk-6.0-nova` 27.25 vs CEF 0.1.8 26.3 (2026-10-28, `spikes/w9-baseline/results/ab-cef.jsonl`); Arch build 18.95 vs own PGO 23.95 in the tool sandbox (`ab-build.jsonl`); env knobs no gain (`ab-env.jsonl`).
- [~] Own WebKitGTK build — PGO + v3 + ThinLTO, `webkitgtk-6.0-nova` 2.54.2-1 installed on novarch (`spikes/wk-build/`). Left: video/site check (W1, W10), introspection decision (Phase W).
- [x] W10 sign-in + site compat — **go** (user, 2026-10-29: Google sign-in and ChatGPT "Sign in with Google" work in the spike with `--safari`). HDFC NetBanking shows an "Upgrade browser … older browser version" modal with the stock WebKit UA and **none with the Safari identity** (`results/w10/hdfc-{webkit,safari}.png`; found by the user, missed by the text check, which had no "upgrade browser" pattern and only 600 chars). Screenshot diff safari vs stock: only hdfc (59.5%) is UA-dependent; reddit/youtube/maps/whatsapp diffs are live content. Not tried: Microsoft/Teams, WhatsApp link, Discord, bank login, Meet call — checked again in Phase 8 daily driving. Part 1 done 2026-10-29 (`spikes/w9-baseline/compat.sh`, `results/w10/compat.jsonl` + PNGs, ephemeral session, Safari and stock UA): Google sign-in, Gmail, YouTube, Maps, Meet, ChatGPT, Microsoft login, Outlook, Teams, WhatsApp Web (QR shown, "Download for Mac"), Discord, GitHub, Slack, Figma, HDFC, SBI YONO, Reddit (JS challenge passed), X all load their normal pages; no "unsupported browser" text on any; identical for both UAs. YouTube video loaded but didn't autoplay (no gesture; playback is W1). Left (needs user): real sign-ins — Google, ChatGPT via Google, Microsoft/Teams, WhatsApp QR link, Discord, a bank; Meet call.
- [ ] W1 Wayland + media (HiDPI, clipboard, `<select>`, IME, VA-API, H.264/AAC, WebRTC)
- [ ] W2 layout + transparent chrome views + focus hand-off
- [ ] W3 key capture
- [ ] W4 world `novabrowser` bridge, all frames
- [ ] W5 input without CDP (site list)
- [ ] W6 frame-targeted reply
- [ ] W7 adblock engine in extension (score, YouTube, RSS per process)
- [ ] W8 session state restore
- [ ] W11 permissions, TLS errors, JS dialogs
- [ ] W12 headless PGO training on novahome
- [ ] W13 Dark Reader overhead ≤ 3%
- [ ] W14 real-site perf: scroll, idle RSS, startup vs CEF

## Phase C — nova-core (PLAN.md §5)

- [ ] C1 repo `../novabrowser/nova-core` + GitHub `Novaturiente/nova-core`, crate builds
- [ ] C2 pure modules moved (urlmatch, keys, config + config.js, settings core, marks, import, theme, history, session format, password backends, net pure, downloads pure, aria2 RPC, spawn, hint labels, command parser, permissions logic, adblock lists/engine build, ipc server, profiles, welcome, settings page, write_atomic, dirs) — all 92 CEF tests pass in nova-core
- [ ] C3 `Host` seam trait; modules with UI calls moved
- [ ] C4 CEF edition on nova-core: `cargo test`, `tests/smoke.sh`, `makepkg` of `packaging/PKGBUILD` and `release.sh` dry run all pass
- [ ] C5 audit fixes landed in nova-core: A1 hint chars < 2 loop, A2 QuickJS limits, A3 prompt grace period, A4 aria2 secret/port race/cookie redirects, A5 hint `{hint-url}` injection, A6 torrent cancel path, A9 history rollback, A10 adblock offline cache + curl flags, B5 history churn, B6/B7 per-request costs, C2 marks wipe, C3 `tab-close` count, C4 small items
- [ ] C6 CEF edition session log + release note for the refactor

## Phase 1 — Skeleton

- [ ] 1.1 GtkApplication `novabrowser-webkit`, Wayland app id + X11 WM_CLASS, window 1200×800, closes/resizes/maximises — CEF ref `window.rs`
- [ ] 1.2 `novabrowser-webkit <url>`: bare host → `https://`; `://`, `about:`, `data:` pass through — `main.rs start_url`
- [ ] 1.3 Tabs in a GtkStack, one WebView per loaded tab; switching keeps page state; hidden tab reports `visibilityState=hidden`
- [ ] 1.4 Chrome views on `nova://ui/*` (tab bar first), transparent, own WebContext; message handler `nova` only there; `ready` handshake
- [ ] 1.5 Tab bar: label, favicon, active, loading, 2 px progress line, crashed, pinned, asleep markers; click switches, middle-click closes
- [ ] 1.6 `tabs.position` top/bottom/left/right, `tabs.width` px or %, `tabs.show` never/always/multiple, re-layout on resize
- [ ] 1.7 Window title from `window.title_format` (`[profile] ` prefix)
- [ ] 1.8 URL, title, load progress, back/forward flags from WebView signals
- [ ] 1.9 Favicons (`notify::favicon`) in the tab bar
- [ ] 1.10 `target=_blank` / `window.open` → tab with opener kept (`create` + `related-view`); background disposition → background tab
- [ ] 1.11 Middle / Ctrl-click link → background tab (`decide-policy` NEW_WINDOW, mouse button)
- [ ] 1.12 Popups without a user gesture refused (CEF audit A7)
- [ ] 1.13 `nova://` scheme secure + local; page token checked on every action; only `nova://` views may act
- [ ] 1.14 Crash page on `web-process-terminated` (crashed / out of memory / killed) with URL, `r` reloads, other tabs keep running — check: `kill -9` the web process
- [ ] 1.15 Hang prompt "This page isn't responding. Kill it? [y]es [n]o" on `is-web-process-responsive` false → `terminate_web_process`; dropped on recovery
- [ ] 1.16 Tab close runs beforeunload (`try_close`); closed pages with unload handlers really stop (YouTube audio) — CEF Decisions 2026-10-11
- [ ] 1.17 Quit with every window closed exits 0, no stray web processes
- [ ] 1.18 Log file `$XDG_STATE_HOME/novabrowser-webkit/log`
- [ ] 1.19 Safari identity (D8): `content.user_agent` safari (default) / webkit / custom, per site; `navigator.platform` shim; check page + server see the same UA (httpbin) and W10 sites still sign in

## Phase 2 — Modal core

- [ ] 2.1 Capture-phase key controller; `key_from_gdk(keyval, state)` with tests replacing the `key_from_cef` cases
- [ ] 2.2 Modes normal / insert / passthrough / command / hint / prompt; mode badge in the tab bar
- [ ] 2.3 qutebrowser key notation (`<Ctrl-[>`, `<Space>tg`, `gT`), trie, pending keys shown, Escape clears, counts (`5j`, `3J`)
- [ ] 2.4 `forward_unbound_keys=auto`: unbound plain chars eaten in normal mode, other keys pass
- [ ] 2.5 Insert/passthrough check only single-key `mode-leave` binds; `Ctrl-v` passthrough, `Shift-Escape` leaves
- [ ] 2.6 Auto insert mode on focusing an editable (all frames); normal again on blur/navigation
- [ ] 2.7 Default keymap (`bindings.rs`, 78 keys); test: no duplicates, every command exists
- [ ] 2.8 Which-key popup 350 ms after a prefix (`g`, `;`, `<Space>`…), `colors.keyhint.*`, `fonts.keyhint`
- [ ] 2.9 `F1` / `:bindings [filter]` with "Your bindings" and per-mode sections
- [ ] 2.10 Command registry, flags, `;;` chaining, aliases expanded before the split, placeholders `{url}` `{url:host}` `{url:yank}` `{title}` `{clipboard}` `{primary}`
- [ ] 2.11 Unknown command → error in the tab bar; messages clear after `messages.timeout`
- [ ] 2.12 `open [-t|-b|-w|-p] [url]`, `url.auto_search`, `url.searchengines` (`DEFAULT`, keywords like `g rust`), `url.default_page`, `url.start_pages`
- [ ] 2.13 `back` / `forward` (+count) through the back/forward list, page can't trap them (CEF audit A8); "At beginning/end of history."
- [ ] 2.14 `reload [-f]`, `stop`
- [ ] 2.15 `scroll up|down|left|right|top|bottom` (40 px × count), `scroll-page x y`, `scroll-to-perc [N] [-x]`, `scrolling.smooth`; moves the active inner scroller
- [ ] 2.16 `zoom-in`, `zoom-out`, `zoom [N|N%]`, `zoom.levels`; per-site zoom kept across restarts (`zoom.json`)
- [ ] 2.17 `tab-next`, `tab-prev`, `tab-focus [last|N]` (negative = from end), `tab-move [+|-|N]`, `tab-close [-f]` with count, `tab-only`, `tab-clone [-b]`, `tab-pin`, `tab-mute`, `undo`
- [ ] 2.18 `mode-enter`, `mode-leave`, `cmd-set-text [-s] [-r] [-a]`, `message-info|warning|error`, `nop`, `paste` (`Ctrl-Shift-v` in normal + insert)
- [ ] 2.19 `tabs.last_close` close / blank / startpage / default-page / ignore
- [ ] 2.20 Pinned tabs first, `tabs.title.format_pinned`, focused pinned tab uses the normal format, `d` / middle-click refuse ("Tab is pinned!…"), `tab-only` keeps them
- [ ] 2.21 Key → command < 2 ms (timing log). Exit: user drives HN without the mouse

## Phase 3 — Palette, completion, history

- [ ] 3.1 Floating palette for `o` / `O` / `:` with hidden prefixes (`:open [-t|-b|-w] `, `:bindings `, `:profile-new `) and placeholders ("Search or enter address…", "Open in new tab…", …); empty-Enter rules as in the CEF edition
- [ ] 3.2 Blurred page backdrop behind the palette (`get_snapshot`); `/` and `?` as a thin strip so the page stays visible
- [ ] 3.3 Readline keys Ctrl-a/e/b/f/u/k/w/y/h, Alt-b/f/d; kills keep the leading `:`
- [ ] 3.4 Tab / Shift-Tab cycle; Up/Down move in the list when shown, else command history; Ctrl-p / Ctrl-n always history
- [ ] 3.5 Completion sources: commands, quickmarks, bookmarks, history, profiles, bindings; `completion.open_categories` order; `completion.height`
- [ ] 3.6 Ranking typed → visits → recency; `open` typed in the palette counts as typed; bookmarked URLs not repeated under History
- [ ] 3.7 History db `history.sqlite` (FTS5 trigram, `typed` column + migration, WAL) on its own thread; superseded queries skipped
- [ ] 3.8 `search_100k_rows_under_16ms` passes; live completion never > 16 ms on a 100k-row profile
- [ ] 3.9 Command history persisted (last 1000), prefix navigation
- [ ] 3.10 `:history [words]` / `Sh` on `nova://history`: newest 500 by day, search, delete row, "Delete shown"

## Phase 4 — Hints, find, yank, clipboard

- [ ] 4.1 `hint [-r] [-f] [group] [target] [args]`; groups from `hints.selectors`, default `all`; flags may follow the group
- [ ] 4.2 Collection: same-origin iframes, open shadow roots, only an open modal `<dialog>`, visibility rules from qutebrowser `webelem.js`
- [ ] 4.3 Labels `_hint_scattered`, `hints.chars`, `hints.uppercase`, `hints.min_chars`; closed shadow root, top-layer popover, `color-scheme: only light`; `fonts.hints`, `colors.hints.*`, `hints.border/radius/padding`
- [ ] 4.4 Typing filters, Backspace, unique prefix auto-follows, "No matching hint." leaves; `-r` rapid; `-f` / `gi` focus first input → insert mode
- [ ] 4.5 Targets normal/current, tab, tab-bg, tab-fg, window, hover, yank, yank-primary, fill, spawn (`{hint-url}`), run
- [ ] 4.6 Exit: `f`, `F`, `;y`, `;o`, `hint links spawn mpv {hint-url}` on GitHub, Jira, YouTube
- [ ] 4.7 Find `/` `?` incremental, smart case, `n` / `N`, match count in the tab bar
- [ ] 4.8 `yank url|pretty-url|title|domain|selection|inline <text> [-s]`, `yy` `yt` `yd`…, "Yanked …" messages
- [ ] 4.9 Clipboard + primary through GdkClipboard (no wl-copy/xclip); `pp` / `Pp`; reads capped 1 MiB / 2 s

## Phase 5 — Config, settings, themes

- [ ] 5.1 `~/.config/novabrowser-webkit/config.ts` → oxc → QuickJS; relative `.ts` imports; `c`, `bind`, `unbind`, `alias`, `pattern`, `readLines`, `configDir`, `profile`
- [ ] 5.2 Errors name the config.ts line; broken file at start → defaults + message; on `:config-source` → old config kept
- [ ] 5.3 `--check-config`: ok / error, keys ignored per site
- [ ] 5.4 `:set [-t] [-u pattern] name [value]`, `config-cycle`, `config-source`; `autoconfig.json`; precedence `:set -t` → config.ts → autoconfig → preset → default; per-profile autoconfig over the default profile's
- [ ] 5.5 Per-site `pattern()` (`get_for`); per-site list updated (cookies global only)
- [ ] 5.6 Schema: drop `chromium.*`, `content.canvas_reading`, `content.webrtc_ip_handling_policy`, `content.persistent_storage`, `content.register_protocol_handler`, `tabs.freeze_after`; add `webkit.hardware_acceleration`, `webkit.site_specific_quirks`, `webkit.features`, `webkit.itp`, `content.user_agent` (per site), `content.storage_access`
- [ ] 5.7 Themes dark / light / tokyo-night / catppuccin; every `colors.*` / `fonts.*` reaches the chrome views and `nova://` pages; test: every colour setting covered by every preset
- [ ] 5.8 Web content: `fonts.web.family.*`, `fonts.web.size.*`, `colors.webpage.bg`, `colors.webpage.preferred_color_scheme`
- [ ] 5.9 `:settings` / `Ss` on `nova://settings`: 17 rows, rows set in config.ts locked, theme warning
- [ ] 5.10 Exit: the user's config.ts (minus `chromium.*`) loads

## Phase 6 — External tools

- [ ] 6.1 `spawn [-u] [-o] cmd args`; errors show the last stderr line; stdout+stderr read together (no deadlock)
- [ ] 6.2 Userscripts from `<config>/userscripts/` then `<data>/userscripts/`; env `QUTE_USERSCRIPT`, `QUTE_MODE`, `QUTE_URL`, `QUTE_TITLE`, `QUTE_SELECTED_TEXT`, `QUTE_HTML`, `QUTE_TEXT`, `QUTE_FIFO`, `QUTE_CONFIG_DIR`, `QUTE_DATA_DIR`, `QUTE_DOWNLOAD_DIR`, `QUTE_COMMANDLINE_TEXT`, `QUTE_VERSION`, `QUTE_COUNT`
- [ ] 6.3 `QUTE_FIFO` lines run in the tab that started the script; tab gone → "userscript tab closed, dropped: …"; temp dir removed
- [ ] 6.4 `spawn -o` output in a text tab
- [ ] 6.5 `fake-key <keys>` (JS events + editing commands); limits from W5 written in docs
- [ ] 6.6 `insert-text <text>` (`InsertText` editing command)
- [ ] 6.7 IPC socket `<data>/ipc` (0600): `url <text>` never parsed as commands, `cmd <line>`, `query state` JSON; refused while quitting
- [ ] 6.8 `--cmd`, `--query [state]`; second launch hands its URL over; no URL → new window; no instance → exit 1
- [ ] 6.9 `dark-toggle` (`<Space>td`) writes `darkmode_excludes` atomically

## Phase 7 — Privacy, blocking, prompts

Adblock (D3, engine in the extension)
- [ ] 7.1 Extension `libnova_extension.so` loaded (`set_web_process_extensions_directory`), gets config + engine path over user messages
- [ ] 7.2 Network blocking in `send-request` (main frame never blocked), `content.blocking.enabled` per site, `content.blocking.method`, `content.blocking.whitelist`
- [ ] 7.3 Lists `content.blocking.adblock.lists`: background fetch (curl, size cap, `--proto-redir`), cache `engine.dat` + `lists.txt`, refresh > 24 h, hourly check, failed list keeps its old copy, offline doesn't mark the cache fresh (CEF audit A10), `:adblock-update`, "adblock: N lists" messages
- [ ] 7.4 Cosmetic per-site CSS + uBO scriptlets (`adblock-resources.json`) injected in `window-object-cleared` before page scripts, main world, CSP-exempt stylesheet
- [ ] 7.5 Generic hiding: `adblock-scan.js` in world `novabrowser`, class/id batches every 100 ms → `hidden_class_id_selectors`
- [ ] 7.6 Exit: adblock-tester.com 100/100; no ads on 3 YouTube videos; Speedometer cost of the extension ≤ 3%; RSS per web process recorded

HTTPS-only
- [ ] 7.7 Main-frame `http://` GET/HEAD → `https://`; subresources and redirect targets upgraded in the extension
- [ ] 7.8 Exempt: localhost, dotless hosts, `.local .lan .internal .home .arpa .ts.net`, RFC1918, 100.64/10, link-local, IPv6 loopback/ULA/link-local; `https_excludes` file; `https-toggle` (`<Space>th`)
- [ ] 7.9 Failed upgraded load → "<host>: https failed (why). HTTPS-only upgraded it; :https-toggle (<Space>th) allows http here"; WebKit `HTTPSByDefault` off so excludes stick

Cookies, headers, network privacy
- [ ] 7.10 `content.cookies.accept` all / no-3rdparty / never via CookieManager; ITP on (`webkit.itp`); per-site cookie patterns reported as ignored
- [ ] 7.11 `content.headers.custom` (e.g. `Sec-GPC`), `content.headers.do_not_track` in `send-request`
- [ ] 7.12 Accept-Language (`set_preferred_languages`), autoplay (`content.autoplay` → WebsitePolicies), hyperlink auditing off, DNS prefetch off
- [ ] 7.13 Idle network capture on `about:blank` shows no third-party hosts

Prompts and permissions
- [ ] 7.14 Tab-bar prompt `[y]es [n]o [A]lways [N]ever`, Escape = no, per-tab queue, prompt mode focuses the tab bar; grace period so typed keys can't answer (CEF audit A3)
- [ ] 7.15 Types: notifications, geolocation, mic, camera, screen share, clipboard read, pointer lock, storage access, device info (follows media answer); media-key-system + missing-plugins denied (message names `gst-libav`)
- [ ] 7.16 `content.*` settings decide true/false/ask per origin; saved A/N in `permissions.json` win over config.ts; notifications restored at start (`initialize_notification_permissions`)
- [ ] 7.17 JS alert / confirm / prompt (typed answer) / beforeunload (leave/reload) in the tab bar via `script-dialog`; dialogs dropped on navigation
- [ ] 7.18 TLS error → own `nova://` page; proceed = `allow_tls_certificate_for_host` + reload; subresource TLS errors fail

Appearance
- [ ] 7.19 Dark mode: Dark Reader engine in world `novabrowser` if W13 ≤ 3%, else color scheme + injected CSS; `colors.webpage.darkmode.enabled` per site; `darkmode_excludes`; internal pages and hint labels unaffected
- [ ] 7.20 Spellcheck (`spellcheck.languages`, Enchant dictionaries)

## Phase 8 — Sessions and daily driving

Sessions and tabs
- [ ] 8.1 Autosave (1 s debounce, atomic, only when changed) to `<data>/sessions/default.json`; restore at start, command-line URL added as a tab
- [ ] 8.2 Per tab: url, title, session-state blob (back/forward + scroll), pinned, keep-awake, icon; legacy url/back/fwd shape still loads
- [ ] 8.3 Lazy restore: background tabs get a WebView only on first show
- [ ] 8.4 `session-save [name]`, `session-load [-c] name`, `session-delete`, `quit -s`, `close -s`; names can't leave the dir
- [ ] 8.5 `undo` across restarts (last 100 closed) brings back history too
- [ ] 8.6 Hibernation `tabs.hibernate_after` (min, per site): save state, destroy the view, restore on show; skip active, loading, crashed, internal, playing media, typed forms, keep-awake; `tab-hibernate [n]`, `tab-keep-awake [n]`; dimmed in the tab bar
- [ ] 8.7 Multiple windows (`open -w`, `Ctrl-n`), per-window state, `:close` one keeps the rest, session restores all windows
- [ ] 8.8 `tab-give` (`gD`) moves the live WebView to a new window (no reload)
- [ ] 8.9 Tab tooltip (title + URL, 450 ms), right-click tab menu (Reload, Duplicate, Copy URL, Mute, Bookmark, Pin, Move to new window, Hibernate now, Keep awake, Close, Close others) with j/k/Enter/Esc
- [ ] 8.10 Drag a tab to reorder; pinned group kept
- [ ] 8.11 Audio indicator + `{audio}` in titles from `is-playing-audio` / mute

Marks
- [ ] 8.12 Quickmarks `<config>/quickmarks` and bookmarks `<config>/bookmarks/urls` (qutebrowser formats); `m`, `M`, `b`, `B`, `gb`, `gB`; `quickmark-save/add/load/del`, `bookmark-add [--toggle]/load/del/list [-j]`
- [ ] 8.13 Manager pages `Sb` / `Sq` on `nova://bookmarks` (edit, delete, validation)

Downloads
- [ ] 8.14 No dialog, `downloads.location.directory` (`~` expanded), sanitized names, unique `(1)` suffixes, started/finished/failed messages, pill `↓ N · P%`
- [ ] 8.15 `:downloads` / `Sd` page, `download [url]`, `download-open [N] [cmd]` (`downloads.open_dispatcher` / xdg-open), `download-cancel [N]`, `download-clear`, `downloads.json` (last 500)
- [ ] 8.16 aria2 backend (`downloads.backend` auto/aria2/chromium→webkit): RPC on 127.0.0.1 with secret not on argv (CEF audit A4), 8 connections, session resume, cookies incl. HttpOnly + UA + referer, magnet links (`decide-policy`), torrents as one entry, cancel removes partial files safely (A6), fallback to WebKit download

Other
- [ ] 8.17 Context menu: open link in tab/background/window, copy link, save link; image/media open/save/copy; search selection
- [ ] 8.18 `devtools` → Web Inspector (own window)
- [ ] 8.19 Page fullscreen fills the window only (`content.fullscreen.window`), `fullscreen [--leave]`
- [ ] 8.20 Profiles: `--profile=x`, `--profile x`, `-P x`, `--list-profiles`, `:profile-new`, `:profile-open`, own process + data dir, `[name]` pill
- [ ] 8.21 Private windows `open -p` on an ephemeral NetworkSession, marked in the tab bar
- [ ] 8.22 Welcome page once per profile (`<data>/welcomed`), `:help`, `--import` offers per browser found
- [ ] 8.23 `--import [name|dir|file]`: Chrome, Chromium, Helium, Brave, Vivaldi, Edge, Firefox, Zen, LibreWolf, Floorp, qutebrowser (history, bookmarks, quickmarks, session) + `novabrowser` (CEF edition: history, marks, sessions, permissions, downloads, config files, zoom); idempotent
- [ ] 8.24 Exit: one week as the only browser, problems logged here

## Phase 9 — Packaging and docs

- [ ] 9.1 Cargo workspace: app + `extension/` cdylib; `[profile.release]` with LTO + strip (CEF audit C1)
- [ ] 9.2 AUR `novabrowser-webkit` (source) + `novabrowser-webkit-bin`: depends `libwebkitgtk-6.0.so`, `gtk4`, `gst-plugins-good`; optdepends `gst-libav`, `gst-plugin-va`, `aria2`, `rbw`, `pass`, `bw`; extension under `/usr/lib/novabrowser-webkit/`
- [ ] 9.3 `.desktop`, icons, `xdg-settings` default-browser instructions
- [ ] 9.4 `tests/smoke.sh` over IPC (start, `open -t`, hand-over, tab-close, `:set` persisted, `quit -s` exits 0, session saved, no GLib CRITICAL)
- [ ] 9.5 Docs ported from `../novabrowser/novabrowser/docs/` (getting-started, keybindings, commands, configuration, settings, tabs-sessions-profiles, passwords, downloads, privacy, scripting, migrating, building, troubleshooting) with WebKit differences
- [ ] 9.6 README with install, keys, differences from the CEF edition

## Phase 10 — Passwords

- [ ] 10.1 `passwords.backend` rbw (default) / bw / pass / none; list, locked check, unlock, secret, TOTP, save, update; 60 s metadata cache
- [ ] 10.2 Field detection (`passwords.js` kinds user / password / new / otp) in world `novabrowser`, all frames; listeners armed only after a login field is focused
- [ ] 10.3 Suggestion list under the field (non-focusing popup view): Down/Up/Enter/Escape, click; rows for entries, "Generate a strong password", "<backend> is locked"
- [ ] 10.4 Fill only into the asking frame (reply object), same-site frames only, never on `http://`, never on load; `Sp` = `password-fill [--otp]` fills the focused field
- [ ] 10.5 Generated passwords (urandom, 20 chars) for sign-up fields
- [ ] 10.6 Save / update prompt after submit (`WebFormManager::will-submit-form` + JS fallback); split logins remember page-1 user; same password → no prompt; newer prompt replaces older; expiry 5 min
- [ ] 10.7 rbw add/edit through novabrowser as `$VISUAL` (secret in env, never argv); locked vault asks before unlocking
- [ ] 10.8 Exit: single-page, split, iframe, TOTP, sign-up pages; save + update land in the vault; cross-site iframe gets nothing

## Phase W — webkitgtk-6.0-nova distribution (D7)

- [ ] W.1 One-command pipeline on novahome under the 20 GB cap: fetch + verify (sha256 + PGP), instrumented build, headless training (W12), PGO build, package
- [ ] W.2 Public package includes GObject introspection (decide), post-install note: AVX2 only, how to go back
- [ ] W.3 GitHub release tarball + AUR `webkitgtk-6.0-nova-bin` (`provides`/`conflicts` `webkitgtk-6.0`)
- [ ] W.4 Update policy in place: rebuild within 3 days of a WebKitGTK stable release or WSA; version watch
- [ ] W.5 Release check: Speedometer ≥ previous build, smoke test, YouTube plays

---

## Decisions

Newest first. Settled — don't reopen without new evidence.

- 2026-10-29 — D9 (user): no DRM in the WebKit edition v1; DRM sites stay in the CEF edition.
- 2026-10-28 — D8 (user): appear as macOS Safari by default (UA + `navigator.platform`); `content.user_agent` keeps `webkit` and custom values, per site. Safari version token followed by hand (27.0 now).

- 2026-10-28 — D1–D7 (user; PLAN.md §2): GTK 4 + WebKitGTK 6.0; chrome in HTML/CSS (user wants full styling freedom, tab bar redesign later — assistant had recommended native GTK); adblock engine in the web-process extension (assistant had recommended content filters; RSS checked in W7); separate dirs + `--import novabrowser`; extract `nova-core` first (assistant had recommended copying first; risk: refactor of the shipping CEF edition); Dark Reader port if ≤ 3% Speedometer cost, else color scheme; ship `webkitgtk-6.0-nova` on the AUR (we own its security updates).
- 2026-10-28 — Go ahead with the build (user) after `webkitgtk-6.0-nova` matched CEF on Speedometer (27.25 vs 26.3). W10 (site compat) still gates it.
- 2026-10-28 — Benchmarks and GUI checks run outside the novacode tool sandbox (nested bwrap kills the web process; scores ~15% low inside).
- 2026-10-28 — novahome builds capped at 20 GB RAM (user: other services must keep running): `systemd-run --user --scope -p MemoryMax=20G`, `-j8`.
- 2026-10-28 — Project created as a separate repo next to `../novabrowser/novabrowser`; plan from novabrowser 0.1.8 (`9574fca`).

## Session log

Newest first.

- 2026-10-29 (IST) — W10 go (user sign-ins). HDFC "Upgrade browser" modal only with the stock UA → first evidence for D8. Next: Phase C.

- 2026-10-29 (IST, 00:15) — D9 added (no DRM in v1). W10 part 1: 18 sites × {safari, webkit} load normally, no unsupported-browser pages. Next: user sign-ins in the spike with `--safari`.

- 2026-10-28 (IST, 23:55) — Safari identity spike (`w9-baseline --safari --probe`): httpbin and page JS both report `Macintosh; Intel Mac OS X 10_15_7 … Version/27.0 Safari/605.1.15`, `platform=MacIntel`, `vendor=Apple Computer, Inc.`, no `userAgentData` (like real Safari). Speedometer default 26.7/26.7 vs safari 26.6/26.8. Decision D8 added.

- 2026-10-28 (IST, 23:40) — Decisions D1–D7 taken; PLAN.md rev 2 (nova-core Phase C, Phase W, spikes W12–W14, rules); PROGRESS.md rebuilt as the full feature checklist from the CEF edition. Next: W10.
- 2026-10-28 (IST, 23:10) — Installed `webkitgtk-6.0-nova` vs CEF novabrowser-bin 0.1.8: 27.25 vs 26.3 (`ab-cef.jsonl`, 4 interleaved rounds, outside the tool sandbox). Earlier in-sandbox numbers ~15% low.
- 2026-10-28 (IST, 21:00) — Env knob A/B: no gain. Own build on novahome (clang 23, v3, PGO from Speedometer, ThinLTO, 20 GB cap, peak 12.5–13.2 GB, ~45 min): +26% over Arch's build in the same setup.
- 2026-10-28 (IST, 18:15) — Spike `spikes/w9-baseline` (gtk4 0.11 + `v4_18`, webkit6 0.6 + `v2_52`), Speedometer runner via `evaluate_javascript`.
- 2026-10-28 (IST) — Read the CEF edition, checked the WebKitGTK 6.0 API, wrote README, PLAN, docs/feature-map, docs/porting.
