# Keys and config: decisions from conversation

Status: input for the product paper, not a paper. Recorded on 2026-10-08 from design conversations with Cyan, so the decisions survive until `/product-design` turns them into `design/product.md`. Nothing here authorizes implementation.

This change lands after dropping sessions and before layout and sidebar. The layout draft (`openspec/changes/layout-sidebar/design/product.md`) already names actions this change defines.

## Decided

### One config file

- `~/.config/ship/config.toml`, located with `etcetera`. Overridden with `--config <file>` or `SHIP_CONFIG`. The override is a file, not a directory.
- Client and server read the same file, and each reads the sections it owns: the client reads `[keys]`, the server reads `[server]`. On a remote machine, that machine's server reads its own copy, as Herdr does.
- Relative paths in the file, when any setting needs one, resolve from the config file's folder (Herdr: `src/config/io.rs:185`).
- No environment-variable layer. `SHIP_`-prefixed config variables would collide with `SHIP_SERVER_URL`, `SHIP_BIN` and `SHIP_PANE_ID`, which the server sets in panes (`crates/ship-server/src/pane.rs:120-122`).
- `deny_unknown_fields`, so typos fail loudly.

### Settings

Only what something reads today:

```toml
[keys.normal]
"alt-n" = "new_tab"
"alt-w" = "toggle_floating"   # "not available yet" until floating panes land

[keys.nav]
"j" = "nav_down"

[server]
shell = "/bin/zsh"            # replaces portable-pty's get_shell() at crates/ship-server/src/pane.rs:241
```

Each later setting arrives with its feature: mouse capture, sidebar width and borders with layout; scrollback size with scrollback; floating pane size with floating panes; theme and notifications later. Port, server URL, frame timing, backoff, timeouts and `TERM`-family variables are not settings.

### Keymap

- **Chord on the left, per mode:** `[keys.normal] "alt-n" = "new_tab"`. The type is `HashMap<KeyCombination, Action>` per mode, so the file is the lookup table and one chord maps to exactly one action.
- **The user's chords replace the defaults' chords.** A chord bound in the file displaces the default action on that chord, which gives Herdr's displacement rule (#747) by construction. `"none"` unbinds a chord. `clear_defaults` drops a mode's defaults entirely, like Zellij's `clear-defaults=true`.
- **Duplicate chords are an error.** TOML only rejects identical spellings, so `"alt-n"` and `"Alt-N"` both parse to the same chord; building the map must reject that instead of letting the last one win.
- **Modes are a fixed enum**, not user-defined: normal, nav, resize, and prefix, plus whatever later changes add.
- **Prefix is one-shot.** One key, then back to normal; an unbound key exits the mode. `[keys.prefix]` is empty by default, so `ctrl-b` reaches programs. Someone who wants tmux habits binds `"ctrl-b" = "prefix"` in normal and fills `[keys.prefix]`, with `send_prefix` to pass `ctrl-b` through.
- **Defaults are plain Alt chords.** The `C-b` placeholder prefix goes away. Docs mention Option-as-Alt on macOS (Ghostty: `macos-option-as-alt`).
- **The full action set is exposed now.** Actions whose feature isn't built yet (splits, sidebar, floating panes) say "not available yet" instead of being missing.
- **No command bindings.** Scripting goes through the `ship` CLI. Cyan's one Herdr command binding was a scratch shell on `alt+w`, which `toggle_floating` replaces. A binding's value can later grow from an action name into a table without breaking the format.
- **No session actions.** `NextSession` and `PrevSession` (`crates/ship-client/src/ui/keys.rs:70-71`) go away with sessions; moving between top-level tabs is ordinary tab navigation.

### Machinery

- A bad file at startup falls back to the defaults with a warning. A bad file on reload keeps the running config.
- `ship config defaults` prints the full default config. `ship config check` parses a file and reports errors and duplicate chords.
- A `reload_config` action. No file watching.

### Dependencies

- `crokey` 1.5.0 for chords. Verified: it depends on crossterm 0.29; its `Deserialize` reads a string and calls `FromStr` (`key_combination.rs:107`), so it works as a TOML map key; it lowercases input (`parse.rs:106`). Separator is `-`, which Cyan accepted. `"ctrl-b-c"` means keys held together, not a sequence.
- `strum` (`EnumIter`) with serde `rename_all` for action names and listing them.
- `etcetera` for the config location.
- `figment` for layering defaults under the file, pending the open question below.

## Open

- **figment or Cyan's `Partial<T>`** (from ttsfx) for merging the file over the defaults. Lean: figment, if it keeps its merge rule (tables recurse, arrays replace) and its error locations. Not checked against its current version.
- **The exact action list and default bindings.**
- **Where `clear_defaults` sits:** per mode (`[keys.nav] clear_defaults = true`) or one table listing modes. Per mode was the direction.
- **Keyboard enhancement flags.** Possibly a `keys.enhanced` toggle later, if enabling them breaks some terminal. Leave it out until the keys work shows whether enhancement flags are needed at all.
