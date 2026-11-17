# Video calls with screen sharing in novabrowser-webkit

*   **Author:** Novaturiente
*   **Status:** Proposed (Goldfish review resolved 2026-11-17)
*   **Date:** 2026-11-17

--------------------------------------------------------------------------------

## Overview

novabrowser-webkit is a keyboard-driven browser written in Rust on WebKitGTK 6.0 and GTK 4. The author wants it to be their only browser for one week, as the test for replacing qutebrowser. Work requires Zoom web calls. Google Meet and Microsoft Teams calls are also needed. All three need camera, microphone and screen sharing on Wayland (Hyprland).

This design covers only calls: roadmap items 0 and 3 in `concepts_brief.md`. The event log, capability report, stability fixes, site gate, `/opt` packaging and hardening are separate designs.

### Glossary

*   **novarch:** the author's laptop, where calls are tested.
*   **novahome:** the author's home build server, where the custom WebKit is built.
*   **nova WebKit:** the author's own WebKitGTK build, AUR package `webkitgtk-6.0-nova-bin`, version 2.54.2-4 on novarch. "Stock WebKit" is Arch's `webkitgtk-6.0`.
*   **Site gate:** a planned fixed list of login and call sites that every shipped default must pass (roadmap item 5, not built yet).
*   **Speedometer A/B:** a benchmark run before and after a change, to show it costs no page speed.

## The Problem

No real call has been made in this browser. What exists:

*   **WebRTC is present.** nova WebKit 2.54.2-4 is installed on novarch and links `libgstwebrtc-1.0` (checked 2026-11-16). `src/web.rs:234-235` turns on WebRTC and media streams. On an older build without WebRTC, Meet showed a blank page.
*   **Screen sharing has no handling of its own.** A `content.desktop_capture` setting exists (nova-core `settings.rs:23`, default `ask`). But `src/permissions.rs:136-147` only reaches it when a request is for neither audio nor video. A screen-plus-audio request is shown as "use your microphone" and saved under the microphone setting. So a site once allowed the microphone can capture the screen without asking. The code never calls `webkit6::functions::user_media_permission_is_for_display_device`, which the `webkit6` 0.6.1 crate provides (`src/auto/functions.rs:59`).
*   **Prompts offer "Always" for every request** (`src/permissions.rs:348`). For screen capture this means one key press allows that site to capture the screen forever.
*   **The packages don't pull in what screen sharing needs on Wayland.** `gst-plugin-pipewire` and an `xdg-desktop-portal` backend are not in any PKGBUILD's optional dependencies. They happen to be installed on novarch.
*   **Zoom's web client has never been tried.** [Guessing] It may need WebAssembly SIMD, `SharedArrayBuffer`, WebCodecs, or a browser check that the Safari identity (`src/web.rs:24`) does not pass.

If calls fail, the trial week fails on the first workday. If calls fail without a record, the cause can't be found: the browser keeps no GStreamer, portal or permission logs.

The proposed solution has two parts. First, fix the screen-capture permission and the package dependencies, which are needed whatever the calls show. Second, a structured spike that makes real calls and tries every option for Zoom in a fixed order, followed only by the changes the spike proves are needed.

## Goals

*   **G1. Zoom works.** Join a Zoom meeting from a browser link, on nova WebKit under Hyprland, and stay in it for **10 minutes** with camera, microphone and screen share working. If it fails, try every step of the Zoom ladder (Technical Plan) and record each result. Zoom moves to another browser only after all of them fail.
*   **G2. Meet and Teams work.** The same test as G1: **10 minutes** in Google Meet, **5 minutes** in Microsoft Teams (web).
*   **G3. Screen share asks on its own.** A screen-capture request always shows its own prompt, offers only Yes, No and Never, and is saved only under `content.desktop_capture`, never under the microphone or camera answer. A unit test covers how a request maps to a prompt.
*   **G4. Failures can be diagnosed.** Every call test leaves a log bundle with the GStreamer WebRTC and PipeWire output, the portal journal, the browser's stderr and the memory log for the call. A failed call can be traced to a cause without running it again.
*   **G5. Packages declare what calls need.** The nova WebKit package depends on `gst-plugins-good`, `gst-plugins-bad` and `libnice`. The browser packages list `gst-plugin-pipewire` and `xdg-desktop-portal` (with a backend) as optional dependencies.
*   **G6. Stock WebKit's WebRTC support is recorded.** `RESULTS.md` says whether Arch's `webkitgtk-6.0` can make calls. If it can't, the optional-dependency text and the troubleshooting doc say calls need `webkitgtk-6.0-nova-bin`.
*   **G7. Screen audio, if possible.** If the spike shows WebKitGTK and the portal can share screen audio (Q1), screen audio is part of G1 and G2. If not, it is recorded in `RESULTS.md` and `troubleshooting.md` as a known gap.

