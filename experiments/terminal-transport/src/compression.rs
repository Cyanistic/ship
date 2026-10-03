//! Saved-message codec probe. No native capture, network, or production schema.
use super::{Screen, bench::OrderedScreen};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    hint::black_box,
    io::{Read, Write},
    time::Instant,
};
use structdiff::StructDiff;

const WARMUP: usize = 10;
const SAMPLES: usize = 50;
const STRATEGIES: [&str; 4] = [
    "full-snapshot",
    "json-patch-cached",
    "structdiff-default",
    "structdiff-ordered",
];
#[derive(Clone, Copy)]
enum Codec {
    Raw,
    Gzip,
    Zstd,
}
impl Codec {
    fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Gzip => "gzip-1",
            Self::Zstd => "zstd-1",
        }
    }
    fn compress(self, bytes: &[u8]) -> Vec<u8> {
        match self {
            Self::Raw => bytes.to_vec(),
            Self::Gzip => {
                let mut e = GzEncoder::new(Vec::new(), Compression::new(1));
                e.write_all(bytes).unwrap();
                e.finish().unwrap()
            }
            Self::Zstd => zstd::stream::encode_all(bytes, 1).unwrap(),
        }
    }
    fn decompress(self, bytes: &[u8]) -> Vec<u8> {
        match self {
            Self::Raw => bytes.to_vec(),
            Self::Gzip => {
                let mut out = Vec::new();
                GzDecoder::new(bytes).read_to_end(&mut out).unwrap();
                out
            }
            Self::Zstd => zstd::stream::decode_all(bytes).unwrap(),
        }
    }
}
#[derive(Serialize)]
struct Timing {
    warmup: usize,
    samples: usize,
    median_us: f64,
    p95_us: f64,
}
fn measure<S, R>(mut setup: impl FnMut() -> S, mut work: impl FnMut(S) -> R) -> Timing {
    for _ in 0..WARMUP {
        black_box(work(black_box(setup())));
    }
    let mut ns = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let state = black_box(setup());
        let start = Instant::now();
        let output = black_box(work(state));
        ns.push(start.elapsed().as_nanos() as u64);
        black_box(&output);
        drop(output);
    }
    ns.sort_unstable();
    Timing {
        warmup: WARMUP,
        samples: SAMPLES,
        median_us: (ns[SAMPLES / 2 - 1] as f64 + ns[SAMPLES / 2] as f64) / 2000.0,
        p95_us: ns[(SAMPLES * 95).div_ceil(100) - 1] as f64 / 1000.0,
    }
}
fn message(strategy: &str, old: Option<&Screen>, new: &Screen, prior: Option<&Value>) -> Vec<u8> {
    if old.is_none() || strategy == "full-snapshot" {
        return serde_json::to_vec(new).unwrap();
    }
    let old = old.unwrap();
    match strategy {
        "json-patch-cached" => serde_json::to_vec(&json_patch::diff(
            prior.unwrap(),
            &serde_json::to_value(new).unwrap(),
        ))
        .unwrap(),
        "structdiff-default" => serde_json::to_vec(&old.diff(new)).unwrap(),
        "structdiff-ordered" => {
            serde_json::to_vec(&OrderedScreen::from(old).diff(&OrderedScreen::from(new))).unwrap()
        }
        _ => unreachable!(),
    }
}
// Real primary pipeline. Reset clones are outside timing; prior JSON eviction and
// clone of retained client JSON for typed reconstruction remain inside timing.
fn roundtrip(
    strategy: &str,
    codec: Codec,
    old: Option<&Screen>,
    new: &Screen,
    prior: Option<Value>,
    client: Option<Value>,
    typed: Option<Screen>,
) -> (Option<Value>, Option<Value>, Screen) {
    if old.is_none() || strategy == "full-snapshot" {
        let bytes = serde_json::to_vec(black_box(new)).unwrap();
        let compressed;
        let decoded;
        let payload = if matches!(codec, Codec::Raw) {
            &bytes
        } else {
            compressed = codec.compress(&bytes);
            decoded = codec.decompress(&compressed);
            &decoded
        };
        let target = serde_json::from_slice(payload).unwrap();
        // Initial messages establish both caches for the JSON strategy.
        if strategy == "json-patch-cached" {
            let doc = serde_json::from_slice::<Value>(payload).unwrap();
            return (Some(serde_json::to_value(new).unwrap()), Some(doc), target);
        }
        return (None, None, target);
    }
    let (cache, bytes) = match strategy {
        "json-patch-cached" => {
            let after = serde_json::to_value(black_box(new)).unwrap();
            let bytes =
                serde_json::to_vec(&json_patch::diff(prior.as_ref().unwrap(), &after)).unwrap();
            drop(prior);
            (Some(after), bytes)
        }
        "structdiff-default" => (
            None,
            serde_json::to_vec(&black_box(old.unwrap()).diff(black_box(new))).unwrap(),
        ),
        _ => unreachable!(),
    };
    let compressed;
    let decoded;
    let payload = if matches!(codec, Codec::Raw) {
        &bytes
    } else {
        compressed = codec.compress(&bytes);
        decoded = codec.decompress(&compressed);
        &decoded
    };
    match strategy {
        "json-patch-cached" => {
            let patch: json_patch::Patch = serde_json::from_slice(payload).unwrap();
            let mut doc = client.unwrap();
            json_patch::patch(&mut doc, &patch).unwrap();
            let screen = serde_json::from_value(doc.clone()).unwrap();
            (cache, Some(doc), screen)
        }
        "structdiff-default" => {
            let patch: Vec<<Screen as StructDiff>::Diff> = serde_json::from_slice(payload).unwrap();
            (None, None, typed.unwrap().apply(patch))
        }
        _ => unreachable!(),
    }
}
fn assert_message(strategy: &str, initial: bool, old: Option<&Screen>, new: &Screen, bytes: &[u8]) {
    if initial || strategy == "full-snapshot" {
        assert_eq!(serde_json::from_slice::<Screen>(bytes).unwrap(), *new);
        return;
    }
    let old = old.unwrap();
    match strategy {
        "json-patch-cached" => {
            let mut doc = serde_json::to_value(old).unwrap();
            let patch: json_patch::Patch = serde_json::from_slice(bytes).unwrap();
            json_patch::patch(&mut doc, &patch).unwrap();
            let target: Screen = serde_json::from_value(doc.clone()).unwrap();
            assert_eq!(target, *new);
            assert_eq!(doc, serde_json::to_value(new).unwrap());
        }
        "structdiff-default" => {
            let patch: Vec<<Screen as StructDiff>::Diff> = serde_json::from_slice(bytes).unwrap();
            assert_eq!(old.clone().apply(patch), *new);
        }
        "structdiff-ordered" => {
            let patch: Vec<<OrderedScreen as StructDiff>::Diff> =
                serde_json::from_slice(bytes).unwrap();
            assert_eq!(
                OrderedScreen::from(old).apply(patch),
                OrderedScreen::from(new)
            );
        }
        _ => unreachable!(),
    }
}
// One persistent gzip encoder, sync-flush after each complete textual SSE event.
// Decoding every accumulated unfinished prefix must produce ALL complete events.
fn continuous(strategy: &str, captures: &[(String, Screen)], messages: &[Vec<u8>]) -> Value {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(1));
    let mut expected = Vec::new();
    let mut prev_len = 0;
    let mut events = Vec::new();
    let mut client: Option<Value> = None;
    let mut typed: Option<Screen> = None;
    let mut ordered: Option<OrderedScreen> = None;
    for (i, ((label, screen), payload)) in captures.iter().zip(messages).enumerate() {
        let kind = if i == 0 || strategy == "full-snapshot" {
            "snapshot"
        } else {
            "patch"
        };
        assert!(!payload.contains(&b'\n'));
        let frame = format!(
            "event: {kind}\ndata: {}\n\n",
            std::str::from_utf8(payload).unwrap()
        )
        .into_bytes();
        expected.extend_from_slice(&frame);
        encoder.write_all(&frame).unwrap();
        encoder.flush().unwrap();
        let prefix = encoder.get_ref();
        let mut recovered = Vec::new();
        let result = GzDecoder::new(prefix.as_slice()).read_to_end(&mut recovered);
        // The footer is intentionally absent, not a completed gzip-file test.
        assert!(
            matches!(result, Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
            "unexpected prefix result: {result:?}"
        );
        assert_eq!(
            recovered, expected,
            "SSE flush failed to recover complete events"
        );
        let recovered_frame = &recovered[recovered.len() - frame.len()..];
        assert!(recovered_frame.ends_with(b"\n\n"));
        let head = format!("event: {kind}\ndata: ");
        let data = &recovered_frame[head.len()..recovered_frame.len() - 2];
        if i == 0 || strategy == "full-snapshot" {
            let target: Screen = serde_json::from_slice(data).unwrap();
            assert_eq!(target, *screen);
            client = Some(serde_json::from_slice(data).unwrap());
            ordered = Some(OrderedScreen::from(&target));
            typed = Some(target);
        } else {
            match strategy {
                "json-patch-cached" => {
                    let patch: json_patch::Patch = serde_json::from_slice(data).unwrap();
                    json_patch::patch(client.as_mut().unwrap(), &patch).unwrap();
                    typed = Some(serde_json::from_value(client.as_ref().unwrap().clone()).unwrap());
                }
                "structdiff-default" => {
                    let patch: Vec<<Screen as StructDiff>::Diff> =
                        serde_json::from_slice(data).unwrap();
                    typed = Some(typed.take().unwrap().apply(patch));
                }
                "structdiff-ordered" => {
                    let patch: Vec<<OrderedScreen as StructDiff>::Diff> =
                        serde_json::from_slice(data).unwrap();
                    ordered = Some(ordered.take().unwrap().apply(patch));
                }
                _ => unreachable!(),
            }
        }
        if strategy == "structdiff-ordered" {
            assert_eq!(ordered.as_ref().unwrap(), &OrderedScreen::from(screen));
        } else {
            assert_eq!(typed.as_ref().unwrap(), screen);
        }
        events.push(json!({"transition": label, "payload_bytes": payload.len(), "sse_frame_bytes": frame.len(),
            "gzip_incremental_bytes": prefix.len()-prev_len, "gzip_accumulated_bytes": prefix.len(),
            "independent_framed_gzip_bytes": Codec::Gzip.compress(&frame).len(),
            "prefix_decode": "PASS: all complete events before stream close; expected UnexpectedEof", "sequential_typed_equality": "PASS"}));
        prev_len = prefix.len();
    }
    let completed = encoder.finish().unwrap();
    let decoded = Codec::Gzip.decompress(&completed);
    assert_eq!(decoded, expected);
    json!({"strategy": strategy, "level": 1, "events": events, "finish_only_bytes": completed.len()-prev_len,
        "completed_bytes": completed.len(), "frame_total_bytes": expected.len(), "timing": "not measured; bytes and prefix validation only"})
}

