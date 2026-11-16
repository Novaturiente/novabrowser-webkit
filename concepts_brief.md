# novabrowser-webkit as a daily-driver browser

* **Author:** Novaturiente
* **Date:** 2026-11-16

## Overview

The goal is to make novabrowser-webkit (a keyboard-driven browser in Rust on WebKitGTK 6.0 and GTK 4) the author's only browser in place of qutebrowser, and later a browser that AUR users can install. Flatpak comes only after the author has tested it for public use.

The author left qutebrowser because of crashes, slowdowns and memory leaks in QtWebEngine. This browser must therefore be stable, work with the sites the author needs, and be as hardened as possible without breaking logins.

**Success:** one week as the only browser, with no fallback to another browser. DRM sites (Netflix, Prime Video and similar) are out of scope for this edition and do not count against the week.

**Fixed decisions (made by the author, 2026-11-16):**

1. DRM is out of scope for the WebKit edition.
2. The custom WebKit build (`webkitgtk-6.0-nova`) moves to a private prefix under `/opt` and stops replacing the system `webkitgtk-6.0`.
3. The memory-kill setup stays as it is: WebKit's 3072 MB per-process limit (`src/web.rs:93-94`) and the watchdog (`src/window.rs:1851`, `1858`). Crashes are rare with the current setup.
4. A hardening feature is on by default only if a fixed list of login and call sites still works with it. If it breaks one, it ships off, or on only for the sites where it is safe. (Reason: custom headers once broke CORS preflight requests and with them the DataForSEO login.)
5. Two builds are supported: one on Arch's stock `webkitgtk-6.0`, one on `webkitgtk-6.0-nova`. The author keeps maintaining the nova build.

**Method:** six independent reviewers (design, codebase details, zoom-out, comprehension, implementation readiness, structural) read this brief's first draft and the code with no shared context. Their findings were merged below. Facts marked "checked 2026-11-16" were checked on novarch in this session; everything else is from reading the code.

## Analysis

### What is true today

- **Calls.** novarch runs `webkitgtk-6.0-nova-bin 2.54.2-4`, which links `libgstwebrtc-1.0` (checked 2026-11-16). `src/web.rs:234-235` turns on WebRTC and media streams. No real Meet, Teams or Zoom call has been made with it. `gst-plugin-pipewire` and `xdg-desktop-portal-hyprland` are installed on novarch, but neither package lists them as optional dependencies.
- **Screen sharing.** Nothing in `src/` handles a screen-capture request on purpose. `src/permissions.rs:136-147` only checks `is_for_audio_device` and `is_for_video_device`. A request with neither becomes "capture your screen" by accident. A request for screen plus audio is shown as "use your microphone" and saved under the microphone setting. **This is a security gap:** a site that was once allowed the microphone can capture the screen without asking.
- **Stock Arch WebKit.** It is not known whether Arch's `webkitgtk-6.0` is built with WebRTC. The nova package replaces it on novarch, so this could not be checked here. The answer decides whether the stock build can ever pass "Zoom works".
- **Packaging.** `packaging/aur/webkitgtk-6.0-nova-bin/PKGBUILD:26-27` has `provides=("webkitgtk-6.0=$pkgver" … libwebkitgtk-6.0.so)` and `conflicts=(webkitgtk-6.0 …)`, so it replaces the system library for every WebKitGTK app. The browser package depends on the bare `libwebkitgtk-6.0.so` (`packaging/aur/novabrowser-webkit-bin/PKGBUILD:10`). `spikes/wk-build/build.sh:21` builds with `prefix=/usr` and `CMAKE_SKIP_RPATH=ON`. `build.sh:20-21` hard-codes `$HOME/wk-build/wk.profdata`, which is gone from novahome.
- **Extension.** The adblock / HTTPS-only / memory-report extension is looked up in `$NOVA_WEBKIT_EXT_DIR`, next to the binary, then `/usr/lib/novabrowser-webkit` (`src/web.rs:144-151`). If it is missing, the browser only prints to stderr (`src/web.rs:109`). Then the memory watchdog also stops working without a word, because `memory_watch` returns early on every empty reply (`src/window.rs:1931`).
- **Records.** Crashes, kills, hangs and load errors are shown on screen and printed to stderr only. The crash page keeps a reason word and the URL (`src/tabview.rs:119-127`, `src/window.rs:825-833`); load errors keep the code on screen only (`src/window.rs:859-874`). There is no time, no web-process PID and no memory figure at death. The extension's `mem` reply carries `(rss, gpu)` but not the PID (`ext/src/lib.rs:112`). Refused permission requests leave no trace (`src/permissions.rs:133`).
- **Known bugs.** 8.27: a tab switch forces Normal mode unless the mode is Command (`src/window.rs:643-646`), so an open y/n prompt stops getting keys. 8.25: the focus-in stutter fix waits for the author's confirmation. "Pages don't load until restart" has never been caught live.
- **Hardening.** `set_enable_developer_extras(true)` is set in two places (`src/web.rs:226` and `src/web.rs:324`). ITP is already on by default (`docs/porting.md:101`), and cookie policy already has settings (`src/web.rs:166-178`).
- **Releases.** Tags 0.1.15 to 0.1.18 shipped while some of `tests/smoke.sh`, `tests/features.sh` and `tests/passwords.sh` were not run.

