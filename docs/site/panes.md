# Panes

Every pane runs a program in its own terminal, your shell unless you say otherwise. When a program exits, its pane stays open with its output so you can read what happened. Close it with `alt-shift-x`.

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

Inside a Ship pane, you can leave out `--pane`; commands act on the pane they run in.