## Non-Goals

*   DRM / Widevine.
*   Other call services (Discord, Slack huddles, Jitsi, WhatsApp calls). They use the same WebRTC path and may work, but they are not tested or fixed here.
*   Building a native Zoom client or wrapping Zoom's desktop app.
*   The general event log, `:report` and the capability report. G4 needs only the logs of a call test.
*   An in-browser "no WebRTC" notice. G6 is covered by package and docs text.
*   Moving nova WebKit to `/opt` and rebuilding PGO. Calls are tested on the installed 2.54.2-4. A rebuild happens here only if Zoom ladder step 6 needs a compiled-in feature.
*   X11. Only Wayland on Hyprland is tested; other compositors get only the generic portal dependency.
*   Making stock WebKit able to make calls if Arch builds it without WebRTC.

## Open Questions

*   **Q1. Can WebKitGTK share screen audio at all?** Unknown. The spike's call runs (A3) answer it in each `result.md` (the "screen audio" field).
*   **Q2. Which flags does a screen request carry?** Unknown whether the audio and video flags are also set on a screen request. B1 logs the three flags of every media request, so A3 answers it. B1's mapping does not depend on the answer, because the display flag wins.
*   **Q3. Which runtime features are off by default?** The installed library contains the feature names `ScreenCaptureEnabled`, `PeerConnectionEnabled`, `WebCodecsVideoEnabled`, `WebCodecsAudioEnabled`, `WebCodecsAV1Enabled`, `WebCodecsHEVCEnabled`, `WebRTCAV1CodecEnabled`, `VP9DecoderEnabled`, `CrossOriginOpenerPolicyEnabled`, `CrossOriginEmbedderPolicyEnabled` (`strings` on `/usr/lib/libwebkitgtk-6.0.so.4`, 2026-11-17). Their defaults are unknown. The feature dump (A1) answers it.
*   **Q4. What does Zoom's web client need?** Unknown. The Zoom ladder answers it.
*   **Q5. Does Arch's stock `webkitgtk-6.0` have WebRTC?** Unknown. nova WebKit replaces it on novarch, so the container check (A2) answers it.

## Critical User Journeys (CUJs)

### CUJ 1: Join a work Zoom meeting

*   **Given**: novabrowser-webkit on nova WebKit under Hyprland, signed in to the work Zoom account, camera and microphone connected.
*   **When**: the author opens a Zoom invite link from Calendar or Gmail, chooses "join from your browser", allows camera and microphone, and shares a window.
*   **Then**: other participants see and hear the author and see the shared window; the call stays up for 10 minutes; the browser asked for camera, microphone and screen separately.

### CUJ 2: Share the screen in Meet after allowing the microphone earlier

*   **Given**: `meet.google.com` already has a saved "Always" for the microphone.
*   **When**: the author clicks "Present now" in a Meet call.
*   **Then**: the browser shows a screen-capture prompt with Yes, No and Never only (it does not reuse the microphone answer). Then the Hyprland share picker appears, and the chosen screen or window is shared.

### CUJ 3: Teams meeting from the web

*   **Given**: a Teams meeting link, signed in to the work Microsoft account.
*   **When**: the author joins in the browser and shares the screen.
*   **Then**: the same result as CUJ 1 for 5 minutes, with no "unsupported browser" page.

### CUJ 4: A call fails

*   **Given**: a call run started through the spike's `call.sh`.
*   **When**: video, audio or screen share fails.
*   **Then**: the bundle holds the GStreamer, portal, browser and memory logs for that run. They are enough to name the failing layer (site check, permission, WebRTC, codec, portal, memory cap) without repeating the call.

--------------------------------------------------------------------------------

## Technical Plan (Architecture)

The work runs in three phases. Phase 0 builds the two changes needed whatever the calls show (B1, B6). Phase A is the spike: it makes real calls on the Phase 0 build and records what calls need. Phase B builds only what Phase A proves.

```text
 Phase 0 (always)          Phase A (spike, spikes/w20-calls/)     Phase B (only if RESULTS.md says so)
 ----------------          ----------------------------------     ------------------------------------
 B1 screen permission ---> A1 feature dump                    --> B2 webkit.features (shipped setting)
 B6 package deps           A2 stock WebRTC check              --> B3 built-in per-site user agent
 spike env switch          A3 call runs + Zoom ladder         --> B5 codec deps / nova build flags
                               |                                   remove the spike env switch
                               +--> RESULTS.md
```

### Phase 0: always built

**B1. Screen-capture permission.** `wanted()` in `src/permissions.rs` reads a third flag, "is for display", with the safe `webkit6::functions::user_media_permission_is_for_display_device`. The mapping moves into a small pure function of three booleans, so it can be unit-tested without a WebKit request object. The prompt texts are the CEF edition's (`../novabrowser/novabrowser/src/permissions.rs:25-30`, `MEDIA_BITS`).

