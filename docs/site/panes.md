# Panes

<!--
Scope: Splitting, moving between, zooming and closing panes, what happens when a pane's program exits, and the matching CLI commands.
Not here: scrolling and copying (scrolling-copying-and-the-mouse.md), scripting recipes (scripting/).
-->

Every pane runs a program in its own terminal, your shell unless you say otherwise. By default, the pane closes when its program exits. You can also close it with `alt-shift-x`.

| Key | Does |
| --- | --- |
| `alt-\|` / `alt--` | Split right / down |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next pane in the tab |
| `alt-shift-h` / `j` / `k` / `l` | Swap with the neighbor in that direction |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode: `h` / `j` / `k` / `l` move the pane's edge, `esc` leaves |
| `alt-shift-x` | Close the pane |

A new split opens in the directory you were working in, so after `cd ~/src/ship`, `alt--` gives you a shell there. You can also resize by dragging the border between two panes.

Zoom belongs to the tab, so everyone looking at it sees the zoomed pane, and its sidebar row shows `Z`. Moving, splitting or resizing unzooms first.

The CLI can do the same things. Panes created from the CLI start in your current directory, or in `--cwd`:

```sh
TAB=$(ship tab create --name work | jq -r .id)
LEFT=$(ship pane create --tab "$TAB" | jq -r .id)
RIGHT=$(ship pane create --pane "$LEFT" --direction right -- htop | jq -r .id)
ship pane zoom --pane "$LEFT"                    # zoom, run again to unzoom
ship pane swap --pane "$LEFT" --other "$RIGHT"
ship pane resize --pane "$LEFT" --direction right --amount 5
ship tab get --tab "$TAB" | jq .layout
ship tab close --tab "$TAB"
```

## When a program exits

| Exit behavior | Does |
| --- | --- |
| `close` (default) | Closes the pane when its program exits |
| `keep` | Keeps the pane's output and exit status until you close it |
| `keep_failed` | Closes after success; keeps the pane after a non-zero exit code or a signal |

When a pane closes, its neighbor takes the space and the selection moves there. If it was the last pane, the tab stays empty.

Set the behavior when creating a pane, or change it before the program exits:

```sh
ship pane create --tab "$TAB" --exit-behavior keep -- cargo test
ship pane set --pane "$PANE" --exit-behavior keep_failed
```

Changing the behavior of an already exited pane only stores the value; it doesn't close the pane. Kept panes can still be read, scrolled and copied, but no longer accept typing or resizing. [Reading output](scripting/reading-output.md) shows the create, wait, read and close workflow for scripts. To change the default for new panes, use [the config file](configuration.md#server-settings).
