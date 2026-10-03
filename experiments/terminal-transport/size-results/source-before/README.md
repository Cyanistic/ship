# Throwaway Ghostty JSON Patch performance probe: passed

JSON Patch reconstructed every actual captured screen exactly and reduced message size. It did **not** reduce CPU time in this probe. Even with cached prior JSON, the serialized-message roundtrip took about 1.8x to 2.2x the full-snapshot baseline. These results answer a scratch feasibility/performance question, not a production format or architecture decision.

All work and outputs are in `/private/tmp/ship-jsonpatch-prototype`. The original prototype, wrapper, and bindings were not changed. No commit, provisioning, permanent tests, HTTP/SSE transport, or architecture adoption was added.

## Run and environment

```sh
cd /private/tmp/ship-jsonpatch-prototype
cargo run --release --locked --offline > run-output.txt 2>&1
```

Run from this directory: the executable writes its JSON artifacts relative to the working directory. Requires the surviving absolute path dependencies in `/private/tmp/ship-screen-prototype` and already-installed toolchains. `--offline` uses existing Cargo dependencies; it is not an independent guarantee against native build-script fetching on a clean target. This run reused the existing native checkout/build artifacts. Do not remove `target/` if reproducing without fetching.

Recorded 2026-10-03 on Apple M4 Max, arm64, 16 logical CPUs, 64 GiB RAM, macOS 26.6.2 (25G83). Rust/Cargo 1.98.0; Zig 0.16.0. Release Rust profile; native build defaults to ReleaseFast / baseline CPU. No affinity, process isolation, or allocator instrumentation.

- Pinned reused bindings checkout: `8953a740bc378cec3e07e1f6ca949f0595eab19b`.
- Its native Ghostty engine checkout: `22d13172cde98a0a4dda05d3d6a3fcb0dd8ed018`, as pinned by the existing bindings build script. These are two distinct repositories/commits.
- `ratatui-ghostty` 0.2.0 (original `wrapper-compat`); `libghostty-vt` / `libghostty-vt-sys` 0.2.1; Ratatui 0.30.0; portable-pty 0.9.0; serde 1.0.229; serde_json 1.0.151; json-patch 4.2.0; structdiff 0.7.3.
- **100 warmup iterations + 1,000 measured iterations per metric**, 116 metrics, four actual native transitions. 116,000 measured calls, not 116,000 separately captured screens. Median averages the central two samples; p95 uses nearest rank. Values are microseconds. One final run, with sequential metric batches, not pooled cross-run statistics.

## Numerical results

All wire sizes are compact, uncompressed JSON. JSON Patch operation counts and structdiff changed-field counts are different units.

| Transition | Snapshot bytes | JSON Patch operations / bytes | structdiff default fields / bytes | structdiff ordered fields / bytes |
| --- | ---: | ---: | ---: | ---: |
| edit | 13,624 | 13 / 800 | 2 / 13,609 | 2 / 668 |
| scroll-output | 13,531 | 75 / 4,038 | 2 / 13,516 | 2 / 7,097 |
| resize | 21,595 | 145 / 14,875 | 4 / 21,603 | 4 / 9,535 |
| unchanged | 21,595 | 0 / 2 | 0 / 2 | 0 / 2 |

Initial full snapshot: **13,548 bytes**, decoded successfully to both typed Screen and client JSON document. Cached and uncached JSON Patch variants emit identical bytes.

### Serialized-message roundtrip: median / p95 µs

This is server encode plus client decode/apply/reconstruction, not network time. Every strategy uses the same captured pair and compact serde_json message.

| Transition | Full snapshot | JSON Patch | JSON Patch cached | structdiff default | structdiff ordered |
| --- | ---: | ---: | ---: | ---: | ---: |
| edit | 137.542 / 147.167 | 297.084 / 315.833 | 244.230 / 261.417 | 139.917 / 149.458 | 143.958 / 156.833 |
| scroll-output | 137.583 / 148.084 | 325.084 / 344.375 | 274.688 / 291.250 | 137.958 / 147.375 | 213.520 / 228.292 |
| resize | 217.792 / 231.208 | 532.395 / 554.000 | 477.854 / 498.958 | 219.730 / 234.417 | 355.000 / 380.792 |
| unchanged | 217.438 / 231.167 | 458.562 / 479.625 | 381.687 / 401.375 | 0.791 / 0.833 | 350.000 / 369.208 |

