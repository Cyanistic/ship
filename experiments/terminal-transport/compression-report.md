# Saved-screen compression comparison: passed

Compression made the filled-screen snapshots much smaller, but it did not remove their full encode/decode CPU cost. At 160x50, the small-edit full snapshot fell from **896,211 bytes to 14,705 gzip bytes or 7,720 zstd bytes**. Cached JSON Patch remained smaller for small edits but slower end to end. Its row-styled one-row scroll was still larger than the compressed snapshot. Ordered structdiff had small messages, but compression does not fix its previously measured roughly 520 ms alignment cost.

This is a local scratch comparison, not a format, transport, or architecture recommendation. The gzip SSE result below uses an actual persistent encoder and verifies complete events at every flush before close. It is not a network, proxy, browser, or event-delivery result. Zstd is an experimental independent-message comparison only.

## Scope and reproduction

All new work is in `/private/tmp/ship-jsonpatch-prototype`. The five approved saved actual native captures were reused without shell capture or synthetic filling: 80x24, 120x40, 160x50, 80x50, 160x25, each with initial-filled, small-edit, one-row-scroll and unchanged. The original RGB-row-styled workload/provenance is in the preserved `size-report.md`; its scroll intentionally changes many fields, unlike a uniform/sparse terminal.

```sh
cd /private/tmp/ship-jsonpatch-prototype
cargo fmt --package ship-jsonpatch-prototype -- --check
cargo build --release --locked --offline
python3 compression-results/run-compression.py
python3 compression-results/report.py
```

Initial dependency resolution/build used normal `cargo build --release` because codecs were not cached. It added only scratch `flate2` 1.1.10 and `zstd` 0.13.3 plus transitive dependencies. Existing package versions and wrapper/native path dependencies stayed intact. Subsequent locked offline release build passed, using the existing native build cache. Offline Cargo is not a clean-native-build/no-fetch guarantee; do not remove native/target caches to reproduce without fetching.

The five final release subprocesses exited 0 in **86.41 seconds total**, bounded at 90 seconds per size; no timeout or skipped transition. One earlier 80x24 smoke run is retained as `first-pass-output.txt`, not used in the tables. Environment is the existing Apple M4 Max / arm64 / macOS 26.6.2, Rust/Cargo 1.98.0 scratch setup. No CPU affinity or process-load isolation. This run did not provision or adopt anything.

- gzip: **level 1**, flate2 default Rust backend, miniz_oxide 0.9.1, completed independent gzip members for codec-only and primary totals.
- zstd: **level 1**, zstd-sys 2.1.0 + libzstd 1.5.7. No dictionary, continuous SSE, or blanket browser compatibility claim.
- All strategies serialize compact JSON using the existing Screen/serde definitions. Default structdiff still replaces the whole changed vector. Ordered still uses `ordered_array_like`; snapshot fields serialize byte-identically.
- **10 warmups + 50 measured samples per metric**, including every codec-only batch and every primary total. 320 codec-only batches + 180 combined-total batches across five sizes: 25,000 measured calls and 5,000 warmups. Native capture/validation, precomputation, equality assertions, setup/reset cloning, sample bookkeeping, and returned-output destruction are outside timers. Inputs/outputs use `black_box`. Median averages the central two, p95 is nearest-rank sample 48. No timer-overhead subtraction. Percentiles describe this run, not production tails or confidence intervals.

## Full-screen example: 160x50

Full = direct typed snapshot; JSON cached = retained prior server Value + JSON Patch; Default and Ordered = distinct structdiff modes. All initial messages are snapshots, not diffs from a synthetic empty screen. Initial JSON combined timing also establishes server/client JSON caches, so it is intentionally not identical to Full.

### Independent raw payload and compressed bytes

These are JSON payload bytes, **not SSE-frame or HTTP wire bytes**. Every compressed message was decompressed byte-exactly and decoded/applied to its exact typed target. Tiny `[]` messages grow to 22 gzip bytes and 11 zstd bytes.

