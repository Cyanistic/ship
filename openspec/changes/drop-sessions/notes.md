# Drop sessions: decisions from conversation

Status: input for the product paper, not a paper. Recorded on 2026-10-08 from a design conversation with Cyan, so the decisions survive until `/product-design` turns them into `design/product.md`. Nothing here authorizes implementation.

This is the first of three planned changes: drop sessions, then keys and config (`openspec/changes/keys-config/notes.md`), then layout and sidebar (`openspec/changes/layout-sidebar/design/product.md`).

## Why

The direction for Ship is that bare `ship` opens a view of everything on the server, and later of every machine: a sidebar tree of machines, then tabs, with the selected tab's panes beside it. Once a client sees the whole server, attaching to one session at a time does nothing, and a session is a tab that can't hold panes but must have a unique name:

```rust
// crates/ship-core/src/model.rs:19
pub struct Session { id, name: SessionName, tabs }
pub struct Tab     { id, name: OptionalName, tabs, panes }
```

Dropping the type makes it clear what a server owns: a tree of tabs and panes, like a filesystem. It also removes the client-side code that exists only to turn session names into IDs.

## Decided

- **No session type.** Top-level tabs take the place sessions had. "Session" may survive as a word in docs and UI, with no type behind it.
- **Attach covers the whole server.** Bare `ship` attaches to the server, not to one container. `ship work` opens with the top-level tab `work` selected. Selection is client-local, so the client finds `work` in the tree it already holds; if it isn't there, the client says so. There is no attach-or-create.
- **Names are resolved by the client, from the tree it holds.** Nothing in the protocol knows about names. The server deals only in IDs.
- **The CLI takes IDs.** Scripts get IDs back as JSON from every create, and panes have `SHIP_PANE_ID`. Path lookup (`work/layout`) can be added later, client-side, by fetching the tree once.
- **Server targeting is unchanged:** `--server-url`, then `--machine` (later), then `SHIP_SERVER_URL` (inherited by panes), then local. IDs are UUIDv4 (`crates/ship-core/src/id.rs:38`), so an ID sent to the wrong server gets a 404 and can't hit the wrong tab.
- **The overview UI is not part of this change.** The sidebar tree arrives with layout. Here the UI keeps its single-pane view, the selection starts at the first top-level tab, and the status line drops its session segment.
- **Multiple machines are out of scope.** They are milestone 6. The model just has to be ready for them.
- **Tabs keep moving freely within a server.** `MoveTab` already covers moving work between what used to be sessions. Moving between machines is impossible, since processes live on one machine.

## What goes away

- `Session`, `SessionName`, `TabParent = UntaggedEither<Session, Tab>` (`crates/ship-core/src/model.rs:19, 119, 174-200`).
- `SessionRef`, `TabParentRef`, `resolve_session`, `resolve_parent`, `ensure_session` with its 409 retry (`crates/ship-client/src/api.rs:30, 49, 168-215`), and its use in `crates/ship/src/main.rs:111`.
- The session routes (`crates/ship-server/src/routes.rs`) and the `ship session` command group (`crates/ship/src/cli.rs:35-60`).
- Session-scoped attach and `EndReason::SessionRemoved` (`crates/ship-server/src/attach.rs:56, 132`).
- `NextSession` / `PrevSession` (`crates/ship-client/src/ui/keys.rs:70-71`) and the session parts of `crates/ship-client/src/ui/draw.rs:77-92`.

## Documents to update

- The locked papers that define sessions: `openspec/changes/archive/2026-10-06-session-tab-roundtrip/` and `openspec/changes/archive/2026-10-07-pane-terminals/`. Reopening a locked paper falls under the ripple rule; old text is marked superseded, not rewritten.
- `AGENTS.md` (lines 3, 5, 15: "the session is the top-level container") and `GLOSSARY.md` (Session, Tab, Selection).
- The layout draft: its sidebar header, FR-009 and U-4 assume sessions. Revise it after this paper is locked, pointing at this paper instead of repeating it.

## Open

- **What the server owns at the top.** Either one unnamed root tab, so `TabParent` becomes just `Tab` and no "or root" case exists anywhere, or a plain list of top-level tabs. A root tab could also hold panes, such as a machine-level scratch shell; whether that is useful is unclear. Lean: root tab.
- **Name uniqueness.** Lean: keep tab names free-form as today, and have the client report ambiguity if `ship work` matches two siblings. Add a sibling-uniqueness rule only if ambiguity shows up in practice.
- **Bare `ship` on an empty server.** Today `ship attach <name>` creates a session with a shell in the current directory. Something has to decide whether bare `ship` creates a first tab with a shell, or shows an empty state.
- **Protocol shape for whole-server attach.** A structure stream for the whole server plus screens for the viewed tab. Viewing is already tracked per tab (`ViewInput`, `ViewingRecord`); how much of attach changes needs checking against `crates/ship-server/src/attach.rs` and `state.rs`.
