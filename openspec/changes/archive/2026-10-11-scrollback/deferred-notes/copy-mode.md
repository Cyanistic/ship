# Add keyboard copy mode

Status: Archived, unfiled design input. Needs further thought; not a backlog item, approved implementation plan or commitment. Create an issue only if Cyan later chooses to pursue it.

## Summary

Ship can scroll and copy text with the mouse, but it has no keyboard-only way to move through output and select text. Copy mode is deferred; mouse selection and reading output into an editor cover the immediate need. This follow-up should reuse the terminal library's selection mechanics rather than build a second history store.

---

## Details

### Behavior

Add a keyboard mode for navigating a pane's retained output, selecting text and copying it. No copy-mode binding is implemented by the scrollback change. `alt-s` was Cyan's Herdr binding, not an approved Ship default.

The scrollback notes propose a pane-owned cursor and selection, shared by viewers, like the implemented scroll position. Treat that as design input for a future locked paper, not permission to implement a command surface now. Include running and exited panes, output arriving during navigation, and history pruning in that design.

### Cause

The pinned libghostty-vt revision `8953a740bc378cec3e07e1f6ca949f0595eab19b` already provides tracked grid references, `Selection::adjust`, `select_word` and text formatting. The wrapper now draws Ghostty's selection and emits `Copied`; Ship already routes a copy to its originating attachment.

The scrollback probe's sections D and E, removed from the branch (source at `452301a:experiments/scrollback-probe/src/main.rs`), exercised potential building blocks. A macOS rerun in slice 5 observed:

- A tracked row follows its text as older rows are pruned and reports `has_value=false` after its own text is pruned. The 200-column, 1,000-line-cap case with the byte cap lifted demonstrates this; the small-terminal case never filled enough storage to prune.
- `select_word` on `foo bar-baz qux` returned `bar-baz`.
- `Selection::adjust(Up)` followed by `EndOfLine` returned `"\nl"` in the probe. This proves the API is callable, not that it already supplies the desired keyboard movement. Verify anchor/endpoint semantics before designing motions.
- The alternate screen has zero history.

### Reference behavior

[tmux copy mode](https://github.com/tmux/tmux/blob/master/tmux.1) is a pane mode with emacs/vi key tables, selection and copy commands. [Herdr's keyboard documentation](https://herdr.dev/docs/keyboard/) describes motions, word movement, selection and copy, with live output and pinned history. These are behavior references, not Ship's chosen keymap.

### Options

Prefer extending typed wrapper commands for terminal movement and selection, leaving key modes and clipboard delivery in Ship. Decide the mode's entry/exit behavior, cursor rendering, bindings, simultaneous-viewer behavior, and response to pruned anchors in a future design. Reuse the existing attachment-directed copy route; do not copy to the server machine's clipboard.

### References

- `openspec/changes/scrollback/design/product.md`: Non-goals and probe evidence.
- `openspec/changes/scrollback/notes.md`: shared cursor/selection direction and historical binding.
- Scrollback probe, removed from the branch (source at `452301a:experiments/scrollback-probe/src/main.rs`): sections D, E and F.
- `openspec/changes/scrollback/evidence-slice-5.md`: probe rerun and platform limits.