| Transition | Strategy | Raw bytes | gzip-1 bytes | zstd-1 bytes |
| --- | --- | ---: | ---: | ---: |
| initial-filled | Full | 896,135 | 13,969 | 10,509 |
| initial-filled | JSON cached | 896,135 | 13,969 | 10,509 |
| initial-filled | Default | 896,135 | 13,969 | 10,509 |
| initial-filled | Ordered | 896,135 | 13,969 | 10,509 |
| small-edit | Full | 896,211 | 14,705 | 7,720 |
| small-edit | JSON cached | 989 | 186 | 169 |
| small-edit | Default | 896,130 | 14,229 | 9,539 |
| small-edit | Ordered | 609 | 186 | 169 |
| one-row-scroll | Full | 896,170 | 13,940 | 9,495 |
| one-row-scroll | JSON cached | 1,373,180 | 84,683 | 45,817 |
| one-row-scroll | Default | 896,089 | 14,001 | 9,429 |
| one-row-scroll | Ordered | 20,832 | 791 | 557 |
| unchanged | Full | 896,170 | 13,940 | 9,495 |
| unchanged | JSON cached | 2 | 22 | 11 |
| unchanged | Default | 2 | 22 | 11 |
| unchanged | Ordered | 2 | 22 | 11 |

### Codec-only CPU: median / p95 microseconds

Messages, including ordered diffs, are precomputed exactly **once** before these samples. Compression includes new encoder creation, allocation, writes and finish. Decompression includes new decoder creation, allocation and complete decoding. Cached compressed bytes are prepared outside the decode timer. No diff generation, JSON decoding or apply is inside codec-only samples.

| Transition | Strategy | gzip compress µs | gzip decompress µs | zstd compress µs | zstd decompress µs |
| --- | --- | ---: | ---: | ---: | ---: |
| initial-filled | Full | 117.834 / 129.083 | 176.083 / 220.250 | 200.979 / 211.375 | 111.937 / 112.875 |
| initial-filled | JSON cached | 118.999 / 147.917 | 192.770 / 201.084 | 184.208 / 207.459 | 101.916 / 113.500 |
| initial-filled | Default | 117.188 / 121.000 | 175.458 / 184.041 | 202.833 / 208.667 | 111.354 / 112.583 |
| initial-filled | Ordered | 118.583 / 121.750 | 174.083 / 216.959 | 204.583 / 215.000 | 110.438 / 124.791 |
| small-edit | Full | 120.416 / 129.792 | 182.479 / 187.875 | 163.397 / 172.625 | 100.145 / 100.667 |
| small-edit | JSON cached | 6.750 / 7.042 | 4.208 / 4.250 | 3.417 / 3.500 | 1.709 / 1.750 |
| small-edit | Default | 117.250 / 127.542 | 182.583 / 192.250 | 192.688 / 199.375 | 98.479 / 103.125 |
| small-edit | Ordered | 7.312 / 8.500 | 4.292 / 4.375 | 3.584 / 3.667 | 1.750 / 1.792 |
| one-row-scroll | Full | 104.520 / 130.166 | 169.458 / 182.542 | 189.500 / 204.750 | 96.125 / 105.541 |
| one-row-scroll | JSON cached | 568.000 / 651.000 | 521.020 / 572.417 | 654.312 / 673.875 | 393.959 / 407.875 |
| one-row-scroll | Default | 116.855 / 127.291 | 174.041 / 185.375 | 189.812 / 194.708 | 96.979 / 103.416 |
| one-row-scroll | Ordered | 11.334 / 14.542 | 10.666 / 10.875 | 9.500 / 9.625 | 4.792 / 4.875 |
| unchanged | Full | 116.229 / 128.250 | 173.438 / 217.292 | 190.292 / 202.000 | 96.167 / 99.208 |
| unchanged | JSON cached | 5.375 / 5.625 | 2.833 / 2.875 | 0.583 / 0.625 | 0.417 / 0.459 |
| unchanged | Default | 5.375 / 5.833 | 2.666 / 2.750 | 0.583 / 0.584 | 0.417 / 0.459 |
| unchanged | Ordered | 5.834 / 5.959 | 2.584 / 2.875 | 0.583 / 0.584 | 0.417 / 0.459 |

