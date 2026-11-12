# ext

## Overview
Workspace member crate `novabrowser-webkit-ext` (`ext/Cargo.toml`): the WebKit
web-process extension, built as `cdylib` (loaded by WebKit) and `rlib` (for
tests). It depends on `webkit-web-process-extension6`, `adblock`, `serde_json`
and the same pinned `nova-core` rev as the main crate.

UI-process side (`src/web.rs:104-151`):
- It looks for `libnovabrowser_webkit_ext.so` in `$NOVA_WEBKIT_EXT_DIR`, then
  the binary's own dir, then `/usr/lib/novabrowser-webkit`. Not found → it prints
  `novabrowser-webkit: web process extension not found; no network adblock or
  subresource HTTPS-only` and runs without them.
- It adds `adblock::dir()` to the web-process sandbox so the extension can read
  `engine.dat`.
- It passes `ext_config(true)` as init user data; later changes come as `cfg`
  user messages.

## Subdirectories & Boundaries
- src: [`./src/EGM.md`](./src/EGM.md) — the whole extension in `lib.rs`: adblock network blocking, HTTPS-only upgrade, request headers, memory stats; config arrives from the UI process as user messages.