### Separately measured server and client totals: median / p95 µs

Cached JSON Patch uses the same client as uncached JSON Patch; its client measurement is shared.

| Transition | Strategy | Server total | Client total |
| --- | --- | ---: | ---: |
| edit | full-snapshot | 22.625 / 23.792 | 114.125 / 123.000 |
| edit | json-patch | 153.250 / 166.459 | 133.625 / 141.917 |
| edit | json-patch-cached | 108.229 / 118.667 | 133.625 / 141.917 |
| edit | structdiff-default | 25.000 / 27.625 | 114.875 / 123.750 |
| edit | structdiff-ordered | 141.583 / 160.083 | 7.417 / 8.458 |
| scroll-output | full-snapshot | 22.667 / 25.042 | 115.438 / 124.750 |
| scroll-output | json-patch | 162.687 / 174.209 | 155.833 / 165.583 |
| scroll-output | json-patch-cached | 111.125 / 119.375 | 155.833 / 165.583 |
| scroll-output | structdiff-default | 24.458 / 25.750 | 114.562 / 123.500 |
| scroll-output | structdiff-ordered | 144.145 / 157.083 | 67.709 / 71.583 |
| resize | full-snapshot | 35.395 / 38.709 | 182.125 / 197.042 |
| resize | json-patch | 220.625 / 236.375 | 299.812 / 329.375 |
| resize | json-patch-cached | 162.291 / 173.625 | 299.812 / 329.375 |
| resize | structdiff-default | 38.916 / 43.584 | 183.000 / 196.667 |
| resize | structdiff-ordered | 251.958 / 273.334 | 105.500 / 112.708 |
| unchanged | full-snapshot | 36.792 / 39.083 | 182.083 / 194.250 |
| unchanged | json-patch | 243.312 / 259.542 | 203.000 / 226.000 |
| unchanged | json-patch-cached | 165.500 / 176.917 | 203.000 / 226.000 |
| unchanged | structdiff-default | 0.750 / 0.750 | 0.041 / 0.042 |
| unchanged | structdiff-ordered | 362.584 / 387.833 | 0.041 / 0.042 |

### JSON Patch components: median / p95 µs

These independent batches are **not additive**: allocation/cache state, temporary destruction, and output retention differ from total-pipeline timings. In particular, standalone reconstruction is slower than the complete client batch in this run. Use directly measured totals to compare pipelines.

| Transition | before+after to_value | after-only to_value | Diff | Encode | Decode patch | Apply | Clone JSON + typed reconstruction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| edit | 75.896 / 112.250 | 36.959 / 37.750 | 52.375 / 60.250 | 0.625 / 0.709 | 2.041 / 2.125 | 3.166 / 3.417 | 196.645 / 214.375 |
| scroll-output | 78.500 / 86.375 | 37.166 / 40.625 | 53.938 / 61.250 | 3.125 / 3.208 | 11.167 / 11.916 | 16.417 / 18.375 | 203.792 / 217.334 |
| resize | 96.645 / 105.959 | 57.709 / 64.084 | 66.458 / 74.625 | 11.500 / 11.750 | 55.042 / 61.750 | 31.375 / 35.208 | 323.541 / 341.167 |
| unchanged | 120.209 / 129.292 | 58.042 / 63.250 | 84.104 / 92.208 | 0.041 / 0.042 | 0.000 / 0.042 | 0.000 / 0.042 | 323.042 / 342.875 |

`measurements.json` contains all 116 results, including structdiff diff/encoding/decode/apply components. Near-zero empty-patch measurements approach clock resolution (about 0.042 µs observed); a printed zero does not mean an operation is free. Percentiles describe this run's samples, not a confidence interval or production latency guarantee.

## Details: pipeline and timing boundaries

