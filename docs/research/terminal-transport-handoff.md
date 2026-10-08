# Ship handoff: compressed full snapshots

Audience: mixed, intended for Cyan and the Ship agent. This records the current design direction and bounded experimental evidence, not an implemented Ship protocol. Historical benchmark sections below retain their original workloads and results.

## Decision to carry back

Use **compressed textual SSE full snapshots for both structural state and terminal screens**, with changed-only publication, a bounded publication rate and coalescing. Cyan's criterion is acceptable performance and simpler implementation, not the smallest possible message for every transition. This supersedes the earlier structdiff/JSON Patch pipeline direction. Exact production schemas and operational semantics remain open.

The measured reasons:

- Full-screen snapshots compressed from about 896 KB to about 14–15 KB with gzip in the saved styled-screen workload.
- Directly measured snapshot encode/compress/decompress/decode CPU totals were about 9.3–9.4 ms, versus 15.7 ms for JSON Patch edits and 28.0 ms for scrolling. These are combined server/client CPU times, not network latency or server-only cost.
- Each snapshot is self-contained. Clients do not need a patch baseline or a retained JSON document solely to apply terminal patches. Reattachment can receive the latest screen rather than replaying missed terminal patches.
- JSON Patch still saves bandwidth for small edits, but its advantage has not justified the additional machinery for this terminal v1 path.

Gzip was the original tested baseline. The later live loopback comparison below verified incremental delivery with gzip, zstd, Brotli and deflate using explicit decoders. Zstd is now the leading candidate; gzip fallback is a recommendation, not a production choice. Proxies, browsers and Reqwest automatic zstd decoding remain unverified.

**Structural application state also uses full snapshots in the current direction.** Terminal screens remain outside structural `SessionState`, and the client view and server runtime are separate from shared structs. Session revisions reject stale/out-of-order snapshots, but attachment still needs coordinated snapshot/subscription. Revision scope, restart baseline identity/reset, exact schemas and connection layout remain open; no patch-base or replay protocol is adopted.

Memory superiority was not measured. Avoiding the extra retained JSON representation may reduce memory work, but peak memory and allocations are unknown. No terminal-specific diff, Termwiz adapter, client-owned PTY emulation or mutation-tracking architecture was adopted.

Do not resume the OSS structdiff attribute-forwarding work for this use case. It remains paused.

## Requirements Cyan clarified

- The server owns authoritative state and validates client requests before performing state-machine transitions.
- State remains ordinary Rust structs. Code must be able to modify it normally, without tracked setters, proxies, or a special mutation API.
- When evaluating diffs, compare ordinary snapshots rather than requiring tracked mutations. Terminal v1 now prefers complete changed-screen snapshots, not patches.
- SSE is the intended delivery transport. The earlier requirement for incremental terminal patches has been superseded by the compressed-snapshot direction. Do not publish every intermediate screen change.
- Rust clients are expected; a future web client should be possible without implementing a Rust-specific diff protocol.
- Initial synchronization and recovery use current full state rather than a patch history. Structural and terminal ordering, attachment and restart semantics still need design.

Client requests are commands to the authoritative server. They are not necessarily JSON Patch operations and should not bypass state-machine validation.

## Intended terminal v1 data flow (not implemented)

```text
server
  process terminal output in the server-owned emulator
  → detect visible screen/cursor changes
  → at a bounded publication opportunity, capture the latest screen
  → serialize directly as a full snapshot
  → frame as a textual SSE event
  → transport compression with timely flushing

client
  transport decompression
  → receive complete snapshot event
  → deserialize screen
  → replace local terminal view
```

Change detection, publication cadence and backpressure must fit the proposed server-owned model; Ship has no implemented publication pipeline. Coalesce superseded screen states instead of queuing every intermediate frame. Snapshot publication does not require a full JSON tree or a terminal patch generator.

Idle terminals should produce no screen snapshots; connection keepalives may still be needed. Do not run serialization/decompression work at a fixed rate when nothing changed. Exact change-detection mechanics are unchosen.

Hidden-pane delivery is a later optimization, not implemented evidence: continue processing output on the server, pause that client's hidden-screen publications, and send the latest snapshot when it becomes visible. Visibility is per client; other clients may still be viewing the terminal. Scrollback/history delivery is not automatically solved by a visible-screen snapshot.