pub(super) fn run(size: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!cfg!(debug_assertions), "release required");
    if !matches!(size, "80x24" | "120x40" | "160x50" | "80x50" | "160x25") {
        return Err("unapproved size".into());
    }
    let captures: Vec<(String, Screen)> = serde_json::from_slice(&std::fs::read(format!(
        "size-results/{size}/captures.json"
    ))?)?;
    assert_eq!(
        captures.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(),
        [
            "initial-filled",
            "small-edit",
            "one-row-scroll",
            "unchanged"
        ]
    );
    for (_, s) in &captures {
        assert_eq!(format!("{}x{}", s.cols, s.rows), size);
        assert_eq!(s.cells.len(), s.cols as usize * s.rows as usize);
        assert_eq!(
            serde_json::to_vec(&OrderedScreen::from(s))?,
            serde_json::to_vec(s)?
        );
    }
    let output = std::path::Path::new("compression-results").join(size);
    std::fs::create_dir_all(&output)?;
    let mut rows = Vec::new();
    let mut streams = Vec::new();
    for strategy in STRATEGIES {
        // Precompute exact compact messages ONCE, including ordered O(n^2) diff.
        let messages: Vec<_> = captures
            .iter()
            .enumerate()
            .map(|(i, (_, new))| {
                let old = i.checked_sub(1).map(|j| &captures[j].1);
                let prior = old.map(|s| serde_json::to_value(s).unwrap());
                message(strategy, old, new, prior.as_ref())
            })
            .collect();
        for (i, ((label, new), bytes)) in captures.iter().zip(&messages).enumerate() {
            let old = i.checked_sub(1).map(|j| &captures[j].1);
            let prior = old.map(|s| serde_json::to_value(s).unwrap());
            std::fs::write(output.join(format!("{strategy}-{label}.json")), bytes)?;
            for codec in [Codec::Raw, Codec::Gzip, Codec::Zstd] {
                let compressed = codec.compress(bytes); // Cached output for decode timing.
                let decoded = codec.decompress(&compressed);
                assert_eq!(decoded, *bytes);
                assert_message(strategy, i == 0, old, new, &decoded);
                let (compression, decompression) = if matches!(codec, Codec::Raw) {
                    (None, None)
                } else {
                    (
                        Some(measure(|| (), |_| codec.compress(black_box(bytes)))),
                        Some(measure(|| (), |_| codec.decompress(black_box(&compressed)))),
                    )
                };
                let combined = if strategy == "structdiff-ordered" {
                    None
                } else {
                    let setup = || {
                        (
                            if strategy == "json-patch-cached" {
                                prior.clone()
                            } else {
                                None
                            },
                            if strategy == "json-patch-cached" {
                                prior.clone()
                            } else {
                                None
                            },
                            if strategy == "structdiff-default" {
                                old.cloned()
                            } else {
                                None
                            },
                        )
                    };
                    let (_, _, target) =
                        roundtrip(strategy, codec, old, new, setup().0, setup().1, setup().2);
                    assert_eq!(target, *new);
                    Some(measure(setup, |(p, c, t)| {
                        roundtrip(strategy, codec, black_box(old), black_box(new), p, c, t)
                    }))
                };
                rows.push(json!({"transition": label, "strategy": strategy, "codec": codec.name(), "raw_bytes": bytes.len(), "compressed_bytes": compressed.len(),
                    "compress": compression, "decompress": decompression, "combined_total": combined, "byte_equality": "PASS", "typed_equality": "PASS"}));
                println!(
                    "PASS {size} {label} {strategy} {} raw={} compressed={}",
                    codec.name(),
                    bytes.len(),
                    compressed.len()
                );
            }
        }
        streams.push(continuous(strategy, &captures, &messages));
        println!(
            "FLUSH PREFIX PASS {size} {strategy}: four SSE events decoded/applied before finish"
        );
    }
    std::fs::write(
        output.join("measurements.json"),
        serde_json::to_vec_pretty(&rows)?,
    )?;
    std::fs::write(
        output.join("continuous-gzip.json"),
        serde_json::to_vec_pretty(&streams)?,
    )?;
    println!(
        "COMPRESSION PASS {size}: 48 size rows, 64 codec batches, 36 directly measured total batches; 16 flushed prefixes"
    );
    Ok(())
}
