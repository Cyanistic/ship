# Codebase

Where things live in Ship. `AGENTS.md` still applies; this is orientation, and the code wins wherever the two disagree.

## Crates

- `crates/ship`: the `ship` binary. CLI parsing, local server discovery and startup.
- `crates/ship-server`: the server. HTTP API, attach stream, tabs, panes and their PTYs.
- `crates/ship-client`: the HTTP client and full-screen client, including the keymap. `src/keymap/defaults.toml` holds the default keys.
- `crates/ship-core`: types shared across the client/server boundary (commands, model, layout, screens, protocol, config) and errors.
- `crates/ship-macros`: Ship's own derives.
- `vendor/ratatui-ghostty`: a vendored Ratatui widget over `libghostty-vt`, built as a workspace member.

## Building

Building needs Zig 0.16 on `PATH` in addition to Rust. `libghostty-vt-sys` fetches Ghostty's source and builds it with Zig on the first build, so that build also needs `git` and network access. On Linux, the client's sound playback goes through ALSA, so building also needs `pkg-config` and ALSA's development headers (`libasound2-dev` on Debian and Ubuntu).

## OpenSpec layout

Papers live in `openspec/changes/<change>/design/` as `product.md`, `architecture.md` and `program.md`, next to the change's `proposal.md`, `design.md` and `tasks.md`. Finished changes move to `openspec/changes/archive/`, and their requirements are synced into `openspec/specs/<capability>/spec.md`, the exact record of shipped behavior.
