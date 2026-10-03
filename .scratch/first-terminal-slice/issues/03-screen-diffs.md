# Which screen collection strategy gives useful structdiff patches?

ID: 03
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: Cyan
Blocked by: None

## Question

For representative owned screen/cell data, which structdiff representation and collection strategy provide correct automatic application with acceptable JSON patch size and calculation cost for typing, scrolling and resizing?

Use the prototype skill after claiming. A temporary screen-shaped probe can proceed independently of live Ghostty capture, but reconcile the candidate with the capture ticket before protocol adoption. Verify serialization/deserialization/application and unchanged state; measure rather than claim optimal bandwidth. Do not substitute handwritten diff/apply code or adopt a new data model without Cyan's live agreement. No permanent tests.

## Comments

Resolution records Cyan's prior live decision after reviewing the collection experiment: use defaults, profile real behavior, and optimize only a demonstrated bottleneck. Cyan explicitly called the feasibility work done. This documentation update does not select an additional algorithm.

## Answer

Start with structdiff 0.7.3 and default whole-vector replacement of changed cells. Library-generated JSON patches and application are correct for the captured samples. The default offers little changed-screen payload savings, accepted for the first runnable slice.

Ordered-sequence and index-map experiments establish tradeoffs, not a production bottleneck. Further row-based experiments, custom algorithms and release benchmarks are deferred. Build real behavior, profile, then benchmark/optimize only if warranted. No handwritten patch engine is selected.

This choice preserves the authoritative server/HTTP/SSE architecture. A later representation change may alter patch schemas and still require coordinated clients.

## Evidence

[Terminal feasibility evidence](../../../docs/research/terminal-feasibility.md) preserves real capture versus synthetic sample distinctions, JSON byte tables, single debug timing limits and 18 passing serialize/deserialize/apply comparisons independently rerun by the parent. Nothing was measured in a release build or over HTTP/SSE.

## Uncertain decisions

Production performance and bandwidth are unknown. Release builds may accelerate computation but do not shrink identical serialized payloads. Future optimization choice remains open by design; no current optimization work is authorized.