**B1 table** (columns: audio, video, display, then the `(setting, prompt text)` pairs asked):

| audio | video | display | asked |
|---|---|---|---|
| false | any | true | `content.desktop_capture`: "capture your screen" |
| true | any | true | `content.desktop_capture`: "capture your screen", "capture your screen audio" |
| true | false | false | `content.media.audio_capture`: "use your microphone" |
| false | true | false | `content.media.video_capture`: "use your camera" |
| true | true | false | both of the two rows above |
| false | false | false | none: deny the request and log it |

A screen request is never covered by a saved microphone or camera answer. Its prompt offers only `y` Yes, `n` No and `N` Never: "Never" may be saved, "Always" is never offered or saved. Every media request writes one stderr line with its three flags and the setting chosen, which answers Q2.

**B6. Package dependencies.** The nova WebKit package depends on `gst-plugins-good` (it holds `rtpmanager` and VP8, which `webrtcbin` needs), `gst-plugins-bad` and `libnice`. The browser packages get `gst-plugin-pipewire` and `xdg-desktop-portal` (with a backend) as optional dependencies. Both go out with the next release.

**Spike switch.** For testing only, `page_settings()` (`src/web.rs:208`) reads an environment variable `NOVA_WEBKIT_FEATURES`: a comma-separated list of WebKit feature names, each optionally prefixed with `!` to turn it off. It uses the same `Settings::all_features()` lookup that turns on `ColorFilter` (`src/web.rs:238-242`). This is what Zoom ladder steps 3 and 5 use. It is removed before the next release. If the spike shows a feature must ship, it is replaced by the `webkit.features` setting (B2), per the owner rule: a mechanism that is part of a release is a setting; one that is only for testing is an environment variable.

### Phase A: the spike

All spike runs use their own profile, `--profile calls`, signed in by hand once to the work Zoom, Google and Microsoft accounts. The user's real profiles are never used (AGENTS.md spike rule). Spike files live in `spikes/w20-calls/`.

**A1. Feature dump.** A short Rust program prints every entry of `Settings::all_features()` with its identifier, category, status, whether it is on by default (`is_default_value()`) and whether it is on in a fresh `Settings` (`is_feature_enabled`). It also prints the WebKit version. It answers Q3 and feeds Zoom ladder step 5. It is needed because nova WebKit has no GObject introspection, so Python cannot list the features.

**A2. Stock WebRTC check.** In a throwaway Arch container, install `webkitgtk-6.0` and check whether its library links `libgstwebrtc-1.0` and contains `PeerConnectionEnabled`. It answers Q5. The container is removed after the run; no running Docker stack is touched.

**A3. Call runs with a log bundle.** A script `call.sh <name> <url>` starts the browser with GStreamer debug output turned on (`GST_DEBUG=webrtc*:4,pipewire*:4,nice:3,webkit*:4`, no colour). There is no `GST_DEBUG_FILE`: every process, including the sandboxed web processes, writes to the inherited stderr, which goes to the bundle's `browser.stderr`. When the browser exits, the script adds:

*   the portal journal for the call window, in `portal.log`,
*   the `nb-memlog` lines for the call window, if that logger is running, in `memlog.txt`,
*   package versions, in `env.txt`,
*   a copy of the per-run form, `result.md`, for the tester to fill in.

Bundles go to `~/.local/state/novabrowser-webkit/calls/<date-time>-<name>/`, outside the repo, because GStreamer logs can contain meeting IDs and IP addresses. The first Meet run checks that `webrtcbin` lines from the web process actually appear in `browser.stderr`.

Each run is a real call with a phone as the other participant, so video, audio and the shared screen can be checked from the far side.

**Every run records** these, for any site:

*   **Console checks** on the meeting page: `crossOriginIsolated`, `typeof SharedArrayBuffer`, `typeof VideoEncoder`, and `WebAssembly.validate` on a SIMD module.
*   **Codec list:** the encoders and decoders `webrtcbin` found, from `browser.stderr`.
*   **Permission flags:** the three flags from B1's stderr line.

**Order of runs:**

1. Meet, 10 minutes. It is the most likely to work and proves the WebRTC and portal path.
2. Teams, 5 minutes.
3. Zoom, 10 minutes, following the ladder until it passes.

**The Zoom ladder.** Each step is tried only if all earlier steps failed. Each attempt gets its own bundle.