### What the `/opt` move really involves

The `/opt` move is decided, but it is not just a packaging step. It touches the build, library loading, the sandbox, the extension and existing users.

- **Build.** WebKit compiles the paths to its helper processes (`WebKitWebProcess`, `WebKitNetworkProcess`) in from the install prefix. So the nova build needs `CMAKE_INSTALL_PREFIX=/opt/<name>` and an install rpath on the library and helpers. It is not something that can be repackaged later. Do it inside the pipeline rebuild, or the PGO pipeline is rebuilt and trained twice.
- **Library loading.** Both libraries have the same soname, `libwebkitgtk-6.0.so.4`. Use a `DT_RUNPATH` pointing at `/opt/<name>/lib` in the browser binary, not a launcher with `LD_LIBRARY_PATH`. A launcher variable leaks into every program the browser starts (aria2, xdg-open, rbw, pass, editors, userscripts). [Likely] With `DT_RUNPATH`, the loader tries `/opt` first and falls back to `/usr/lib` when `/opt` is absent. If so, one browser binary can serve both builds, and the capability report (below) tells the user which one loaded. Verify this before choosing between one and two browser packages.
- **Sandbox.** It is not known whether WebKit's bubblewrap launcher mounts `/opt` into web processes. Test with the sandbox on, never with `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS`. If it is not mounted, the browser already uses `add_path_to_sandbox` for the adblock dir (`src/web.rs:113`), and the same call can add the prefix.
- **Extension.** `libnovabrowser_webkit_ext.so` is built against one WebKit's headers (`ext/Cargo.toml:13`, feature `v2_52`) but loads into whichever library the web process has. Each build needs its own extension, or a check that the extension and the running library match.
- **Existing users.** Dropping `provides`/`conflicts` from the nova package means `pacman -Syu` fails with "breaks dependency" until stock `webkitgtk-6.0` is installed first, because other WebKitGTK apps relied on those provides. The release needs a stated install order, an `.install` message, and a rollback note: going back to `2.54.2-4` brings the conflict back and removes stock again.
- **Shared profile.** Both builds use the same data dir (`NetworkSession::new(data, cache)`, `src/web.rs:83`). Switching from a newer to an older WebKit can downgrade IndexedDB, CacheStorage, ITP and cookie data. Record the WebKit version in the profile and warn (or back up the profile) on a downgrade.

### Calls are the go/no-go question

