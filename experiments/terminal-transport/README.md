# Terminal transport experiment

This is the durable copy of Ship's scratch terminal transport benchmark. It preserves the original native captures, measurements, reports, scripts, and independent parent verification. Saved-capture runs now build from relative vendored Rust dependencies and write into new ignored directories, not the original evidence.

This is an **independent Cargo workspace, not production code**. The current research direction is compressed full terminal snapshots, with changed-only publication, bounded cadence and coalescing. Application-state replication is undecided. This import does not adopt a protocol or choose Ship's production dependency delivery strategy.

Start with the [unaltered research handoff](../../docs/research/terminal-transport-handoff.md), [compression report](compression-report.md), and [durable-copy verification](VERIFICATION.md). The handoff and old reports retain historical `/tmp` paths intentionally; use the mapping below for their preserved equivalents.

## Build and run saved captures

From the Ship repository root:

```sh
cd experiments/terminal-transport
cargo metadata --locked --format-version 1 > /dev/null
cargo build --release --locked --target-dir target
python3 scripts/run-saved.py --size 160x50 --mode compression
```

The runner creates a unique `.runs/<timestamp>-<size>-<mode>-<suffix>/` for each invocation. It links only the saved `size-results/` inputs into that directory and sets the binary's working directory there. Measurements, encoded messages, stdout and run metadata all stay in the new directory. **It does not spawn a PTY or overwrite historical results.** Nonzero exits or timeouts make the runner fail; the log and metadata remain available.

Supported sizes: `80x24`, `120x40`, `160x50`, `80x50`, `160x25`. Supported modes:

```sh
# Default mode is compression: raw, gzip-1, zstd-1, all four strategies.
python3 scripts/run-saved.py --size 160x50
# The original saved-size comparisons, in independent bounded processes:
python3 scripts/run-saved.py --size 160x50 --mode json
python3 scripts/run-saved.py --size 160x50 --mode default
python3 scripts/run-saved.py --size 160x50 --mode ordered-probe
python3 scripts/run-saved.py --size 160x50 --mode ordered
# Optional full compression sweep, using the five original saved sizes:
for size in 80x24 120x40 160x50 80x50 160x25; do
  python3 scripts/run-saved.py --size "$size" --mode compression || break
done
```

Timeouts preserve the original bounded approach: compression 90 seconds, JSON 45 seconds, default 15 seconds, ordered probe 10 seconds, ordered sampling 60 seconds. Ordered alignment is quadratic; performance varies by hardware. These are process limits, not performance guarantees. See the original reports for sample settings and timing boundaries.

**Do not run the binary or historical Python runners from the experiment root.** The unchanged Rust harness writes relative to its current directory. `compression-results/run-compression.py`, `compression-results/report.py`, and `structdiff-size-results/run-saved.py` are preserved historical scripts; running them in place can overwrite evidence. Use `scripts/run-saved.py` for new saved-capture measurements. The native default and `--sizes` modes remain available in the original harness but are not exercised by this runner or this import.

## Toolchains and offline limits

Verified on macOS arm64 with Rust/Cargo 1.98.0 and Zig 0.16.0, using a new repository-local `target/` directory. You need Python 3, Cargo/Rust, Git, Zig, and the host's native build tools. No toolchain provisioning or other-platform build was performed here.

The three previously temporary path dependencies are preserved under `vendor/`; their exact revisions, compatibility patch, and licenses are documented in [vendor/PROVENANCE.md](vendor/PROVENANCE.md). The root `Cargo.lock` is unchanged. No build cache or native archive was copied from scratch.

This is **not a fresh offline build bundle**. Registry crates are locked but not vendored. The native bindings' build script fetches Ghostty at `22d13172cde98a0a4dda05d3d6a3fcb0dd8ed018` into `target/` and invokes Zig, which may need package downloads. Cargo `--offline` does not stop native build-script network access. The verification build fetched that Ghostty revision and used installed toolchains plus the host's existing Cargo registry/default Zig cache. No scratch-native source override was used. For native source/package overrides, see provenance and the unchanged upstream build script.

## Preserved layout and historical path map

| Historical source | Durable location |
| --- | --- |
| `/private/tmp/ship-jsonpatch-prototype` | This experiment root, with exceptions below |
| Prototype `Cargo.toml` | `archive/original/Cargo.toml`, byte-identical; active root manifest has relative paths/workspace exclusions |
| Prototype `README.md` | `archive/original/README.md`, byte-identical; this README supplies durable instructions |
| Prototype `src/`, `Cargo.lock`, captures, logs and reports | Same relative paths, unchanged |
| Worker `size-results/`, `structdiff-size-results/`, `compression-results/` | Same relative paths, unchanged, including scripts and source snapshots |
| Parent `parent-verification/`, `parent-size-verification/`, `parent-structdiff-verification/`, `parent-compression-verification/` | Same relative paths, unchanged |
| `/private/tmp/ship-screen-prototype/wrapper-compat` | `vendor/wrapper-compat/` |
| `/private/tmp/ship-screen-prototype/libghostty-rs` | `vendor/libghostty-rs/`, without `.git` |
| OSS `research/ship-json-patch-handoff.md` | `../../docs/research/terminal-transport-handoff.md`, byte-identical |

The original screen-prototype harness and the broader ecosystem survey were not imported; the handoff still references them historically. Only its required wrapper and bindings sources were included here. The original parent verification input symlinks (`../size-results`) are unchanged and resolve within this copy.

`preservation/source-comparison.json` records every imported source file's SHA-256, byte size and destination mapping, plus literal symlink targets and the handoff hash. All **328 prototype files and two symlinks, 21 wrapper files, and 52 binding files** matched the scratch originals. The imported file data totals **109,356,283 bytes (104.3 MiB)**, before new documentation and preservation records. Full tree copying, not Git-tracked-only export, retained the supplied patch and untracked evidence.

Excluded: the prototype's approximately 587 MiB `target/`, bindings `.git`, and named build/cache artifacts. The experiment `.gitignore` covers local `target/`, `.runs/`, Python caches, and optional vendored build caches. No original generated evidence or FFI binding file was edited. New documentation, the relative-path root manifest, one safe runner, a generated wrapper patch and preservation records are the only additions/configuration changes.

## What this proves, and what it does not

The preserved benchmark demonstrates exact saved-screen reconstruction and codec flushing in memory. It is not a rendered UI, browser, HTTP/proxy/SSE delivery, sustained-load, memory, or production lifecycle test. Historical reports explain the measured CPU and bandwidth tradeoffs; do not treat independently measured component medians as additive pipeline totals.

Some older Ship planning/research documents still describe earlier structdiff/JSON Patch directions and scratch paths. Reconciliation with the compressed-snapshot handoff is future documentation work, not part of this import. Production schemas, publication rate/backpressure, reconnect semantics, screen versus scrollback scope, client compression support and application-state replication remain unchosen.

reused: original Rust capture/benchmark harness, saved screens, worker/parent evidence, lockfile and supplied dependency sources.
new code: one Python runner isolates future saved-capture outputs; root manifest changes only relocate dependencies and isolate workspaces.
proof: locked repository-local release build, successful 160x50 saved compression/JSON runs, exact preservation comparison and scoped format checks in VERIFICATION.md.
