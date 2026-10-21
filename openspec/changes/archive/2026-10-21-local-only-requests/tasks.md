# Tasks

Verification uses real commands and temporary probes, not permanent authored tests. Checks run on macOS; Linux is unverified unless run there.

## 1. Refuse requests that aren't local

- [x] 1.1 Add `ErrorCode::Forbidden` with status 403 in `crates/ship-core/src/error.rs`.
- [x] 1.2 Add `local_only` to `crates/ship-server/src/middleware.rs` and layer it over the whole router in `crates/ship-server/src/lib.rs`, inside the request tracing so refusals are logged.
- [x] 1.3 Add the refusal to the guide's "Using Ship on another machine" section.
- [x] 1.4 Verify against a server on a spare port:
  - `ship --server-url http://127.0.0.1:<port> server status` succeeds, and `ship` attaches and types normally.
  - curl with `Host: localhost:<port>`, `Host: [::1]:<port>` and `Host: 127.0.0.2:<port>` gets 200 on `/api/v0/tabs`.
  - curl with `Host: attacker.example:<port>` gets 403 with `.type.kind` equal to `forbidden`.
  - curl `-X POST /api/v0/server/stop` with `Origin: https://attacker.example` gets 403, and the server keeps running.
  - curl over HTTP/2 (`--http2-prior-knowledge`) with a loopback address gets 200, and with a foreign `Host` gets 403.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate local-only-requests --type change --strict`.
