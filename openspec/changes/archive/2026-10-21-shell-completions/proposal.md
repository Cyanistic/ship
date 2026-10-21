# Proposal

Cyan approved this change in conversation on 2026-10-21. It is small enough to need no design papers: this proposal and the deltas under `specs/` are the contract.

## Why

Ship's CLI has grown to dozens of commands and flags, and nothing helps a user type them. Shells complete a command's names and flags when given a script describing them, and clap can generate that script from the definitions Ship already has. The release pipeline (#45) is the moment to offer it, since installed binaries are where people would set it up.

## What Changes

- **`ship completions <shell>`** prints a tab-completion script for bash, zsh, fish, elvish or PowerShell to stdout and exits 0. It reads no config file and needs no server. An unknown shell is an argument error that exits 2.
- The script is generated from the same clap definitions that parse the command line, through `clap_complete`, so it can't drift from the CLI.

Not in this change:
- Completing live values such as tab and pane IDs.
- A man page.
- Shipping pre-generated scripts in release archives.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `local-foundation`: a new requirement for `ship completions`, and help lists it.

## Impact

- `ship`: one subcommand in `cli.rs`, one match arm in `main.rs`, and the `clap_complete` dependency.
- Docs: a Shell completions section in the guide. The README doesn't change.
