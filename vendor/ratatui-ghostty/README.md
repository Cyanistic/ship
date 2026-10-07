# ratatui-ghostty

A [ratatui](https://github.com/ratatui/ratatui) integration for [libghostty-vt](https://crates.io/crates/libghostty-vt), the Ghostty VT state machine.

Renders a `libghostty_vt::Terminal` into a ratatui `Buffer` and encodes crossterm input events into raw PTY bytes.

## Features

- **`TerminalWidget`** — renders a `Terminal` + `RenderState` snapshot directly into a ratatui `Buffer`; supports cursor styling (Block, Bar, Underline, BlockHollow) and wide characters
- **`convert::from_crossterm`** — maps crossterm `KeyCode`, `KeyModifiers`, `MouseButton`, and `MouseEventKind` to libghostty-vt types
- **`convert::to_ratatui`** — maps libghostty-vt `style::Style` and `style::RgbColor` to ratatui `Style` and `Color`
- **`input::encode_key`** — encodes crossterm `KeyEvent` to PTY bytes; respects Kitty keyboard protocol flags and other terminal modes
- **`input::encode_mouse`** — encodes crossterm `MouseEvent` to PTY bytes; respects mouse tracking mode and SGR encoding
  - **Not supported:** `ScrollLeft` / `ScrollRight` events are silently dropped — `libghostty-vt` has no `Button::Six/Seven`; will be added once upstream exposes them
- **`Session`** — Wraps all ghostty state and provides a ready to use but less customizable API.

## Known upstream limitations

### libghostty-vt logs to stderr

The compiled `libghostty-vt` C library uses Zig's default `std.log`, which writes directly to stderr. This means unrecognised terminal sequences produce warnings like:

```
warning[stream]: unimplemented mode: 9001
```

The root cause is in `lib_vt.zig` inside the ghostty source — the C library build path uses `std.Options {}` (all defaults), with a comment acknowledging a custom `logFn` is still needed. Until that is fixed upstream, there is no runtime API to suppress the output.

**Temporary workaround** — redirect stderr to a log file at the start of your application:

```rust
fn redirect_stderr() {
    use std::os::unix::io::AsRawFd;
    if let Ok(log) = std::fs::File::create("/tmp/my-app.log") {
        unsafe { libc::dup2(log.as_raw_fd(), libc::STDERR_FILENO) };
    }
}
```
- **`input::encode_focus`** — encodes focus gained/lost (CSI I / CSI O)
- **`input::encode_paste`** — wraps paste data in bracketed paste sequences when the terminal has the mode enabled

## Examples

### Single terminal

A minimal example that spawns one terminal in a bordered box, with the terminal title shown in the border.

```sh
cargo run --example single_terminal
```

Press `C-q` to exit.

### Demo

A small tmux-like multiplexer built on top of this library. It supports multiple tabs and panes with a tall auto-layout and a subset of tmux keybindings (`C-b` prefix).

```sh
cargo run --example demo
```

The prefix key defaults to `C-b`. Override it with the `RATATUI_GHOSTTY_PREFIX_KEY` env var (single character):

```sh
RATATUI_GHOSTTY_PREFIX_KEY=a cargo run --example demo  # use C-a instead
```

### Keybindings

| Sequence | Action |
|---|---|
| `prefix c` | New tab |
| `prefix &` | Close active tab |
| `prefix n` | Next tab |
| `prefix p` | Previous tab |
| `prefix %` | Split pane (add pane to active tab) |
| `prefix x` | Close active pane |
| `prefix h/j/k/l` or arrows | Focus adjacent pane |
| `prefix q` | Quit |

Any other key after the prefix cancels prefix mode and forwards the key to the active pane.