At the measured sizes, one continuously changing 160×50 terminal at 60 publications/sec projects to about 0.85–0.88 MB/sec per client before transport overhead. This is a sustained-load scenario, not ordinary idle usage or a worst-case bound. Combined snapshot CPU projects to about 0.56 CPU-seconds/sec at that rate; it is not measured sustained utilization. Multiple busy visible terminals add up. Rates and compression ratios vary with content and hardware. Loopback/LAN reduce bandwidth concerns but do not eliminate capture, serialization, decoding or rendering work.

## Historical JSON Patch proposal (superseded for terminal v1)

The following flow and rationale explain the earlier experiments. They are not instructions to build a terminal patch pipeline. The preference then was standardized RFC 6902 operations using existing Serde mappings, without a separate derive or field/path mapping. Structural application state now also follows the full-snapshot direction.

### Server

The Rust struct remains authoritative. The cached `Value` is the previous **published state**, not the previous patch.

```text
initialization
  authoritative Rust state → JSON Value → initial snapshot
  retain that Value as the published baseline

update
  modify Rust state normally
  → serialize current state to a new Value
  → diff cached published Value against new Value
  → encode RFC 6902 patch
  → publish update
  → retain new Value as the next baseline
```

After initialization, this needs one full-state `to_value` conversion per publication, rather than converting both before and after snapshots each time. Correct baseline advancement must be integrated with Ship's publication/replay semantics; it should not be advanced independently of the corresponding published update.

### Rust client

```text
initial snapshot → retained JSON Value

patch message
  → decode json_patch::Patch
  → apply to retained Value
  → reconstruct typed state if the client needs it
  → retain updated Value for the next patch
```

The client need not serialize its typed state back into JSON before each patch. In the tested implementation, it retains both a JSON document and a typed `Screen`. `serde_json::from_value` consumes a Value, so the prototype clones the retained document before typed reconstruction. That clone and reconstruction are included in the client timing.

A web client can apply the same RFC operations to its ordinary JSON-shaped state with a compatible JSON Patch implementation. A Rust-to-web roundtrip was not tested.

### APIs used in the probe

Illustrative calls, not a complete production update function:

```rust
let next_json = serde_json::to_value(&state)?;
let patch = json_patch::diff(&published_json, &next_json);
let message = serde_json::to_vec(&patch)?;

// Client, already holding its initial JSON document:
let patch: json_patch::Patch = serde_json::from_slice(&message)?;
json_patch::patch(&mut client_json, &patch)?;
let screen: Screen = serde_json::from_value(client_json.clone())?;
```

A normal message looks like:

```json
[{"op":"replace","path":"/cols","value":120}]
```

“JSON Patch” here means RFC 6902, not JSON Merge Patch or jsondiffpatch's proprietary delta format.

## Historical JSON Patch rationale versus structdiff or serde-diff

| Approach | Advantage | Cost for Ship's stated needs |
|---|---|---|
| Cached JSON Patch | Standard protocol; uses existing Serde shape; no custom diff derive | Full current-state JSON conversion and typed client reconstruction |
| structdiff | Direct typed comparison and application; avoids full JSON-tree boundary | Separate diff derive and library-specific patch semantics for non-Rust clients |
| serde-diff | Typed snapshot comparison and direct application during deserialization | Library-specific commands, not RFC operations; no verified web receiver |
| Mutation tracking | Can record changes without snapshot comparison | Explicitly contrary to Cyan's preferred editing model |

In the original tiny samples, the typed diff/apply timing advantage was small in absolute terms. Cyan then favored JSON Patch's portability and existing serialization contract. The later realistic-size and compression results supersede that terminal preference; this rationale is historical.

Using the actual serialized representation handles Serde renames, omitted properties, and custom serializers without independently reproducing their mapping. This is not a claim that every Rust type roundtrips through JSON: the selected wire state must still be JSON-serializable and deserializable as needed. Skipped or otherwise unrepresented state cannot be replicated by a JSON patch. Serialization should be stable enough that repeated unchanged states do not generate accidental changes.

