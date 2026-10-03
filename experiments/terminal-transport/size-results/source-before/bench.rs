//! Independent component and total-pipeline measurements on native captures only.
use super::{Cursor, OwnedCell, Screen};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use structdiff::{Difference, StructDiff};

const WARMUP: usize = 100;
const ITERATIONS: usize = 1000;

// Identical serialized fields. This attribute is only a comparator, not a schema choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Difference)]
struct OrderedScreen {
    cols: u16,
    rows: u16,
    #[difference(collection_strategy = "ordered_array_like")]
    cells: Vec<OwnedCell>,
    cursor: Cursor,
}
impl From<&Screen> for OrderedScreen {
    fn from(s: &Screen) -> Self {
        Self {
            cols: s.cols,
            rows: s.rows,
            cells: s.cells.clone(),
            cursor: s.cursor.clone(),
        }
    }
}

#[derive(Serialize)]
struct Measurement {
    transition: String,
    strategy: String,
    metric: String,
    warmup: usize,
    iterations: usize,
    median_us: f64,
    p95_us: f64,
}
#[derive(Default)]
struct Results(Vec<Measurement>);
impl Results {
    fn measure<S, R>(
        &mut self,
        label: &str,
        strategy: &str,
        metric: &str,
        mut setup: impl FnMut() -> S,
        mut work: impl FnMut(S) -> R,
    ) {
        // Setup/reset clones and returned-output destruction are outside the timer.
        for _ in 0..WARMUP {
            black_box(work(black_box(setup())));
        }
        let mut ns = Vec::with_capacity(ITERATIONS);
        for _ in 0..ITERATIONS {
            let state = black_box(setup());
            let start = Instant::now();
            let output = black_box(work(state));
            ns.push(start.elapsed().as_nanos() as u64);
            black_box(&output);
            drop(output);
        }
        ns.sort_unstable();
        // Even-sized median averages central two; p95 uses nearest-rank.
        let median_us = (ns[ITERATIONS / 2 - 1] as f64 + ns[ITERATIONS / 2] as f64) / 2000.0;
        let p95_us = ns[(ITERATIONS * 95).div_ceil(100) - 1] as f64 / 1000.0;
        println!(
            "TIMING {label} {strategy} {metric}: median_us={median_us:.3} p95_us={p95_us:.3} n={ITERATIONS} warmup={WARMUP}"
        );
        self.0.push(Measurement {
            transition: label.into(),
            strategy: strategy.into(),
            metric: metric.into(),
            warmup: WARMUP,
            iterations: ITERATIONS,
            median_us,
            p95_us,
        });
    }
}

fn json_client(mut document: Value, bytes: &[u8]) -> (Value, Screen) {
    let patch: json_patch::Patch = serde_json::from_slice(black_box(bytes)).unwrap();
    json_patch::patch(&mut document, &patch).unwrap();
    // Retain the JSON document for the next patch. This clone is timed, not hidden setup.
    let typed: Screen = serde_json::from_value(document.clone()).unwrap();
    (document, typed)
}

