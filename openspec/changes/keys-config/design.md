# Keys and config design

## Context

This is an assembled index, not a fourth design authority or a copy of the papers. All three papers are locked as of 2026-10-09. No implementation is authorized until Cyan requests it.

The papers are attached here:

- [Product](design/product.md): one config file with `[server]` (the shell) and `[client]` (user-defined sticky and one-shot modes), Alt-chord defaults from Herdr, `server.`/`client.` actions where `server.` paths are CLI paths, `close` replacing `rm`, and `--tab`/`--pane` with in-pane defaults. Amendment A-1 nests the modes under `[client]`, and A-2 drops config warnings.
- [Architecture](design/architecture.md): shape B, where each process owns its part of the file and the binary resolves the path. The command tree lives in `ship-core` with clap behind a feature, one `execute` in `ship-client` serves the CLI and keys, figment loads the file with a crokey provider that canonicalizes chords, and notify watches the config's directory.
- [Program](design/program.md): the virtual skeleton, five implementation slices and an empty deviation log.

Decisions from conversation before the product paper are in [notes.md](notes.md). The product paper supersedes it wherever the two differ.

## Goals / Non-Goals

Use the [product paper](design/product.md) as scope authority. Floating panes, keyboard enhancement flags, the CLI driving a client, command bindings, configurable mouse gestures and the layout actions themselves belong to later changes, and Windows stays best effort.

## Decisions

Use the [architecture paper](design/architecture.md) as the approved shape. The [program paper](design/program.md#build-order) turns it into five slices:

1. One command tree for the CLI.
2. The config path and server settings.
3. Keymap and modes.
4. Reload on save.
5. Docs with the size report.

Program snippets are declarations inside a document, not source files or compilation evidence.

## Risks / Trade-offs

See the architecture paper's [Risks and unknowns](design/architecture.md#risks-and-unknowns): enhancement flags and Shift on symbols, one server per config path, the directory key-made panes start in, figment error locations, the command queue over a network, and Option-as-Alt on macOS.
