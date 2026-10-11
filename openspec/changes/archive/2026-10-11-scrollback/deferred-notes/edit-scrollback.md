# Edit-scrollback: deferred design notes

Status: Archived, unfiled design input. Needs further thought; not a backlog item, approved implementation plan or commitment. Create an issue only if Cyan later chooses to pursue it.

## Idea

Open a pane's retained output in an editor in that pane's spot, while the original program keeps running behind it. Return to the original when the editor closes. `ship pane read` piped into an editor is the current workaround.

## Preserved product requirements

The [product paper](../design/product.md), P2, FR-014 to FR-022 and SC-002, preserves the full deferred contract. Its proposed behavior is:

- `server.pane.edit_scrollback`, `ship pane edit-scrollback` and `alt-e` export history plus the screen as unwrapped plain text, including for exited panes.
- A new editor pane has its own ID and occupies the original's exact layout slot. The covered original stays running, recording output at its existing size, and remains targetable by its own ID. Reading it returns original output, not editor output.
- Use the server's `$VISUAL`, else `$EDITOR`, else `vi`; vi-family editors open at the end.
- All viewers see the editor. Clients selected on the original switch to the editor.
- Closing or exiting the editor deletes its temporary file and restores the original into the slot, including the editor's zoom and selections pointing to it.
- Closing the original leaves the editor as an ordinary pane. Its later close behaves normally.
- Editing an already-covered pane or editor fails with `already editing` and changes nothing.
- Inspection reports both covering and covered relationships.
- The file is private to the user and is removed when the server stops.
- The former performance target was opening a full 10 MB history in under a second. It remains deferred, not verified.

These are preserved design input, not authority to implement this follow-up now.

## Model and mechanisms to revisit

The amendment proposes a leaf holding a visible pane and the pane it covers. Layout commands targeting the covered pane act on its slot; covered panes keep their sizes until restored. Earlier architecture sketches instead put `covers: Option<Box<Pane>>` on the editor's pane record. Reconcile the leaf shape, lookup rules and lifecycle before implementation; G3 was explicitly deferred.

Keep structural changes in the state actor, with live resources private to the server. Reuse the existing pane-task read reply for history; do not let an editor silently replace the original's target identity.

Use `tempfile::NamedTempFile` as the candidate for secure creation and owned cleanup, rather than a hand-written filename generator. Decide where ownership lives, deletion on failure/close/shutdown, and permissions across priority platforms. Keep the file alive while the editor needs its path. G6 was deferred.

Use `shell-words` as the candidate for splitting editor settings into executable and arguments, rather than treating the whole setting as an executable path or writing a parser. Decide quoting expectations and vi-family detection/jump-to-end handling. G4 was deferred. These dependencies are not added by the scrollback change.

## Reference behavior

- [tmux](https://github.com/tmux/tmux/blob/master/tmux.1) offers `capture-pane` and `run-shell`; external-editor workflows can be composed rather than using a dedicated edit-scrollback action.
- [Zellij](https://zellij.dev/documentation/keybindings-possible-actions) offers `EditScrollback` and in-place command panes; its [editor setting](https://zellij.dev/documentation/options#scrollback_editor) selects the editor. The Ship product paper takes in-place covering as its behavioral inspiration, not an adopted storage model.
- [Herdr's configuration](https://herdr.dev/docs/config-reference/) binds `edit_scrollback`. Its [implementation](https://github.com/herdrdev/herdr/blob/master/src/app/custom_commands.rs), `open_focused_scrollback_in_editor`, reads unwrapped history, writes a temporary file and opens an overlay editor. Ship's zoom composes with an in-place editor rather than requiring the same full-tab presentation.
- [kitty](https://sw.kovidgoyal.net/kitty/conf/#scrollback) supports piping screen/history into a program through `launch --stdin-source=@screen_scrollback --type=overlay`, including a pager opened at the end. An overlay's user behavior does not settle Ship's pane identity or close semantics.

## Evidence and unresolved choices

The scrollback probe, removed from the branch, demonstrated fast formatting, soft-wrap joining and tracked references, but did not exercise covering, editor launch or restoration. Resolve storage, lifecycle, layout targeting, editor arguments and temporary-file ownership in a future locked design before writing code.