Zoom is required for the author's work. If Zoom web fails, the week fails on day one, and no other item matters. [Guessing] Zoom's web client relies on WebRTC, WebAssembly SIMD, `SharedArrayBuffer` and WebCodecs, and checks the browser. The Safari identity (`src/web.rs:24`) may not be enough, and a per-site user agent may be needed for Zoom and Teams. A failed call is useless without its cause, so call tests must run with `GST_DEBUG=webrtc*:4,pipewire*:4` written to a file and the portal journal (`journalctl --user -u xdg-desktop-portal-hyprland`) saved.

### Stability needs evidence before more fixes

The week can only be judged if crashes, kills and hangs are recorded. The event log therefore comes before the call work, not after. It must also cover the browser's own UI process: one UI crash loses every tab. Autosave already runs with a 1 s debounce (8.1), but "`kill -9` of the UI process restores all tabs" has not been tested.

The "pages don't load" detector should only detect and log. [Guessing] WebKitGTK 6 has no public API that exposes or restarts the network process, so "offer to restart the network process" stays out until such an API is found.

### Hardening under the site gate

The site gate must catch the bug that created the rule. A text scan for "upgrade browser" and feature probes would not have caught the DataForSEO CORS-preflight failure. So the gate also needs a local test server that sends `Access-Control-*` headers and credentialed requests. Gate runs use throwaway profiles, because turning on ITP or blocking third-party cookies deletes or splits stored site data, and turning it off again does not bring logins back. Test hardening candidates in batches, and narrow down only when a batch fails. The settings schema is shared with the CEF edition (`settings::other_edition`), so new per-site keys affect both editions.

### Ranked roadmap

Effort: **S** = under a day, **M** = 1-3 days, **L** = more than 3 days.

