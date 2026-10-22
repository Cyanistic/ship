# Tabs

<!--
Scope: Nested tabs, the sidebar tree, and the tab keys.
Not here: keeping tabs across restarts (saved-tabs.md).
-->

Tabs can hold panes, other tabs, or both, so a big task can keep its smaller pieces underneath it. The sidebar on the left shows the tree. `▾` marks an open tab and `▸` a folded one. A tab without a name is labeled with its first pane's directory.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-g` | Tabs mode: `j` / `k` move, `h` / `l` fold / unfold, `esc` leaves |
| `alt-b` | Hide or show the sidebar |
| `alt-x` | Close the tab and everything in it |

Coming back to a tab puts you on the pane you last used there. You can also click a tab to select it, click its `▾`/`▸` to fold it, or drag it to reorder or nest it.

From the CLI, every command prints JSON, so `jq` can pull out IDs for the next step:

```sh
PROJECT=$(ship tab create --name project | jq -r .id)
REVIEW=$(ship tab create --name review "$PROJECT" | jq -r .id)   # nested under project
ship pane create --tab "$REVIEW" --name checks
ship tab list
ship tab set --tab "$REVIEW" --name code-review
ship tab close --tab "$PROJECT"   # closes review and its pane too
```

`ship tab move` reorders or reparents a tab. Setting a name to `""` clears it and brings back the automatic label.
