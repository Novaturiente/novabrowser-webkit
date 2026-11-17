# W20 calls spike: results

Design: `docs/designs/video-calls.md`. Bundles live in `~/.local/state/novabrowser-webkit/calls/` (not in the repo).

## Runs

| date (IST) | site | entry point | user agent | features | camera | microphone | screen | screen audio | length | bundle | result | cause |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| — | Meet | | | | | | | | 10 min target | | pending | |
| — | Teams | | | | | | | | 5 min target | | pending | |
| — | Zoom | | | | | | | | 10 min target | | pending | |

## Answers

### Q3: which runtime features are off by default (A1, 2026-11-17)

Source: `features-2.54.2-4.tsv` (`cargo run` in this dir on novarch, nova WebKit 2.54.2-4; 555 features; columns identifier, category, status, on by default, on in a fresh `Settings`).

On by default, nothing to do: `PeerConnection`, `ScreenCapture`, `MediaDevices`, `MediaRecorder`, `WebCodecsVideo`, `WebCodecsAudio`, `WebCodecsAV1`, `WebCodecsHEVC`, `WebRTCEncodedTransform`, `WebRTCAV1Codec`, `WebRTCVP9Profile0Codec`, `WebRTCH264HardwareEncoder`, `WasmSIMD`, `WasmRelaxedSIMD`, `CrossOriginOpenerPolicy`, `CrossOriginEmbedderPolicy`, `PermissionsAPI`.

Off by default, candidates for Zoom ladder steps 3 and 5 (`NOVA_WEBKIT_FEATURES`):

| feature | status | why it may matter |
|---|---|---|
| `MediaStreamTrackProcessing` | Mature | `MediaStreamTrackProcessor` (raw camera frames to WebCodecs) |
| `ExposeSpeakers` | Internal | speaker selection lists output devices |
| `WebRTCVP9Profile2Codec` | Mature | VP9 profile 2 in WebRTC |
| `WebRTCH265Codec` | Mature | H.265 in WebRTC |
| `MediaSessionCaptureToggleAPI` | Stable | page-side mute toggles for capture |
| `ImageCapture` | Stable | `ImageCapture` API (camera stills) |

Notes:

- The feature names are `ScreenCapture`, `PeerConnection` and so on, without the `Enabled` suffix seen in `strings` output. `NOVA_WEBKIT_FEATURES=ScreenCaptureEnabled` is reported as unknown (checked 2026-11-17).
- `SharedArrayBuffer` is not a feature switch: pages get it only when they are cross-origin isolated (COOP + COEP headers), which the site controls.

### Q5: does Arch's stock `webkitgtk-6.0` have WebRTC (A2, 2026-11-17)

`stock-webrtc.sh` in a throwaway `archlinux` container: `webkitgtk-6.0 2.54.2-1`; `ldd /usr/lib/libwebkitgtk-6.0.so.4` shows **no** `libgstwebrtc` / `libgstsdp`; the string `PeerConnectionEnabled` is present once (the preference name exists in every build). nova WebKit 2.54.2-4 links `libgstwebrtc-1.0` (checked 2026-11-16).

Answer: [Likely] stock Arch WebKit is built without WebRTC, so calls need `webkitgtk-6.0-nova-bin`. To confirm, open a page that checks `typeof RTCPeerConnection` on a stock install.

### First real calls (user, 2026-11-17): both fail

- **Meet:** the page went blank, then showed "Sorry, we encountered a problem joining this video call."
- **Zoom** (personal meeting room, web client): "Join Audio by Computer" spun, then showed "Unable to use computer audio". Video was also unavailable, although the user had chosen "Always".
- No bundle: the browser was not started through `call.sh`.

### Local probe (`media-probe.html` over `http://localhost:8765`, profile `w20probe`, run outside the agent sandbox, 2026-11-17)

Result for the focused tab:

```
secure:true crossOriginIsolated:false SharedArrayBuffer:undefined RTCPeerConnection:function
VideoEncoder:function AudioWorklet:function getDisplayMedia:function hasFocus:true
devicesBefore: audioinput:-        (no videoinput, no audiooutput)
gum_audio: ok audio/live/Built-in Audio Analog Stereo
gum_video: ERR OverconstrainedError: Invalid constraint
pc (two local RTCPeerConnections, data channel, 3 s): new/new
```

Relevant `browser.stderr` lines (web process, pid 2 inside its namespace):

```
ERROR pipewire gstpipewiredeviceprovider.c:829:gst_pipewire_device_provider_start: Failed to connect
Video capture was requested but no device was found amongst 0 devices
webrtcbin ... ICE gathering state change from new(0) to gathering(1)      (never completes, no candidates)
GStreamer element audiornnoise not found. Please install it
```

Findings:

1. **No camera in the web process.** The PipeWire device provider runs inside the bubblewrap sandbox and can't reach `$XDG_RUNTIME_DIR/pipewire-0`. There is also no v4l2 fallback, because `/dev/video*` is not in the sandbox. So `videoinput` is never listed, and `getUserMedia({video})` fails with `OverconstrainedError`. The microphone works because PulseAudio's socket is in the sandbox.
2. **WebRTC has no network in the web process.** Every `WebKitWebProcess` has its own network namespace: `readlink /proc/<pid>/ns/net` differs from the UI and network processes, which share the host's. nova WebKit is built with `USE_LIBRICE=OFF` (`spikes/wk-build/build.sh:41`), so ICE runs through libnice inside the web process, where only loopback exists. ICE gathering starts and never yields a candidate, so no peer connection can connect. [Likely] This is why Meet fails to join and Zoom can't join computer audio. The W16 Meet check on 2026-11-04 ran from `/tmp` libs **without the sandbox**, which is why it didn't show this.
3. The focus rule (`GetUserMediaRequiresFocus`, on by default): `enumerateDevices`/`getUserMedia` wait while the page has no focus. This is not the cause for the user (their window was focused), but automated probes must focus the window.

Fix options (owner decision needed):

- **A. Rebuild nova WebKit with `USE_LIBRICE=ON`.** ICE sockets are then proxied through the network process (the `WebRTCSocketsProxying` feature, on by default). Needs the `librice` C libraries (rice-proto and rice-io, Rust with a C API), which are in no Arch repo, so they must be packaged as well. The camera still needs fix C.
- **B. Turn off the web-process sandbox** (`WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS`). Rejected for a daily driver: every page would run unsandboxed.
- **C. Camera:** expose `pipewire-0` to web processes with `add_path_to_sandbox` (`src/web.rs:113` already does this for the adblock dir). Cost: a compromised web process could then open any PipeWire stream (microphone, camera, screen casts) without a prompt. The alternative is WebKit's camera portal path (`org.freedesktop.portal.Camera` is in the library); it is unknown when WebKit uses it outside Flatpak.

### Q1: screen audio — pending (call runs)

### Q2: flags on a screen request — pending (call runs; see `media request` lines in `browser.stderr`)

### Q4: what Zoom needs — pending (Zoom ladder)

## Phase B changes required

Blocked: calls can't connect until fix A, or an equivalent, lands (see "Local probe" above). Known so far:

- Stock WebKit has no WebRTC [Likely]: the browser `optdepends` line for `webkitgtk-6.0-nova-bin` should say it is needed for video calls (design file 4), and `docs/user/troubleshooting.md` should say so (design file 19).
- Features to ship: none yet.
- Per-site defaults: none yet.
- Codec packages: none yet.
- Build flags: none yet.
