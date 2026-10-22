# Reading output

```sh
ship pane read --pane "$PANE"                 # what's on screen now
ship pane read --pane "$PANE" --lines 200     # the last 200 lines, including history
ship pane read --pane "$PANE" --ansi | cat    # keep the colors
```

Reading doesn't move anyone's scroll position. Panes whose program has exited can still be read.
