# Larger native-screen JSON Patch probe: passed

The five approved cell sizes passed exact snapshot/patch reconstruction and native-shell cleanup. Small edits reduced encoded bytes, but cached JSON Patch still took about 1.7x the full-snapshot encoded roundtrip CPU time. The deliberately varied, RGB-styled **one-row scroll** produced patches about 1.5x the snapshot size and took about 3.5x to 3.6x its roundtrip CPU time. Unchanged patches were two bytes, but this unchanged workload still reconstructs a typed view and costs about 1.7x a snapshot roundtrip. These are local serialization measurements, not network or production frame latency.

All additions are local to `/private/tmp/ship-jsonpatch-prototype`. Existing README, tiny-run artifacts, Cargo manifest/lockfile, and the entire original `/private/tmp/ship-screen-prototype` tree remain unchanged. No Ship/notebook edits, provisioning, commits, HTTP/network experiment, or adopted data-model decision.

## Run and scope

```sh
cd /private/tmp/ship-jsonpatch-prototype
cargo fmt --package ship-jsonpatch-prototype -- --check
cargo build --release --locked --offline
cargo run --release --locked --offline -- --sizes > size-results/run-output.txt 2>&1
```

Run from this directory; the new mode writes only beneath `size-results/`. The actual execution used a Python subprocess wrapper with a **120-second wall-time stop**, recording exit status and elapsed time in `size-results/run-meta.json`. It finished successfully without the stop firing. This is not a silent iteration cap: every one of the 90 selected metrics completed **20 warmups and 100 measured samples** (1,800 warmups + 9,000 measured calls). No metric batch was truncated. Release only. No structdiff comparator, default or ordered, was measured in the size mode; the quadratic ordered-sequence path is never invoked there. The no-argument run still uses its original 116 metrics, 100 warmups and 1,000 samples each.

Measured grid command wall time: **78.48 seconds**, exit 0. Offline release build completed in **3.66 seconds**, exit 0. Default no-argument regression: **16.84 seconds**, exit 0.

Environment: 2026-10-03, Apple M4 Max, arm64, macOS 26.6.2 (25G83), Rust/Cargo 1.98.0. Existing pinned wrapper/native dependencies and target cache were reused; no new dependency was added. The build recompiled the Rust path-dependency crates, but full original-tree hashes stayed identical. As with the old probe, `--offline` constrains Cargo, not a promise that a clean native build script cannot fetch; do not delete the existing native/target cache to reproduce offline. No affinity, load isolation, memory profile, compression, or allocator instrumentation.

### Chosen size grid

These are chosen **columns x rows**, not detected monitor dimensions or pixel measurements. The half-screen labels are examples, not assertions about this machine's display.

| Cells | Label | Native cells |
| --- | --- | ---: |
| 80x24 | small | 1,920 |
| 120x40 | medium | 4,800 |
| 160x50 | full-screen example | 8,000 |
| 80x50 | vertical half | 4,000 |
| 160x25 | horizontal half | 4,000 |

## Workload and correctness

This workload is **not the old twelve-line `scroll-output` burst**. Do not treat the new one-row numbers as the same case at a larger resolution.

At each size a real `/bin/sh` in portable-pty performs:

1. Clear and fill every native row to its last column with a row-number prefix, varying digits, and row-specific RGB foreground/background. Use absolute cursor positioning to cancel pending wrap between full-width rows. Overlay the existing accented/wide/combining Unicode examples (`café 界 é`) and bold/RGB styling in the first row; write `SIZE-READY` on the last row, then pause at bottom-left.
2. Change exactly four cells at row 2 to `EDIT`, with RGB foreground, italic and underline. Return the cursor to bottom-left and pause.
3. Explicitly position at the bottom row and emit **one CR LF**. This scrolls once at every height, rather than relying on a fixed number of lines to reach the bottom. Fill the new bottom row natively, write `ONE-SCROLL`, return to bottom-left and pause.
4. Capture that paused screen again without output for the unchanged case, then release the shell to exit.

Each capture reuses marker/dimension checks plus 120 ms of equal samples, with a five-second settling deadline. No synthetic or padded Screen is benchmarked: every owned cell comes from the native Ghostty session blit. Blank spaces inside Unicode text and its wide-cell continuation are genuine terminal cells, not padding. Each initial row is varied and populated beyond half its width; artifact verification additionally checks all rows are distinct and every final-column cell is nonblank.

