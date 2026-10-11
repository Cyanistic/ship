# Add terminal text search

Status: Archived, unfiled design input. Needs further thought; not a backlog item, approved implementation plan or commitment. Create an issue only if Cyan later chooses to pursue it.

## Summary

Ship has no built-in way to find text in a pane's output. Search is deferred; reading output into a text filter or editor stands in for milestone 1's search evidence. The probe shows that scanning formatted history is practical, but matching across wrapped lines and mapping matches back to terminal cells need deliberate design.

---

## Details

### Behavior

Design forward/backward search, match navigation and highlighting over retained pane output. No search UI, binding or search API is implemented by the scrollback change. The current workaround is `ship pane read --pane <id> --lines N --unwrap` piped into `grep` or an editor.

### Cause

`experiments/scrollback-probe/src/main.rs` sections G and H format text and use an owned literal substring scan. This is an experiment, not a production search mechanism or proof that Ghostty lacks a native search API.

The slice-5 macOS release rerun observed:

- With the byte cap lifted, about 9,955 retained rows at 200 columns formatted to about 1 MB in 2.18 ms. A no-match full scan took 0.80 ms.
- At a 64 MiB byte cap, about 38,587 retained rows formatted in 9.04 ms and a no-match scan took 3.68 ms. These are synthetic timings, not an interactive latency guarantee.
- For `日本語 needle`, display width mapped the match to column 7 and the cell contained `n`.
- A row-wise scan found zero hits for `needle` split across a soft wrap; unwrapped text found one. Unwrapping alone loses the row/column map required for highlighting.
- Merely asking for 100,000 lines without lifting the default byte cap retained only 279 history rows in that run. Do not mistake that case's timings for a 100,000-row search.

Tracked references follow text and report pruning, as section D demonstrates. Use that evidence when deciding whether results survive output, pruning and resize.

### Reference behavior

[tmux](https://github.com/tmux/tmux/blob/master/tmux.1) exposes forward/backward search and repeated search in copy mode. [Herdr](https://herdr.dev/docs/keyboard/) documents literal forward/backward search, repeat navigation and smart-case matching. These do not settle Ship's search syntax or keys.

### Options

Check for a reusable dependency API before owning search. If formatted-text scanning is needed, preserve a mapping through soft wraps and wide/grapheme cells. Decide literal versus regex matching, case rules, match ownership, cancellation and update behavior in a locked design. Profile realistic history before adding an index or background search infrastructure; formatting currently runs on the pane's terminal thread.

### References

- `openspec/changes/scrollback/design/product.md`: Non-goals, amendment and probe evidence.
- `docs/product-brief.md`: milestone 1's temporary search evidence.
- `experiments/scrollback-probe/src/main.rs`: sections D, G and H.
- `openspec/changes/scrollback/evidence-slice-5.md`: observed timings and limits.
