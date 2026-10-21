# Handoff: pane-defaults architecture

## Update, later on 2026-10-21

The sections below "Update" are history. Cyan chose shape A in chat instead of D: keep the `watch`, add a `broadcast` ring of 256 beside it, and let each attachment pick `feed: latest | every`. The exit handler commits `Exited(status)` and then the close. Waits stay client-side on the `every` feed, and overflow ends the stream with `EndReason::Lost`. `product.md` FR-003 is amended for that overflow case (approved without review). `design/architecture.md` and `design/program.md` are locked (Plannotator, "LGTM" each). The OpenSpec artifacts (proposal, design, delta specs, tasks) are written and pass `openspec validate --strict`. Next is implementation, starting with tasks.md slice 0 (rebase onto main), on Cyan's request.

Status on 2026-10-21: the product paper is locked (`design/product.md`), and the architecture stage is in progress. `design/architecture.md` has not been written. Nothing on branch `pane-defaults` (worktree `ship.pane-defaults`) is committed.

## Where it stands

The only contested part of the architecture is FR-003. A `ship pane wait` that is already waiting when a pane's program exits has to see the exit and its status, even when the pane then closes because of its exit behavior. The rest of the shape was not disputed (see "Settled" below).

The leading candidate is shape D, server-side waits. Cyan has not approved it yet. Their last reaction came before D was proposed. The next step is to get Cyan's verdict on D, then draft `design/architecture.md` from the template with the rejected shapes as candidates.

## Why it's hard

Replicas reach clients through a `watch` channel that keeps only the newest value (`crates/ship-server/src/lib.rs:198`, read in `crates/ship-server/src/attach.rs:141`). Today the client's `wait()` and `watch()` (`crates/ship-client/src/automation.rs:90`, `:193`) walk the replicas. A pane that exits and closes leaves the waiter seeing "pane gone", and the wait fails with `pane P closed` and no status.

## Shapes considered

| Shape | Idea | Verdict, with Cyan's reason |
|---|---|---|
| A | Commit "exited", then commit the close | Rejected. The two revisions coalesce in the `watch` channel, so the waiter usually never sees the exited one. |
| B | Remove the pane and add `{pane, status}` to a capped `recentExits` list in `Replica`, in the same commit | Rejected. It publishes a history log alongside the latest state. |
| C | Keep an exit log on the server, with its own endpoint | Not pursued. It's B with more mechanism. |
| Zellij-style `create --wait` | The creator's request is held and answered from the exit handler | Rejected. It isn't composable, the request had to narrow FR-003, and Cyan already finds `send --wait` dirty. |
| A plus a lossless stream | `broadcast` instead of `watch` for event streams, with a `Lagged` error | Rejected. The close isn't atomic (it lands one revision later), a laggy client becomes a problem, and clients get intermediate states they don't care about. |
| **D: server-side waits** | Waiters live in the `ServerState` actor and are settled inside `commit()`. A pane removed by a commit is checked against its final value. | **Proposed, not yet approved.** |

### Shape D in brief

- `POST /panes/{id}/wait {status, after}` registers `(Condition, oneshot)` in `ServerState.waiters`.
- After the edit, `commit()` calls `settle(finals)`. For each waiter it uses the pane from the new tabs, or the final value of a pane this commit removed. A match replies with the pane. A removed pane that doesn't match replies `pane P closed`.
- The exit handler (`state.rs:~971`) sets `Exited(status)` and, if the exit behavior closes, calls `layout::close` in the same commit, so there's one revision.
- Open detail: `commit` should collect removed panes and their final values generically, for example by diffing or by having `layout::close` return the pane, so `settle` doesn't care who removed it.
- What moves:
  - `Condition` (`automation.rs:239`) moves to ship-core.
  - The client's `wait` becomes one call.
  - `send --wait` keeps its refuse, write and stall steps on the client and uses the server wait, with `after` set to the write's revision, for the end.
  - The UI's key-binding waits (`ship-client/src/ui/mod.rs:451`) and `execute.rs:254/264` call the same route.
- What it gives:
  - The exit and close happen atomically.
  - Publication stays latest-only.
  - No client lag issues and no stale states.
  - FR-003 holds as written, so product.md doesn't change.
- What it costs:
  - Wait logic crosses the client/server boundary as an explicit command and response, as AGENTS.md allows.
  - A new route.
  - The protocol bump, which this change needs anyway.

## Reference multiplexers (checked from source)

Clones are at `/tmp/refmux/{zellij,herdr}`. They may be gone after a reboot; reclone from GitHub.

- **Zellij:**
  - `zellij run --blocking` and `--block-until-exit[-success|-failure]` store the CLI's pending reply on the pane (`notification_end`). The exit handler fills in the status and sends it (`zellij-server/src/panes/terminal_pane.rs:1138`, `:1394`; `route.rs:135`).
  - This is a server-side wait, but only at creation.
  - `zellij run` panes are held by default; `--close-on-exit` is opt-in.
- **Herdr:**
  - Always closes on exit and has no keep option.
  - It pushes `pane.exited` (pane and workspace ids, no exit code) into a numbered 512-event ring (`src/api/event_hub.rs`) before `layout.updated`. Waiters read events after their sequence number and get `Lost` on overflow.
  - Its agent wait treats an exit as "not running" (`src/api/wait.rs:441`).
- **Neither** gives an arbitrary waiter the exit status of a pane that closed itself. D would.

## Settled (goes into architecture.md as-is)

- `Pane` (`ship-core/src/model.rs:35`) gains `exit_behavior` and `scrollback_bytes`, filled from `[server.defaults.pane]` at creation. Server settings move from `[server] shell` and `scrollback_bytes` to `defaults.pane.*` (`ship-core/src/defaults.toml`, read at spawn in `pane.rs:~111`).
- The vendored `vendor/ratatui-ghostty/src/session.rs` gets one new session-thread message that calls libghostty `Terminal::set_scrollback_max_bytes(Option<usize>)`, which exists at the locked rev and prunes immediately when lowered. Today the limit is only set at build (~line 1096).
- Old saved files missing the new fields get the current defaults on restore.
- `PROTOCOL_VERSION` goes from 14 to 15 (`ship-core/src/protocol.rs:23`).
- Specs touched (SC-005):
  - pane-programs: "Exited programs"
  - pane-history: the scrollback limit
  - config-file: the shell and scrollback settings
  - session-structure: set attributes
  - session-restore: "Restore on startup"
  - pane-automation: wait `exited`, plus the wait route if D wins

## Next steps

1. Get Cyan's verdict on D. If D is still being fought, the architecture-design skill says to draw fresh shapes rather than patch it.
2. Write `design/architecture.md` from `~/.claude/skills/architecture-design/references/template.md`. For house style, see `openspec/changes/archive/2026-10-20-cli-json-errors/design/architecture.md`.
3. Lock it with `plannotator annotate` in the background, and loop until Cyan has no objections.
4. Then program-design, implementation, and a Linux pass on Cyan's machine.

## Suggested skills

- `architecture-design`, to resume the stage. It loads `design-rules`.
- `style`, before writing the paper (mixed audience).
- `plannotator-annotate`, for the review loop.
- `codebase-design`, if the `settle`/`finals` seam needs shaping.
