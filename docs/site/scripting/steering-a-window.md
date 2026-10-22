# Steering a window

<!--
Scope: Listing attached windows and moving one to a tab or pane.
Not here: creating tabs and panes beyond the `--select` flag.
-->

`ship view list` shows every attached window, and `ship view select` moves one of them to a tab or pane. Adding `--select` to a create command shows the new tab or pane in a window right away:

```sh
WINDOW=$(ship view list | jq -r 'keys_unsorted[0]')
ship tab create --name demo --starter --select --attachment "$WINDOW"
```

Without `--select`, creating something never moves anyone's view.
