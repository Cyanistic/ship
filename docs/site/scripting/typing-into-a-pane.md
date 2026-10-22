# Typing into a pane

<!--
Scope: Typing text and pressing keys in a pane with `ship pane send`.
Not here: waiting for the answer (handing-work-to-another-agent.md).
-->

`ship pane send` types text and presses keys, as if you typed them:

```sh
ship pane send --pane "$PANE" "cargo test" --key enter
ship pane send --pane "$PANE" --key ctrl-c
git diff | ship pane send --pane "$PANE" -        # text from stdin
```

Text goes in first, then each `--key` in order. Key names are the same as in the [config](../configuration.md#keys-and-modes).
