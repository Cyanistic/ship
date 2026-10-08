# Pane terminals transport probe

Evidence for the pane-terminals architecture paper. Run on 2026-10-06 on macOS (Darwin arm64, rustc 1.98.0), loopback only. Nothing here was run on Linux, so every result is unverified there.

## Summary

The streaming input POST works as planned, and nothing in the probe calls for a change of direction.

- reqwest sends each NDJSON line as soon as it is queued. Keys reached the server in about 0.3 ms (p50) and never more than 1.1 ms, in order, over both HTTP/1.1 and HTTP/2.
- A raw `axum::body::Body` is not capped by `DefaultBodyLimit`. A 3 MiB streamed body arrived whole.
- zstd-compressed SSE delivers each event immediately. Compressed and uncompressed latencies match (p50 0.2 to 0.3 ms).
- With axum's `http2` feature, `axum::serve` accepts HTTP/1.1 and h2c prior knowledge on the same listener. reqwest with `http2_prior_knowledge()` negotiated HTTP/2.0 for both the SSE stream and the POST.
- utoipa schemas on serde remote mirrors work for the whole key surface, including `KeyCode::Media` and `KeyCode::Modifier`. On those tuple variants, `#[serde(with)]` and `#[schema(value_type)]` have to go on the variant itself, not on the field inside it.
- `TCP_NODELAY` made no measurable difference on loopback. reqwest turns it on by default. `axum::serve` does not, and Ship's server doesn't set it.
- Herdr's three portable-pty patches are all Windows-only. Plain `portable-pty 0.9.0` is fine for Linux and macOS.

Decided with Cyan on 2026-10-06: the server sets `TCP_NODELAY` on accepted connections (one `tap_io` call). Nagle's delay only appears with real round trips, which this probe can't simulate.

## Details

### Setup