Assertions at every size prove Buffer/cursor conversion equality, Unicode/bold/RGB presence, exactly the four expected cell indices changed, RGB/underline/italic edit styling, unchanged cursor state, the complete retained `(rows - 1) * cols` cells shifted exactly one row, a changed new bottom row, and an identical unchanged Screen. The initial compact snapshot decodes to exactly Screen and to the retained JSON client document. Every encoded patch decodes, applies sequentially, and reconstructs **exactly** its target Screen, including all cell fields and cursor. The cached server Value advances along that same chain and emits identical patch bytes. Each changed transition is nonempty; unchanged emits `[]`.

## Encoded bytes and operations

Compact, uncompressed JSON. Initial snapshot sizes are shown separately; target snapshot bytes apply to each transition. These operations are JSON Patch field operations, not edited-cell counts. The four-cell edit emits 16 operations because text and several style fields change.

| Cells | Initial snapshot bytes |
| --- | ---: |
| 80x24 | 215,174 |
| 120x40 | 537,735 |
| 160x50 | 896,135 |
| 80x50 | 448,134 |
| 160x25 | 448,135 |

| Cells | Transition | Patch operations | Patch bytes | Target snapshot bytes |
| --- | --- | ---: | ---: | ---: |
| 80x24 | small-edit | 16 | 973 | 215,250 |
| 80x24 | one-row-scroll | 5,699 | 325,533 | 215,209 |
| 80x24 | unchanged | 0 | 2 | 215,209 |
| 120x40 | small-edit | 16 | 989 | 537,811 |
| 120x40 | one-row-scroll | 14,292 | 821,180 | 537,770 |
| 120x40 | unchanged | 0 | 2 | 537,770 |
| 160x50 | small-edit | 16 | 989 | 896,211 |
| 160x50 | one-row-scroll | 23,863 | 1,373,180 | 896,170 |
| 160x50 | unchanged | 0 | 2 | 896,170 |
| 80x50 | small-edit | 16 | 973 | 448,210 |
| 80x50 | one-row-scroll | 11,863 | 681,193 | 448,169 |
| 80x50 | unchanged | 0 | 2 | 448,169 |
| 160x25 | small-edit | 16 | 989 | 448,211 |
| 160x25 | one-row-scroll | 11,936 | 685,195 | 448,170 |
| 160x25 | unchanged | 0 | 2 | 448,170 |

## Timing results: median / p95 microseconds

Median averages the central two of 100 samples; p95 is nearest-rank sample 95. These are independently timed batches, so the server and client medians must not be added to infer roundtrip. Percentiles describe this one run, not confidence intervals. All source-level timing boundaries are the same as the existing tiny probe.

### Cached JSON Patch

The shared JSON client is stored under strategy `json-patch` in machine-readable results; it is also the client for cached JSON Patch. Only the cached server/roundtrip variant is measured here.

| Cells | Transition | Server total | Client total | Encoded roundtrip total |
| --- | --- | ---: | ---: | ---: |
| 80x24 | small-edit | 1743.479 / 1854.500 | 2165.021 / 2190.250 | 3843.416 / 3943.250 |
| 80x24 | one-row-scroll | 2195.021 / 2278.708 | 4330.188 / 4396.500 | 8314.479 / 8781.250 |
| 80x24 | unchanged | 1765.271 / 1856.708 | 2182.396 / 2221.666 | 3887.604 / 4016.334 |
| 120x40 | small-edit | 4430.645 / 4607.417 | 5448.667 / 6006.917 | 9846.229 / 10394.375 |
| 120x40 | one-row-scroll | 5553.354 / 5652.209 | 10974.646 / 11431.625 | 20167.771 / 20807.042 |
| 120x40 | unchanged | 4404.062 / 4582.791 | 5437.417 / 5705.417 | 9817.104 / 10111.250 |
| 160x50 | small-edit | 7244.875 / 7926.833 | 8876.208 / 9042.750 | 16464.646 / 16972.000 |
| 160x50 | one-row-scroll | 9415.772 / 9724.041 | 18553.166 / 18914.875 | 34342.791 / 35630.416 |
| 160x50 | unchanged | 7437.438 / 7590.958 | 9218.792 / 9476.084 | 16535.438 / 17250.917 |
| 80x50 | small-edit | 3620.562 / 3737.959 | 4598.146 / 4712.291 | 8293.084 / 8503.250 |
| 80x50 | one-row-scroll | 4705.417 / 4999.916 | 9378.229 / 9801.167 | 17408.458 / 17738.167 |
| 80x50 | unchanged | 3655.979 / 3812.375 | 4571.042 / 4737.666 | 8092.500 / 8280.833 |
| 160x25 | small-edit | 3579.834 / 3689.667 | 4460.188 / 4543.125 | 8174.271 / 8317.875 |
| 160x25 | one-row-scroll | 4617.229 / 4702.291 | 9220.146 / 9537.625 | 16931.687 / 18495.916 |
| 160x25 | unchanged | 3629.604 / 3793.458 | 4469.750 / 4611.500 | 8020.833 / 8306.875 |

