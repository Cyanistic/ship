# Keys

All of Ship's default keys are Alt chords. Any other key goes to the program in the selected pane.

> [!NOTE]
> On macOS, set your terminal to use Option as Alt (often called "Option as Meta"), or these keys will type special characters instead.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-x` | Close the tab |
| `alt-\|` / `alt--` | Split the pane right / down |
| `alt-shift-x` | Close the pane |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next pane in the tab |
| `alt-a` | Jump to the next agent that needs you |
| `alt-shift-h` / `j` / `k` / `l` | Swap the pane with its neighbor in that direction |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode (below) |
| `alt-g` | Tabs mode (below) |
| `alt-b` | Hide or show the sidebar |
| `alt-pageup` / `alt-pagedown` | Scroll the pane up / down a page |
| `alt-q` | Detach |

## Resize and tabs modes

In resize mode, `h` / `j` / `k` / `l` move the pane's edge, `shift-h` / `shift-l` resize the sidebar and `shift-j` / `shift-k` move the line between tabs and agents. In tabs mode, `j` / `k` move between tabs and `h` / `l` fold and unfold. `esc` leaves either mode.

## The mouse

Click a pane, a tab or an agent to select it, scroll with the wheel, drag to select and copy text, drag pane borders and the sidebar's dividers to resize, and drag sidebar rows to move tabs. Right-click for actions, or click `≡` beside `tabs` for the global menu. [Scrolling, copying and the mouse](scrolling-copying-and-the-mouse.md) has the details.

## Changing them

Every key here can be rebound or turned off, and you can add your own modes, like a tmux-style `ctrl-b` prefix. [Configuration](configuration.md#keys-and-modes) shows how.
