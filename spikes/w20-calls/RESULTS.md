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

### Q1: screen audio — pending (call runs)

### Q2: flags on a screen request — pending (call runs; see `media request` lines in `browser.stderr`)

### Q4: what Zoom needs — pending (Zoom ladder)

## Phase B changes required

Pending the call runs. Known so far:

- Stock WebKit has no WebRTC [Likely]: the browser `optdepends` line for `webkitgtk-6.0-nova-bin` should say it is needed for video calls (design file 4), and `docs/user/troubleshooting.md` should say so (design file 19).
- Features to ship: none yet.
- Per-site defaults: none yet.
- Codec packages: none yet.
- Build flags: none yet.
