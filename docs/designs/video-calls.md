# Video calls with screen sharing in novabrowser-webkit

*   **Author:** Novaturiente
*   **Status:** Proposed
*   **Date:** 2026-11-17

--------------------------------------------------------------------------------

## Overview

novabrowser-webkit is a keyboard-driven browser written in Rust on WebKitGTK 6.0 and GTK 4. The author wants it to be their only browser for one week, as the test for replacing qutebrowser. Work requires Zoom web calls. Meet and Teams calls are also needed. All three need camera, microphone and screen sharing on Wayland (Hyprland).

This design covers only calls: roadmap items 0 and 3 in `concepts_brief.md`. The event log, capability report, stability fixes, site gate, `/opt` packaging and hardening are separate designs.

## The Problem

No real call has been made in this browser. What exists:

*   The custom WebKit build `webkitgtk-6.0-nova-bin 2.54.2-4` is installed on novarch and links `libgstwebrtc-1.0` (checked 2026-11-16). `src/web.rs:234-235` turns on WebRTC and media streams. On the 2026-11-04 build without WebRTC, Meet showed a blank page.
*   Screen sharing is not handled on purpose. A `content.desktop_capture` setting exists (nova-core `settings.rs:23`, default `ask`), but `src/permissions.rs:136-147` only reaches it when a request is for neither audio nor video. A screen-plus-audio request is shown as "use your microphone" and saved under the microphone setting. A site that was once allowed the microphone can then capture the screen without asking. That is a privacy hole, not just wrong wording. The cause: the `webkit6` 0.6.1 crate has no safe binding for `webkit_user_media_permission_is_for_display_device`, although the C function exists in the installed header and in `webkit6-sys`.
*   The packages don't pull in what screen sharing needs on Wayland. `gst-plugin-pipewire` and an `xdg-desktop-portal` backend are not in the optional dependencies (`packaging/aur/*/PKGBUILD`). They happen to be installed on novarch.
*   Zoom's web client has never been tried. [Guessing] It may need WebAssembly SIMD, `SharedArrayBuffer`, WebCodecs, or a browser check that the Safari identity (`src/web.rs:24`) does not pass.

If calls fail, the trial week fails on the first workday. If calls fail without a record, the cause can't be found: today the browser keeps no GStreamer, portal or permission logs.

The proposed solution has two parts. First, a structured spike that makes real calls and tries every option for Zoom, in a fixed order. Second, the code and packaging changes the spike shows are needed: a separate screen-capture permission, any WebKit runtime features or per-site user agents that calls need, and the missing dependencies.

## Goals

*   **G1. Zoom works.** Join a Zoom meeting from a browser link, on nova 2.54.2-4 under Hyprland, and stay in it for 30 minutes with camera, microphone and screen share working. If it fails, try every option in the Zoom ladder (Technical Plan) and record each result. Zoom moves to another browser only after all of them fail.
*   **G2. Meet and Teams work.** Pass the same test as G1 in Google Meet and Microsoft Teams (web).
*   **G3. Screen share asks on its own.** A screen-capture request always shows its own prompt, saved under its own setting, never under the microphone or camera answer. A unit test covers how a request maps to a prompt.
*   **G4. Failures can be diagnosed.** Every call test leaves a log bundle: the GStreamer WebRTC and PipeWire debug output, the portal journal, and the browser's stderr. A failed call can be traced to a cause without running it again.
*   **G5. Packages declare what calls need.** A fresh AUR install that adds the optional dependencies has everything screen sharing needs on Wayland.
*   **G6. Stock WebKit is known.** It is known and written down whether Arch's stock `webkitgtk-6.0` can make calls. If it can't, the browser says so on call pages instead of showing a blank page.

## Non-Goals

*   DRM / Widevine.
*   Other call services (Discord, Slack huddles, Jitsi, WhatsApp calls). They use the same WebRTC path and may work, but they are not tested or fixed here.
*   Building a native Zoom client or wrapping Zoom's desktop app.
*   The general event log, `:report` and the capability report. G4 only needs the logs of a call test, not a permanent log.
*   Moving the nova build to `/opt` and rebuilding PGO. The calls work happens on the installed 2.54.2-4. A nova rebuild happens here only if Zoom ladder step 6 needs a compiled-in feature.
*   X11. Only Wayland on Hyprland is tested; other compositors only get the generic portal dependency.
*   Making stock WebKit able to make calls if Arch builds it without WebRTC.

## Open Questions

