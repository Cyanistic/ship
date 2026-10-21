# Tasks

Verification uses real commands and temporary probes, not permanent authored tests. Checks run on macOS; Linux is unverified unless run there.

## 1. Print the defaults

- [x] 1.1 Add `ConfigCommand::Defaults` in `crates/ship/src/cli.rs` and print `ship_core::config::DEFAULTS` from its arm in `crates/ship/src/main.rs`.
- [x] 1.2 Add the "copy only what you change" note to the header of `crates/ship-core/src/defaults.toml`, and show the command in the guide's Configuration section.
- [x] 1.3 Verify:
  - `ship config defaults` matches `defaults.toml` byte for byte and exits 0, with no server running.
  - Its output passes `ship config check`.
  - With `SHIP_CONFIG` pointing at a broken file, it still prints and exits 0.
  - `ship config --help` lists it.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate config-defaults --type change --strict`.
