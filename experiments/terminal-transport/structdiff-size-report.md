# Saved larger-screen structdiff comparison: passed

Default structdiff behaved like a full snapshot for edits and scrolling, with similar CPU time and almost the same bytes. Ordered structdiff made much smaller messages, but its quadratic cell-vector alignment dominated CPU time: about **31 ms at 80x24 and 520 ms at 160x50**, even for unchanged screens. This is a local byte/CPU tradeoff, not an architecture recommendation.

All five approved sizes and all three transitions completed. No timeout fired and no size or strategy is missing. The four captures at each size were read directly from existing `size-results/<size>/captures.json`. No shell was spawned, no input rebuilt, and no native capture performed by this mode.

## Run and scope

From `/private/tmp/ship-jsonpatch-prototype`:

```sh
cargo fmt --package ship-jsonpatch-prototype -- --check
cargo build --release --locked --offline
python3 structdiff-size-results/run-saved.py
```

The runner uses the newly built release binary, with individual commands of this form:

```sh
target/release/ship-jsonpatch-prototype --saved-size 80x24 ordered-probe
target/release/ship-jsonpatch-prototype --saved-size 80x24 json
target/release/ship-jsonpatch-prototype --saved-size 80x24 default
target/release/ship-jsonpatch-prototype --saved-size 80x24 ordered
```

The CLI accepts only the five approved sizes and these four strategies. Paths are relative to this scratch directory. Each run writes beneath `structdiff-size-results/<size>/<strategy>/`, never to the original capture/measurement paths. Repeating the runner replaces its own new results, not prior size/tiny artifacts.

First, a separate process computed **one ordered small-edit diff per size**, with no warmup and a **10-second wall timeout**. This times only `.diff`, not encoding/applying; subsequent serde decoding and exact equality checks also passed within that process timeout. Only after all five probes did ordered sampling begin. Each size/strategy then ran independently with timeouts of **45 seconds for JSON/full snapshot, 15 seconds for default structdiff, 60 seconds for ordered structdiff**. Python `subprocess.run(timeout=...)` kills and waits for the benchmark on expiry. Completed transitions are saved as they finish, so a future timeout can retain partial results; a timed-out partial transition is not reported as measured.

JSON Patch/full snapshot and default structdiff used **20 warmups + 100 samples per metric**. Ordered structdiff used **3 warmups + 10 samples per metric**, deliberately bounded because alignment is O(n²). There are 180 completed total metrics: 90 JSON/full, 45 default, 45 ordered. That is 13,500 measured calls at 100 samples plus 450 at 10 samples, and 2,835 warmup calls. No component suite or 1,000-sample batch was run. JSON/full timings below were **rerun on the same saved captures**, not borrowed prior measurements.

Environment is the existing Apple M4 Max / macOS scratch benchmark with pinned serde, structdiff 0.7.3, json-patch 4.2.0 and native dependencies documented in `README.md` and `size-report.md`. The successful offline locked release build took 2.96 seconds. Existing native build cache was reused; `--offline` is a Cargo constraint, not a clean-native-build fetching guarantee. No affinity/load isolation, network, compression, memory or allocator measurement.

## Probe and bounded-run cost

Single probes are feasibility checks, not percentile data. Process times include loading captures, validation, all strategy batches and output writing.

| Cells | One ordered diff (ms) | JSON/full process (s) | Default process (s) | Ordered process (s) |
| --- | ---: | ---: | ---: | ---: |
| 80x24 | 31.609 | 5.58 | 1.17 | 2.59 |
| 120x40 | 177.777 | 14.31 | 2.84 | 15.18 |
| 160x50 | 502.845 | 24.16 | 4.73 | 42.25 |
| 80x50 | 126.029 | 11.88 | 2.39 | 10.91 |
| 160x25 | 125.305 | 11.90 | 2.39 | 10.80 |

All 20 subprocesses exited 0; aggregate measured subprocess wall time was **164.38 seconds**. The expensive case was 160x50 ordered, at **42.25 seconds** for its complete 3/10 run, below its 60-second stop. Missing/skipped/timeout cases: **none**.

## Bytes and total CPU time

Compact uncompressed serde JSON bytes. Times below are **median / p95 milliseconds**, independently measured per total, not sums of component medians. JSON cached uses the shared `json-patch` client. Full snapshot target bytes appear for each transition. Initial snapshot bytes remain in wire evidence.