JSON Patch specifies the operations, not the comparison algorithm. The inspected Rust `json-patch` generator compares arrays positionally and does not infer moves, although the applicator supports moves. This matters for collection reordering and scrolling, but is not a reason to build a custom generator before measuring actual workloads.

## Experiment and results

### Location and reuse

Original working structdiff experiment:

`/private/tmp/ship-screen-prototype`

JSON Patch sibling:

`/private/tmp/ship-jsonpatch-prototype`

The sibling reused a real `/bin/sh` in portable-pty, the supplied Ghostty `SessionHandle`, and owned Ratatui cell/cursor conversion. It captured a styled Unicode screen, a small edit, a burst of scrolling output, a resize, and an unchanged screen. No synthetic captures were used in the completed benchmark.

The original prototype, wrapper, and native bindings were preserved. Native bindings checkout: `8953a740bc378cec3e07e1f6ca949f0595eab19b`.

### Measurement conditions

- Release mode on Apple M4 Max.
- Rust 1.98.0; `json-patch` 4.2.0; structdiff 0.7.3.
- 100 warmup iterations and 1,000 measured iterations per metric; 116 metrics.
- Actual viewports were only 20×6 and 24×8 cells.
- CPU totals include server diff/encoding and Rust client decode/apply/reconstruction.
- Cached JSON Patch includes old server-cache disposal. Prior-state reset clones are outside timers; typed client reconstruction's JSON clone is inside.
- Byte counts are compact, uncompressed JSON.
- No HTTP/SSE transport, network latency, compression, rendering cost, or allocator instrumentation was measured.

### Message bytes

| Transition | Full snapshot | JSON Patch | structdiff ordered |
|---|---:|---:|---:|
| Small edit | 13,624 | 800 | 668 |
| Scrolling output | 13,531 | 4,038 | 7,097 |
| Resize | 21,595 | 14,875 | 9,535 |
| Unchanged | 21,595 | 2 | 2 |

JSON Patch saved 94% versus a snapshot for the edit and 70% for scrolling. Compared with ordered structdiff, it used only 132 extra bytes for the edit and about 43% fewer bytes for scrolling. It was larger for resize. Cyan considers resizing relatively infrequent, so the edit/scroll results were more important to his choice.

The scroll capture is a burst of twelve output lines, not a generic benchmark of every scrolling pattern.

### Serialized-message roundtrip, median milliseconds

| Transition | Full snapshot | JSON Patch, both snapshots converted | JSON Patch, prior JSON cached | structdiff ordered |
|---|---:|---:|---:|---:|
| Small edit | 0.138 | 0.297 | 0.244 | 0.144 |
| Scrolling output | 0.138 | 0.325 | 0.275 | 0.214 |
| Resize | 0.218 | 0.532 | 0.478 | 0.355 |
| Unchanged | 0.217 | 0.459 | 0.382 | 0.350 |

The cached JSON Patch penalty over ordered structdiff was roughly 0.06–0.12 ms for changed samples. That is small relative to typical network delays, though network delays were not measured. CPU still accumulates across states and update frequencies; these figures do not establish scale limits.

The unchanged JSON Patch path still rebuilt typed state despite receiving `[]`. That is avoidable work, not evidence for an optimized no-change path. Ordered structdiff also spent time aligning its collection even for unchanged snapshots. The original default structdiff strategy was cheap but replaced the entire cell vector for changed frames; the report includes those results separately.

Do not add individual component medians to recreate totals: they were measured in independent batches with different temporary lifetimes. Use the directly measured pipeline totals. Full p95 and boundary details are in the sibling README.

## Larger-size follow-up: performance is not settled

The same scratch prototype now supports `--sizes`. This tests **filled actual native screens**, a four-cell text/style edit, a genuine one-row scroll, and unchanged state at 80×24, 120×40, 160×50, 80×50, and 160×25. The last two are example halves of the chosen 160×50 full-screen grid, not detected monitor dimensions.

Each initial row has varied text and row-specific RGB styles. Scrolling shifts those style fields along with text. This differs from the original tiny twelve-line burst; it is not merely the old test at a larger size.

