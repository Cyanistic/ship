# Tasks

Verification uses real commands and temporary probes, not permanent authored tests. Checks run on macOS; Linux is unverified unless run there.

## 1. Print completion scripts

- [x] 1.1 Add `Command::Completions` in `crates/ship/src/cli.rs` and generate the script with `clap_complete::generate` from its arm in `crates/ship/src/main.rs`.
- [x] 1.2 Add a Shell completions section to the guide.
- [x] 1.3 Verify:
  - `ship completions <shell>` exits 0 for bash, zsh, fish, elvish and powershell, and an unknown shell exits 2.
  - The bash and zsh scripts pass `bash -n` and `zsh -n`; sourced into bash, they complete subcommands and shell names; sourced into zsh after `compinit`, they register `_ship`.
  - `ship --help` lists `completions`.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate shell-completions --type change --strict`.