At 100 samples, median averages the central two and p95 is nearest-rank sample 95. At **10 ordered samples, p95 is the maximum observed sample**, so it is only a descriptive upper sample, not a stable tail estimate. Empty-patch near-zero timings also approach clock resolution. No confidence interval or production latency claim.

### 80x24

| Transition | Strategy | Bytes | Server total ms | Client total ms | Encoded roundtrip ms |
| --- | --- | ---: | ---: | ---: | ---: |
| small-edit | JSON cached | 973 | 1.610229 / 1.655958 | 2.071229 / 2.108916 | 3.682187 / 3.733459 |
| small-edit | Full snapshot | 215,250 | 0.344666 / 0.359375 | 1.880562 / 1.904958 | 2.147229 / 2.189375 |
| small-edit | structdiff default | 215,170 | 0.368708 / 0.382458 | 1.861667 / 1.895750 | 2.216896 / 2.283083 |
| small-edit | structdiff ordered (n=10) | 605 | 31.031354 / 31.323167 | 0.048333 / 0.049541 | 31.095458 / 31.334292 |
| one-row-scroll | JSON cached | 325,533 | 2.132667 / 2.279541 | 4.218895 / 4.340416 | 6.343437 / 6.466792 |
| one-row-scroll | Full snapshot | 215,209 | 0.334708 / 0.349042 | 1.842646 / 1.871583 | 2.151229 / 2.217542 |
| one-row-scroll | structdiff default | 215,129 | 0.367354 / 0.386625 | 1.862499 / 1.888292 | 2.210584 / 2.245166 |
| one-row-scroll | structdiff ordered (n=10) | 10,431 | 31.324188 / 31.590834 | 0.153438 / 0.154625 | 31.429583 / 32.035916 |
| unchanged | JSON cached | 2 | 1.605000 / 1.630166 | 2.033041 / 2.068083 | 3.688833 / 3.739750 |
| unchanged | Full snapshot | 215,209 | 0.338229 / 0.367542 | 1.844479 / 1.872500 | 2.149062 / 2.181334 |
| unchanged | structdiff default | 2 | 0.007167 / 0.008833 | 0.000000 / 0.000042 | 0.007250 / 0.007292 |
| unchanged | structdiff ordered (n=10) | 2 | 30.900291 / 31.480125 | 0.000041 / 0.000042 | 30.769000 / 31.223750 |

### 120x40

| Transition | Strategy | Bytes | Server total ms | Client total ms | Encoded roundtrip ms |
| --- | --- | ---: | ---: | ---: | ---: |
| small-edit | JSON cached | 989 | 4.120104 / 4.233208 | 5.164062 / 5.638166 | 9.711834 / 10.543917 |
| small-edit | Full snapshot | 537,811 | 0.824354 / 0.883750 | 4.649042 / 4.701500 | 5.508750 / 5.609250 |
| small-edit | structdiff default | 537,730 | 0.924271 / 0.944208 | 4.646020 / 4.741042 | 5.552834 / 5.677958 |
| small-edit | structdiff ordered (n=10) | 609 | 185.763896 / 192.332042 | 0.111709 / 0.113458 | 185.717896 / 187.521500 |
| one-row-scroll | JSON cached | 821,180 | 5.336917 / 5.453750 | 10.642584 / 11.420000 | 15.914166 / 17.688917 |
| one-row-scroll | Full snapshot | 537,770 | 0.825458 / 0.851375 | 4.999249 / 5.064250 | 5.528042 / 5.656417 |
| one-row-scroll | structdiff default | 537,689 | 0.904813 / 0.936000 | 4.680146 / 4.745209 | 5.562083 / 5.660958 |
| one-row-scroll | structdiff ordered (n=10) | 15,632 | 187.161271 / 193.384542 | 0.297083 / 0.304167 | 186.116938 / 187.610625 |
| unchanged | JSON cached | 2 | 4.193792 / 4.476666 | 5.156750 / 5.216417 | 9.541334 / 10.584875 |
| unchanged | Full snapshot | 537,770 | 0.869375 / 0.905958 | 4.684833 / 4.797166 | 5.514105 / 5.582167 |
| unchanged | structdiff default | 2 | 0.017875 / 0.017917 | 0.000000 / 0.000042 | 0.017917 / 0.018125 |
| unchanged | structdiff ordered (n=10) | 2 | 185.900875 / 190.266042 | 0.000021 / 0.000042 | 186.674500 / 188.496833 |

### 160x50

