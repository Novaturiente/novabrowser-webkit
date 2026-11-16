# Vendored crates

## webauthn-authenticator-rs 0.6.2 (MPL-2.0, `LICENSE.md`)

From crates.io, used through `[patch.crates-io]` in `../Cargo.toml`. Changes (all marked
`novabrowser-webkit:` in the source):

- `src/ctap2/commands/get_assertion.rs`: `GetAssertionResponse.user_id` reads `user.id`
  (key 0x04) from the authenticator's reply.
- `src/ctap2/ctap20.rs`: the assertion's `userHandle` comes from it (upstream always
  `None`), which username-less passkey sign-in needs (Microsoft).
- `src/authenticator_hashed.rs`: passes it back when proxying.
- `src/ctap2/ctap20.rs`: `perform_auth` / `perform_register` honour `timeout_ms`
  (`with_timeout`, when a tokio runtime is present); upstream ignores it, so a phone that
  connects and never answers blocked the thread forever.

Test the patch (outside the workspace, so the tree stays clean):

```sh
cd vendor/webauthn-authenticator-rs
CARGO_TARGET_DIR=/tmp/wa-target cargo test --features ctap2 --lib response_user_id; rm -f Cargo.lock
```

Drop the patch once upstream parses `user` (still a TODO on master, 2026-11-16).
