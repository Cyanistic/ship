# Bindable run action and clipboard copy: deferred design notes

Status: Archived, unfiled design input. Needs further thought; not a backlog item, approved implementation plan or commitment. Create an issue only if Cyan later chooses to pursue it.

## Idea

Let a key run a command on the server with the targeted pane identified in its environment. Pair it with `ship clipboard copy` so a script can read or transform pane output and deliver text to a viewing client's clipboard. This means starting a separate command, not typing a shell command into the target pane's program.

The [product amendment](../design/product.md#amendment-2026-10-11-edit-scrollback-deferred) preserves this idea separately from edit-scrollback. Neither interface is implemented or settled.

## Mechanisms to reuse

- Resolve an explicit target when the binding is invoked; make that pane identity available in the command's environment rather than consulting whichever pane is focused later. Existing spawned panes receive `SHIP_PANE_ID` and `SHIP_TAB_ID`; whether the run action also supplies these and other context is a future design decision.
- Run on the server, where panes and their output live, not on the client's machine. Keep execution distinct from pane input and ordinary pane creation.
- `ship pane read` already supplies plain/styled and wrapped/unwrapped output.
- Reuse slice 4's `Clipboard` bus value, attach-stream delivery and client OSC 52 output for clipboard copy. Do not introduce a clipboard on the server machine or clipboard reads.

## Choices still open

Decide the action/CLI shape, executable arguments versus shell text, working directory and environment, output/error reporting, process lifetime and shutdown behavior. Decide how `ship clipboard copy` accepts text, identifies a target and chooses recipients. A program write currently reaches all viewers of its tab; a mouse selection reaches only the selecting attachment. A bound command needs an explicit choice between those policies, including how the initiating attachment survives through a subprocess and what happens with no viewers. Do not silently inherit a clipboard trust policy from one route.

## Reference behavior

- [tmux `run-shell`](https://github.com/tmux/tmux/blob/master/tmux.1) executes a shell command without creating a window and expands target context through formats.
- [Zellij `Run`](https://zellij.dev/documentation/keybindings-possible-actions#run) starts a command in a new pane, with working-directory and in-place options. It is precedent for server-hosted command execution, not an identical background run action.
- [Herdr command bindings](https://github.com/herdrdev/herdr/blob/master/src/config/keybinds.rs) distinguish shell, pane and popup actions. Its [custom-command implementation](https://github.com/herdrdev/herdr/blob/master/src/app/custom_commands.rs) uses target environment and directory context; endpoint command invocation is also routed through its server. Its machinery is a reference, not Ship's proposed architecture.

## Evidence

Slice 4 verified attachment-directed selection copies and tab-viewer program writes, without clipboard reads. That supplies a delivery mechanism, not acceptance evidence for a future run action. The references above were inspected during slice 5; no run-action prototype was built.
