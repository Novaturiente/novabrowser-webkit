# ext/src — web process extension

## Overview
`lib.rs` is the whole web-process extension (crate `novabrowser-webkit-ext`,
built as a `cdylib`). WebKit loads it into every web process. On each
`send-request` it applies adblock network rules, the HTTPS-only upgrade and
custom request headers. It also answers memory-stat queries. Config and
per-page navigation state arrive from the UI process (`src/`) as WebKit user
messages.

## Interface
- `webkit_web_process_extension_initialize_with_user_data` (`lib.rs:43`) — C entry point; optional GVariant string = initial cfg JSON; connects extension and page signals.
- Extension user message `cfg` (`lib.rs:57`) — runtime config update (JSON string), handled by `set_cfg(json)` (`lib.rs:78`): engine path, whitelist, excludes, optional `reload`.
- `on_page_message` (`lib.rs:103`) — page message `nav` stores per-page state; `mem` replies with RSS and GPU MiB.
- `connect_send_request(page)` (`lib.rs:162`) — raw glib connect of `send-request` with a manual trampoline.
- `on_send_request(page, req, redirected) -> bool` (`lib.rs:186`) — `true` cancels (blocks) the request; otherwise may upgrade to https and replace headers.
- `should_block(url, source, kind) -> bool` (`lib.rs:257`) — whitelist check, then `Engine::check_network_request`.
- `guess_type(url, accept) -> &str` (`lib.rs:138`) — Accept header or file extension to adblock request type.
- `status_kib(status, key)` / `gpu_kib(fdinfos)` (`lib.rs:223-229`) — parse `/proc/self/status` and DRM fdinfo.
- `Cfg` (`lib.rs:19`), `PageState` (`lib.rs:26`); thread-local `ENGINE`, `CFG`, `PAGES` (`lib.rs:32-36`).

## Invariants
- Each `cfg` replaces the whole config. A missing `whitelist` or `excludes` key clears that list. A missing `engine` key sets the path to `""`, which counts as a change and sets `ENGINE` to `None` (blocking off) (`lib.rs:87-93`). A `cfg` that is not valid JSON is ignored silently (`lib.rs:79`).
- The engine reloads only when the engine path changes or `reload: true` is sent; a rewritten engine file at the same path needs `reload: true`.
- An unreadable or undeserializable engine sets `ENGINE` to `None`, which silently disables network blocking.
- Whitelist entries that fail `UrlPattern::parse` are dropped silently.
- The `nav` message must arrive before the navigation's first `send-request` (same IPC connection keeps order). Without it, `PageState` defaults to `main = ""`, `block = false`: no blocking, no headers. A `nav` that fails to parse keeps the previous state.
- When the main document redirects, `PageState.main` follows the new URL.
- Adblock never blocks the main frame and applies only when `PageState.block` is true.
- `should_block` matches the whitelist against the request URL, not the page URL, and always passes method `"get"` to adblock (`lib.rs:258-263`).
- The HTTPS upgrade applies only to GET/HEAD (or no method) and honours excludes via `nova_core::net::https_upgrade_with`.
- Custom headers are replaced on every request, including redirects, after the upgrade.
- A page's `PAGES` entry is removed when the `WebPage` is disposed (weak-ref notify set in `connect_page_created`; `try_with` because disposal can run during thread teardown). The trampoline's `Box::new(())` user data is zero-sized, so it allocates nothing.
- `redirected` is NULL for non-redirect requests; the trampoline checks it.
- `mem` reply is `(rss_kib >> 10, gpu_kib >> 10)`.

## Side Effects
- Reads the serialized adblock engine file on each reload.
- Reads `/proc/self/status` and every `/proc/self/fdinfo/*` for `mem`.
- Sends the `mem` reply message to the UI process.
- Rewrites request URIs (`set_uri`) and outgoing headers in place.
- Registers a raw GObject signal handler on each `WebPage`.
- No network, sockets, env vars or disk writes.

## Verification
- Run: `cargo test -p novabrowser-webkit-ext` (inline tests at the end of `lib.rs`).