### Full snapshot

| Cells | Transition | Server total | Client total | Encoded roundtrip total |
| --- | --- | ---: | ---: | ---: |
| 80x24 | small-edit | 354.083 / 368.875 | 1916.250 / 1974.875 | 2239.916 / 2295.458 |
| 80x24 | one-row-scroll | 343.604 / 354.875 | 1914.396 / 2253.708 | 2318.979 / 2556.958 |
| 80x24 | unchanged | 365.896 / 432.125 | 1951.271 / 2189.000 | 2252.896 / 2376.583 |
| 120x40 | small-edit | 889.938 / 1013.666 | 4870.312 / 5396.958 | 5670.375 / 5775.041 |
| 120x40 | one-row-scroll | 886.375 / 909.459 | 4726.624 / 4780.583 | 5619.021 / 5682.334 |
| 120x40 | unchanged | 907.645 / 926.167 | 4804.396 / 4921.542 | 5648.958 / 5807.042 |
| 160x50 | small-edit | 1504.042 / 1732.792 | 8165.166 / 9443.834 | 9584.146 / 9717.417 |
| 160x50 | one-row-scroll | 1463.833 / 1514.459 | 8055.771 / 8268.292 | 9591.626 / 9865.750 |
| 160x50 | unchanged | 1455.458 / 1517.208 | 8061.167 / 8220.291 | 9639.959 / 9813.458 |
| 80x50 | small-edit | 750.521 / 776.208 | 4048.604 / 4197.833 | 4822.562 / 5113.208 |
| 80x50 | one-row-scroll | 717.000 / 754.083 | 4062.666 / 4282.167 | 4785.771 / 5013.250 |
| 80x50 | unchanged | 752.500 / 782.292 | 3959.771 / 4154.125 | 4749.979 / 4955.083 |
| 160x25 | small-edit | 721.458 / 738.625 | 3990.083 / 4048.500 | 4722.792 / 4904.709 |
| 160x25 | one-row-scroll | 713.125 / 740.959 | 3950.125 / 4039.500 | 4723.854 / 4790.000 |
| 160x25 | unchanged | 711.417 / 724.917 | 3992.000 / 4121.250 | 4662.084 / 4744.000 |

## Timing boundaries

`bench::json_transition` and `json_client`, including their proof and total-work closures, are reused **byte for byte** from the prior source. `Results::measure` adds explicit per-run sample settings and a six-total allowlist; omitted component/uncached batches return before setup or work. No expensive comparator is created or invoked by `run_quick`.

- Native capture, correctness assertions, shell/session cleanup, setup/reset clones, returned-output destruction, sample bookkeeping/sorting are outside timers. Inputs/outputs pass through `black_box`; no timer-overhead subtraction.
- Cached server total: start with a cached prior JSON Value; convert new typed Screen to Value, diff, encode compact Patch, destroy local Patch and **evict the prior cache inside timing**. Return new cache and bytes, whose final destruction is outside timing.
- JSON client total: decode Patch, apply to retained client Value, **clone that JSON document inside timing**, deserialize clone to typed Screen, destroy Patch. Return both retained document and typed Screen. The reset prior-document clone and old typed-view eviction are excluded, not claimed as a whole production frame budget.
- Cached encoded roundtrip: both server and client work plus the in-memory encoded message. Both prior-state reset clones are excluded; old server cache eviction and message/Patch destruction are included; new server cache and client document/Screen are retained outputs.
- Full snapshot: direct typed Screen JSON encode/decode, without a retained JSON document. Standalone server/client outputs are destroyed outside timing; the roundtrip's message destruction is inside timing. Final Screen destruction is excluded.
- No unchanged-frame suppression or patch-only UI update optimization: even the empty patch performs JSON clone plus typed reconstruction, and the snapshot baseline still serializes/deserializes a full Screen. No network/HTTP/SSE, native-capture cost, scheduling guarantee, transport compression, or memory-cost claim.

