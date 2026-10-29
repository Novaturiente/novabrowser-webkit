# Manual test pass (WebKit edition)

Everything here is built but needs a person: looks, real keyboards, real sites,
media, and anything that would touch your own clipboard or accounts. Items match
the `[m]` marks in `PROGRESS.md`. The automated part is `tests/smoke.sh`.

## Setup

```sh
cd ~/personal/novabrowser-webkit
cargo build --release --workspace     # --workspace also builds ext/ (the adblock extension)
target/release/novabrowser-webkit --profile=test   # own data dir; drop the flag for your real profile
```

The extension `libnovabrowser_webkit_ext.so` must sit next to the binary
(`target/release/`), or set `NOVA_WEBKIT_EXT_DIR`. Without it a warning goes to
stderr and network adblock is off.

Write down anything that fails as: item number, what you did, what happened.

## Phase 1 — window and tabs

- 1.1 Resize, maximise, close the window with the WM.
- 1.3 Open two tabs; in the hidden one, a page polling `document.visibilityState` should say `hidden`.
- 1.5 Tab bar: favicons, active tab, loading line, crashed and pinned markers, asleep (`:tab-hibernate`) dimmed. Click switches, middle-click closes.
- 1.6 `:set tabs.position left` / `right` / `bottom` / `top`, `tabs.width 20%`, `tabs.show multiple`; resize the window.
- 1.7 `:set window.title_format '{perc}{current_title}'`; with `--profile=test` the title starts `[test] `.
- 1.9 Favicons appear after load.
- 1.10 A `target=_blank` link and a `window.open` button open as new tabs (login popups like "Sign in with Google" must still work).
- 1.11 Middle-click and Ctrl-click a link: background tab.
- 1.12 A page calling `window.open` on load (no click): "Blocked a popup…" message.
- 1.15 A page running `while(1){}`: after a while "This page isn't responding. Kill it?"; `y` shows the crash page, `r` reloads.
- 1.16 Play a YouTube video, `d`: audio stops at once. A page with an unsaved-changes guard asks "Leave this page?".
- 1.19 https://httpbin.org/user-agent shows the Safari string; `:set -u '*://httpbin.org/*' content.user_agent webkit` then reload shows the stock one.

## Phase 2 — keys and commands

- 2.2 Mode badge in the tab bar for insert / passthrough / hint / prompt.
- 2.3 `g` then wait: pending keys shown; Escape clears; `5j` scrolls 5 steps.
- 2.4 In normal mode, unbound letters do nothing on the page; `Ctrl-l` etc. still reach it.
- 2.5 In a text field type `jk`: nothing triggers. `Ctrl-v` passthrough, `Shift-Escape` leaves.
- 2.6 Click into a text field: insert mode; click outside: normal. Also inside an iframe (e.g. a comment box).
- 2.8 Press `g`, `;`, `<Space>`: which-key popup after ~350 ms.
- 2.9 `F1` and `:bindings tab`.
- 2.10 `:open {clipboard}` and `{primary}` with something copied.
- 2.11 Messages disappear after `messages.timeout`.
- 2.12 `O`, `o g rust`, `:open -b`, `:open -p`, `url.start_pages`.
- 2.14 `r`, `R`, `:stop` on a slow page.
- 2.15 `j k h l gg G`, `Ctrl-d/u`, `50%`; inside a scrollable div (GitHub file view, Gmail).
- 2.16 `+ - =`; zoom on one site, open it again after restart: same zoom; other sites 100%.
- 2.17 `J K gt gT`, `tab-focus last`, `tab-move +1`, `tab-only`, `tab-clone`, `tab-mute`, `3d`.
- 2.18 `:cmd-set-text -s :open`, `Ctrl-Shift-v` paste in normal and insert.
- 2.19 `:set tabs.last_close close` then close the last tab: window closes.
- 2.20 Pin tabs: they go first, short titles, focused pinned tab shows the full title, `tab-only` keeps them.
- 2.21 `NOVA_KEY_TIMING=1`: no key over 2 ms in the log. Browse Hacker News without the mouse.

## Phase 3 — palette, completion, history

- 3.1 `o`, `O`, `:`: placeholders and hidden prefixes; empty Enter.
- 3.2 Blurred page behind the palette; `/` is a thin strip.
- 3.3 Readline keys Ctrl-a/e/b/f/u/k/w/y/h, Alt-b/f/d.
- 3.4 Tab / Shift-Tab; Up/Down; Ctrl-p/n.
- 3.5 / 3.6 Completion order and ranking with your real history (`--import` first).
- 3.10 `Sh`: search, delete a row, "Delete shown".

## Phase 4 — hints, find, yank, clipboard

- 4.2 Hints inside same-origin iframes, open shadow roots, an open `<dialog>`.
- 4.3 `hints.chars`, `hints.uppercase`, colours and fonts settings.
- 4.4 Type labels, Backspace, `f` on a single match follows; `;r` rapid; `gi` focuses the first input.
- 4.5 / 4.6 `f`, `F`, `;y`, `;o`, `;h` (hover menus), `hint links spawn mpv {hint-url}` on GitHub, Jira, YouTube. Note sites where a hinted click does nothing (synthetic events, W5).
- 4.7 `/text`, `n`, `N`, `?text`; smart case.
- 4.8 `yy`, `yt`, `yd`, `;y` then paste somewhere.
- 4.9 `pp`, `Pp`; copy a 5 MB text and `pp`: no hang.

