# Proposal

Cyan approved this change in conversation on 2026-10-21. It is small enough to need no design papers: this proposal and the deltas under `specs/` are the contract.

## Why

There's no way to see Ship's defaults without reading its source, so a user, or an agent helping them, can't easily find what a setting accepts or how to change it. Every default now lives in one commented file embedded in the binary, so printing that file answers the question. Issue #15.

## What Changes

- **`ship config defaults`** prints the embedded defaults file to stdout, comments included, and exits 0. It reads no config file and needs no server.
- The defaults file's header says to copy only what you change, because a copied value stops following Ship's defaults.
- This reverses the keys-config product paper's "There is no `ship config defaults`."

Not in this change:
- Writing the file anywhere (`--write`). Redirect the output instead.
- Printing the config in use, defaults merged with the user's file.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `config-file`: a new requirement for `ship config defaults`.
- `local-foundation`: help lists `ship config defaults`.

## Impact

- `ship`: one subcommand in `cli.rs` and one match arm in `main.rs`.
- `ship-core`: two comment lines in `defaults.toml`.
- Docs: the guide's Configuration section. The README doesn't change.