1. **Entry points:** the `app.zoom.us` web app, the "join from your browser" link (`zoom.us/wc/join/<id>`), and an invite link opened from Calendar or Gmail.
2. **User agent for `*.zoom.us`:** the current Safari identity, then a recent Chrome on Linux, then a recent Firefox. No code: `content.user_agent` is already a per-site setting (`src/web.rs:199-205`, applied per load in `src/tabview.rs:12-28`), set with `:set -u`.
3. **Features the checks point to:** if a console check shows a missing API, turn on the matching feature from the A1 dump with `NOVA_WEBKIT_FEATURES` and restart.
4. **Codecs:** if the codec list lacks one Zoom offers (H.264 through `gst-plugins-ugly`, `gst-libav` or VA-API; VP8 and VP9 through `gst-plugins-good`; Opus), install it and retry.
5. **Remaining features:** every other feature that is off by default and related to media, WebCodecs, WebRTC codecs, `SharedArrayBuffer`, WebAssembly or screen capture, one at a time, with a restart between tries.
6. **The WebKit build:** if a needed feature is compiled out, add it to the nova WebKit build flags and rebuild on novahome.
7. **Stop:** after steps 1-6 all fail, Zoom stays in another browser. This is the one accepted exception to the trial week.

The spike ends with `spikes/w20-calls/RESULTS.md`: one row per run, the answers to Q1-Q5, and a section "Phase B changes required" that names the exact features, per-site defaults, codec dependencies and build flags to ship, or "none".

### Phase B: only what the spike proves

**B2. Shipped feature switches.** Built only if `RESULTS.md` names a feature to ship. A new nova-core setting `webkit.features` (a list, default from `RESULTS.md`) replaces the spike switch in `page_settings()`. Rules:

*   It is global only. `page_settings()` builds one settings object per view with no URL, so per-site values are ignored, and the docs say so.
*   Changes reach new tabs only.
*   `ColorFilter` stays on whatever the list says (dark mode needs it).
*   When a name is listed twice, the last entry wins.
*   An unknown name is logged once per name per process and skipped, so a newer or older WebKit never breaks startup.
*   The `webkit.` prefix means `other_edition` (nova-core `settings.rs:165-169`) already refuses it in the CEF edition.

**B3. Built-in per-site user agent.** Built only if `RESULTS.md` names a per-site default, for example a `content.user_agent` for `*.zoom.us`. nova-core has no shipped per-site defaults today, so this adds a short built-in pattern list in nova-core. Entries may only use keys the CEF edition ignores, such as `content.user_agent` (WebKit-only, `settings.rs:15`) or `webkit.*` keys. The full lookup order for a setting on a URL:

1. a `:set -u` value for a matching pattern,
2. a `config.ts` `pattern()` value,
3. a user global value (`:set`, `config.ts` or `autoconfig.json`),
4. a built-in pattern,
5. the schema default.

**B5. Codec and build changes.** Built only if the ladder needs them. Codec plugins become optional dependencies with a reason. A compiled-out feature becomes a nova WebKit build flag, with a Speedometer A/B (owner practice for engine changes; AGENTS.md requires one for "anything injected into every frame").

**Where shipped fixes live** (owner decision, 2026-11-17): shipped settings in nova-core (B2, B3), so every user gets them and can override them with `:set` or `config.ts`. Cost: a nova-core commit and a pin bump in this repo.

## Alternatives Considered

### Alternative A: Ship likely fixes without a spike

Turn on every media, WebCodecs and WebRTC feature and send a Chrome user agent to call sites, then test.

*   **Pros**: one step instead of two; no throwaway programs or scripts.
*   **Cons**: a feature that is off by default is off for a reason (unfinished, buggy or unsafe). Turning many on at once can break other sites or crash web processes. When a call then works or fails, nobody knows which change mattered. A Chrome user agent on a WebKit engine can make sites send Chrome-only code that fails.
*   **Reason for Rejection**: it breaks the brief's hardening rule (nothing ships on by default without evidence) and makes failures impossible to attribute.

### Alternative B: Hard-code call fixes in `src/web.rs`

*   **Pros**: no nova-core change; works for every user without config.
*   **Cons**: a fix that breaks a site can't be turned off without a rebuild and a release.
*   **Reason for Rejection**: users need a way to undo a shipped fix that breaks another site. A setting gives that; code does not.

### Alternative C: User config only, documented

*   **Pros**: smallest change; nothing shipped that could break other sites.
*   **Cons**: AUR users who never read the docs get a broken Zoom.
*   **Reason for Rejection**: the browser is meant for other users too, so a fix calls need must work out of the box.

### Alternative D: Keep detecting screen requests as "neither audio nor video"

This is the current code (`src/permissions.rs:145`).

*   **Pros**: no change.
*   **Cons**: a screen-plus-audio request is treated as a microphone request, so a saved microphone answer grants screen capture with no prompt.
*   **Reason for Rejection**: privacy hole (G3).

### Alternative E: Automated call tests

Use GStreamer fake sources as camera and microphone, and drive calls through IPC.