The reused real `/bin/sh` runs in portable-pty. It emits styled Unicode, pauses for edit, emits enough scroll output to exceed the viewport, pauses for a resize from 20x6 to 24x8, then provides an unchanged capture. Capture waits for marker/dimensions plus 120 ms of equal samples. Captures and correctness checks are outside benchmark timers; cleanup finishes before benchmarking.

`Screen` has ordinary Serialize/Deserialize fields, preserving every accessible Ratatui cell field and separately exposed cursor position/style/blinking. The standard JSON Patch path is:

```text
server typed before/after -> serde_json::to_value -> json_patch::diff
-> compact serde_json::to_vec(Patch)
-> client serde_json::from_slice::<Patch> -> json_patch::patch(Value)
-> clone retained Value -> serde_json::from_value::<Screen>
```

- `Results::measure` performs setup/reset before `Instant::now()`. Inputs and returned outputs go through `black_box`. Only the work closure is timed. Returned-output destruction and sample bookkeeping/sorting are outside timing. Local temporary destruction before closure return is inside timing; no timer-overhead subtraction.
- Conversion measures only typed Screen -> JSON Value, not native -> owned Screen capture. Diff, encoding, and client decode components return their outputs for destruction outside timing.
- Client apply resets a clone of the old JSON document outside timing; replacing/removing JSON values during patch application is timed. Typed reconstruction clones the target JSON **inside** timing to preserve the retained JSON document.
- Uncached server total includes both conversions, diff, encoding, and destruction of the local before/after Values and Patch. It returns only bytes. Cached server total resets an owned prior JSON document outside timing, converts only after, diffs/encodes, **drops the old cache inside timing**, and returns `(new_cache, bytes)`. New-cache lifetime is retained in the result, so its final destruction is outside timing. This models cache replacement without timing the reset clone.
- JSON client total includes patch decode, apply, Patch destruction, and timed JSON clone + typed reconstruction. It returns `(retained_document, typed_screen)`; both output destructions are excluded. Old document reset clones and old typed view eviction are excluded, not hidden inside a claimed production frame budget.
- Full-snapshot server/client totals are direct typed JSON encoding/decoding. It has no retained JSON document. Both sides' returned outputs are destroyed outside their respective timers. Its encoded roundtrip includes encoded-message destruction inside timing.
- Uncached JSON encoded roundtrip converts both values, creates/encodes a Patch, decodes that message, applies it to the reset client document, and reconstructs Screen. Server temporary Values/Patch and message bytes die inside timing. Cached encoded roundtrip additionally retains the new server cache as returned output, and includes old server-cache eviction inside timing. Both reset clones (server prior and client document) are outside timing.
- structdiff default retains the original whole-vector replacement behavior. The already-present ordered comparator serializes identical snapshot fields, but uses `ordered_array_like` for the cell diff. Setup creates its owned old Screen outside timing; standalone apply additionally resets an owned decoded diff outside timing. Server total includes diff/JSON encode and diff destruction. Client total includes JSON decode/apply and yields an already-typed screen, with no Value reconstruction. The encoded roundtrip includes message destruction. Final Screen destruction remains outside timing. This deliberate prior-state reset exclusion is especially consequential for unchanged structdiff's near-zero client cost.
- Timings are independent of wire compression, HTTP/SSE, scheduling, capture costs, and production state lifecycle. Empty JSON Patch still reconstructs the typed target in this probe, and full-snapshot baseline still sends a snapshot. No unchanged-frame suppression optimization is adopted.

## Evidence

