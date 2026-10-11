# Slice 5: docs, deferred notes and size

## Result

README commands, keys, mouse/clipboard behavior, settings and protocol-9 compatibility are documented. `AGENTS.md` distinguishes client-local tab/pane viewing from shared scroll position and text selection. The product brief records `ship pane read` as the temporary stand-in for milestone 1's search evidence.

Cyan replaced the five-issue requirement during this slice: drop the 10 KB follow-up and retain four archived, unfiled notes. [Copy mode](deferred-notes/copy-mode.md), [search](deferred-notes/search.md), [edit-scrollback](deferred-notes/edit-scrollback.md) and [run/clipboard](deferred-notes/run-and-clipboard.md) preserve findings and unresolved choices without becoming backlog items or commitments. No GitHub issue was created. The product amendment, proposal, design summary, program paper and task 5.3 now agree.

## Command verification

Temporary probe `/tmp/ship-docs-slice5.py` extracted README shell blocks and ran all nine CLI example blocks as written using the debug binary on PATH. `SHIP_SERVER_URL` selected an owned loopback server on an ephemeral port, and `SHIP_CONFIG` selected a temporary config. Installation commands and the interactive client launch were not part of this command check.

The blocks covered tab/pane creation and inspection, splitting/closing, directory inspection, zoom/swap/resize, nested tabs, child creation, every newly documented scroll/read command, ANSI output through `cat`, and a read through `grep` matching line 150. The numbered-output pane exited before the reads. The probe checked output containing lines 151 onward and ending at 200. It also ran the two documented help commands, config validation and server status, whose protocol was 9. Config validation passed with both `mouse = true` and `false`, and with history caps of 10,000,000 and 20,000. It did not repeat live config reload behavior already checked in earlier slices.

Artifacts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-docs-slice5-1c3ywlgx/`, including `readme-commands.sh`, `commands.stdout`, `commands.stderr` and `server.log`. The probe stopped its own server and asserted a successful exit; existing user work was not restarted.

## Build and dependency checks

Passed on macOS, on the implementation at `9166d9c` (this slice changes Markdown only):

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace`
- `cargo build --workspace --release`
- `cargo check -p ship-core --no-default-features`
- `cargo check -p ship-core --features clap`

Wrapper tests ran in `/tmp/ship-wrapper-slice5.ukU7ex/wrapper`, a copy outside the repo with a standalone workspace/lint table added only to that copy. `cargo test` passed 80 tests; one pre-existing crash test remained ignored. Log: `/tmp/ship-wrapper-slice5.ukU7ex/tests.log`.

Linux builds and live execution remain unverified in this slice. Prior user-confirmed Linux-terminal-over-SSH checks are recorded in the slice-3 and slice-4 evidence; they do not constitute a Linux Ship build.

## Probe findings retained in the notes

Reran the existing dependency experiment with `cargo run --release --manifest-path experiments/scrollback-probe/Cargo.toml`. Log: `/tmp/ship-scrollback-probe-slice5.log`. No new test or experiment Rust was authored.

The notes retain tracked-reference pruning, word selection, the adjustment API's incomplete movement evidence, alternate-screen history behavior, formatting/search timings with the byte cap lifted, display-width mapping, and the failed row-wise match across a soft wrap. They explicitly distinguish callable APIs and synthetic timings from a verified future implementation. The run and edit-scrollback notes link inspected primary references for tmux, Zellij, Herdr and kitty, and leave consequential interfaces and lifecycle choices unsettled.

## Size report

`cloc` 2.10 counts Rust code lines, excluding comments and blanks. Baseline is planning commit `452301a`, immediately before slice 1; current implementation is `9166d9c`. The baseline was extracted with `git archive` into `/tmp/ship-slice5-size.qZPBcD`, without switching the working tree.

| Category | Before | After | Net change |
| --- | ---: | ---: | ---: |
| Ship implementation Rust (`crates/`) | 6,712 | 7,287 | +575 |
| Ship authored test Rust | 0 | 0 | 0 |
| Ship authored total, implementation plus tests | 6,712 | 7,287 | +575 |
| Wrapper implementation (`vendor/ratatui-ghostty/src/`), uncounted | 1,453 | 1,773 | +320 |
| Wrapper tests, uncounted | 988 | 988 | 0 |
| Wrapper examples, uncounted | 1,206 | 1,212 | +6 |

Commands: `cloc --json --include-lang=Rust crates`, repeated on the baseline copy, and separate calls for wrapper `src`, `tests` and `examples`. No `#[test]`, `#[cfg(test)]` or `tokio::test` code was found in `crates/`. Build-generated Rust and dependency sources outside these roots are excluded, as are experiments and temporary verification scripts.

The Ship change exceeds the program/architecture estimate of 320 to 420 lines by 155 lines above its upper bound, about 37%. The 7,287-line implementation remains below the roughly 20,000-line ceiling. The wrapper's +320 implementation lines are close to the separately estimated +300, but are not used to hide the counted overrun. No working behavior was cut or moved into tests to fit the estimate.