*   **Pros**: repeatable; could join the test suites.
*   **Cons**: the Wayland share picker needs a person to choose a screen. Whether the far side sees and hears the call can't be checked from the browser. Zoom, Meet and Teams need real signed-in accounts and change their pages often.
*   **Reason for Rejection**: high cost for the information it gives. Calls are tested by hand with a phone as the far side, and every run leaves a log bundle. The automated part is the B1 unit test.

### Alternative F: Zoom outside WebKit from the start

Use the Zoom desktop app or a Chromium browser for Zoom only.

*   **Pros**: Zoom works on day one with no work.
*   **Cons**: the trial week no longer tests the browser on the author's main work tool.
*   **Reason for Rejection**: the owner requires every option to be tried first (2026-11-16). It stays as the last ladder step.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| A feature that is off by default crashes web processes or breaks other sites | Turned on one at a time, only from the spike switch; shipped only after `RESULTS.md` names it, as a setting users can undo |
| A shipped default reaches the CEF edition | Built-in entries use only keys the CEF edition ignores |
| Zoom, Meet or Teams change their pages after the spike passes | Call rows in `docs/manual-test.md` are re-checked before each release; the site gate (roadmap item 5) takes this over later |
| The 3072 MB per-web-process cap (`src/web.rs:93-94`) kills a long call | Each bundle holds the memory log for the call, so a kill is visible; the cap itself stays (owner decision, 2026-11-16) |
| GStreamer output from sandboxed web processes is lost | Output goes to the inherited stderr, not a file; the first Meet run checks that web-process lines appear |

--------------------------------------------------------------------------------

## Non-Functional Requirements

*   **Security & Privacy:** a screen request always gets its own prompt, with no "Always", and its own saved answer (B1). Call bundles stay outside the repo in `~/.local/state/novabrowser-webkit/calls/`, because GStreamer logs can contain meeting IDs and IP addresses. The spike's `calls` profile holds work sign-ins; its data dir is never copied into the repo.
*   **Performance:** a nova WebKit build-flag change gets a Speedometer A/B. A `webkit.features` default that changes page behaviour on every site gets one before it ships.
*   **Rollout & Rollback:**
    *   B1: revert its commit.
    *   B2, B3: undo per user with `:set` or `config.ts`, or revert the nova-core default and bump the pin.
    *   B5 build flags: downgrade to the cached nova WebKit 2.54.2-4 package.
    *   B6: set `packaging/aur/webkitgtk-6.0-nova-bin/PKGBUILD` back to its previous `pkgrel` and dependencies.
    *   Spike switch: removed before release; nothing to roll back.

--------------------------------------------------------------------------------

### Additional Resources

*   `concepts_brief.md`: the daily-driver roadmap this design comes from.
*   `AGENTS.md`: project rules (spikes, Speedometer A/B, reference implementation).
*   `../novabrowser/novabrowser/src/permissions.rs`: the CEF edition's prompt texts.

### Future Improvements

*   **Other call services:** Discord, Slack huddles, Jitsi, WhatsApp calls through the same path, once the site gate (roadmap item 5) exists.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

### Build order and conditions

| Step | Files | Built when | Depends on |
|---|---|---|---|
| 1 | 1 `src/permissions.rs` | always | — |
| 2 | 2 `src/web.rs` (spike switch) | always | — |
| 3 | 3, 4 (PKGBUILDs) | always | — |
| 4 | checkpoint: build and test | always | 1-3 |
| 5 | 5-10 (spike) | always | 4 |
| 6 | 11, 13, 14, 15, 2 (B2 part) | `RESULTS.md` § Phase B names a feature | 5 |
| 7 | 12, 13, 14 | `RESULTS.md` § Phase B names a per-site default | 5 |
| 8 | 2 (remove spike switch) | if step 6 is not built | 5 |
| 9 | 16, 3 (rebuild part) | `RESULTS.md` § Phase B names a build flag | 5 |
| 10 | 17-20, 22 | always | 5-9 |
| 11 | 21 | step 6 or 7 built | 6, 7 |

The spike switch never reaches a release: step 6 replaces it, or step 8 removes it.

### Target Files to Create/Modify

#### 1. `src/permissions.rs` (Modify, always)