All runs are release mode, with 20 warmups and 100 samples for each of six pipeline totals per transition, 90 metrics overall. Cached JSON Patch versus full snapshots only; **neither structdiff strategy was measured at these sizes**.

### Worker run: median combined server/client milliseconds

| Size | Small edit: cached patch / snapshot | One-row scroll: cached patch / snapshot | Scroll patch bytes / snapshot bytes |
|---|---:|---:|---:|
| 80×24 | 3.84 / 2.24 | 8.31 / 2.32 | 325,533 / 215,209 |
| 120×40 | 9.85 / 5.67 | 20.17 / 5.62 | 821,180 / 537,770 |
| 160×50 | 16.46 / 9.58 | 34.34 / 9.59 | 1,373,180 / 896,170 |
| 80×50 | 8.29 / 4.82 | 17.41 / 4.79 | 681,193 / 448,169 |
| 160×25 | 8.17 / 4.72 | 16.93 / 4.72 | 685,195 / 448,170 |

Four-cell edits emit 16 operations, only 973–989 bytes at every size. Unchanged patches are `[]`, but the unoptimized client still clones and reconstructs the entire typed screen. At 160×50 that unchanged roundtrip took 16.54 ms in the worker run.

The parent independently reran all five native workflows. Captures and wire evidence were exactly identical. Timing varied: full-screen edit was 15.56 ms and scroll 26.84 ms, versus 9.22/9.32 ms for full snapshots. Other scroll results were also faster than the worker run. Do not pool these as confidence bounds: they are two sequential runs with no load isolation. Both demonstrate the same qualitative result, not a sub-millisecond pipeline at realistic sizes.

### Meaning for Ship

- Small edits preserve a major bandwidth advantage, but the full-state conversion/reconstruction work scales with all cells, not edited cells.
- This generator's positional one-row scrolling patches are about 1.5× the full snapshot size in this varied-style workload. The earlier claim that scrolling favored JSON Patch does not generalize.
- Network latency does not erase CPU throughput cost at this scale. Combined CPU time is not a measured network/frame deadline or a server-only cost.
- The screen schema serializes every cell's full metadata. These byte counts are compact but uncompressed; real transport compression and uniform/sparse styles may substantially change results.
- Possible next evaluations include choosing a snapshot when its encoded representation is smaller, skipping typed reconstruction on empty patches, or reducing the retained-JSON/typed-view conversion cost. These are **unimplemented options**, not adopted changes. A standard wire format need not imply one naive generator forever.
- At the time of the first size sweep no larger-size structdiff comparison existed. The subsequent saved-capture comparison below now supplies that evidence; do not use the tiny samples in its place.

Artifacts:

- `/private/tmp/ship-jsonpatch-prototype/size-report.md`: complete worker results and timing boundaries.
- `/private/tmp/ship-jsonpatch-prototype/size-results/<cols>x<rows>/`: native captures, workload scripts, wire evidence, and measurements.
- `/private/tmp/ship-jsonpatch-prototype/parent-size-verification/`: independent native rerun and its per-size artifacts.

Run:

```sh
cd /private/tmp/ship-jsonpatch-prototype
cargo run --release --locked --offline -- --sizes
```

All five shell lifecycles, exact four-cell edits/one-row shifts, initial snapshot and sequential patched typed equality, and unchanged checks passed in both runs. Original source/manifest hashes still match. Existing tiny artifacts and default run were preserved; the worker also ran a successful default regression. No HTTP/SSE, compression, rendering, memory, or allocation measurements were added.

## Subsequent structdiff comparison on the same saved larger captures

At Cyan's request, a Sol worker with medium thinking added bounded saved-capture runs for both existing structdiff strategies. No terminal was respawned and no inputs were rebuilt. Cached JSON Patch and full-snapshot timings were rerun contemporaneously on those same captures.

All five sizes and all three transitions completed, with exact serialized/deserialized typed patch equality. No size, strategy, or timeout case was omitted. JSON/default/full totals used 20 warmups and 100 samples. Ordered alignment used 3 warmups and 10 samples, with independent single-diff probes and process timeouts. Its p95 is the maximum of only 10 samples, not a stable tail estimate. All runs were release mode and reused the prior timing boundaries.

### Worker run: median combined server/client milliseconds