*   **Q1. Can WebKitGTK share screen audio at all?** [Likely] `xdg-desktop-portal-hyprland`'s ScreenCast portal gives video only. WebKitGTK may not capture system audio for `getDisplayMedia` at all. If so, "screen share with sound" is out of reach, and G1/G2 cover video-only screen share. Needs a check.
*   **Q2. What do display-capture requests report?** Unknown whether `is_for_video_device` and `is_for_audio_device` are also true for a screen request. Known (2026-11-17): `is_for_display_device` has no safe binding in `webkit6` 0.6.1, only the raw one in `webkit6-sys`.
*   **Q3. Which runtime features are off by default?** The installed library contains the feature names `ScreenCaptureEnabled`, `PeerConnectionEnabled`, `WebCodecsVideoEnabled`, `WebCodecsAudioEnabled`, `WebCodecsAV1Enabled`, `WebCodecsHEVCEnabled`, `WebRTCAV1CodecEnabled`, `VP9DecoderEnabled`, `CrossOriginOpenerPolicyEnabled`, `CrossOriginEmbedderPolicyEnabled` (`strings` on `/usr/lib/libwebkitgtk-6.0.so.4`, 2026-11-17). Their defaults are unknown: the nova build has no GObject introspection, so they must be listed from Rust.
*   **Q4. What does Zoom's web client need?** Unknown. The Zoom ladder answers it.
*   **Q5. Does Arch's stock `webkitgtk-6.0` have WebRTC?** Unknown. The nova package replaces it on novarch, so checking needs a container or another machine.

## Critical User Journeys (CUJs)

### CUJ 1: Join a work Zoom meeting

*   **Given**: novabrowser-webkit on nova 2.54.2-4 under Hyprland, signed in to the work Zoom account, camera and mic connected.
*   **When**: the author opens a Zoom invite link from Calendar or Gmail, chooses "join from your browser", allows camera and mic, and shares a window.
*   **Then**: other participants see and hear the author and see the shared window; the call stays up for 30 minutes; the browser asked for the camera, mic and screen permissions separately.

### CUJ 2: Share the screen in Meet after allowing the mic earlier

*   **Given**: `meet.google.com` already has a saved "allow" for the microphone.
*   **When**: the author clicks "Present now" in a Meet call.
*   **Then**: the browser shows a screen-capture prompt (it does not reuse the mic answer), then the Hyprland share picker appears, and the chosen screen or window is shared.

### CUJ 3: Teams meeting from the web

*   **Given**: a Teams meeting link, signed in to the work Microsoft account.
*   **When**: the author joins in the browser and shares the screen.
*   **Then**: same result as CUJ 1, with no "unsupported browser" page.

### CUJ 4: A call fails

*   **Given**: a call test run with the debug bundle turned on.
*   **When**: video, audio or screen share fails.
*   **Then**: the bundle holds the GStreamer, portal and browser logs for that run, enough to name the failing layer (site check, permission, WebRTC, codec, portal) without repeating the call.

### CUJ 5: An AUR user on stock WebKit opens Meet

*   **Given**: an install on Arch's stock `webkitgtk-6.0` that turns out to have no WebRTC (Q5).
*   **When**: the user opens a Meet call.
*   **Then**: the browser shows "This WebKit build has no WebRTC" instead of a blank page.

--------------------------------------------------------------------------------

## Technical Plan (Architecture)

The work has two phases. Phase A finds out what calls need, with no change to the shipped browser. Phase B builds only the changes Phase A proves are needed, plus the screen-capture permission fix, which is needed either way.

```text
 Phase A (spike, spikes/w20-calls/)              Phase B (browser changes)
 ----------------------------------              -------------------------
 A1 feature dump  --> which features are off --> B2 enable call features
 A2 stock WebRTC  --> yes / no ----------------> B4 "no WebRTC" notice (only if no)
 A3 call runs     --> pass / fail + log bundle
      |                                           B1 screen-capture permission
      +-- Zoom ladder steps 1-6 ----------------> B2 features, B3 per-site UA,
                                                  B5 codecs / nova rebuild flags
                                                  B6 package dependencies
```

### Phase A: the spike

All spike runs use their own profile, `--profile calls`, signed in to the work Zoom, Google and Microsoft accounts by hand once. The user's real profiles are never used (AGENTS.md spike rule). Spike files live in `spikes/w20-calls/`.

