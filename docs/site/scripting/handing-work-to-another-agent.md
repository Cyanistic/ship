# Handing work to another agent

<!--
Scope: The loop for one agent starting another, giving it a task, waiting and reading the answer.
Not here: each command's full options (`--help`).
-->

This is the loop for one agent starting another, giving it a task and reading the answer:

```sh
TAB=$(ship tab create --name workers | jq -r .id)
WORKER=$(ship pane create --tab "$TAB" --name worker -- claude | jq -r .id)
ship pane wait --pane "$WORKER"                                     # until it's ready
ship pane send --pane "$WORKER" "Write a haiku about ships" --key enter --wait \
  | jq -r '.program[""].state'
ship pane read --pane "$WORKER" --lines 40
```

`--wait` returns once the agent has stopped working and prints the pane. The state tells you how the turn ended: `done` or `idle` means it finished, `blocked` means it's asking you something.

To answer a prompt, send the keys and say you expect it to be blocked:

```sh
ship pane send --pane "$WORKER" --key down --key enter --wait --if blocked
```

Other options:

- `--if idle,done` only sends if the agent is in one of those states, so you don't type into an agent that's still busy.
- `pane wait --status done,exited` waits for specific states instead of "anything but working".
- Waits have no time limit. Wrap them in `timeout 600 ...` if you want one (`gtimeout` on macOS, from coreutils).
- If a `send --wait` fails or times out, the text may already have gone in. Read the pane before sending it again.

This works with any program that reports its status. For one that doesn't, use plain `send` and `read`.
