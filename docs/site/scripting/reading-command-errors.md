# Reading command errors

<!--
Scope: The JSON error a failing command prints, and how a script reads and branches on it.
Not here: the HTTP API's errors (openspec/specs/).
-->

When a command fails, its last stderr line is a JSON error and it exits 1, with nothing on stdout. Logs can come before that line. Capture stderr and read the error with `jq`:

```sh
if ship pane send --pane "$PANE" "Continue" --key enter --wait 2>ship-error.log; then
  echo "Turn finished"
else
  tail -n 1 ship-error.log | jq -r '.message, .causedBy.message'
fi
```

For a pane blocked on permission, that prints:

```text
nothing was sent
pane pane:0371d3719292450397e5cd5c40295d25 is blocked on permission
```

For `send --wait`, the outer `message` tells you whether the input went out:

- `nothing was sent`: the command refused before writing.
- `the input was sent`: don't send it again; read the pane or use `pane wait`.
- `the input may have been sent`: inspect the pane before deciding whether to retry.

The specific failure is under `causedBy`. Other command errors also have a category, such as `.type.kind` equal to `not_found` for a missing pane.

A mistyped flag or invalid argument keeps the argument parser's text and exits 2, not JSON. `ship config check` is another exception: it prints findings as text and exits 1, without an extra error line; a valid config prints nothing and exits 0. `--help` and `--version` still exit 0.