**A1. Feature dump.** A small throwaway Rust program in the spike dir, linked against the installed `webkit6`, prints every entry of `Settings::all_features()` with its identifier, category, status and default. This answers Q3 and is the input to Zoom ladder step 3. It is needed because the nova build has no GObject introspection, so Python cannot list them.

**A2. Stock WebRTC check.** In a throwaway Arch container (`docker run --rm archlinux`), install `webkitgtk-6.0` and check `ldd /usr/lib/libwebkitgtk-6.0.so.4 | grep gstwebrtc` and `strings ... | grep PeerConnectionEnabled`. This answers Q5. The container is removed after the run; no running Docker stack is touched.

**A3. Call runs with a log bundle.** A script `spikes/w20-calls/call.sh <name>` starts the browser with:

*   `GST_DEBUG=webrtc*:4,webrtcbin:4,pipewire*:4,nice:3` and `GST_DEBUG_FILE` set to the bundle dir,
*   browser stderr sent to the same dir,
*   on exit, `journalctl --user --since <start>` for `xdg-desktop-portal` and `xdg-desktop-portal-hyprland` appended,
*   a short `result.md` the tester fills in: site, entry point, user agent, features on, camera / mic / screen / screen audio pass or fail, call length.

Bundles go to `~/.local/state/novabrowser-webkit/calls/<date-time>-<name>/`, outside the repo, because GStreamer logs can contain meeting IDs and IP addresses.

Each run is a real call with a second device (phone) as the other participant, so video, audio and the shared screen can be seen from the far side.

**Order of runs:**

1. Meet (the most likely to work; proves the WebRTC and portal path).
2. Teams.
3. Zoom, following the ladder below until it passes.
4. One 30-minute call on whichever site the author uses most at work, to check G1's duration bar.

**The Zoom ladder.** Each step is tried only if all earlier steps failed. Each attempt gets its own bundle.

1. **Entry points:** the `app.zoom.us` web app, the "join from your browser" link (`zoom.us/wc/join/<id>`), and an invite link opened from Calendar or Gmail.
2. **User agent for `*.zoom.us`:** the current Safari identity, then a recent Chrome on Linux, then a recent Firefox. This needs no code: `content.user_agent` is already a per-site setting (`src/web.rs:199-205`, applied per load in `src/tabview.rs:12-28`), set through `config.ts` or `:set -u`.
3. **Runtime features:** from the A1 dump, every feature that is off by default and related to media, WebCodecs, WebRTC codecs, `SharedArrayBuffer`, WebAssembly or screen capture, turned on one at a time with a restart between tries.
4. **Cross-origin isolation:** in the console on the Zoom meeting page, check `crossOriginIsolated`, `typeof SharedArrayBuffer`, `typeof VideoEncoder`, and `WebAssembly.validate` on a SIMD module. If Zoom asks for a feature that is missing, that names the gap.
5. **Codecs:** in the GStreamer log, check which encoders and decoders `webrtcbin` found (H.264 through `gst-plugins-ugly` x264, `gst-libav` or VA-API; VP8 and VP9 through `gst-plugins-good`; Opus). Install what is missing and retry.
6. **The WebKit build:** if a needed feature is compiled out, add it to the nova build flags in `spikes/wk-build/build.sh` and rebuild on novahome.
7. **Stop:** after steps 1-6 all fail, Zoom stays in another browser; this is the one accepted exception to the trial week.

The spike ends with `spikes/w20-calls/RESULTS.md`: one row per run, the answers to Q1-Q5, and the list of Phase B changes the results call for.

### Phase B: browser changes

**B1. Screen-capture permission (always built).** `wanted()` in `src/permissions.rs` gets a third flag, "is for display", read through the raw `webkit6::ffi::webkit_user_media_permission_is_for_display_device` call, since the safe crate lacks it. The mapping moves into a small pure function of three booleans (audio, video, display) so it can be unit-tested without a WebKit request object:

| audio | video | display | setting(s) asked |
|---|---|---|---|
| any | any | true | `content.desktop_capture` only |
| true | false | false | `content.media.audio_capture` |
| false | true | false | `content.media.video_capture` |
| true | true | false | both mic and camera |
| false | false | false | `content.desktop_capture` (old fallback) |

When display is true, the prompt says "capture your screen" (or "capture your screen and its sound" when audio is also true) and the answer is saved only under `content.desktop_capture`. A saved microphone "allow" never covers a screen request. If Q2 shows WebKit sets the video flag on screen requests too, the table above already handles it, because display wins.

