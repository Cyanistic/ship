# Local Markdown issue tracker

This is the Wayfinder fallback tracker for this repository. No external tracker is configured. To configure one later, run `/setup-matt-pocock-skills`.

## Wayfinding operations

- Map: `.scratch/<effort>/map.md`, labelled `wayfinder:map`.
- Children: one file per issue in `.scratch/<effort>/issues/NN-<slug>.md`; numeric ID is identity, title is the human-facing name.
- Metadata: `Type:` research/prototype/grilling/task; `Status:` open/claimed/resolved; `Assignee:` unassigned or the developer.
- Blocking: `Blocked by: NN, NN`; all listed issues must be resolved before the child is unblocked. Local files lack native dependencies, so this body convention is used.
- Frontier: open, unassigned, unblocked children, ordered by numeric ID. Do not list open children in the map body.
- Claim: assign to the developer and set status claimed, saving before any work.
- Resolve: append a resolution under `## Answer` (comments under `## Comments`), set status resolved, then add a titled link and one-line gist to the map.
- Preserve concurrent changes and resolve at most one non-research decision per session.

The tracker conventions follow the global setup skill's local tracker document. Current map: [Plan the first client/server terminal slice](first-terminal-slice/map.md).
