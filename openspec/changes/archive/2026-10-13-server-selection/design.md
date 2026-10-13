# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Product, architecture and program were locked by Cyan on 2026-10-13, after an amendment the same day gave bindings a named `action` and an optional `hint`. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P3, FR-001 to FR-018, success criteria.
- [`design/architecture.md`](design/architecture.md): shape A, one lane where every step waits, and key decisions 1 to 9.
- [`design/program.md`](design/program.md): the planned files, types and signatures, the build order and the deviation log.

Slice-1 amendment: Cyan waived strict list rejection. Tables remain the documented binding syntax, with derived Serde parsing and no guard against incidental positional forms. The approval is recorded in the product paper and the program deviation log.

## Context

Each client's selection already lives on the server, one per attachment, but only the client can change it, by sending it with its view. Keys typed right after an action that changes the selection go to the old pane on a slow link, because the client stamps typing with the selection at press and sends it at once (#20). This change makes the selection readable and settable by attachment, lets creates select what they made in the same edit, and runs everything a client does through one ordered lane. [handoff.md](handoff.md) records an earlier direction, server-side navigation and a "selected" target, which was dropped.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (type right after creating), P2 (create without stealing focus) and P3 (steer a client from a script).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Ordering a server action after keys typed just before it. Pipeline fixes it and is deferred.
- Sending a key's action before the previous one's result arrives.
- A "selected" target on any endpoint.
- Moving navigation, folds, memory or modes to the server.
- `server.run` and `client.run`.

## Decisions

- **One lane, every step waits** (architecture shape A, decisions 1 and 2). Bound actions, keys, pastes, clicks and mouse frames queue in order. Targets resolve when an item reaches the front. A server action, including the select a navigation becomes, waits for its reply and for the replica to catch up. Input is written to the stream and doesn't wait (decision 4).
- **Selection gets its own endpoint** (decision 3). `GET /api/v0/views`, `GET /api/v0/views/{attachment}` and `PUT /api/v0/views/{attachment}/selection`, used by the lane and `ship view`. The view PUT keeps only `area`.
- **Creates select inside the commit** (decision 5), with `--select` and `--attachment`, or `select = true` on a key.
- **The client keeps its own unzoom for navigation** (decision 6), and selecting a zoom-hidden pane unzooms on the server (FR-004).
- **Disconnect clears the lane** (decision 8).
- **Bindings are one named action with an optional hint** (FR-018, decision 7). `{ action = { … }, hint = "…" }`, `{}` unbinds, and an unset hint comes from `Action::hint` through `strum`.
- **The client's state is one `Local` struct**, and each turn is a method on it (program paper, rationale).
- **Settled program gates:**
  - G1: while the lane is busy, a press on any pane selects it.
  - G2: `ship view select --attachment ID NODE`, or `--none`; omitting both is an error.
  - G3: the lane treats a 404 from its own select as nothing to do.
  - G4: a throwaway latency proxy in `experiments/latency-proxy/`, removed after SC-001.
- **Protocol version 10.**

## Build order

Program paper, "Build order". Four feature slices, then a slice for docs, specs and size:

1. Bindings hold one named action.
2. Views and `ship view`.
3. Creates select.
4. The lane.
5. Docs, specs and size.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- Key repeat lags behind over a slow link, one round trip per navigation step.
- Two unzoom paths, the client's and the select endpoint's.
- SC-001 needs the delaying proxy.

Estimated size: within 150 lines either way of today's implementation Rust, unmeasured.

## Migration Plan

Config files rewrite every binding from `[{ … }]` to `{ action = { … } }`, and `[]` to `{}`. A bare action or unknown binding field fails `ship config check` with its line and column. Strict rejection of positional list forms is waived; their incidental acceptance by Serde is left alone, not documented as binding syntax. Protocol 9 goes to 10, so server and client are rebuilt as a pair, and mismatched versions fail the health check as before.

## Open Questions

None blocking. The deferred issues listed in the product paper are filed when the pull request lands.