fn json_transition(
    r: &mut Results,
    label: &str,
    old: &Screen,
    new: &Screen,
    server_prior: &mut Value,
    client_document: &mut Value,
) -> Value {
    let before = serde_json::to_value(old).unwrap();
    let after = serde_json::to_value(new).unwrap();
    assert_eq!(*server_prior, before);
    assert_eq!(*client_document, before);
    let patch = json_patch::diff(&before, &after);
    assert_eq!(patch.0.is_empty(), old == new);
    let bytes = serde_json::to_vec(&patch).unwrap();
    let cached_patch = json_patch::diff(server_prior, &after);
    assert_eq!(serde_json::to_vec(&cached_patch).unwrap(), bytes);
    let (applied, reconstructed) = json_client(client_document.clone(), &bytes);
    assert_eq!(applied, after);
    assert_eq!(reconstructed, *new); // Includes every owned cell field and cursor.
    *client_document = applied;
    *server_prior = after.clone(); // Demonstrate prior-JSON cache advancing through the chain.
    let snapshot = serde_json::to_vec(new).unwrap();
    assert_eq!(serde_json::from_slice::<Screen>(&snapshot).unwrap(), *new);
    println!(
        "WIRE {label} json-patch: operations={} patch_bytes={} snapshot_bytes={} equality=PASS cached=PASS",
        patch.0.len(),
        bytes.len(),
        snapshot.len()
    );

    r.measure(
        label,
        "json-patch",
        "to_value_before_after",
        || (),
        |_| {
            (
                serde_json::to_value(black_box(old)).unwrap(),
                serde_json::to_value(black_box(new)).unwrap(),
            )
        },
    );
    r.measure(
        label,
        "json-patch-cached",
        "to_value_after",
        || (),
        |_| serde_json::to_value(black_box(new)).unwrap(),
    );
    r.measure(
        label,
        "json-patch",
        "diff",
        || (),
        |_| json_patch::diff(black_box(&before), black_box(&after)),
    );
    r.measure(
        label,
        "json-patch",
        "encode",
        || (),
        |_| serde_json::to_vec(black_box(&patch)).unwrap(),
    );
    r.measure(
        label,
        "json-patch",
        "client_decode_patch",
        || (),
        |_| serde_json::from_slice::<json_patch::Patch>(black_box(&bytes)).unwrap(),
    );
    r.measure(
        label,
        "json-patch",
        "client_apply",
        || before.clone(),
        |mut doc| {
            json_patch::patch(&mut doc, black_box(&patch)).unwrap();
            doc
        },
    );
    r.measure(
        label,
        "json-patch",
        "client_typed_reconstruction",
        || (),
        |_| serde_json::from_value::<Screen>(black_box(&after).clone()).unwrap(),
    );
    r.measure(
        label,
        "json-patch",
        "server_total",
        || (),
        |_| {
            let before = serde_json::to_value(black_box(old)).unwrap();
            let after = serde_json::to_value(black_box(new)).unwrap();
            let patch = json_patch::diff(&before, &after);
            serde_json::to_vec(&patch).unwrap()
        },
    );
    r.measure(
        label,
        "json-patch-cached",
        "server_total",
        || before.clone(),
        |prior| {
            let after = serde_json::to_value(black_box(new)).unwrap();
            let patch = json_patch::diff(&prior, &after);
            let bytes = serde_json::to_vec(&patch).unwrap();
            drop(prior); // Include old cache eviction; retain the new cache as output.
            (after, bytes)
        },
    );
    r.measure(
        label,
        "json-patch",
        "client_total",
        || before.clone(),
        |doc| json_client(doc, &bytes),
    );
    r.measure(
        label,
        "full-snapshot",
        "server_total",
        || (),
        |_| serde_json::to_vec(black_box(new)).unwrap(),
    );
    r.measure(
        label,
        "full-snapshot",
        "client_total",
        || (),
        |_| serde_json::from_slice::<Screen>(black_box(&snapshot)).unwrap(),
    );
    // This measures both sides and the in-memory encoded message, no network.
    r.measure(
        label,
        "json-patch",
        "encoded_roundtrip_total",
        || before.clone(),
        |doc| {
            let before = serde_json::to_value(black_box(old)).unwrap();
            let after = serde_json::to_value(black_box(new)).unwrap();
            let bytes = serde_json::to_vec(&json_patch::diff(&before, &after)).unwrap();
            json_client(doc, &bytes)
        },
    );
    r.measure(
        label,
        "json-patch-cached",
        "encoded_roundtrip_total",
        || (before.clone(), before.clone()),
        |(prior, doc)| {
            let after = serde_json::to_value(black_box(new)).unwrap();
            let bytes = serde_json::to_vec(&json_patch::diff(&prior, &after)).unwrap();
            drop(prior); // Reset outside timing; old cache eviction inside.
            (after, json_client(doc, &bytes))
        },
    );
    r.measure(
        label,
        "full-snapshot",
        "encoded_roundtrip_total",
        || (),
        |_| {
            let bytes = serde_json::to_vec(black_box(new)).unwrap();
            serde_json::from_slice::<Screen>(&bytes).unwrap()
        },
    );
    json!({"transition": label, "source": "actual-native-capture", "operations": patch.0.len(),
        "patch_bytes": bytes.len(), "snapshot_bytes": snapshot.len(), "equality": "PASS", "cached_equality": "PASS",
        "patch": patch})
}