**B2. Call features (only what A1 and A3 prove).** Turn on, in `page_settings()` (`src/web.rs:208`), the runtime features the spike shows calls need. They use the same lookup that already turns on `ColorFilter` (`src/web.rs:238-242`). Where these switches live (in code, as a shipped setting, or in user config) is an open owner decision, below.

**B3. Per-site user agent (only if the ladder needs it).** No code. A shipped or documented `content.user_agent` pattern for the call host. Where it lives is part of the same owner decision.

**B4. "No WebRTC" notice (only if A2 says stock has no WebRTC).** At page load on a known call host (`meet.google.com`, `teams.microsoft.com`, `teams.live.com`, `*.zoom.us`), if `window.RTCPeerConnection` is missing, show the browser message "This WebKit build has no WebRTC: calls need webkitgtk-6.0-nova". It uses the page-to-browser message path `MEDIA_JS` already uses (`src/web.rs:28-39`) for missing codecs.

**B5. Codec and build changes (only if the ladder needs them).** Codec plugins become optional dependencies with a reason. A compiled-out feature becomes a build flag in `spikes/wk-build/build.sh`, with a Speedometer A/B per the project rule that any engine change gets one.

**B6. Package dependencies (always built).** Add to the browser packages' optional dependencies: `gst-plugin-pipewire` (screen capture on Wayland) and `xdg-desktop-portal` with a backend (`xdg-desktop-portal-hyprland`, `-gnome` or `-kde`). Make `gst-plugins-bad` and `libnice` hard dependencies of `webkitgtk-6.0-nova-bin`, because its WebRTC fails without them.

### Owner decision: where call fixes live

**Chosen (owner, 2026-11-17): shipped settings in nova-core.**

*   **Feature switches:** a new setting `webkit.features` (kind `list`, default chosen from the spike results, maybe empty) in nova-core `settings.rs`, next to `webkit.itp` (line 17). Each entry is a WebKit feature identifier from `Settings::all_features()`, with an optional `!` prefix to turn a feature off. `page_settings()` applies the list; an unknown name is logged once to stderr and skipped, so a newer or older WebKit never breaks startup. The `webkit.` prefix means `other_edition` (nova-core `settings.rs:165-169`) already refuses `:set webkit.features` in the CEF edition, with no extra code.
*   **Per-site user agent:** nova-core has per-site values only from the user (`config.ts` patterns, `:set -u`; `config.rs:131-140`), and no shipped per-site defaults. This adds a short built-in pattern list in nova-core, checked by `get_for` after the user's runtime and `config.ts` patterns, so any user value still wins. It holds only what the spike proves (for example a `content.user_agent` for `*.zoom.us`). It is shared with the CEF edition, so CEF gets the same defaults; entries are kept to keys both editions read, or to `webkit.*` keys.
*   **Cost:** a nova-core change, a new nova-core commit, and a pin bump in this repo's `Cargo.toml` / `Cargo.lock`.
*   **Risk:** a shipped default that breaks another site is now on for every user until the next release. Users can override it with `:set -u` or `config.ts`. Each shipped default must pass the site gate once that exists (roadmap item 5); until then, the call sites in this design are the gate.

## Alternatives Considered

### Alternative A: Ship likely fixes without a spike

Turn on every media, WebCodecs and WebRTC feature and send a Chrome user agent to call sites, then test.

*   **Pros**: one step instead of two; no throwaway programs or scripts.
*   **Cons**: each feature that is off by default is off for a reason (unfinished, buggy or unsafe). Turning many on at once can break other sites or crash web processes, and when a call then works or fails, nobody knows which change mattered. A Chrome user agent on a WebKit engine can make sites send Chrome-only code paths that fail.
*   **Reason for Rejection**: it breaks the hardening rule from the brief (nothing ships on by default without evidence) and makes failures impossible to attribute.

### Alternative B: Hard-code call fixes in `src/web.rs`

*   **Pros**: no nova-core change; works for every user without config.
*   **Cons**: a fix that breaks a site can't be turned off without a rebuild and release.
*   **Reason for Rejection**: owner chose shipped settings in nova-core (2026-11-17), which keeps the out-of-the-box result and adds a user override.

### Alternative C: User config only, documented

*   **Pros**: smallest change; nothing shipped that could break other sites.
*   **Cons**: AUR users who never read the docs get a broken Zoom.
*   **Reason for Rejection**: owner chose shipped settings (2026-11-17); the browser is meant for other users too.