| Transition | Strategy | Bytes | Server total ms | Client total ms | Encoded roundtrip ms |
| --- | --- | ---: | ---: | ---: | ---: |
| small-edit | JSON cached | 989 | 7.065708 / 7.596416 | 8.676146 / 10.175083 | 15.710021 / 17.458250 |
| small-edit | Full snapshot | 896,211 | 1.488813 / 1.525375 | 8.509125 / 9.158083 | 9.256104 / 9.582750 |
| small-edit | structdiff default | 896,130 | 1.576354 / 1.625166 | 7.867458 / 8.115791 | 9.381959 / 9.614125 |
| small-edit | structdiff ordered (n=10) | 609 | 518.302771 / 521.958125 | 0.180687 / 0.186042 | 520.048105 / 526.156792 |
| one-row-scroll | JSON cached | 1,373,180 | 8.917874 / 9.481208 | 17.966125 / 20.142000 | 26.856730 / 29.493833 |
| one-row-scroll | Full snapshot | 896,170 | 1.439750 / 1.485750 | 7.885813 / 8.575459 | 9.275813 / 9.886375 |
| one-row-scroll | structdiff default | 896,089 | 1.586291 / 1.655416 | 7.833833 / 8.057625 | 9.415437 / 9.667417 |
| one-row-scroll | structdiff ordered (n=10) | 20,832 | 520.298166 / 528.001250 | 0.455104 / 0.472833 | 521.987084 / 529.795917 |
| unchanged | JSON cached | 2 | 6.976375 / 7.078542 | 8.629583 / 9.506459 | 15.753813 / 17.609750 |
| unchanged | Full snapshot | 896,170 | 1.435938 / 1.472834 | 7.887958 / 9.157584 | 9.264729 / 9.516833 |
| unchanged | structdiff default | 2 | 0.029750 / 0.032500 | 0.000041 / 0.000042 | 0.029792 / 0.032625 |
| unchanged | structdiff ordered (n=10) | 2 | 519.089646 / 526.353875 | 0.000042 / 0.000084 | 518.700667 / 525.431417 |

### 80x50

| Transition | Strategy | Bytes | Server total ms | Client total ms | Encoded roundtrip ms |
| --- | --- | ---: | ---: | ---: | ---: |
| small-edit | JSON cached | 973 | 3.474375 / 3.561166 | 4.334333 / 4.492166 | 7.856833 / 8.081667 |
| small-edit | Full snapshot | 448,210 | 0.696917 / 0.711583 | 3.855604 / 4.061500 | 4.572083 / 4.706250 |
| small-edit | structdiff default | 448,130 | 0.809020 / 0.824625 | 3.910020 / 3.969417 | 4.704105 / 4.849750 |
| small-edit | structdiff ordered (n=10) | 605 | 131.986833 / 132.538291 | 0.092395 / 0.102458 | 132.633646 / 137.456750 |
| one-row-scroll | JSON cached | 681,193 | 4.497980 / 4.685083 | 8.989875 / 9.244583 | 13.378021 / 14.048917 |
| one-row-scroll | Full snapshot | 448,169 | 0.692271 / 0.709167 | 3.820875 / 3.959500 | 4.578250 / 4.707959 |
| one-row-scroll | structdiff default | 448,089 | 0.783645 / 0.807250 | 3.915167 / 4.025209 | 4.663542 / 4.799833 |
| one-row-scroll | structdiff ordered (n=10) | 10,431 | 133.904833 / 136.323375 | 0.211917 / 0.224375 | 134.051604 / 134.741291 |
| unchanged | JSON cached | 2 | 3.531354 / 3.636750 | 4.316583 / 4.463209 | 7.856021 / 8.113458 |
| unchanged | Full snapshot | 448,169 | 0.711063 / 0.740208 | 3.825584 / 3.948208 | 4.612083 / 4.718458 |
| unchanged | structdiff default | 2 | 0.014917 / 0.015167 | 0.000041 / 0.000042 | 0.014958 / 0.015042 |
| unchanged | structdiff ordered (n=10) | 2 | 132.975271 / 138.810459 | 0.000021 / 0.000083 | 132.891688 / 137.957625 |

### 160x25

