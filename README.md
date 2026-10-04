# Ship

Ship checks a server's health and can run a local loopback server. Running `ship` without arguments reuses the default server or starts a missing one in the background. The background server stays running after the client exits or its terminal closes.

This checkpoint provides health requests and server startup/shutdown, not terminal hosting. The lifecycle workflow has been exercised on macOS arm64. Other platforms are not yet verified.

## Prerequisites

- A current stable Rust toolchain, including Cargo, with support for Rust 2024 and Cargo resolver 3.
- The `rustfmt` and Clippy components.
- A native C toolchain and linker for dependencies. On macOS, install the Xcode Command Line Tools with `xcode-select --install` if needed.
- CMake if required by the resolved TLS dependency's native build.
- Access to crates.io for the first dependency download, or a populated local Cargo cache.

With Rust installed through rustup, add the check tools:

```sh
rustup component add rustfmt clippy
```

## Build

From the repository root:

```sh
cargo build --workspace
cargo build --workspace --release
```

The executable is `target/debug/ship` or `target/release/ship`. The examples below use the debug binary; either works.

## Check health

```sh
./target/debug/ship
./target/debug/ship --help
./target/debug/ship --version
```

Bare `ship` uses `http://127.0.0.1:43179`. It starts a server only after the HTTP health request reports a positive connection refusal. If the port is occupied by an incompatible or unhealthy listener, Ship fails without replacing or stopping it. Service/protocol checks detect accidental mismatches; they are not authentication.

Success prints one JSON line on stdout:

```json
{"service":"ship","protocol_version":1,"version":"0.1.0"}
```

The package version is informational. Diagnostics go to stderr. Operational failures exit 1; invalid command usage exits 2. Help and version do not need or start a server.

## Run an explicit server or choose a target

Run a foreground server in one terminal:

```sh
./target/debug/ship server
# Or choose a nonzero port:
./target/debug/ship server --port 43180
```

Check a custom-port server from another terminal:

```sh
./target/debug/ship --server-url http://127.0.0.1:43180
```

An explicit `--server-url`, even `http://127.0.0.1:43179`, is connect-only. It never launches a local server or falls back to one. URLs must be absolute HTTP/HTTPS URLs with a host. Supplied paths, queries and fragments are discarded: health always uses `/health` at the target's root. Redirects are refused, HTTPS certificates are verified, and URL credentials are omitted from diagnostics.

The server binds only to loopback, with no public host option. Port zero is rejected. Do not combine `--server-url` with `server`.

To build and run through Cargo:

```sh
cargo run -p ship --
cargo run -p ship -- server --port 43180
```

## Logs and shutdown

Automatic startup reports the launched child PID and a unique `ship-server-*.log` path in the OS temporary directory. The log has Unix mode 0600 and is retained after client exit or startup failure, until OS or manual temporary-file cleanup. When concurrent launches race, another compatible server may win; the launch message says so, and losing children exit. Inspect the reported log for that attempt's outcome and the serving server's startup PID.

Startup logs show address, PID and protocol version. Request logs show method, path, status and duration. The default filter is `info`. Set `RUST_LOG` to change it; an invalid filter fails rather than being ignored. For example:

```sh
RUST_LOG=debug ./target/debug/ship server
```

Stop a foreground server with Ctrl-C. To stop a background server, use its recorded serving PID:

```sh
kill -TERM <PID>
# SIGINT is also supported:
kill -INT <PID>
```

Normal shutdown drains requests within five seconds and stops accepting connections. There is no server-management command or shutdown endpoint. A later bare `ship` can start a new server; use an explicit `--server-url` if you only want to check whether the stopped endpoint is reachable.

Local readiness is bounded by five seconds after launch. HTTP connections have a one-second bound, and complete health requests have a two-second bound. Failed startup cleans up only its own child and retains the log. If log creation fails, the error identifies the attempted directory without claiming a log exists.

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --workspace --release
```
