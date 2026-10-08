# Drop sessions design

## Context

This is an assembled index, not a fourth design authority or a copy of the papers. All three papers are locked as of 2026-10-08. No implementation is authorized until Cyan requests it.

The papers are attached here:

- [Product](design/product.md): a server holds top-level tabs instead of sessions, bare `ship` opens the client on the whole server, and tabs and panes are targeted by ID. Amendment A-1 moves the first tab to server startup (`ship server --starter`).
- [Architecture](design/architecture.md): shape A, a top-level `IndexMap` of `Arc<Tab>` with `Arc` at every level, viewing records with an optional selection, and seeding between bind and accept. Amendment A1 makes a tab move name exactly one destination.
- [Program](design/program.md): the virtual skeleton, four implementation slices and an empty deviation log.

Decisions from conversation before the product paper are in [notes.md](notes.md).

## Goals / Non-Goals

Use the [product paper](design/product.md) as scope authority. The sidebar, overview and several machines belong to later changes, and Windows stays best effort.

## Decisions

Use the [architecture paper](design/architecture.md) as the approved shape. The [program paper](design/program.md#build-order) turns it into four slices: `Arc` at every level, top-level tabs replacing sessions, the starter, and docs with the size report. Program snippets are declarations inside a document, not source files or compilation evidence.