### Alternative D: Keep detecting screen requests as "neither audio nor video"

This is the current code (`src/permissions.rs:145`).

*   **Pros**: no FFI call, no change.
*   **Cons**: a screen-plus-audio request is treated as a microphone request, so a saved mic "allow" grants screen capture with no prompt.
*   **Reason for Rejection**: privacy hole (G3).

### Alternative E: Add a safe `is_for_display_device` binding to the `webkit6` crate

Vendor the crate and add the method, or send the binding upstream and wait.

*   **Pros**: no `unsafe` block in this repo.
*   **Cons**: vendoring a whole crate for one function adds a maintenance burden; waiting upstream blocks the fix for an unknown time.
*   **Reason for Rejection**: one `unsafe` call to a function the `webkit6-sys` crate already declares is smaller. An upstream patch is listed under Future Improvements; when it lands, the `unsafe` call is replaced.

### Alternative F: Automated call tests

Use GStreamer fake sources as camera and mic, and drive calls through IPC.

*   **Pros**: repeatable; could join the test suites.
*   **Cons**: the Wayland share picker needs a person to choose a screen; whether the far side sees and hears the call can't be checked from the browser; Zoom, Meet and Teams need real signed-in accounts and change their pages often.
*   **Reason for Rejection**: the cost is high for the information it gives. Calls are tested by hand with a phone as the far side, and every run leaves a log bundle. The automated parts are the B1 unit test and, later, the site gate's feature probes (roadmap item 5).

### Alternative G: Zoom outside WebKit from the start

Use the Zoom desktop app or a Chromium browser for Zoom only.

*   **Pros**: Zoom works on day one with no work.
*   **Cons**: the trial week no longer tests the browser on the author's main work tool.
*   **Reason for Rejection**: owner requires every option to be tried first (2026-11-16). It stays as the last ladder step.

--------------------------------------------------------------------------------

## Non-Functional Requirements

*   **Security & Privacy:** screen capture always gets its own prompt and its own saved answer (B1). Call log bundles stay outside the repo in `~/.local/state/novabrowser-webkit/calls/`, because GStreamer logs contain meeting IDs and IP addresses. The spike's `calls` profile holds work sign-ins; its data dir is never copied into the repo.
*   **Performance:** any change to the WebKit build (ladder step 6) gets a Speedometer A/B (AGENTS.md rule). `webkit.features` entries that change page behaviour on every site get the same A/B before they ship as a default.
*   **Rollout & Rollback:** every shipped call fix is a setting default, so a user can undo it with `:set` or `config.ts` without a new release. The nova package dependency change keeps the same release asset (see file 16).

### Future Improvements

*   **Upstream binding:** send a safe `is_for_display_device` binding to the `webkit6` crate; replace the `unsafe` call when it lands.
*   **Other call services:** Discord, Slack huddles, Jitsi, WhatsApp calls through the same path, once the site gate (roadmap item 5) exists.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

Order: Phase A files 1-5 first. Then file 6-8 (nova-core), 9 (pin bump), 10-15 (browser), 16-17 (packaging), 18-23 (docs and state). Files marked **conditional** are changed only if `spikes/w20-calls/RESULTS.md` says so; if the spike does not call for them, they are left alone and the results file says why.

**Always built:** the spike (1-5), the screen-capture permission (10), packaging (16-17), and the docs and state parts that describe them (18-21, 23). **Built only if `RESULTS.md` names at least one feature switch or per-site default:** nova-core (6-8), the pin bump (9), the `webkit.features` parts of 11, 13, 18 and 22. This keeps an empty setting and an empty lookup step from shipping when calls need neither (Zoom Out review, 2026-11-17).

### Target Files to Create/Modify

#### Phase A: spike (`spikes/w20-calls/`)

#### 1. `spikes/w20-calls/Cargo.toml` (Create)

*   **Type / Scope**: standalone throwaway Rust crate (the workspace already excludes `spikes/`, root `Cargo.toml:11`). Its `target/` is ignored by the root `.gitignore` (`target/`).
*   **Proposed Changes**: package `w20-features`; dependencies `gtk4` 0.11 and `webkit6` 0.6 with feature `v2_52`, the same versions as the browser.

#### 2. `spikes/w20-calls/src/main.rs` (Create)

*   **Type / Scope**: feature dump program (answers Q3).
*   **Proposed Changes**: initialise GTK, create `webkit6::Settings::new()`, walk `Settings::all_features()`, print one tab-separated line per feature: identifier, category, status, `default_value()`, current value on a fresh `Settings`. Also print `webkit6::major_version()` / minor / micro. No window is opened. Output saved as `spikes/w20-calls/features-2.54.2-4.tsv`.