| Size | Small edit: JSON cached / structdiff default / ordered | One-row scroll: JSON cached / structdiff default / ordered |
|---|---:|---:|
| 80×24 | 3.68 / 2.22 / 31.10 | 6.34 / 2.21 / 31.43 |
| 120×40 | 9.71 / 5.55 / 185.72 | 15.91 / 5.56 / 186.12 |
| 160×50 | 15.71 / 9.38 / 520.05 | 26.86 / 9.42 / 521.99 |
| 80×50 | 7.86 / 4.70 / 132.63 | 13.38 / 4.66 / 134.05 |
| 160×25 | 7.91 / 4.67 / 131.30 | 13.46 / 4.66 / 133.57 |

### Full-screen 160×50 message bytes

| Transition | JSON Patch | structdiff default | structdiff ordered |
|---|---:|---:|---:|
| Small edit | 989 | 896,130 | 609 |
| One-row scroll | 1,373,180 | 896,089 | 20,832 |
| Unchanged | 2 | 2 | 2 |

Default structdiff replaces the whole changed cell vector: it is faster here, but gives almost no bandwidth savings versus a snapshot for changed frames. Ordered structdiff finds compact sequence edits and is cheap to apply on the client, but its quadratic cell-vector alignment dominates server CPU. At full-screen size it takes about half a second even for unchanged snapshots. Default unchanged comparison is about 0.03 ms; no typed reconstruction is needed for its empty patch (reset cloning remains excluded).

The parent inspected the new source and independently reran all strategies at 160×50. Small edit: JSON cached 15.26 ms, default 9.16 ms, ordered 519.19 ms. Scroll: 26.32 ms, 9.20 ms, and 521.37 ms respectively. All encoded equality assertions passed. This confirms the substantial tradeoff in this configuration, not a universal ranking of every structdiff strategy or terminal model. No Zellij-style encoder was implemented or measured.

Report and results:

- `/private/tmp/ship-jsonpatch-prototype/structdiff-size-report.md`
- `/private/tmp/ship-jsonpatch-prototype/structdiff-size-results/`
- `/private/tmp/ship-jsonpatch-prototype/parent-structdiff-verification/`

Commands:

```sh
cd /private/tmp/ship-jsonpatch-prototype
python3 structdiff-size-results/run-saved.py
# Individual saved-input run:
target/release/ship-jsonpatch-prototype --saved-size 160x50 ordered
```

At this stage Cyan still preferred JSON Patch's standard/Serde contract; the later compressed-snapshot decision supersedes that terminal preference. Switching back to ordered structdiff was not an evidenced fix for high-frequency large terminal updates with the tested representation. Default structdiff is effectively a fast full-vector replacement path; terminal-specific encoding and other collection strategies remain separate, untested possibilities.

## Compression follow-up: snapshots are much smaller on the wire

Cyan authorized gzip and another efficient codec for full snapshots, JSON Patch and both structdiff strategies. Sol (medium thinking) reused all five saved sizes, with initial/edit/scroll/unchanged transitions. Independent-message gzip level 1 and zstd level 1 measurements used 10 warmups and 50 samples. All exact byte and typed reconstruction checks passed. No new native capture or production change was made.

At 160×50, independent compressed JSON payload sizes were:

| Transition / strategy | Raw bytes | gzip bytes | zstd bytes |
|---|---:|---:|---:|
| Edit / snapshot | 896,211 | 14,705 | 7,720 |
| Edit / JSON Patch | 989 | 186 | 169 |
| Edit / default structdiff | 896,130 | 14,229 | 9,539 |
| Edit / ordered structdiff | 609 | 186 | 169 |
| Scroll / snapshot | 896,170 | 13,940 | 9,495 |
| Scroll / JSON Patch | 1,373,180 | 84,683 | 45,817 |
| Scroll / default structdiff | 896,089 | 14,001 | 9,429 |
| Scroll / ordered structdiff | 20,832 | 791 | 557 |

Directly measured gzip combined CPU totals were about 9.3–9.4 ms for snapshots, 15.7 ms for JSON Patch edits and 28.0 ms for JSON Patch scrolling. Snapshot gzip compression plus decompression alone was about 0.3 ms. Ordered compressed totals were not directly measured: its previous approximately 520 ms alignment cost remains, with only codec work newly timed. Do not present additive estimates as measured pipelines.

