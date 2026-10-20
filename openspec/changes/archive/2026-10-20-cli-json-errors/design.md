# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three on 2026-10-20. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P4, FR-001 to FR-009, success criteria SC-001 to SC-004, no open unknowns.
- [`design/architecture.md`](design/architecture.md): shape A, where `run` returns an exit code, and key decisions 1 to 7.
- [`design/program.md`](design/program.md): the touched files and signatures, the three slices and the deviation log.

## Context

`main` is the only place a failed command's error reaches stderr. clap's errors exit before it. `ship config check` prints its findings itself and then returns an error just to get exit 1. `AppError` is shared by the server and the client, so the client's final error already carries the server's causes. See proposal.md for why.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (a script reads the error), P2 (telling whether a send went out), P3 (mistyped commands still explain themselves) and P4 (config check stays text).

**Non-goals:** see the product paper's Non-goals.

## Decisions

The work splits into three vertical slices. Each runs from a real command to output you can check.

1. **Error spelling** (architecture decision 5). Two serde attributes on the shared types, so HTTP bodies and the CLI line can't drift apart. Rejected: renaming only in the CLI printer.
2. **JSON line in `main`, and an exit code for config check** (decisions 1 to 4 and 6). `main` serializes the error, falling back to today's text if that fails. `dispatch` returns `Result<ExitCode>`, and only `config check` returns `Ok(FAILURE)`. Rejected: a "reported" marker error in `ship-core`, and `process::exit` inside the command.
3. **Specs and guide** (decision 7). The error body's shape goes into `health-exchange`, the spec that already owns the API's camelCase rule.

## Risks / Trade-offs

- [HTTP error bodies change spelling] → The only known consumers are Ship's client, which shares the types, and Cyan's scripts. Rollback is reverting two attributes.
- [A person sees JSON at the terminal] → Accepted in the product paper. A later change can add pretty-printing if it grates.
- [A future command could return `Ok(FAILURE)` silently] → Only `config check` does, and it prints its findings first.

## Migration Plan

None beyond the spelling change. Ship's client and server come from the same build, so they agree on the new spelling. External scripts reading `caused_by` or PascalCase kinds need updating.