| Transition | Strategy | Bytes | Server total ms | Client total ms | Encoded roundtrip ms |
| --- | --- | ---: | ---: | ---: | ---: |
| small-edit | JSON cached | 989 | 3.485459 / 3.627500 | 4.354375 / 4.475500 | 7.906437 / 8.237291 |
| small-edit | Full snapshot | 448,211 | 0.702395 / 0.733667 | 3.907771 / 3.982792 | 4.606917 / 4.718917 |
| small-edit | structdiff default | 448,130 | 0.766749 / 0.786333 | 3.897021 / 4.090959 | 4.666979 / 4.733500 |
| small-edit | structdiff ordered (n=10) | 609 | 131.796583 / 135.396875 | 0.092792 / 0.102000 | 131.295896 / 135.902083 |
| one-row-scroll | JSON cached | 685,195 | 4.518750 / 4.602875 | 8.954812 / 9.258709 | 13.455500 / 14.010833 |
| one-row-scroll | Full snapshot | 448,170 | 0.704979 / 0.730000 | 3.908416 / 3.974167 | 4.622417 / 4.707792 |
| one-row-scroll | structdiff default | 448,089 | 0.770292 / 0.789708 | 3.865020 / 3.975708 | 4.662875 / 4.782208 |
| one-row-scroll | structdiff ordered (n=10) | 20,832 | 132.724604 / 133.904666 | 0.328333 / 0.332750 | 133.566354 / 135.779709 |
| unchanged | JSON cached | 2 | 3.524146 / 3.617875 | 4.328437 / 4.510791 | 7.837562 / 8.066167 |
| unchanged | Full snapshot | 448,170 | 0.700187 / 0.724459 | 3.877396 / 4.052459 | 4.612833 / 4.809625 |
| unchanged | structdiff default | 2 | 0.014917 / 0.015042 | 0.000041 / 0.000042 | 0.014958 / 0.015083 |
| unchanged | structdiff ordered (n=10) | 2 | 131.533042 / 135.205334 | 0.000042 / 0.000042 | 131.112146 / 134.255417 |

## Details: reuse, equality and timing boundaries

The larger-size mode previously called only `bench::run_quick`, whose total allowlist excluded structdiff. The existing `struct_transition` and `OrderedScreen` already provide the needed serde diff/apply behavior. The new saved-input entry point reuses these, rather than changing native captures, adding a model, or inventing a diff algorithm. The total allowlist now also admits structdiff totals when invoked; the native `--sizes` path still never creates/calls either comparator. No-argument mode still has the original 100/1,000 configuration and all-component selection.

- Both structdiff strategies call standard `.diff`, compact `serde_json::to_vec`, `from_slice::<Vec<T::Diff>>`, and `.apply`. Every transition's encoded/deserialized patch is applied to its typed prior and asserted exactly equal to its typed target, including every cell field and cursor. Changed patches are nonempty; unchanged is `[]`. Snapshot decode also asserts exact typed equality.
- Ordered snapshots are asserted **byte-identical** to the ordinary Screen snapshots, before and after every transition. Only the existing `ordered_array_like` vector diff strategy differs. Default still replaces the entire changed cell vector. No tracking, custom diff, field omission or schema change.
- Initial snapshot typed decoding is checked in each strategy process. JSON cached server/client documents advance sequentially through the saved captures; encoded patches and typed reconstruction equal the target exactly. Rerun JSON wire evidence matches every prior size wire transition, including the full patch content.
- `Results::measure` is reused: native work/input loading, ordered model conversion, correctness assertions and setup/reset clones are outside timers. Inputs/outputs use `black_box`. Returned-output destruction and bookkeeping/sorting are excluded. Local temporaries destroyed before closure return are included; no overhead subtraction.
- Structdiff **server_total** is diff + JSON encode plus local diff destruction; it returns bytes. **client_total** decodes the already encoded patch and applies it to a prior typed Screen cloned outside timing; it returns the typed target, without JSON Value reconstruction. **encoded_roundtrip_total** independently runs both sides and includes encoded message and patch destruction, but excludes reset clone and final typed output destruction.
- JSON cached server total converts only the new Screen to Value, diffs cached prior/new, encodes, destroys local Patch and evicts prior cache inside timing, returning new cache and bytes. Client total decodes/applies to a reset retained Value, then **clones that Value inside timing** for typed reconstruction. Cached roundtrip retains the new server cache and the client document/typed screen. Both prior-state reset clones and final outputs' destruction are excluded.
- Full snapshot directly encodes/decodes typed Screen. Returned bytes/Screen destruction is excluded in standalone totals; roundtrip message destruction is included. No retained JSON cache.
- Old typed-view eviction is excluded, not a hidden claim of complete frame lifetime cost. In particular, default unchanged client is cheap because `.apply([])` need not reconstruct the screen, but an old Screen reset clone is excluded. Ordered unchanged still runs costly alignment. No unchanged-frame suppression optimization was added.