## Phase 5 — config

- 5.2 Break `config.ts` and start: defaults + error message naming the line.
- 5.3 `novabrowser-webkit --check-config`.
- 5.4 A setting set in both config.ts and `:set`: config.ts wins.
- 5.7 / 5.8 Each theme (`:set theme …`): tab bar, palette, internal pages; web fonts settings.
- 5.9 `Ss`: 17 rows, config.ts rows locked.
- 5.10 Copy your CEF-edition `config.ts` (minus `chromium.*`) and start.

## Phase 6 — external tools

- 6.1 `:spawn false`: error with exit code.
- 6.5 `:fake-key abc` in a text field; `:fake-key <Tab>`; `:fake-key <Return>` submits a search form.
- 6.6 `:insert-text hello` in a text field.
- 6.8 `novabrowser-webkit` with no URL while running: new window. `--cmd` with nothing running: exit 1.

## Phase 7 — privacy, prompts

- 7.4 / 7.5 / 7.6 https://adblock-tester.com, three YouTube videos (no ads), a news site (no empty ad boxes).
- 7.7 / 7.9 Open `http://` of a site with https: lands on https. A site without https: the "https failed … :https-toggle" message, `<Space>th` loads http.
- 7.10 `:set content.cookies.accept no-3rdparty`: embedded third-party logins (e.g. Disqus) break; sign-ins on the site itself work.
- 7.11 `:set content.headers.custom '{"Sec-GPC": "1"}'` then https://httpbin.org/headers.
- 7.12 Autoplay off (`content.autoplay false`): YouTube waits for a click.
- 7.15 Notifications (any site's "allow notifications"), camera + mic (https://webcamtests.com), screen share (Meet), clipboard read, pointer lock (a game), storage access (embedded login).
- 7.16 Answer `A` then reload: no second question; check `permissions.json`.
- 7.17 confirm(), prompt() (`y` returns the default), beforeunload on reload and close.
- 7.18 https://expired.badssl.com and https://self-signed.badssl.com: prompt; `y` loads it.
- 7.19 `<Space>td` on a white site: dark; images look normal; internal pages unchanged. On a light site with a dark header or dark code blocks (GitHub with a light system theme, MDN, a blog): those parts stay dark. A site already dark (GitHub with a dark system theme) is left alone. Look at the sites you use most.
- 7.20 `:set spellcheck.languages '["en_US"]'`: misspellings underlined in a text field.

## Phase 8 — sessions and daily driving

- 8.2 Restart with a tab that was scrolled down and has back history: same scroll, `H` goes back.
- 8.6 `:set tabs.hibernate_after 1`, wait 2 minutes: background tabs dim in the tab bar; a tab playing audio or with typed text stays awake.
- 8.8 `gD` on a playing video: it keeps playing in the new window (no reload).
- 8.9 Hover a tab: tooltip after a moment. Right-click a tab: menu; try each entry; j/k/Enter/Esc.
- 8.10 Drag a tab; drag a pinned tab: it stays in the pinned group.
- 8.11 Play audio in a background tab: indicator; `Alt-m` mutes.
- 8.13 `Sb` (quickmarks and bookmarks page): edit a name, delete an entry, a bad URL is refused.
- 8.14 / 8.15 Download a big file: pill `↓ 1 · P%`, finished message; `:download-open 1`; `:download-cancel` on a running one.
- 8.16 With `downloads.backend aria2`: a magnet link, quit mid-download and restart (it resumes), cancel (partial file removed).
- 8.17 Right-click a link, an image, a video, selected text: each menu entry.
- 8.18 `<F12>` (devtools): Web Inspector in its own window.
- 8.19 Fullscreen a YouTube video: it fills the window, not the screen.
- 8.20 `:profile-new work`: new window with a `[work]` pill; logins there are separate.
- 8.21 `:open -p`: window marked private; after closing it, nothing in history.
- 8.23 `--import` alone lists your browsers; import one; run it again: no duplicates.

## Phase 10 — passwords (with your real vault)

- 10.1 Default backend rbw: locked vault shows "rbw is locked", picking it unlocks. TOTP: `:password-fill --otp` on a 2FA page.
- 10.3 Click a row in the suggestion list.
- 10.6 Sign in to a site not in the vault: save prompt; `y`; check the vault. Change a password: update prompt.
- 10.7 rbw edit opens through novabrowser as `$VISUAL`.
- 10.8 A login inside an iframe gets nothing (WebKit edition fills the top frame only).

## Spikes needing a person

- W1 HiDPI scaling, clipboard, `<select>` popups, IME (fcitx/ibus), video with VA-API (`vainfo`, then a 4K YouTube video and CPU use), H.264/AAC files, a WebRTC call.
- W7 adblock memory: RSS of each `WebKitWebProcess` with and without the extension (`NOVA_WEBKIT_EXT_DIR=/nonexistent`).