*   **Type / Scope**: permission mapping and prompt choices (B1).
*   **Proposed Changes**:
    *   New pure function `media_wanted(audio, video, display) -> Vec<(setting, prompt text)>` returning the B1 table. The no-flags row returns an empty list.
    *   `wanted()` (line 134): for a `UserMediaPermissionRequest`, read display with `webkit6::functions::user_media_permission_is_for_display_device(m)` and call `media_wanted`. An empty list means deny (the existing `None` path, line 133). Write one stderr line per media request: `novabrowser: media request audio=<0|1> video=<0|1> display=<0|1> -> <settings or deny>`.
    *   `choices()` (line 346): when the pending entry's settings include `content.desktop_capture`, offer `y` Yes, `n` No, `N` Never only. The pending entry already holds its settings (`push`, `Pending.settings`).
    *   The `DeviceInfoPermissionRequest` rule (lines 159-171) stays: device labels follow the camera and microphone settings, never `content.desktop_capture`.
    *   Unit test `media_wanted_cases`: each row of the B1 table; "display with audio never asks for `content.media.audio_capture`"; "a screen prompt offers no Always choice".

#### 2. `src/web.rs` (Modify)

*   **Type / Scope**: page settings.
*   **Proposed Changes**:
    *   **Always (step 2):** in `page_settings()` (line 208), replace the single `ColorFilter` lookup (lines 238-242) with one pass over `all_features()`. It turns on `ColorFilter`, then applies `NOVA_WEBKIT_FEATURES` (comma-separated, `!` prefix turns a feature off, last entry wins). `ColorFilter` is set on again after the list. An unknown name is logged once per name per process (a process-wide set of names already logged): `novabrowser: unknown WebKit feature <name>`.
    *   **Step 6:** the list comes from the `webkit.features` setting instead of the environment variable.
    *   **Step 8:** remove the environment variable read; keep the single-pass lookup for `ColorFilter`.

#### 3. `packaging/aur/webkitgtk-6.0-nova-bin/PKGBUILD` (Modify, always) and its `.SRCINFO` (Create, generated)

*   **Type / Scope**: AUR binary package for nova WebKit.
*   **Proposed Changes**:
    *   Move `gst-plugins-bad` and `libnice` from `optdepends` (lines 24-25) to `depends`, and add `gst-plugins-good` to `depends` (it moves out of `optdepends`, line 26).
    *   Add `'gst-plugin-pipewire: screen sharing on Wayland'` to `optdepends`.
    *   Bump `pkgrel` to 5. Replace the `_tag` expression (line 30) with the literal `webkitgtk-6.0-nova-2.54.2-4`, so the existing release asset and `sha256sums` (line 32) stay valid.
    *   Generate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO` and push the package to its AUR repo. `packaging/release.sh` does not handle this package (`release.sh:44`), so this is a manual step.
    *   **Step 9 (rebuild):** restore `_tag=webkitgtk-6.0-nova-$pkgver-$pkgrel`, upload the new tarball as that GitHub release asset, update `sha256sums`, and bump `pkgrel`.

#### 4. `packaging/PKGBUILD`, `packaging/aur/novabrowser-webkit/PKGBUILD`, `packaging/aur/novabrowser-webkit-bin/PKGBUILD` (Modify, always)

*   **Type / Scope**: browser packages (-git, tagged release, -bin).
*   **Proposed Changes**: add to the `optdepends` array:
    *   `'gst-plugin-pipewire: screen sharing on Wayland'`
    *   `'xdg-desktop-portal: screen sharing (needs a backend: xdg-desktop-portal-hyprland, -gnome or -kde)'`

    Step 10 adds any codec package `RESULTS.md` names, for example `'gst-plugins-ugly: H.264 encoding for calls'`. If A2 shows stock WebKit has no WebRTC, the `webkitgtk-6.0-nova-bin` entry in `optdepends` becomes `'webkitgtk-6.0-nova-bin: video calls (WebRTC), dark mode, PGO'`. These ship with the next browser release through `packaging/release.sh` (no change to it).

#### 5. `spikes/w20-calls/Cargo.toml` (Create)

*   **Type / Scope**: standalone throwaway crate. The workspace excludes `spikes/` (root `Cargo.toml:11`), and the root `.gitignore` ignores `target/`.
*   **Proposed Changes**: package `w20-calls`, with the same `gtk4` and `webkit6` dependency lines as `spikes/w9-baseline/Cargo.toml`.

#### 6. `spikes/w20-calls/src/main.rs` (Create)

*   **Type / Scope**: feature dump (A1).
*   **Proposed Changes**:
    *   Initialise GTK. Create a `Settings`.
    *   For every entry of `Settings::all_features()`, print one tab-separated line: `identifier()`, `category()`, `status()`, `is_default_value()`, `settings.is_feature_enabled(&f)`.
    *   First print `webkit6::functions::{major,minor,micro}_version()`.
    *   No window is opened. Run as `cargo run > features-<version>.tsv` (committed).

#### 7. `spikes/w20-calls/stock-webrtc.sh` (Create)

*   **Type / Scope**: Bash, A2.
*   **Proposed Changes**: `docker run --rm archlinux` running `pacman -Syu --noconfirm webkitgtk-6.0 binutils`. It then prints `pacman -Q webkitgtk-6.0`, the `ldd` lines for `gstwebrtc` and `gstsdp`, and the count of `PeerConnectionEnabled` in `strings`. `--rm` removes the container.

#### 8. `spikes/w20-calls/result-template.md` (Create)

*   **Type / Scope**: per-run form.
*   **Proposed Changes**: fields: date, site, entry point, user agent, `NOVA_WEBKIT_FEATURES` value, console checks (four results), codec list, permission flags (audio/video/display per request), camera, microphone, screen, screen audio (each pass/fail), call length, result, cause if failed.

#### 9. `spikes/w20-calls/call.sh` (Create)

*   **Type / Scope**: Bash, A3 call-run wrapper.
*   **Interface**: `call.sh <name> <url>`, run from the repo root against `target/release/novabrowser-webkit --profile calls <url>`.
*   **Proposed Changes**:
    *   Make the bundle directory `~/.local/state/novabrowser-webkit/calls/<YYYY-MM-DD-HHMM>-<name>/`, mode 700.
    *   Record the start time in a form `journalctl --since` accepts.
    *   Write `env.txt`: versions of `webkitgtk-6.0-nova-bin`, `gst-plugin-pipewire`, `xdg-desktop-portal-hyprland`, `gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly`, `gst-libav`, `libnice`, and the browser binary.
    *   Run the browser with `GST_DEBUG=webrtc*:4,pipewire*:4,nice:3,webkit*:4` and `GST_DEBUG_NO_COLOR=1`, passing `NOVA_WEBKIT_FEATURES` through, with stderr to `browser.stderr`.
    *   After exit: write the user journal of `xdg-desktop-portal` and `xdg-desktop-portal-hyprland` since the start time to `portal.log`. Copy that day's `~/.local/state/novabrowser-webkit/memlog/<date>.log` lines from the call window to `memlog.txt`, if the file exists. Copy `result-template.md` to `result.md`.

#### 10. `spikes/w20-calls/RESULTS.md` (Create)

*   **Type / Scope**: spike results, committed.
*   **Proposed Changes**:
    *   A table with one row per run: date, site, entry point, user agent, features, camera, microphone, screen, screen audio, call length, bundle name, pass or fail, cause.
    *   The answers to Q1-Q5.
    *   A section `## Phase B changes required`, listing: features to ship (or "none"), per-site defaults (or "none"), codec packages (or "none"), build flags (or "none").

