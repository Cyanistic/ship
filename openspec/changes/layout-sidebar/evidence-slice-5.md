# Slice 5: new panes where you are, and child tabs

Tasks 5.1–5.5 are implemented and verified on macOS. The release build passed 106 temporary workflow checks against owned servers, run once with this host's SSH variables and once without, plus a lookup benchmark and an idle CPU check. Linux is unverified and was not compiled. Windows is unverified and has no working-directory lookup by design. Slice 6 has not started.

## Implementation

- `crates/ship-core/src/model.rs`: `Pane.dir`, the current directory, optional and separate from the starting `cwd`.
- `crates/ship-server/src/pane.rs`:
  - `PaneChange::Dir` carries the directory from OSC 7 (`local_path`, any host, as before the slice's short-lived local-host check).
  - Until a pane's shell reports OSC 7, each `publish` also reads the program's working directory through `process_cwd` and sends a change only when it differs. The task stops reading once the program is reaped.
  - `process_cwd` is `std::fs::read_link("/proc/PID/cwd")` on Linux, `libc::proc_pidinfo(PROC_PIDVNODEPATHINFO)` on macOS, and `None` elsewhere. The macOS branch is Ship's first `unsafe` block outside `vendor/`, with `SAFETY` comments as the workspace lint requires.
- `crates/ship-server/Cargo.toml`, `Cargo.toml`: `libc` as a macOS-only `ship-server` dependency. It was already in the lock file through tokio, nix and the vendored wrapper, so nothing new is built.
- `crates/ship-server/src/state.rs`: `Dir` updates the pane record like `Title`. Key-made panes, which send no `cwd`, start in the anchor's `dir`, else its `cwd`. CLI creation keeps sending the caller's directory or `--cwd`.
- `crates/ship-core/src/command.rs`: `ship tab create --child` and `--pane`. `--pane` is read only with `--child`, because inside a pane `SHIP_PANE_ID` fills it on every `ship tab create` (G1). `--child` conflicts with a parent, `--before` and `--after`.
- `crates/ship-client/src/execute.rs`: `current_tab` resolves the tab holding `--pane`, else, on keys, the selected tab or the selected pane's tab. `--child` places the new tab as its last child, or at the top level with none.
- `crates/ship-client/src/keymap/defaults.toml`: `alt-shift-n` creates a child tab with a starter shell.
- `crates/ship-client/src/ui/draw.rs`: `tab_label` gives an unnamed tab the last component of its first pane's `dir`, `~` for home, else that pane's label, else its position.
- `vendor/ratatui-ghostty/src/session.rs`, `PROVENANCE.md`: change 11. `CwdChanged` now comes from libghostty's `on_pwd_changed` callback, as titles do. The old `pwd()` poll ran only after draining queued output, so an idle shell's first prompt report was missed.
- `README.md`: the status paragraph, `alt-shift-n` and the directory-aware split in the keys table, a "Where new panes start" section and child tabs and labels under "Working in the tab tree", each with a runnable recipe.
- Papers: architecture A-7 and program P-4 record the working-directory fallback, approved by Cyan in chat. The pane-programs and terminal-client specs, `design.md` and tasks 5.1, 5.2 and 5.4 follow them.

Protocol stays 8. `dir` is an optional field that older clients ignore, and `--child`/`--pane` are client-side. No permanent authored test code was added.

## Evidence

Temporary harness and logs: `/tmp/ship-slice5/`. `harness.py` launches owned servers on random ports and full-screen clients in real PTYs, rendered by pyte, with a TCP proxy logging one client's requests. Final runs used the release binary. Panes run Cyan's real zsh with oh-my-zsh.

This host's Claude session was started over SSH, so its environment carries `SSH_CLIENT` and `SSH_TTY`. oh-my-zsh defines no OSC 7 hook with those set, so every pane of Cyan's zsh takes the lookup path. Each probe ran twice: as inherited ("SSH"), and with `SSH_CLIENT`, `SSH_CONNECTION`, `SSH_TTY` and `SSH_COPY_PORT` stripped ("local"), where zsh reports OSC 7.

### Directories, splits and child tabs: 40/40 in each case (`slice5.py`)

- `alt-shift-n` with nothing selected makes a top-level tab and selects its shell.
- zsh's first `dir` is home, with `cwd` home. After `cd /tmp`, `ship pane get` shows `dir` `/tmp` locally and `/private/tmp` over SSH, and `cwd` stays home. `alt--` then starts a selected shell there, whose `pwd` agrees.
- `zsh -f` loads no rc and never sends OSC 7. Its `dir` reads `/usr`, its `cwd`. After `cd /usr/share` its `dir` follows in 0.02 s, and an `alt--` split of it starts in `/usr/share`.
- A zsh whose OSC 7 hook is removed with `add-zsh-hook -d precmd omz_termsupport_cwd`, followed by `cd /usr/lib`: locally `dir` keeps its last report, `/usr/share`, and a split starts there; over SSH it had never reported, so `dir` follows to `/usr/lib` and the split starts there.
- `ship pane create --pane P1` run from `/var` starts in `/var`, although P1's directory is `/tmp`; with `--cwd /usr` it starts in `/usr`. The proxy log shows key-made pane requests carrying no `cwd`.
- `alt-shift-n` on a pane appends a child to its tab, and again from that child, making three levels. From the top tab it appends a second and third child in order. On a selected empty tab it adds that tab's child. Each time the new shell is selected.
- `ship tab create --child` inside a pane (`SHIP_PANE_ID`) appends to that pane's tab; plain `ship tab create` there stays top-level; `--child --pane P5` from outside appends to P5's tab; `--child` with no pane is top-level. `--child` with `--parent`, a positional parent, `--before` or `--after` exits 2, and the tree is unchanged.

### Labels: 9/9 in each case (`label.py`)

In a 160x40 client:

- A new shell's row reads `~`.
- After `cd ~/Documents/projects/ship` it reads `ship`, in 0.04–0.05 s SSH and 0.05–0.06 s local.
- After `cd ~` it reads `~` again, in 0.04–0.05 s.
- `ship tab rename` wins over the directory, and clearing the name returns to it.
- An unviewed tab running `sleep 100` reads `sleep`: it has never drawn, so nothing read its directory. Once navigation passed over it, the resize's publish read `/usr`, and the row read `usr`.
- An unviewed `zsh -f` tab reads `usr`. After a `cd /usr/share` in it, the row reads `share` in 0.03–0.05 s.

### Lookup latency, cost and idle

- `poll.py`, 8/8: `cd /usr/share` reached `dir` in 0.01 s in both cases. The `/tmp` result stayed `/tmp` locally, so the lookup never overrides OSC 7, and stayed `/private/tmp` over SSH. With the one-second timer tried before A-7, the SSH case took 0.73 s.
- `cwdbench/`, a temporary Rust crate running the macOS branch: 0.77 µs per lookup over 100,000 lookups in release. A pane at the 4 ms frame ceiling would spend about 0.2 ms per second on it. `sysinfo`'s cwd refresh measured about 0.4 ms, because it lists every process on each call.
- `idle.py`, with 40 panes of zsh that never report (SSH case), release server: 0.00 s of CPU over 20 idle seconds. The timer used 0.34 s.

### Documentation

`docs.py` ran the README's two new recipes through `ship` and `jq` against owned servers, in both cases. The directory recipe printed `cwd` and `dir` as `/usr/share`. The tree recipe's `--child --pane` printed a `logs` tab running `/bin/zsh`, `checks` listed it as its only child, and closing the project left no tabs. `ship tab create --help` shows `--child` and `--pane` with their descriptions.

### Checks and size

Passed on this macOS host:

- `cargo build -p ship`
- `cargo build --release -p ship`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check -p ship-core`
- `cargo check -p ship-core --features clap`
- `openspec validate layout-sidebar --strict`
- `git diff --check`

Measured project implementation Rust (all Rust under `crates/`, excluding test paths): HEAD 7,950 lines, then 8,109, +159. Authored test Rust: 0 lines before and after. The vendored wrapper's change 11 is outside that count; its 80 tests pass, run in a copy outside the repo.

All harness servers and clients were shut down. Cyan's Ship server and Herdr server were left untouched.

## Unverified

- Linux. The `/proc/PID/cwd` branch has not been compiled, because this host has no Linux target installed.
- Windows. It relies on OSC 7 alone by design (A-7); nothing was run.
- A person looking at a GUI terminal. Appearance checks used pyte-rendered output from real client PTYs.

## Uncertain decisions

- A shell that has reported OSC 7 once is never looked up again, so disabling its hook later freezes `dir` at the last report.
- Paths from the lookup have symlinks resolved, so `/tmp` shows as `/private/tmp` on macOS. Labels read the same.
- The lookup runs on every publish, including resizes, not only on output. A silent program gains a `dir` once it is viewed.

## Follow-ups

- New tabs' starter shells, from `alt-n`, `alt-shift-n` or `--starter`, still start in home. Cyan prefers them to start in the selected pane's directory, as Herdr's do, and deferred it. It needs an anchor and a `cwd` on `CreateTab`, a public-interface change.
- The Herdr comparisons in pane-terminals D56 ran with Cyan's config, whose `[experimental] reveal_hidden_cursor_for_cjk_ime = true` forces full renders. Default Herdr is probably faster than those figures; not measured.

```text
reused:   PaneChange/commit for titles, the key-split anchor path, tree placement for child tabs
new code: the cwd lookup and its fallback rule, --child/--pane targeting, directory labels, vendor change 11
proof:    106 release workflow checks in both SSH environments, a 0.77 µs lookup benchmark, zero idle CPU, strict Clippy, fmt, builds and core feature checks on macOS
```