## Evidence

- `size-results/build-output.txt`: scoped format check and offline locked release build passed. `size-results/environment.txt` records actual tools/machine.
- `size-results/run-output.txt` and `run-meta.json`: actual grid command exited 0 in 78.48 seconds. Log contains five `SIZE WORKFLOW PASS`, five `CLEANUP PASS`, five `SIZE BENCH PASS: 18 metrics`, and final `SIZE GRID PASS`. Each native `/bin/sh` exited 0 and was reaped; its observed reader was dropped; the session stopped and emitted `Exited`. The shared original bounded lifecycle cleanup runs before capture assertion panics are rethrown.
- Each `size-results/<cols>x<rows>/` contains the exact emitted `workload.sh`, native `captures.json`, `wire-evidence.json` (including each patch), and all 18 `measurements.json` entries. Encoded initial snapshot, cached patch equivalence, decoded sequential patch application, and exact typed target equality assertions passed for all sizes. Runtime proof covers every capture; it is not a check of synthetic stand-in data.
- Default regression command: `cd size-results/tiny-regression && ../../target/release/ship-jsonpatch-prototype`, no arguments. Its log ends with the original 116-metric PASS and cleanup PASS; exit 0 in 16.84 seconds. New regression `captures.json` and `wire-evidence.json` are exactly equal to the saved tiny baseline. It runs in a separate working directory so no old artifacts were overwritten.
- `size-results/reuse-and-artifact-proof.json`: exact source hashes for the reused capture/text/JSON proof/timing functions; unchanged marker-stabilization function; independent persisted-artifact assertions for every size's dimensions, distinct filled initial rows, nonblank final columns, complete one-row shift, unchanged equality, and 18 metrics with 20/100 counts. It also records default capture/wire equality.
- `size-results/preservation-after.json`: all 31,134 original files/symlinks (2,057,179,955 bytes) match the prior preservation record, with zero changed/missing/extra paths. Manifest/lockfile, README, all prior tiny run/build/JSON artifacts, and parent-verification artifacts match their before hashes. OSS short status is unchanged.

## Paths changed and saved

- `src/main.rs`: dispatch `--sizes`; extract the existing real shell/session setup and bounded cleanup into shared `run_native`; keep the default shell workload and tiny proof intact.
- `src/bench.rs`: explicit sample configuration/allowlist plus `run_quick`, reusing identical existing JSON proof and selected total timing closures. Default measurement selection/counts and structdiff behavior remain unchanged.
- `src/sizes.rs`: new approved five-size native workload, stronger filled-row/four-cell-edit/one-row-scroll assertions, sequential quick measurements and separate output destinations.
- `size-report.md`: this report. `size-results/`: separate source-before/source-after snapshots, source diff, build/grid/default-regression logs, environment, preservation/reuse proof, per-size scripts/captures/patches/measurements. `target/`: local sibling build products only.

## Residual risk

One staged filled-screen workload and one sequential sample run per size, not a universal ranking. Row-specific styles intentionally make one-row scrolling alter many cell fields; a uniform-style or sparse screen may differ greatly. Native buffer/dimensions/cursor reads remain non-atomic, covered only by paused-shell marker stabilization here. Nondefault skip/diff options and other cursor visibility/style/blink variants were not generated. Screen still uses existing Ratatui serde fields and the scratch string cursor style; none is an adopted public schema. Cache/reset cloning, capture, old typed-view lifetime, memory, and network cost are not included in a claimed production budget. Structdiff, particularly its ordered comparator, has no new large-size performance evidence.

## Uncertain decisions

None. The approved size grid, throwaway CLI extension, existing JSON shape, and existing timing boundaries were retained. Omitting both structdiff comparators is explicit and within the requested scope; no consequential product/public-interface/data-model choice was made.

reused: existing native PTY/Ghostty stack, owned Buffer/cursor conversion and stable captures, observed reader and bounded native lifecycle, exact JSON patch proof and cached/full total closures, existing dependencies.
new code: `--sizes`, five fully populated native workloads and scroll/edit invariants, explicit 20/100 six-total selection, separate result destinations and report. Existing tiny captures were too small and their scroll burst was a different case.
proof: offline release format/build, five actual shells with conversion/workflow/encoded sequential equality and cleanup assertions, 90 complete metric batches, separate unchanged default 116-metric regression, exact original-tree and old-artifact preservation.
