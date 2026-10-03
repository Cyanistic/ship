# Durable-copy verification

The repository copy built and ran successfully on macOS arm64. Saved 160x50 compression and JSON comparisons passed without new terminal capture. Original evidence remained unchanged after both runs. Fresh offline reproducibility was not established.

## Commands actually run

From `experiments/terminal-transport/`:

```sh
cargo metadata --locked --format-version 1
cargo build --release --locked --target-dir target
python3 scripts/run-saved.py --size 160x50 --mode compression
python3 scripts/run-saved.py --size 160x50 --mode json
cargo metadata --locked --offline --format-version 1
cargo fmt --all -- --check
python3 -m py_compile scripts/run-saved.py
```

From the repository root:

```sh
git diff --check
git check-ignore experiments/terminal-transport/target/release/ship-jsonpatch-prototype \
  experiments/terminal-transport/.runs/build-verification/metadata.json
```

A Python SHA-256 comparison checked every imported file against its scratch source and recorded `preservation/source-comparison.json`. A second comparison after both benchmark runs checked every source and preserved destination against that record, plus the handoff and active lockfile. A `patch --dry-run -p1` against a disposable copy of the published wrapper baseline successfully checked all five files in `vendor/wrapper-compat.patch`.

## Results

- Metadata: exactly one experiment workspace member; all three patched dependencies resolve to repository `vendor/` manifests with no registry source. Metadata contains no scratch paths.
- First metadata attempt exposed nested dependency workspace inheritance. Explicit workspace exclusions fixed it; no dependency source changes were needed.
- Locked release build: successful, **38.52 seconds**, new repository-local `target/`, with no copied scratch target or native source override.
- Native build stderr confirms a new Ghostty clone/check-out at **`22d13172cde98a0a4dda05d3d6a3fcb0dd8ed018`** under repository `target/release/build/`. The build used installed Rust/Cargo 1.98.0, Zig 0.16.0, and existing host registry/default Zig caches. It was not a clean-room or offline build.
- Compression run: exit 0, **30.87 seconds**. All 48 byte/strategy/equality rows, 64 codec batches, 36 directly measured total batches, and 16 flushed prefixes passed. All 16 encoded-message files, byte counts, and complete `continuous-gzip.json` match the preserved original 160x50 compression results exactly. Timings are new measurements and were not expected to match.
- JSON run: exit 0, **23.10 seconds**, 18 metrics with 20 warmups and 100 samples. Initial and sequential saved-state reconstruction assertions passed.
- Output isolation: two distinct directories under `.runs/`; original input symlinks and every preserved evidence file remained unchanged.
- Preservation: **328 prototype files + two symlinks**, **21 patched wrapper files**, **52 binding files**, all exact matches. Combined preserved file data: **109,356,283 bytes**. Durable experiment footprint excluding builds/runs/Python caches: approximately **105 MiB**.
- Research handoff: byte-identical, SHA-256 **`5ca794eecd812f0ed3c98028f62e268a0b0f4375bb795e0ba6a6ea39df7a7ee5`**.
- Root `Cargo.lock`: byte-identical to scratch after locked metadata and build. Rust harness and all vendored source/generated bindings remain unchanged.
- Scoped Rust formatting, Python compilation, ignore coverage, and `git diff --check`: passed.
- Active root manifest, harness, runner, binding manifests, and native build script contain no required `/tmp` paths. Upstream binding tests contain historical OSC working-directory URL samples (`file://localhost/tmp/...`); they are data literals, not filesystem dependencies. Historical documents/logs intentionally retain scratch paths.

## Local raw output

Build/metadata/environment logs are in ignored `.runs/build-verification/`. Benchmark output directories:

- `.runs/20261003T220545Z-160x50-compression-lwapiow_/`
- `.runs/20261003T220957Z-160x50-json-f4qxut4y/`

These new raw outputs are local, ignored artifacts, not preserved historical measurements. The checked-in preservation record and this summary retain the durable verification facts.

## Independent import review

After the import worker finished, the parent inspected the manifest, runner, provenance and verification notes. An independent SHA-256 check matched all **401 preserved source files** against the preservation record; the 328 prototype files also matched a separately captured pre-import baseline. Both symlink targets and the research handoff hash matched.

The parent reran locked offline Cargo metadata, confirming all three path dependencies resolve inside repository `vendor/`. It independently executed the 160x50 ordered probe (exit 0) and complete compression mode (exit 0, 31.15 seconds): all 48 size rows, 64 codec batches, 36 directly measured pipeline batches and 16 flushed prefixes passed. New output is isolated in ignored `.runs/20261003T221535Z-160x50-ordered-probe-kwxi8pf7/` and `.runs/20261003T221605Z-160x50-compression-z47v814a/`. `git diff --check` passed. Offline metadata is not a fresh offline native build claim.

## Limits and follow-up

No HTTP/proxy/browser transport, rendered UI, sustained throughput, memory, other-platform build, or native recapture was tested. Original evidence and successful saved reconstruction are the scope of this verification. Native engine and registry dependencies may need fetching on a fresh machine; Cargo offline mode does not constrain native build-script networking.

The wrapper's published MIT declaration lacks an upstream copyright/license file. The local license notice documents that gap; resolve it before public redistribution. Binding license files and example font license were preserved as supplied. No public publishing occurred.

No production code, architecture, application-state strategy, schemas, publication cadence, or planning-document reconciliation was changed. Existing modified files outside the assigned import paths were left alone. No commit was made.

reused: original saved captures, benchmark assertions, codec implementation, lockfile, wrapper/bindings, and Cargo/native tooling.
new code: safe saved-run wrapper and relative-path/workspace configuration; no Rust rewrite or new automated tests.
proof: repository-local locked release build, actual saved compression/JSON runs, exact source/evidence comparisons, patch dry-run, format/compile checks, and Git ignore/diff checks.
