# Reading output

<!--
Scope: Reading a pane's screen and history with `ship pane read`.
Not here: waiting for output (handing-work-to-another-agent.md).
-->

```sh
ship pane read --pane "$PANE"                 # what's on screen now
ship pane read --pane "$PANE" --lines 200     # the last 200 lines, including history
ship pane read --pane "$PANE" --ansi | cat    # keep the colors
```

Reading doesn't move anyone's scroll position.

## Reading after a program exits

Use `--exit-behavior keep` when you need a command's output after it exits. Without it, the pane normally closes and its output is gone:

```sh
TAB=$(ship tab create --name output | jq -r .id)
PANE=$(ship pane create --tab "$TAB" --exit-behavior keep -- sh -c 'echo hi; exit 3' | jq -r .id)
ship pane wait --pane "$PANE" --status exited
ship pane read --pane "$PANE"
ship pane close --pane "$PANE"
ship tab close --tab "$TAB"
```

The wait prints the pane with exit code 3, and the read prints `hi`. Close the pane when you've read what you need. For commands you send to an agent that keeps running, see [Handing work to another agent](handing-work-to-another-agent.md).