#### 3. `spikes/w20-calls/stock-webrtc.sh` (Create)

*   **Type / Scope**: Bash, answers Q5.
*   **Proposed Changes**: `docker run --rm archlinux` that runs `pacman -Syu --noconfirm webkitgtk-6.0 binutils`, then prints `pacman -Q webkitgtk-6.0`, `ldd /usr/lib/libwebkitgtk-6.0.so.4 | grep -E 'gstwebrtc|gstsdp'` and `strings /usr/lib/libwebkitgtk-6.0.so.4 | grep -c PeerConnectionEnabled`. `--rm` removes the container; no existing container, image or stack is touched beyond pulling `archlinux`.

#### 4. `spikes/w20-calls/call.sh` (Create)

*   **Type / Scope**: Bash, the call-run wrapper (G4).
*   **Proposed Changes**: usage `call.sh <name> [url]`. Steps:
    1. `dir=~/.local/state/novabrowser-webkit/calls/$(date +%F-%H%M)-<name>`, `mkdir -p`, `chmod 700`.
    2. Record start time; write `env.txt` with `pacman -Q webkitgtk-6.0-nova-bin gst-plugin-pipewire xdg-desktop-portal-hyprland gst-plugins-bad gst-plugins-good gst-plugins-ugly gst-libav libnice` and the browser version.
    3. Run `target/release/novabrowser-webkit --profile calls <url>` with `GST_DEBUG=webrtc*:4,webrtcbin:4,pipewire*:4,nice:3`, `GST_DEBUG_FILE=$dir/gst.log`, `GST_DEBUG_NO_COLOR=1`, stderr to `$dir/browser.stderr`. The script is run through the `process` tool, never with `&`.
    4. After the browser exits: `journalctl --user --since "$start" -u xdg-desktop-portal -u xdg-desktop-portal-hyprland > $dir/portal.log`.
    5. Copy `spikes/w20-calls/result-template.md` to `$dir/result.md` for the tester.

#### 5. `spikes/w20-calls/RESULTS.md` (Create)

*   **Type / Scope**: spike results (Markdown), committed.
*   **Proposed Changes**: a table with one row per run (date, site, entry point, user agent, features on, camera / mic / screen / screen audio, call length, bundle dir name, pass or fail, cause). Then answers to Q1-Q5, then "Phase B changes required": the exact `webkit.features` entries, built-in `content.user_agent` patterns, codec dependencies and build flags, or "none". The template file `spikes/w20-calls/result-template.md` (Create) holds the per-run form used by file 4.

#### Phase B: nova-core (`../novabrowser/nova-core`, its own repo)

#### 6. `../novabrowser/nova-core/src/settings.rs` (Modify, conditional)

*   **Type / Scope**: settings schema.
*   **Proposed Changes**: add `("webkit.features", "list", false, <default>, &[])` next to `webkit.itp` (line 17). `<default>` is the JSON list from `RESULTS.md`, or `[]`. The comment above the `webkit.*` block (line 15) already marks these as WebKit-only. Extend the existing `other_edition` test (line 177) with `webkit.features`.

#### 7. `../novabrowser/nova-core/src/config.rs` (Modify, conditional: only if a per-site default is needed)

*   **Type / Scope**: config lookup.
*   **Proposed Changes**:
    *   A constant `BUILTIN_PATTERNS: &[(&str /*url pattern*/, &str /*setting*/, &str /*JSON value*/)]`, parsed once with the existing `UrlPattern` type. Contents come from `RESULTS.md` (for example `("*://*.zoom.us/*", "content.user_agent", "\"<ua>\"")`), or empty.
    *   `get_for` (line 131) checks, in order: runtime `:set -u` patterns, `config.ts` patterns, then built-in patterns, then `self.get(name)`.
    *   A built-in pattern applies only if the user has not set that setting globally (in `config.ts` or `autoconfig.json`). Otherwise a user's global `content.user_agent = "webkit"` would lose to a shipped per-site value. This needs a way to ask "did the user set this name?"; if `Config` has none, add `fn user_set(&self, name) -> bool` from the same maps `get` reads.
    *   Unit tests: built-in applies on a matching URL with no user value; a `config.ts` pattern beats it; a `:set -u` value beats it; a user global value beats it; a non-matching URL gets the normal value.