A separate persistent gzip encoder wrapped complete textual SSE events and flushed after each event. Every unfinished prefix decoded to all complete events so far and sequential application recovered the exact target screen. Full-screen snapshot increments were 14,668 bytes for edit and 14,071 for scroll; JSON Patch increments were 188 and 84,672 bytes. This tests codec flushing in memory, not actual HTTP/browser/proxy delivery. Persistent stream CPU was not measured; independent-message CPU numbers are not its timing. Raw compressed messages cannot go directly in SSE `data:`; HTTP Content-Encoding wraps the textual stream. Zstd results do not establish browser or SSE compatibility.

The parent read the source and independently reran all full-screen strategies/codecs and prefix checks in `/private/tmp/ship-jsonpatch-prototype/parent-compression-verification/`. Compressed sizes matched exactly; gzip CPU totals were snapshot edit/scroll 9.17/9.29 ms, JSON Patch 15.43/27.75 ms. Assertions passed.

These results informed Cyan's subsequent preference for compressed terminal snapshots for v1, as recorded at the top of this handoff. No production implementation or protocol was adopted. At the observed approximately 14–15 KB per flushed snapshot, 60 changed frames/sec projects to about 0.85–0.88 MB/sec per client before transport overhead, rather than the raw approximately 54 MB/sec. This is arithmetic from saved captures, not measured sustained throughput. Conversion CPU, real content/compression ratio, publication rate, multiple panes/clients and transport behavior still need consideration.

Full evidence and reproduction: `/private/tmp/ship-jsonpatch-prototype/compression-report.md`, `compression-results/`; run `python3 compression-results/run-compression.py` from the prototype. No memory, network, rendering or browser compatibility measurement was added.

## Live HTTP/SSE codec follow-up

A later temporary probe exercised actual loopback HTTP/SSE, not just an in-memory encoder. `/tmp/ship-sse-flush-RkwfoE/report-codecs.md` records 20 passing release-mode scenarios across identity, gzip, zstd, Brotli (`br`) and deflate. Every scenario checked exact encoding, event contents/count/order and delivery of the final event while the producer kept the stream open awaiting acknowledgment. Idle/resume and ready-burst scenarios also passed.

The heavy workload was 300 deterministic synthetic screen snapshots, 900,391 JSON bytes each, at 60/sec, in two runs per codec. These totals cover approximately five seconds, not bytes per event or per second:

| Codec | Encoded body bytes, run 1 / run 2 | Complete-event p50 ms | Complete-event p99 ms |
| --- | ---: | ---: | ---: |
| zstd | 120,499 / 172,668 | 3.100 / 3.057 | 3.800 / 3.833 |
| gzip | 2,743,923 / 2,743,944 | 4.192 / 4.336 | 5.273 / 5.939 |

Byte counts include SSE envelopes, telemetry and keepalives, but exclude HTTP headers, transfer framing and TCP overhead. Latency starts before screen generation/serialization and includes capacity wait, compression, loopback transport, explicit decompression and parser completion. It is not isolated codec CPU or production end-to-end rendering latency. Tower default quality was used, not equal numeric tuning across codecs.

The roughly 43% zstd byte-count variation is unresolved; both runs must remain visible. Repetitive synthetic data rewards long-distance redundancy. These results do not replace the earlier actual Ghostty captures or establish production bandwidth, capture/render cost, web/proxy compatibility, remote network behavior or multi-client capacity.

The probe used Tower HTTP 0.7.1 and Reqwest 0.13.4 with automatic decoding disabled. Explicit `async-compression` stream decoders validated the bodies. Tower's default compression predicate excludes SSE; the probe used a custom predicate allowing it. Source polling and codec behavior affect flushing; ready-burst delivery before EOF is not proof that every all-Ready event flushes separately.

The original gzip slow-reader probe accumulated 94 events after a two-second pause. It demonstrates buffering, not latest-state coalescing. Neither that probe nor the historical RelayBus dropping a message on `Full` provides the required eventual final-state delivery guarantee. A pending latest state and eventual flush still need implementation.