### Real combined pipeline: median / p95 milliseconds

These are **directly measured independent totals**, not component sums: generate/encode JSON message, compress, decompress, JSON decode, and apply/reconstruct. Raw bypasses codecs without adding copy-only work. No network or SSE framing in these totals. Cached server prior JSON eviction, local Patch/message/compressed/decoded buffer destruction and JSON client retained-document clone are timed. New server cache, retained client document and final typed Screen are returned for destruction outside timing. Initial JSON total adds server cache conversion and client Value decoding; other initial totals use direct snapshot encoding/typed decoding.

| Transition | Strategy | Raw total ms | gzip total ms | zstd total ms |
| --- | --- | ---: | ---: | ---: |
| initial-filled | Full | 9.120 / 9.329 | 9.373 / 9.601 | 9.410 / 9.684 |
| initial-filled | JSON cached | 14.468 / 14.775 | 14.782 / 15.070 | 14.677 / 15.034 |
| initial-filled | Default | 9.135 / 9.428 | 9.417 / 9.681 | 9.420 / 9.659 |
| small-edit | Full | 9.161 / 9.489 | 9.406 / 9.575 | 9.356 / 9.668 |
| small-edit | JSON cached | 15.510 / 15.946 | 15.662 / 16.547 | 15.748 / 16.059 |
| small-edit | Default | 9.326 / 9.592 | 9.567 / 9.843 | 9.527 / 9.787 |
| one-row-scroll | Full | 9.144 / 9.407 | 9.305 / 9.634 | 9.325 / 9.584 |
| one-row-scroll | JSON cached | 26.538 / 27.321 | 27.956 / 29.391 | 27.793 / 28.371 |
| one-row-scroll | Default | 9.310 / 9.742 | 9.621 / 9.839 | 9.609 / 9.883 |
| unchanged | Full | 9.046 / 9.300 | 9.318 / 9.573 | 9.429 / 9.683 |
| unchanged | JSON cached | 15.595 / 15.996 | 15.593 / 15.962 | 15.705 / 15.947 |
| unchanged | Default | 0.030 / 0.033 | 0.041 / 0.043 | 0.031 / 0.039 |

Cached JSON still converts the entire new Screen to Value, compares cached prior/new, decodes/applies the Patch, and clones retained client JSON into a typed Screen. Compression reduces bytes, not that conversion/reconstruction work. Full still directly encodes and decodes every cell. Default still has near-full changed-vector messages, but its empty patch applies without typed reconstruction. Prior Screen reset clones and old typed-view eviction remain excluded, so the cheap unchanged default is not a whole-frame-lifetime budget.

### Ordered full-pipeline cost: historical totals + explicit estimates

The expensive ordered `.diff()` was run once per saved transition for precomputation and equality, never inside codec sampling. There is **no directly measured ordered compressed full-pipeline total**. The following estimates use the preserved historical ordered raw encoded-roundtrip median (3 warmups / 10 samples) plus this run's independent codec compress/decompress medians. They illustrate scale only: independently timed medians need not add, allocation/cache states differ, and the measurements are from separate runs. We do not estimate p95. Historical p95 at 10 samples is the maximum observed sample, not a stable tail estimate. Initial ordered total is not measured or estimated.

| Transition | Historical ordered raw total median / p95 ms | Estimated gzip total median ms | Estimated zstd total median ms |
| --- | ---: | ---: | ---: |
| small-edit | 520.048 / 526.157 | 520.060 | 520.053 |
| one-row-scroll | 521.987 / 529.796 | 522.009 | 522.001 |
| unchanged | 518.701 / 525.431 | 518.709 | 518.702 |