A temporary crate held both a server and a client in one process, sharing one `Instant` clock so latency could be measured directly. It used the same crate versions as Ship: axum 0.8.9 (plus the `http2` feature), reqwest 0.13.5 (Ship's features plus `http2`), tower-http 0.7.1, utoipa 6.0, crossterm 0.29. The server stack copied Ship's compression layer from `crates/ship-server/src/lib.rs:90` (zstd and gzip, `SizeAbove(32)`, SSE not excluded).

Routes:

- `POST /input` reads the raw `Body` through `StreamReader` and `FramedRead<LinesCodec::new_with_max_length(64 KiB)>`, the reader planned for `/api/v0/attach/input`. Each line carries a sequence number and a send timestamp.
- `GET /sse` sends 40 events of about 220 bytes, 25 ms apart, with the default keepalive.

The client sends through `reqwest::Body::wrap_stream` over an unbounded mpsc channel. It sends 50 key lines 20 ms apart, then 3072 lines of 1 KiB with no gap (3,114,008 bytes), then closes the channel. The SSE stream is open the whole time. The matrix covers server nodelay off and on, HTTP/1.1 and HTTP/2, and compression off and on. Compression was confirmed on the server side by recording the `content-encoding` of the SSE response. reqwest strips that header once it decompresses, so the client never sees it.

Two full runs. Latencies are in ms.

| Protocol | Server nodelay | SSE encoding | SSE p50 / p99 / max | Key p50 / max | Ordered | Body received |
| --- | --- | --- | --- | --- | --- | --- |
| HTTP/1.1 | off | none | 0.1 to 0.2 / 0.5 / 0.5 | 0.2 to 0.3 / 0.4 to 0.5 | yes | full |
| HTTP/1.1 | off | zstd | 0.2 / 0.6 to 1.0 / 1.0 | 0.2 / 0.4 to 1.0 | yes | full |
| HTTP/2.0 | off | none | 0.2 to 0.3 / 0.3 to 0.4 / 0.4 | 0.3 / 0.5 to 0.6 | yes | full |
| HTTP/2.0 | off | zstd | 0.3 / 0.4 to 0.8 / 0.8 | 0.3 / 0.6 to 0.9 | yes | full |
| HTTP/1.1 | on | none | 0.2 / 0.4 to 1.0 / 1.0 | 0.3 / 0.5 to 0.7 | yes | full |
| HTTP/1.1 | on | zstd | 0.2 / 0.4 / 0.4 | 0.3 / 0.4 to 0.6 | yes | full |
| HTTP/2.0 | on | none | 0.3 / 0.8 to 1.4 / 1.4 | 0.3 / 0.5 to 0.6 | yes | full |
| HTTP/2.0 | on | zstd | 0.3 / 0.5 to 0.8 / 0.8 | 0.3 / 0.5 to 0.6 | yes | full |

"Full" means the server counted every line: the 50 keys plus 3,114,008 bytes of bulk lines, 3122 lines in all.

### Findings

**Chunk flushing.** Every key line arrived well under a millisecond after it was queued, so neither hyper's HTTP/1.1 chunked encoder nor its HTTP/2 data frames hold small chunks back. The client needs no explicit flush.

**DefaultBodyLimit.** axum's default 2 MB limit applies to buffering extractors such as `Bytes`, `String` and `Json`. A handler that takes `Body` and streams it is unaffected. The 64 KiB `LinesCodec` maximum is the only bound, and the route has to keep it. Paste size is bounded per line, not per request.

**SSE compression.** Compressed events show up as fast as uncompressed ones, so the encoder does not hold events back while it waits for more input. That is observed behavior. The mechanism (tower-http wraps the body in an async-compression `bufread` encoder, which appears to flush when its input is pending) wasn't traced to the line. The probe didn't measure compression ratio for small events; `docs/research/terminal-transport-handoff.md` has the earlier codec evidence.

**HTTP/2.** Turning on axum's `http2` feature is all the server needs. `axum::serve` uses hyper-util's auto builder and detects h2c prior knowledge on the existing listener. Ship's client can stay on HTTP/1.1, which needs no change to its reqwest features. To use HTTP/2 later, the client needs reqwest's `http2` feature plus `http2_prior_knowledge()`.

**TCP_NODELAY.** Loopback hides Nagle's algorithm: with a 20 to 25 ms gap between writes, the previous segment is always acknowledged before the next one is sent. The delay Nagle can cause (a small write held back until the previous one is acknowledged, made worse by delayed ACKs) needs a real round trip. Simulating one on macOS takes root (`dnctl`/`pfctl`), and on Linux `tc netem`. Neither was run. reqwest sets nodelay by default (`reqwest-0.13.5/src/async_impl/client.rs:370`). `axum::serve` doesn't (axum 0.8.9 only sets it in its own tests). The fix is `listener.tap_io(|tcp| tcp.set_nodelay(true))`.

**Key mirrors and utoipa.** Serde remote mirrors (`KeyEventDef`, `KeyCodeDef`, `KeyEventKindDef`) with `#[derive(Serialize, Deserialize, ToSchema)]` compile. `#[schema(value_type = ...)]` works on named fields. The bitflags fields (`KeyModifiers`, `KeyEventState`) work through one generic `with` module built on `bitflags::Flags` (`iter_names` and `from_name`) and `#[schema(value_type = Vec<String>)]`. Wire shape:

```json
{"type":"key","pane":7,"key":{"code":{"Char":"b"},"modifiers":["SHIFT","CONTROL"],"kind":"Press","state":[]}}
```

The generated components are `InputFrame`, `KeyEventDef`, `KeyCodeDef`, `KeyEventKindDef`, `MediaKeyCodeDef` and `ModifierKeyCodeDef`. `KeyEventDef` is an object with `code` and `kind` as refs and `modifiers` and `state` as string arrays. All four fields are required.

The tuple variants `Media(MediaKeyCode)` and `Modifier(ModifierKeyCode)` need their attributes on the variant. On the unnamed field inside the variant, utoipa rejects `value_type`:

```text
error: unexpected attribute: value_type, expected any of: inline
  Media(#[serde(with = "MediaKeyCodeDef")] #[schema(value_type = MediaKeyCodeDef)] MediaKeyCode)
```

`#[schema(inline)]` in that position fails too, because it still needs `MediaKeyCode: ToSchema`. Putting both attributes on the variant works:

```rust
#[serde(with = "MediaKeyCodeDef")]
#[schema(value_type = MediaKeyCodeDef)]
Media(MediaKeyCode),
```

The schema then includes `MediaKeyCodeDef` and `ModifierKeyCodeDef` as components, referenced from `{"Media": ...}` and `{"Modifier": ...}` object variants, and both round trip:

```json
{"type":"key","pane":7,"key":{"code":{"Modifier":"LeftShift"},"modifiers":["SHIFT"],"kind":"Press","state":[]}}
```

Something to watch: flag names serialize in bit order (`SHIFT` before `CONTROL`), and an empty set serializes as `[]`. The schema says only "array of strings". It doesn't list the flag names.

### Herdr's portable-pty patch

Herdr (commit `d6b40d4`) pins `portable-pty = "=0.9.0"` and replaces it with `vendor/portable-pty` through `[patch.crates-io]`. `vendor/portable-pty.patches.md` lists three active patches, all for Windows:

1. `0001-control-conpty-loading`: portable-pty finds `conpty.dll` through the DLL search path. Herdr loads a hash-checked bundled ConPTY from an absolute path instead. This is a DLL-hijacking concern for a packaged Windows app (herdr#761, #1533).
2. `0002-windows-raw-command-tail`: lets Herdr pass a raw `cmd.exe /d /c` command tail without argv quoting (herdr#1041).
3. `0003-reject-malformed-windows-environments`: Windows environments can contain registry values that portable-pty serializes into an invalid environment block. `CreateProcessW` then fails with error 87 (herdr#3430, wezterm#4364).

Nothing changes Unix behavior. For Ship, Windows stays best effort, and 0003 is the one to remember: on an affected Windows machine, pane creation can fail with error 87. That gets fixed when a Windows user reports it.

### Relay bus saturation

A second probe asked whether the relay bus fills up once pane tasks publish screens and titles on it. It used the real `ship_core::relay::RelayBus` with the server's bounded(64) mailbox. Each pane task published one screen (about 240 KB of cells behind an `Arc`) and one title per tick, using an awaited `tell`. Viewers subscribed closure sinks that write the latest screen per pane into a `watch`. A stand-in state actor received titles through the `Recipient` sink, and for each title it cloned a 200-entry tree and published a replica back through the bus, the way `commit` does. "Paced" means one tick per 16 ms frame. "Flood" means publishing in a loop with no pause. Each run lasted 2 s.

| Scenario | Bus msg/s | Max publish wait | Titles sent | Titles dropped |
| --- | --- | --- | --- | --- |
| paced, 10 panes, 2 viewers | 1,076 | 70 µs | 1,080 | 0 |
| paced, 50 panes, 4 viewers | 5,394 | 613 µs | 5,400 | 0 |
| flood, 10 panes, 2 viewers | 1,051,495 | 269 µs | 1,051,576 | 638,332 (61%) |
| flood, 50 panes, 4 viewers | 1,581,865 | 665 µs | 1,581,910 | 1,518,221 (96%) |

The bus itself never became the bottleneck. A full mailbox makes publishers wait, and nothing is dropped. The drops come from the state actor's mailbox: under a flood, its republish-per-title work can't keep up, and the `Recipient` sink drops what doesn't fit. At frame pace, every title arrived. Conclusions for the architecture: pane tasks pace titles along with screens, and exit codes travel through a sink that never drops.

## Evidence

- Transport probe output from two full runs on macOS, summarized in the first table.
- Bus probe output from one run on macOS, in the relay bus table. Every scenario printed `ordered: true` and a full received byte count.
- The utoipa compiler errors quoted above, and the generated schema and round-tripped frames printed by the build with variant-level attributes.
- Herdr's `vendor/portable-pty.patches.md` at `d6b40d4`, read directly.
- Source references: `axum-0.8.9/src/serve/mod.rs` (nodelay only in tests, `http2().enable_connect_protocol()` in the auto builder) and `reqwest-0.13.5/src/async_impl/client.rs:370` (`nodelay: true`).

Both probes' sources lived in the session scratchpad and were not kept in the repository.

## Uncertain decisions

- Nagle's effect over a real network is untested. The nodelay recommendation is based on known behavior, not a measurement.
- Linux is entirely unverified. EIO and readiness behavior there is a portable-pty question, not a transport one, but the transport numbers themselves were also only taken on macOS.
