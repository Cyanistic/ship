# Handoff: server-owned selection

Written 2026-10-12 at the end of a long design session. Audience: the next agent. This is not a paper and authorizes nothing. It records where the discussion stopped so tomorrow's session can start without rebuilding it.

## Where things are

- **Branch `server-selection`.** Cut from `main` at `87cd38b`. It contains only this file, which is uncommitted. No product paper has been written and no code has changed.
- **Branch `mouse-drag`, commit `a5a8ec0`.** It holds the locked product paper, with amendment A-1, and the draft architecture paper at `openspec/changes/mouse-drag/design/`. Two things wait on Cyan there:
  - Drops should call `client.move_tab` directly instead of going through the key action queue. Cyan questioned the queue and this was proposed, but the paper hasn't been revised.
  - Key decision 5, the rule for which drop depths are valid. The paper itself flags it for review.

  A Plannotator review of that paper was started earlier and may still be open. Switch to `mouse-drag` before revising it, because the file doesn't exist on this branch.
- **Issue #20.** https://github.com/Cyanistic/ship/issues/20 records:
  - the key ordering race;
  - the race simulation table;
  - the full Pipeline design: one ordered NDJSON stream of calls and input, with replies carrying status, headers and body;
  - the prototype's friction list and size.

  Don't repeat that content here. Its scratch directories, `/tmp/race-sim` and `/tmp/ship-proto`, will likely be gone.

## What Cyan has decided, in chat

- **Selection, folds and selection memory become server-owned**, kept per attachment. Cyan said: "yeah server owned in that case is probably smarter."
- **Targets gain a "selected" form next to explicit IDs.** It is an extra way to address things, not a replacement; IDs stay for the CLI, plugins, drops and border drags. Cyan's framing: actions that depend on the selection don't manipulate it, they tunnel through with the selection as the target.
- **Ordering uses sequential sends for now (option B in #20).** Keys and later actions wait for the pending action's reply. Pipeline is deferred and recorded in #20. Cyan said: "what if we wrote down something about this batch approach down somewhere later and then just... used sequential calls?"
- **This is its own change on its own branch.** Cyan asked for that.

## Why move selection, as last summarized to Cyan

1. **Some actions do two things across the boundary.** Create-then-select and unzoom-then-navigate both make the client wait on the server's reply. Moving selection makes each one a single server edit. The client waits through `running`, `caught_up` and `chosen` in `crates/ship-client/src/ui/mod.rs` `drive`.
2. **Client-side resolution reads stale state.** For example, `alt-t` (create a tab) then `alt-down` (next tab) lands one row off.
3. **No added visible latency.** The client already draws the server's `record.selection`.
4. **The change is additive.** Targets gain `Selected`.
5. **Folds and memory come along** because the server resolves navigation, and "next tab" and "land on tab" depend on them.

The earlier estimate was 50 to 100 fewer lines overall, which is rough and unmeasured.

**Caveat already told to Cyan:** the "type right after a focus move with no wait" benefit needs Pipeline. With B, a key typed after an action waits for that action's reply. That is still better than today, which also waits for the replica.

## Proposed shape (not approved)

**Moves to the server, per attachment:**
- `navigate`, `moved` and `Step` from `ui/mod.rs`;
- `ui/memory.rs`;
- the fold set from `ui/sidebar.rs`;
- the unzoom-first step;
- the neighbor lookup for swap.

The affected client actions become `server.view.*`. Cyan suggested names like `server.view.tab.next`.

**Deleted from the client:**
- `chosen`;
- `unsent`, which carries `chosen` in the view PUT;
- the `creates` special case;
- `Scope::Keys` resolution in `crates/ship-client/src/execute.rs`.

The view PUT keeps only `area`.

**Stays client-side:** the keymap and modes, sidebar width and visibility, and mouse hit-testing. A click becomes an absolute `select { node }`.

**The queue becomes:** wait for the previous reply, then send. It no longer waits for the replica and no longer resolves anything itself.

**Errors:** a failed action cancels the rest of its own binding only, which is today's behavior (`queued.pop_front()` on error). Under B this is free. Under a future Pipeline it would need a `group` on each line, with skipped replies. That should be added to #20, and hasn't been yet.

## Open questions for the product paper

Each has the recommendation given to Cyan. He hasn't answered any of them yet.

1. **Selection scope.** Per attachment, as today, or shared tmux-style? Recommended: per attachment.
2. **Detach.** Do folds and memory survive a detach? Recommended: no for now; the record dies with the attachment, and persistence comes later.
3. **CLI.** Does the CLI get `Selected`? Recommended: no, because it has no attachment.
4. **Ordering.** B now and Pipeline later? Recommended: yes.
5. **Ordering of changes.** Which lands first, mouse-drag or this change? Recommended: finish mouse-drag first. Its papers are nearly done, and drops and border drags use explicit IDs either way. The only point where the two touch is FR-024 ("a moved selected tab stays selected"), which holds under both models.

## Rules this change touches

- **AGENTS.md, "Client/server boundary".** It currently says selection, focus and sidebar expansion stay client-local. This change reverses that. Edit it only once the product paper locks.
- **Specs likely affected:** session-observation, keymap and observer-recovery. These were named earlier but not yet checked line by line.
- **Locked papers authorize work.** Bring every open question to Cyan with a recommendation. Lock papers only on his explicit approval, and record his quote.

## Next steps

1. Ask Cyan which comes first, mouse-drag or this change. Recommend mouse-drag.
2. If mouse-drag: switch branches, pick up the Plannotator review, apply the direct `move_tab` revision and settle key decision 5.
3. If this change: run the product-design interview using the open questions above, then draft `openspec/changes/server-selection/design/product.md`.
4. Add the "error cancels the rest of the binding, so Pipeline needs groups" note to #20 when convenient.

## Suggested skills

- `product-design` and `design-rules`: for the server-selection product paper.
- `architecture-design`: for finishing the mouse-drag architecture paper.
- `grilling`: if Cyan wants to settle the open questions in depth first.
- `style` (human-first in chat, mixed for papers): for all writing.
- `show-me`: Cyan often asks for code snippets or diagrams when a point is abstract.
- `plannotator-annotate`: for the paper review loop.
- `subagent-delegation`: before any Agent call. This is required by Cyan's CLAUDE.md.