#### 11. `../novabrowser/nova-core/src/settings.rs` (Modify; step 6)

*   **Type / Scope**: settings schema.
*   **Proposed Changes**: add `("webkit.features", "list", false, <default>, &[])` next to `webkit.itp` (line 17). Here `false` is `none_ok`, and `<default>` is the JSON list from `RESULTS.md`. Extend the test `cef_edition_refuses_webkit_settings` (line 177) with `webkit.features`.

#### 12. `../novabrowser/nova-core/src/config.rs` (Modify; step 7)

*   **Type / Scope**: config lookup.
*   **Proposed Changes**:
    *   A constant list of built-in patterns `(url pattern, setting, JSON value)` from `RESULTS.md`, parsed once with the existing `UrlPattern`.
    *   `get_for` (line 131) follows the lookup order in B3.
    *   "User global value" means the runtime `:set` map (`runtime`, line 32), `in_config_ts` (lines 118-120) or the saved `autoconfig.json` values (`SAVED`, line 39). It does not include the theme preset or the schema defaults. Extend `in_config_ts` into a helper that checks all three, rather than adding a separate one.
    *   Unit tests: a built-in value applies on a matching URL; a `config.ts` pattern beats it; a `:set -u` value beats it; a `config.ts` global beats it; a runtime `:set` global beats it; a non-matching URL gets the normal value.

#### 13. nova-core commit and push (step 6 or 7; not a file)

*   **Type / Scope**: release step.
*   **Proposed Changes**: nova-core `cargo test` green; commit naming what was added (`webkit.features`, built-in per-site defaults, or both); push to `Novaturiente/nova-core`. The CEF edition stays on its current pin.

#### 14. `Cargo.toml`, `Cargo.lock` (Modify; step 6 or 7)

*   **Type / Scope**: dependency pin.
*   **Proposed Changes**: `nova-core` `rev` (line 18) moves to the step 13 commit; `cargo update -p nova-core`; `cargo metadata --locked` passes.

#### 15. `tests/features.sh` (Modify; step 6)

*   **Type / Scope**: IPC feature test.
*   **Proposed Changes**: one check plus a reset:
    *   `--cmd 'set webkit.features ["NoSuchFeature"]'`, then `--cmd 'open -t data:text/html,<title>f</title>'`.
    *   Pass if the tab's title becomes `f` and the browser stderr (`$tmp/stderr`, `tests/lib.sh:18`) has `unknown WebKit feature NoSuchFeature`.
    *   Then `--cmd 'set webkit.features []'`.