## Critical SSE comparison: actual event-flushed persistent gzip

Server-Sent Events (SSE) is a textual stream. Here each complete event is exactly `event: snapshot\ndata: <compact JSON>\n\n` (24 framing bytes) or `event: patch\ndata: <compact JSON>\n\n` (21 framing bytes). Initial always uses snapshot; Full always uses snapshot. JSON payloads contain no literal newlines. One `GzEncoder<Vec<u8>>` remains alive for the entire initial/edit/scroll/unchanged sequence and `.flush()` is called after every complete event. This flush emits pending compressed data without closing the member.

For **every flush**, an independent read decoder consumes the accumulated still-unfinished gzip prefix. `read_to_end` must return the expected `UnexpectedEof` from the absent footer, but its recovered bytes must equal **all complete framed events so far**. The latest recovered event is parsed and sequentially applied to retained client state, with exact typed target equality. After the fourth assertion, finishing the encoder and decoding the complete member also passes. This explicitly verifies pre-close completeness, rather than using finished-file compression as a substitute.

| Strategy | Event | Payload bytes | Framed SSE bytes | Independent framed gzip bytes | Persistent gzip incremental bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| Full | initial-filled | 896,135 | 896,159 | 14,135 | 14,131 |
| Full | small-edit | 896,211 | 896,235 | 14,705 | 14,668 |
| Full | one-row-scroll | 896,170 | 896,194 | 14,189 | 14,071 |
| Full | unchanged | 896,170 | 896,194 | 14,189 | 14,127 |
| JSON cached | initial-filled | 896,135 | 896,159 | 14,135 | 14,131 |
| JSON cached | small-edit | 989 | 1,010 | 202 | 188 |
| JSON cached | one-row-scroll | 1,373,180 | 1,373,201 | 84,756 | 84,672 |
| JSON cached | unchanged | 2 | 23 | 43 | 29 |
| Default | initial-filled | 896,135 | 896,159 | 14,135 | 14,131 |
| Default | small-edit | 896,130 | 896,151 | 14,193 | 14,131 |
| Default | one-row-scroll | 896,089 | 896,110 | 14,631 | 14,569 |
| Default | unchanged | 2 | 23 | 43 | 29 |
| Ordered | initial-filled | 896,135 | 896,159 | 14,135 | 14,131 |
| Ordered | small-edit | 609 | 630 | 202 | 163 |
| Ordered | one-row-scroll | 20,832 | 20,853 | 830 | 772 |
| Ordered | unchanged | 2 | 23 | 43 | 12 |

The first incremental count includes the gzip header; every count includes that event's flush output. A separate **10-byte finish-only footer** follows the final event for each strategy, excluded from event increments and recorded in `continuous-gzip.json`. Initial snapshots are identical, so their increments are identical. The first-event flushed member and a finished independent member can differ in size. Persistent history and flush boundaries are why these increments must not be substituted with static payload compression sizes. Continuous-stream CPU was **not measured**; these are bytes and decode-completeness checks only, while the CPU tables above are independent-message timings.

Independent framed gzip bytes above are completed gzip files around the entire textual event, used only as a size comparator. Neither those files nor independently compressed JSON bytes can be put raw into SSE `data:`. Application-level per-message compression would require text encoding (for example base64) and a protocol/framing decision, whose overhead is **not measured here**. HTTP `Content-Encoding: gzip` instead wraps the textual SSE stream externally, preserving SSE text after transport decompression. This in-memory flushed encoder/decoder models that compression boundary, not an actual HTTP server, browser EventSource, proxy buffering, scheduling, or delivery. No Axum/network experiment was run. Zstd independent-message numbers establish no continuous SSE support or browser compatibility.

## Per-size evidence and preservation

Each `compression-results/<size>/` has:

- `summary.md`: all four strategies/transitions with raw/gzip/zstd bytes, codec median/p95, primary measured totals, ordered historical additive estimates, and persistent SSE increments.
- `measurements.json`: 48 strategy/transition/codec rows with explicit 10/50 counts, equality PASS and measured versus null totals.
- `continuous-gzip.json`: four independent strategy streams, each with four pre-close prefix assertions, incremental/frame/payload byte counts, completed stream/footer counts and sequential typed equality.
- `<strategy>-<transition>.json`: exact precomputed compact messages used by codec-only samples. Combined primary totals regenerate their messages each sample; they do not benchmark replay of a precomputed patch.
- `run-output.txt`, `run-meta.json`: actual command, successful exit, elapsed time and timeout bound.

`build-output.txt` retains the first compile failure (structdiff derive rejects `pub(super)`; JSON Patch needed explicit decode type), followed by successful final release build. `codec-features.txt` records actual backend features; `environment.txt` records the machine/tools and `format-output.txt` records the final successful scoped format check. `source-before/`, `main-diff.txt`, `bench-diff.txt` and `preservation-and-proof.json` record preservation. Original dependency-tree comparison found **31,134 file/symlink paths unchanged**, with zero changed/missing/extra paths; OSS short status remained unchanged. All prior non-target scratch artifacts, reports, capture inputs and verification folders were hashed: only `Cargo.toml`, `Cargo.lock`, `src/main.rs` and the OrderedScreen visibility line in `src/bench.rs` changed. The manifest/lockfile changes are local codec dependencies; no previously locked package version was removed. `src/sizes.rs` is identical. Existing bench closures are byte-identical except that one type visibility line; no native/baseline mode semantics or sample settings changed. No Ship or notebook write, commit, provisioning or architecture adoption.

The final run passed **240 independent byte/typed-equality cases**, **180 primary full-pipeline equality cases** (one untimed assertion per measured batch), **80 pre-close flushed-prefix/sequential-apply cases**, and **20 finished-stream validations**. Ordered snapshot JSON equals normal snapshot JSON for every saved screen. Saved JSON patches match the prior wire artifacts exactly; all prior recorded snapshot/default/ordered lengths match the newly emitted compact messages. Original default/native workflows were not rerun; structural preservation covers those boundaries rather than a fresh native behavior claim.

## Residual risk and limits

One staged row-specific RGB workload, one local run per size, no generalized ranking. Saved captures retain historical native provenance; this run proves saved-message equality, not fresh atomic capture. Compression reduces serialized bandwidth potential but does not remove full conversion CPU. Default and ordered structdiff have very different generation costs even when compressed byte sizes converge for small messages. Raw and independent-compressed totals exclude SSE/network framing. Continuous bytes are tied to this backend, level, sequence, history window and explicit flush cadence. No memory/allocator measurements, network/proxy results, browser compatibility proof, render timings, capture cost, production state lifecycle, or unchanged-frame suppression optimization. No architecture was adopted.

## Uncertain decisions

None. Both codec levels, saved input grid, independent-message measurements, bounded primary totals, historical additive ordered estimates and explicit persistent gzip checks stay within the approved scratch scope. Production protocol and deployment choices remain unmade.

reused: saved actual filled native captures, existing Screen and OrderedScreen serde/diff definitions, cached JSON Patch and typed/default apply semantics, baseline reset/black_box boundaries, pinned wrapper/native build cache, prior ordered timing artifacts.
new code: `src/compression.rs`, dedicated `--compression SIZE` dispatch, local flate2/zstd dependencies, bounded runner and generated compression artifacts/report. Existing modes did not cover compression or pre-close SSE flush validation.
proof: successful locked offline release build/format check, five successful bounded release runs, 500 completed 10/50 timing batches, exact byte/typed equality for all strategies/codecs, persistent gzip prefix/sequence assertions before close, unchanged original dependency tree and prior artifact hashes.