Temporary reproduction: `sh reproduce-codecs.sh` from `/tmp/ship-sse-flush-RkwfoE`. This summary preserves the bounded findings if that directory disappears; no live workflow was rerun for this docs update. Reqwest automatic zstd decoding remains untested.

## Proposed next work and remaining protocol decisions

The proposed metadata checkpoint, before PTYs, has since shipped; see the [session structure spec](../../openspec/specs/session-structure/spec.md). There is no existing Ship state/publication code to reconcile or patch cache to retain.

For implementation planning, retain ordinary authoritative structs and full snapshots. Design coordinated attachment, latest-state coalescing and eventual flush rather than copying the temporary transport producer or historical bus unchanged. Validate the actual client decompression path, reconnects, slow clients and any deployed proxies. Only real terminal integration can establish capture/render cost and representative sustained-load performance.

Unresolved choices include exact message schemas, creation response semantics, attachment headers/ID lifetime, revision scope and restart baseline identity/reset, pane lifecycle and resize ordering, screen versus scrollback scope, compression policy, cadence, visibility subscriptions and reconnect handling. Self-contained snapshots remove patch-base dependence, not all ordering or lifecycle problems. SSE `Last-Event-ID` is not proof of successful application; replay is not an adopted requirement.

## Evidence and reproduction

The worker completed scoped format/build checks and a successful release run. The parent inspected `src/main.rs` and `src/bench.rs` and independently executed the release binary in `parent-verification/`; it also passed.

Runtime assertions established:

- Initial snapshot decoding into typed and JSON state.
- Every encoded JSON Patch decoding, applying, and reconstructing exactly the next captured Screen.
- Cached and uncached server paths producing identical patch bytes.
- Buffer reconstruction and cursor preservation for the captured states.
- Changed patches nonempty; unchanged patch exactly `[]`.
- structdiff comparator patches also reconstructing their targets.
- Shell successful exit/reaping, reader Drop, session termination, and `Exited` event.

The original prototype's five recorded source/manifest hashes were independently rechecked and matched. The worker additionally reported a full preservation comparison with no changed/missing/extra original entries.

Rerun from the sibling directory:

```sh
cd /private/tmp/ship-jsonpatch-prototype
cargo run --release --locked --offline
```

This needs surviving absolute-path wrapper/native dependencies and existing toolchain/build artifacts. Cargo `--offline` alone does not prevent a native build script fetching on a fresh build. The executable writes measurements relative to its working directory.

Important artifacts:

```text
/private/tmp/ship-jsonpatch-prototype/
├── README.md                  # complete benchmark report and timing boundaries
├── src/main.rs                # native capture and cleanup
├── src/bench.rs               # correctness and repeated measurements
├── captures.json              # actual owned screens
├── measurements.json          # worker's final 116 metrics
├── wire-evidence.json         # operations, sizes, equality evidence
├── run-output.txt             # worker's final release run
└── parent-verification/       # independent rerun output and measurements
```

Broader ecosystem survey:

`/Users/cyan/Documents/projects/oss/research/structural-state-diffing-2026-10-03.md`

Specifications and API:

- https://www.rfc-editor.org/rfc/rfc6902
- https://docs.rs/json-patch/4.2.0/json_patch/
- https://docs.rs/json-patch/4.2.0/src/json_patch/diff.rs.html

Temporary artifacts can disappear; preserve relevant source/captures/results before depending on their continued availability.

## Uncertain decisions

Compressed full snapshots are the direction for structural state and terminal screens, not a universal benchmark winner. Production codec choice and operational protocol remain unresolved. Adequacy for real workloads, sustained CPU, memory, automatic client decoding, proxies and browsers remain unverified. The later loopback suite is network evidence within one local synthetic setup, not a Ship integration or production network benchmark. No production edits or public API were completed by these experiments.

reused: existing Ghostty/PTY capture and cleanup, owned Screen conversion, Serde and standard JSON Patch APIs.
new code: sibling roundtrip/benchmark harness and this notebook handoff; no Ship or upstream contribution edits.
proof: successful release workflow, exact captured-state equality and cleanup assertions, independent parent rerun, and original-source preservation checks.