fn struct_transition<T>(r: &mut Results, label: &str, strategy: &str, old: &T, new: &T) -> Value
where
    T: StructDiff + Clone + PartialEq + std::fmt::Debug + Serialize + DeserializeOwned,
    T::Diff: Serialize + DeserializeOwned,
{
    let patch = old.diff(new);
    assert_eq!(patch.is_empty(), old == new);
    let bytes = serde_json::to_vec(&patch).unwrap();
    let decoded: Vec<T::Diff> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(old.clone().apply(decoded), *new);
    let snapshot = serde_json::to_vec(new).unwrap();
    assert_eq!(serde_json::from_slice::<T>(&snapshot).unwrap(), *new);
    println!(
        "WIRE {label} {strategy}: fields={} patch_bytes={} snapshot_bytes={} equality=PASS",
        patch.len(),
        bytes.len(),
        snapshot.len()
    );
    r.measure(
        label,
        strategy,
        "diff",
        || (),
        |_| black_box(old).diff(black_box(new)),
    );
    r.measure(
        label,
        strategy,
        "encode",
        || (),
        |_| serde_json::to_vec(black_box(&patch)).unwrap(),
    );
    r.measure(
        label,
        strategy,
        "client_decode_patch",
        || (),
        |_| serde_json::from_slice::<Vec<T::Diff>>(black_box(&bytes)).unwrap(),
    );
    r.measure(
        label,
        strategy,
        "client_apply",
        || (old.clone(), patch.clone()),
        |(s, decoded)| s.apply(decoded),
    );
    // Owned old screen and decoded patch are reset outside this component timer.
    r.measure(
        label,
        strategy,
        "server_total",
        || (),
        |_| {
            let patch = black_box(old).diff(black_box(new));
            serde_json::to_vec(&patch).unwrap()
        },
    );
    r.measure(
        label,
        strategy,
        "client_total",
        || old.clone(),
        |s| {
            let decoded: Vec<T::Diff> = serde_json::from_slice(black_box(&bytes)).unwrap();
            s.apply(decoded) // Already typed; no serde_json::Value reconstruction.
        },
    );
    r.measure(
        label,
        strategy,
        "encoded_roundtrip_total",
        || old.clone(),
        |s| {
            let bytes = serde_json::to_vec(&black_box(old).diff(black_box(new))).unwrap();
            let decoded: Vec<T::Diff> = serde_json::from_slice(&bytes).unwrap();
            s.apply(decoded)
        },
    );
    json!({"transition": label, "strategy": strategy, "fields": patch.len(), "patch_bytes": bytes.len(),
        "snapshot_bytes": snapshot.len(), "equality": "PASS"})
}

pub(super) fn run(captures: &[(&str, Screen)]) {
    println!(
        "BENCH release={} source=actual-native-capture warmup={WARMUP} iterations={ITERATIONS} timing_unit=us no_network=true",
        !cfg!(debug_assertions)
    );
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    std::fs::write(
        "captures.json",
        serde_json::to_vec_pretty(captures).unwrap(),
    )
    .unwrap();
    let initial_bytes = serde_json::to_vec(&captures[0].1).unwrap();
    let initial_typed: Screen = serde_json::from_slice(&initial_bytes).unwrap();
    assert_eq!(initial_typed, captures[0].1);
    let mut client_document: Value = serde_json::from_slice(&initial_bytes).unwrap();
    let mut server_prior = serde_json::to_value(&captures[0].1).unwrap();
    assert_eq!(client_document, server_prior);
    println!(
        "INITIAL snapshot_bytes={} typed_and_json_decode=PASS",
        initial_bytes.len()
    );
    let mut results = Results::default();
    let mut evidence =
        vec![json!({"initial_snapshot_bytes": initial_bytes.len(), "equality": "PASS"})];
    for pair in captures.windows(2) {
        let (label, new) = &pair[1];
        let old = &pair[0].1;
        evidence.push(json_transition(
            &mut results,
            label,
            old,
            new,
            &mut server_prior,
            &mut client_document,
        ));
        evidence.push(struct_transition(
            &mut results,
            label,
            "structdiff-default",
            old,
            new,
        ));
        let old_ordered = OrderedScreen::from(old);
        let new_ordered = OrderedScreen::from(new);
        assert_eq!(
            serde_json::to_value(&old_ordered).unwrap(),
            serde_json::to_value(old).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&new_ordered).unwrap(),
            serde_json::to_value(new).unwrap()
        );
        evidence.push(struct_transition(
            &mut results,
            label,
            "structdiff-ordered",
            &old_ordered,
            &new_ordered,
        ));
    }
    assert_eq!(
        serde_json::from_value::<Screen>(client_document).unwrap(),
        captures.last().unwrap().1
    );
    std::fs::write(
        "measurements.json",
        serde_json::to_vec_pretty(&results.0).unwrap(),
    )
    .unwrap();
    std::fs::write(
        "wire-evidence.json",
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    println!(
        "BENCH PASS: {} metrics, native sequential client exact through unchanged; saved captures/measurements/wire-evidence.json",
        results.0.len()
    );
}
