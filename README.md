# Ship

Ship currently provides a buildable command-line foundation with help and version output. Running it without arguments displays help. Server startup and health requests are not available yet.

## Prerequisites

- A current stable Rust toolchain, including Cargo, with support for Rust 2024 and Cargo resolver 3.
- The `rustfmt` and Clippy components.
- A native linker. On macOS, install the Xcode Command Line Tools with `xcode-select --install` if needed.
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

The executable is `target/debug/ship` or `target/release/ship`.

## Run

No running server is required:

```sh
./target/debug/ship --help
./target/debug/ship --version
```

To build and run through Cargo:

```sh
cargo run -p ship -- --help
cargo run -p ship -- --version
```

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --workspace --release
```