#### 8. `../novabrowser/nova-core` commit and push (conditional on 6 or 7)

*   **Type / Scope**: release step, not a file.
*   **Proposed Changes**: `cargo test` in nova-core green, commit `config: webkit.features + built-in per-site defaults`, push to `Novaturiente/nova-core`. The CEF edition is not moved to the new rev here (its own repo, C4).

#### Phase B: this repo

#### 9. `Cargo.toml`, `Cargo.lock` (Modify, conditional on 8)

*   **Type / Scope**: dependency pin.
*   **Proposed Changes**: `nova-core` `rev` (line 18) moves to the commit from step 8; `cargo update -p nova-core`; `cargo metadata --locked` passes.

#### 10. `src/permissions.rs` (Modify)

*   **Type / Scope**: permission mapping (B1).
*   **Proposed Changes**:
    *   New pure function `media_wanted(audio: bool, video: bool, display: bool) -> Vec<(&'static str, &'static str)>` that returns the table in the Technical Plan. With display true it returns only `("content.desktop_capture", "capture your screen")`, or `"capture your screen and its sound"` when audio is also true.
    *   `wanted()` (line 134) reads `display` with one `unsafe` call to `webkit6::ffi::webkit_user_media_permission_is_for_display_device(m.to_glib_none().0)`, commented with why (no safe binding in `webkit6` 0.6.1), and calls `media_wanted`.
    *   The `DeviceInfoPermissionRequest` rule (lines 163-171) stays: device labels follow the camera/mic setting, never `content.desktop_capture`.
    *   Unit test `media_wanted_cases`: each table row, plus "display with audio never asks for the microphone setting".

#### 11. `src/web.rs` (Modify, conditional)