- `build-output.txt` preserves the interrupted failure and appended successful scoped format/build logs. The source failures came from inserting `mod bench` above the crate attribute and removing the `StructDiff` import needed by the `Difference` derive expansion. Continuation moved the crate attribute first and restored that import. `cargo fmt --package ship-jsonpatch-prototype -- --check` and `cargo build --release --locked --offline` passed.
- Final `cargo run --release --locked --offline` exited 0 (`run-output.txt`). It asserts reconstructed Buffer equality and cursor preservation at every capture; the Unicode/bold/RGB/underline shell examples also pass their assertions.
- Initial snapshot typed/JSON decode passed. Every patch was serialized, deserialized into `json_patch::Patch`, applied to the sequential client JSON document, and reconstructed into Screen with exact target equality. The cached server advances through the same chain and emits the same serialized patch. Every changed transition has a nonempty patch; unchanged has zero operations and exactly `[]`. Default/ordered structdiff serialized patches also decode/apply to exactly their target screens. Ordered snapshots serialize identically to Screen snapshots.
- The shell exited 0 and was reaped; the reader adapter reported Drop; `SessionHandle::is_alive()` became false; the session emitted `Exited`. Workflow assertions use the reused bounded cleanup path before being rethrown. Log ends with `BENCH PASS: 116 metrics` and the final equality/cleanup PASS.
- `preservation-after.json` compares all 31,134 files/symlinks (2,057,179,955 bytes) against the prior preservation record. No changed, missing, or extra files. Original wrapper/bindings/build artifacts are included. OSS short status equals the pre-existing `oss-status-before.txt`; no Ship/OSS files were edited.
- `reuse-evidence.txt` records prior exact reuse and continuation exact chunk comparison after formatting the baseline through rustfmt stdin/stdout without writing it. Capture/stable sampling, reader observer, and PTY/shell/session setup match.

Original baseline SHA-256 values remain:

```text
Cargo.toml               e58bc950810f2b0c1f387d4c672997ccaa1211c181d23f0195ec2e3d590a5fc6
Cargo.lock               f1032376b6cbb0fb4991fd38ce07017a5d72762b6d0db9ef22e8a7a47ee0a459
src/main.rs              61c362d1b877ab32c105818af5ffa5121527ae9cec5541e99f7fe39ef4807c91
src/main-baseline.rs.bak 30ef9eddd6c61370309aebf4972db850e873e5348ed5ac67d09e95d21f1ad973
README.md                782e4a307d448cbc3c47e07bcda62224bbd2dbde917cec2e62d615ac7403d028
```

## Artifacts and changes

Relative to `/private/tmp/ship-jsonpatch-prototype`:

- `src/main.rs`: continued reused capture/cleanup harness; fixed attribute/import and formatted locally.
- `src/bench.rs`: continued existing JSON Patch/full-snapshot/structdiff comparisons; timed prior-cache eviction and added cached encoded roundtrip. No synthetic cases added.
- Existing `Cargo.toml`, `Cargo.lock`: retained unchanged from interrupted worker.
- `run-output.txt`, `captures.json`, `wire-evidence.json`, `measurements.json`: final run and machine-readable evidence.
- `build-output.txt`, `environment.txt`, `preservation-after.json`, `reuse-evidence.txt`: build, versions/source hashes, preservation, reuse evidence.
- `run-first-pass-output.txt`: earlier successful 112-metric run, **not** the final cached timing boundaries; not used for the numerical tables above.
- `original-files-before.json`, `oss-status-before.txt`: untouched preservation inputs from the interrupted worker.
- This `README.md`: report. `target/`: sibling build products only.

## Residual risk

Only these tiny captured viewports and staged outputs are measured. Results are not a universal JSON Patch or structdiff ranking. Native buffer/dimensions/cursor reads are not atomic; stable sampling while the shell pauses is narrower evidence. Nondefault skip/diff options and hidden/bar/underline/blinking cursor variants were not generated. Screen still couples to Ratatui serde definitions and uses the existing scratch cursor-style string; none is an adopted public schema. Memory footprint and network cost were not measured. Prior debug samples are not comparable release evidence and were not used here.

## Uncertain decisions

None. No consequential low-confidence product, architecture, public-interface, or data-model choice was made. All existing scratch semantics and dependency pins were kept; optimization/adoption decisions remain outside this probe.

reused: existing real PTY/shell/SessionHandle capture, owned Buffer/cursor conversion, staged workflow and cleanup; pinned wrapper/native stack; serde, json-patch, and existing structdiff comparison paths.
new code: local compile fixes, accurate cached old-document eviction and cached serialized-message roundtrip timing; report/evidence artifacts. Existing captures and comparison machinery already covered the rest.
proof: successful release format/build and actual-shell run with 116 repeated metrics, exact serialized-message target equality, observed child/reader/session cleanup, and full original-tree preservation comparison.