0. **Go/no-go spike: Zoom call and stock WebRTC check (S).**
   - *What:* one real Zoom web call on the installed nova 2.54.2-4 with camera, mic and screen share on Hyprland, with GStreamer and portal logs saved. Then Meet and Teams the same way. Check whether stock Arch `webkitgtk-6.0` has WebRTC (`ldd /usr/lib/libwebkitgtk-6.0.so.4 | grep gstwebrtc` on a stock install, e.g. a container).
   - *Why:* it is the cheapest test that can end the plan, and the stock answer decides what the stock build can promise.
   - *Done when:* each of Zoom, Meet, Teams has a written pass/fail with logs, and a named fallback (per-site user agent, or a missing WebKit feature to build) for each failure.
   - *Zoom: try every option before giving up (author, 2026-11-16).* Record each attempt and its result:
     1. Every entry point: `app.zoom.us` (web app), the "join from your browser" link (`zoom.us/wc/join/<id>`), and a meeting started from a Zoom link in Calendar or Gmail.
     2. Per-site user agent for `*.zoom.us`: the current Safari identity, a recent Chrome on Linux, and a recent Firefox.
     3. WebKit runtime features from `Settings::all_features()`: list every feature that is off by default and related to media, WebCodecs, WebRTC encoders, `SharedArrayBuffer`, WebAssembly or screen capture; turn them on for Zoom one at a time.
     4. Cross-origin isolation: check in the console whether `crossOriginIsolated` and `SharedArrayBuffer` exist on the Zoom page (Zoom's web client may need them for its WebAssembly media path).
     5. Codecs: check which GStreamer encoders and decoders WebRTC finds (H.264 via `gst-plugins-ugly` / `gst-libav` / VA-API, VP8/VP9 via `gst-plugins-good`, Opus), and install the missing ones.
     6. The WebKit build: if a needed feature is compiled out (for example WebCodecs or a WebRTC option), add it to the nova build flags and rebuild.
     7. Only after all six fail: Zoom stays in another browser. That is accepted as the one exception to the one-week test (author, 2026-11-16).

1. **Event log and extension health check (S-M).**
   - *What:* append every crash, out-of-memory or watchdog kill, hang prompt, load error and permission decision to `~/.local/state/novabrowser-webkit/events.log`. Each line has a format version, time, profile, tab id, URL host only (not the full URL), WebKit termination reason, last memory sample and web-process PID. Add the PID to the extension's `mem` reply (`ext/src/lib.rs:112`). At startup the extension answers a `hello` with its version, WebKit version and PID; a missing extension or a missing `mem` reply is an event, not silence. Rotate the file. A `:report` command prints recent events.
   - *Why:* the week, the call spike and every bug fix need evidence, and today a missing extension disables adblock and the watchdog without a word.
   - *Done when:* `kill -9` of a web process, a forced hang and a missing extension each produce one line; a unit test parses the format.

2. **Capability report (S).**
   - *What:* one module that knows the loaded library path (from `/proc/self/maps`), WebKit version, stock or nova, extension ok or missing, and whether WebRTC, screen capture and passkeys exist. `:version` shows it. Pages show "This WebKit build has no WebRTC" when `RTCPeerConnection` is missing.
   - *Why:* items 0, 3, 4 and 6 all need this, and users can't tell which build they run.
   - *Done when:* `:version` is correct on both builds.

3. **Screen sharing done on purpose (S-M).**
   - *What:* use `is_for_display_device` in `src/permissions.rs:134-147`. Add a separate screen-capture permission key that always prompts and never inherits the microphone answer. Turn on the screen-capture runtime feature if WebKit needs it (check `Settings::all_features()` and its default). Add `gst-plugin-pipewire` and `xdg-desktop-portal` (plus a backend such as `xdg-desktop-portal-hyprland`) to the browser package's optional dependencies, since both builds need them. Make `libnice` and `gst-plugins-bad` hard dependencies of the nova package.
   - *Why:* calls need it, and today a screen-plus-audio request is treated as a microphone request.
   - *Done when:* the Hyprland share picker appears from Meet, Teams and Zoom; screen audio is shared; a site allowed the mic still gets a screen prompt (unit test on the permission mapping).

4. **Known stability bugs (S).**
   - *What:* fix 8.27 at `src/window.rs:643-646` (don't leave Prompt mode on a tab switch). Get the author's answer on 8.25. Test that `kill -9` of the UI process restores all tabs.
   - *Done when:* a feature test with a prompt open across a tab switch passes; the UI-kill test passes.

5. **Site gate list and harness (S-M).**
   - *What:* `tests/sites.sh` built on `tests/lib.sh` (throwaway profiles, IPC queries), no new harness. It scans the whole page text for "upgrade / unsupported browser" messages and probes `RTCPeerConnection`, `getDisplayMedia`, `SharedArrayBuffer`, `VideoEncoder`, `PublicKeyCredential`. It runs on both builds. Add a local CORS-preflight and credentialed-request test page. Keep sites as data with a "last passed" date per build; sign-ins and calls stay manual rows in `docs/manual-test.md`.
   - *Why:* it is the gate for every hardening default and for every release.
   - *Done when:* it runs green on nova and gives an honest result on stock.

6. **WebKit pipeline rebuild with the `/opt` prefix (M-L).**
   - *What:* a scripted fetch, verify, build, PGO training and package run on novahome under the 20 GB disk cap, with `CMAKE_INSTALL_PREFIX=/opt/<name>` and install rpaths. Keep `wk.profdata` and the debug symbols outside the build tree. Spike first: sandbox on, check `/proc/<web process>/maps` shows only `/opt` libraries and the extension resolves to the nova library. Browser binary gets `DT_RUNPATH`; decide one or two browser packages from the fallback test. Drop `provides`/`conflicts` from the nova package, with install order, `.install` message and rollback note. Add the profile WebKit-version guard. Add a release and advisory watch. 2.54.3 is due and drops patch 0000.
   - *Why:* the author's 3-day rebuild promise has no working pipeline, and the current package replaces the system library for AUR users.
   - *Done when:* a clean novahome run produces the `/opt` package; it installs next to stock `webkitgtk-6.0`; `tests/smoke.sh` passes on both; Speedometer is within noise of 2.54.2-4.

7. **Release gate (S).**
   - *What:* `packaging/release.sh` refuses to tag unless smoke, features, passwords and the automated part of the site gate pass.
   - *Done when:* a deliberately failing test stops a dry-run release.

8. **HDFC "upgrade browser" (S-M).**
   - *What:* find the feature check behind the modal (it shows with both user agents, so it is a missing feature, not the user agent).
   - *Done when:* the cause is named; fixed or listed as a known gap.

9. **Hardening candidates, one gate run each batch (M).**
   - *What:* turn off `developer_extras` (both `src/web.rs:226` and `:324`) and turn it on only when `devtools` is invoked; third-party cookie blocking; other candidates. Each batch runs through item 5 on a throwaway profile; release notes for any changed default.
   - *Done when:* each candidate is recorded as on, off or per-site, with its gate result.

### Deferred or cut

- **Cut:** any change to the memory-kill policy, including "one owner" and "hibernate instead of kill" (decision 3). Logging which kill fired is part of item 1.
- **Cut from this roadmap:** memory reduction toward PSS + GPU of 1.2 GB or less. Memory is not part of the success test and crashes are rare.
- **Deferred:** passwords in iframes, until a gate site fails because of it.
- **Deferred:** network-process restart, until a public API is found.
- **Deferred:** BOLT, Flatpak, WebExtensions.

### Open Risks

1. **Zoom may not work in WebKitGTK at all.** If every option in item 0 fails, Zoom stays in another browser; the author accepts that as the one exception to the one-week test (2026-11-16).
2. **The `/opt` move can break existing installs and the sandbox.** Dropping `provides` breaks upgrades for current users of the nova package, and the sandbox may not see `/opt`. Both need the spike in item 6 before release.
3. **The week has no evidence today.** Until item 1 lands, crashes, kills, hangs and silently missing extensions leave no record, so "a week with no problems" can't be shown.

## Goals vs. Non-Goals

* **In-Scope Goals:**
  - Zoom, Meet and Teams web calls with camera, mic and Wayland screen share.
  - Banking (HDFC), Google Docs, and logins that keep working under every hardening default.
  - A local event log and a capability report, so problems and build differences are visible.
  - Two builds (stock and nova in `/opt`) that install side by side with the system library.
  - A release gate that stops tags on failing tests.
* **Out-of-Scope Non-Goals:**
  - DRM / Widevine.
  - Zoom outside WebKit, but only after every option in item 0 has failed.
  - Changes to the memory-kill setup.
  - Memory reduction work.
  - WebExtensions, Flatpak, BOLT.
  - Network-process restart.

## Key Decisions & Staging for Next Steps

**Stage 0 (before the week):** items 0, 1, 2, 3, 4. Item 0 runs first; if Zoom fails with no fallback, stop and decide before doing the rest.

**Stage 1 (during the week, on nova 2.54.2-4):** item 5, then live use. The trial does not wait for the PGO rebuild or the `/opt` move (author, 2026-11-16). Keep the `2.54.2-4` package cached and back up the profile before any WebKit upgrade during the week.

**Stage 2 (before any AUR announcement):** items 6, 7, 8, 9.

**Still to verify (cheap checks):**
- Whether stock Arch `webkitgtk-6.0` has WebRTC.
- Whether WebKit's bubblewrap launcher mounts `/opt`.
- Whether `DT_RUNPATH` fallback lets one browser binary serve both builds.
- The name and default of the screen-capture runtime feature, and whether display requests report `is_for_video_device`.
- Whether any public WebKitGTK 6 API exposes the network process.

### Glossary

- **novarch / novahome:** the author's laptop and home build server.
- **nova build:** `webkitgtk-6.0-nova`, the author's WebKitGTK build with PGO and patches 0000-0003.
- **PGO:** profile-guided optimisation; the compiler uses a recorded run (`wk.profdata`) to optimise.
- **W-numbers (W6, W10, W16 …), D-numbers (D3, D9), 8.25, 8.27:** items and decisions in `PROGRESS.md` and `PLAN.md`.
- **Site gate:** the fixed list of login and call sites that every hardening default must pass.