*   **Type / Scope**: page settings (B2, only if file 6 is built) and the no-WebRTC check (B4, only if A2 says stock has no WebRTC). If neither applies, this file is not changed.
*   **Proposed Changes**:
    *   In `page_settings()` (line 208): replace the single `ColorFilter` lookup (lines 238-242) with one pass over `all_features()` that turns on `ColorFilter` plus each name in `webkit.features`, and turns off names given as `!Name`. Names not found are collected and printed once per process to stderr as `novabrowser: webkit.features: unknown feature <name>`. `ColorFilter` stays hard-coded on (dark mode needs it, D6).
    *   **Conditional (A2 says stock has no WebRTC):** a page-world user script, top frame only, on the hosts `meet.google.com`, `teams.microsoft.com`, `teams.live.com`, `*.zoom.us` (via the user script's allow list), that posts `nortc` through the existing `nova` handler when `window.RTCPeerConnection` is undefined. Same pattern as `MEDIA_JS` (lines 28-39).

#### 12. `src/tabview.rs` (Modify, conditional on A2)

*   **Type / Scope**: page message handling.
*   **Proposed Changes**: next to the `"media"` arm (line 314), a `"nortc"` arm that shows the message "This WebKit build has no WebRTC: calls need webkitgtk-6.0-nova" once per tab per load.

#### 13. `tests/features.sh` (Modify, conditional on file 6)

*   **Type / Scope**: IPC feature test.
*   **Proposed Changes**: two checks: `:set webkit.features '["NoSuchFeature"]'` then `open -t data:...` loads (unknown names don't break views) and stderr has the `unknown feature NoSuchFeature` line; `:set webkit.features` back to default.

#### 14. `tests/smoke.sh` (No change)

*   **Type / Scope**: listed so it is run, not edited. It already checks the geolocation prompt path that `wanted()` shares.

#### 15. `spikes/wk-build/build.sh` (Modify, conditional on ladder step 6)

*   **Type / Scope**: WebKit build flags.
*   **Proposed Changes**: add the `-D ENABLE_...=ON` flags the spike names, next to the WebRTC flags (line 39). Rebuild on novahome; Speedometer A/B vs 2.54.2-4 into `spikes/w9-baseline/ab-nova5.jsonl`.

#### Phase B: packaging

#### 16. `packaging/aur/webkitgtk-6.0-nova-bin/PKGBUILD` (Modify)

*   **Type / Scope**: AUR binary package.
*   **Proposed Changes**: move `gst-plugins-bad` and `libnice` from `optdepends` (lines 24-25) to `depends`. Add `gst-plugin-pipewire: screen sharing on Wayland` to `optdepends`. Bump `pkgrel` to 5. `_tag` (line 30) is built from `$pkgrel`, so pin it to the existing `webkitgtk-6.0-nova-2.54.2-4` asset; a dependency-only change must not need a new 300 MB upload.

#### 17. `packaging/PKGBUILD`, `packaging/aur/novabrowser-webkit/PKGBUILD`, `packaging/aur/novabrowser-webkit-bin/PKGBUILD` (Modify)

*   **Type / Scope**: browser packages (-git, tag, -bin).
*   **Proposed Changes**: add to `optdepends`: `'gst-plugin-pipewire: screen sharing on Wayland'`, `'xdg-desktop-portal: screen sharing (with a backend: xdg-desktop-portal-hyprland, -gnome or -kde)'`, and any codec packages `RESULTS.md` names (for example `'gst-plugins-ugly: H.264 encoding for calls'`). Change the `webkitgtk-6.0-nova-bin` line to `'webkitgtk-6.0-nova-bin: video calls (WebRTC), dark mode, PGO'` if A2 shows stock has no WebRTC. These ship with the next browser release via `packaging/release.sh` (no change to it).

#### Phase B: docs and project state

#### 18. `docs/user/settings.md` (Modify)

*   **Type / Scope**: user settings reference.
*   **Proposed Changes**: only if file 6 is built: new `### webkit.features` entry (what it does, `!` to turn off, unknown names logged, example), and, only if file 7 is built, a note listing the built-in per-site defaults. Always: `### content.desktop_capture` (line 161): screen sharing is now always asked on its own and never follows the microphone answer.

#### 19. `docs/user/privacy.md` (Modify)

*   **Type / Scope**: user privacy doc.
*   **Proposed Changes**: in the permissions part, one paragraph: screen capture is its own permission; a saved mic or camera answer never covers it.

#### 20. `docs/user/troubleshooting.md` (Modify)

*   **Type / Scope**: user troubleshooting.
*   **Proposed Changes**: a "Video calls" section: needs `webkitgtk-6.0-nova-bin` (if stock has no WebRTC), `gst-plugin-pipewire`, a portal backend; how to collect logs with `GST_DEBUG` / `GST_DEBUG_FILE` and the portal journal; how to try another user agent per site with `:set -u`.

#### 21. `docs/manual-test.md` (Modify)

*   **Type / Scope**: manual test list.
*   **Proposed Changes**: rows for CUJ 1-3: Zoom, Meet, Teams call with camera, mic and screen share from a phone as far side; CUJ 2's "mic allowed, screen still asks" row.

#### 22. `docs/feature-map.md` (Modify, conditional on file 6 or 7)

*   **Type / Scope**: CEF vs WebKit differences.
*   **Proposed Changes**: one line: `webkit.features` is WebKit-only; built-in per-site defaults live in nova-core and apply to both editions.

#### 23. `PROGRESS.md` (Modify)

*   **Type / Scope**: project state (AGENTS.md session rule).
*   **Proposed Changes**: new Phase 0 item `W20 calls (Zoom, Meet, Teams + screen share)` with its pass condition (G1, G2) and evidence; update W16 and 7.15 (screen capture permission); dated decisions (shipped settings in nova-core, display-capture mapping); session log entry in IST; new "Now".

### Verification Plan

*   **Compilation/Build**: `cargo build --release` (no warnings); in nova-core `cargo build`; spike `cargo build` in `spikes/w20-calls/`.
*   **Unit Tests**: `cargo test --workspace` here (new `media_wanted_cases`); `cargo test` in nova-core when files 6-7 are built (new built-in pattern tests, `other_edition` case).
*   **Static Analysis & Linting**: `cargo clippy --workspace` with no new warnings; `bash -n` on the two spike scripts; `makepkg --printsrcinfo` succeeds for all four PKGBUILDs.
*   **Test Coverage Requirement**: every row of the B1 mapping table and every `get_for` precedence case has a unit test.
*   **E2E/Integration**: `tests/smoke.sh`, `tests/features.sh` (with the new checks when file 13 is built) and `tests/passwords.sh` all pass on the release build (pinned to the new nova-core rev when file 9 is built).
*   **Manual (owner)**: CUJ 1-3 pass with a phone as the far side, each with a log bundle; CUJ 2 shows a separate screen prompt on a site with a saved mic "allow"; one call lasts 30 minutes (G1). Results recorded in `spikes/w20-calls/RESULTS.md` and `docs/manual-test.md`.

--------------------------------------------------------------------------------

*Drafted with EGM*