## Evidence

- `structdiff-size-results/build-output.txt`: scoped format check and `cargo build --release --locked --offline` passed. The build used the existing native cache; no generated source was edited.
- `structdiff-size-results/run-saved.py`, `run-output.txt`, `run-meta.json`: exact orchestration, commands, timeouts, elapsed time and successful exit status for all five probes and all 15 sampled strategy runs. Each per-size/strategy directory has its own log/meta; probe directories have `probe.json`. All sampled logs end with `SAVED BENCH PASS` and contain typed encoded equality proof for all transitions.
- Per-size `json/`, `default/`, `ordered/` hold `measurements.json` with explicit counts and median/p95 microseconds, plus `wire-evidence.json` with bytes and exact encoded patch equality PASS. JSON evidence retains the actual patches. Structdiff evidence records patch field counts/bytes; the runtime assertion deserializes/applies the emitted message, rather than trusting a byte-count-only check.
- `preservation-and-proof.json`: independently checked 180 metrics, strategy counts, all three transition equality records, successful process status and exact old/new JSON wire evidence equality. The five source capture files remain byte-identical to the saved manifest.
- `source-before/`, `source-after/`, `source-diff.txt`: saved source snapshots/diff. Exact extracted source hashes establish that `json_client`, `json_transition`, `struct_transition`, and `run_quick` were reused unchanged. Only `src/main.rs` (scratch CLI dispatch) and `src/bench.rs` (total allowlist + saved input entry point) changed among all pre-existing non-target files. `src/sizes.rs`, README, size-report, Cargo manifest/lock, tiny artifacts, previous logs/JSON and verification folders remained identical.
- Full original `/private/tmp/ship-screen-prototype` file/symlink hash comparison: **31,134 paths unchanged**, no changed/missing/extra paths. Wrapper/native source and build dependencies remained unchanged. OSS notebook short status still equals `oss-status-before.txt`. No Ship/notebook file was written and no commit, provisioning, architecture adoption or permanent test was added.
- The original default native workflow was **not rerun**, because this task forbids spawning native shells. Preservation and exact reused-function comparison cover that boundary structurally, not as a fresh native behavior claim.

## Paths changed

All paths below are inside `/private/tmp/ship-jsonpatch-prototype`:

- `src/main.rs`: add `--saved-size SIZE STRATEGY` dispatch; default and `--sizes` dispatch retained.
- `src/bench.rs`: admit structdiff totals in quick selection and add saved capture loading, initial/snapshot shape checks, one-diff probe, 20/100 or 3/10 total comparison and per-transition persistence.
- `structdiff-size-results/`: separate runner, source snapshots, hashes/proof, build/run logs, metadata and per-size results. `target/`: local release build products only.
- `structdiff-size-report.md`: this mixed-audience report. Existing reports were preserved, including their historical scope statements.

## Residual risk

This is one local sequential run of staged, filled, row-specific RGB native captures. Its one-row scroll is deliberately unlike a sparse/uniform terminal workload. Ordered p95 has only 10 samples; the larger-size cost already makes this a feasibility signal, not a finely resolved ranking. Reset cloning, old typed-view lifetime, memory, native capture, transport and UI rendering are outside the claimed totals. The initial native-capture provenance and conversion checks are reused historical evidence; this run proves exact serialized patch reconstruction from those persisted captures. The scratch Screen schema still couples to Ratatui serde fields, with no public schema or architecture adoption.

## Uncertain decisions

None. Only the approved scratch CLI extension, saved captures, existing comparator semantics, and bounded sample settings were used. Production format/adoption choices remain outside this probe.

reused: saved actual native captures, existing default/ordered structdiff serde diff/apply and JSON cached/full closures, existing timer/reset/output boundaries, pinned dependencies and native cache.
new code: minimal saved-size/probe dispatch and bounded independent runner, explicit per-transition result persistence and separate report/proof. Existing `--sizes` mode did not invoke structdiff and would recapture native shells.
proof: offline locked release format/build; five bounded one-diff probes and 15 successful release strategy runs; 180 total metrics with exact encoded typed target equality, unchanged prior artifacts/input hashes and full original dependency-tree preservation.