#### 16. `spikes/wk-build/build.sh` (Modify; step 9)

*   **Type / Scope**: nova WebKit build flags (engine build).
*   **Proposed Changes**: add the `-D ENABLE_...=ON` flags `RESULTS.md` names, next to the WebRTC flags (line 41). Rebuild on novahome. Speedometer A/B against 2.54.2-4 into `spikes/w9-baseline/ab-nova5.jsonl`. Then do file 3's rebuild part.

#### 17. `docs/user/settings.md` (Modify, step 10)

*   **Type / Scope**: user settings reference.
*   **Proposed Changes**:
    *   `### content.desktop_capture` (line 161): screen sharing is asked on its own, never follows the microphone or camera answer, and offers no "Always".
    *   **Step 6 only:** a `### webkit.features` entry: what it does, `!` to turn a feature off, global only, new tabs only, unknown names logged.
    *   **Step 7 only:** a list of the built-in per-site defaults, and that any user value overrides them.

#### 18. `docs/user/privacy.md` (Modify, step 10)

*   **Type / Scope**: user privacy doc.
*   **Proposed Changes**: one paragraph in the permissions part: screen capture is its own permission, a saved microphone or camera answer never covers it, and it can't be allowed permanently.

#### 19. `docs/user/troubleshooting.md` (Modify, step 10)

*   **Type / Scope**: user troubleshooting.
*   **Proposed Changes**: a "Video calls" section:
    *   needed packages (`webkitgtk-6.0-nova-bin` if stock has no WebRTC, `gst-plugin-pipewire`, a portal backend);
    *   how to collect logs (`GST_DEBUG` to stderr, the portal journal);
    *   how to try another user agent per site with `:set -u`;
    *   screen audio, if G7 found it unsupported.

#### 20. `docs/manual-test.md` (Modify, step 10)

*   **Type / Scope**: manual test list.
*   **Proposed Changes**: rows for CUJ 1-3: Zoom 10 minutes, Meet 10 minutes, Teams 5 minutes, each with camera, microphone and screen share, and a phone as the far side. Plus CUJ 2's row: microphone allowed, screen still asks, with no "Always".

#### 21. `docs/feature-map.md` (Modify; step 11)

*   **Type / Scope**: CEF vs WebKit differences.
*   **Proposed Changes**: one line per built item: `webkit.features` is WebKit-only; built-in per-site defaults live in nova-core and apply to both editions.

#### 22. `PROGRESS.md` (Modify, step 10)

*   **Type / Scope**: project state (AGENTS.md session rule).
*   **Proposed Changes**:
    *   A new Phase 0 item `W20 calls (Zoom, Meet, Teams + screen share)`, with pass conditions G1 and G2 and its evidence.
    *   Updates to W16 and 7.15 (screen-capture permission).
    *   Dated decisions (shipped settings in nova-core; screen prompt without "Always"; spike switch is an environment variable).
    *   A session-log entry in IST, and the next "Now".

### Verification Plan

*   **Checkpoint after steps 1-3 (step 4):**
    *   `cargo build --release` and `cargo clippy --workspace`, with no new warnings.
    *   `cargo test --workspace`, including `media_wanted_cases`.
    *   `tests/smoke.sh`.
    *   `makepkg --printsrcinfo` for all four PKGBUILDs.
*   **Spike (step 5):**
    *   `bash -n` on `call.sh` and `stock-webrtc.sh`.
    *   `cargo build` in `spikes/w20-calls/`.
    *   The first Meet bundle shows `webrtcbin` lines from the web process.
*   **After steps 6-7, only if built:**
    *   `cargo test` in nova-core: built-in pattern tests and the extended edition test.
    *   Here, after the pin bump: build, `cargo test --workspace`, then `tests/smoke.sh`, `tests/features.sh` (with file 15) and `tests/passwords.sh`.
*   **Before release:** `tests/smoke.sh`, `tests/features.sh` and `tests/passwords.sh` all pass on the release build. A grep for `NOVA_WEBKIT_FEATURES` in `src/` finds nothing.
*   **Manual (owner):**
    *   CUJ 1-3 pass with a phone as the far side, each with a bundle: Zoom 10 minutes, Meet 10 minutes, Teams 5 minutes.
    *   CUJ 2 shows a separate screen prompt with no "Always" on a site with a saved microphone answer.
    *   Results go in `spikes/w20-calls/RESULTS.md` and `docs/manual-test.md`.
*   **Rollback per phase:**
    *   Phase 0: revert its commit, and set `pkgrel` back to 4.
    *   Phase B: revert the pin-bump commit here, and the nova-core commit there.

--------------------------------------------------------------------------------

*Drafted with EGM*
