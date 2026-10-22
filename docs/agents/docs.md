# Writing Docs

Rules for agents that write or change Ship's user-facing docs: `README.md` and the pages in `docs/site/`. `AGENTS.md` still applies, and `website/AGENTS.md` covers how the site builds from these pages.

## What goes where

`README.md` is for people deciding whether to try Ship: what it is, an honest split between what works and what isn't built, installation, and links into the guide. Keep it that short, in the spirit of Herdr's README. Keys, configuration and CLI walkthroughs live in `docs/site/`. Exact semantics (JSON shapes, protocol rules, geometry) live in `openspec/specs/` only.

When a change ships user-visible behavior, update the pages in `docs/site/` that cover it. Touch the README only if "What works today", "Not built yet" or the install steps changed.

The guide uses progressive disclosure, like the Hyprland wiki: each page leads with what most readers need first and links to the rest, so detail is there when someone goes looking but isn't in their way before then. The overview's first-session keys and the "When you need them" group are examples.

## Page scopes

Every page carries a comment right after its title saying what it covers and what it leaves to other pages:

```markdown
# Panes

<!--
Scope: Splitting, moving between, zooming and closing panes, and what happens when a pane's program exits.
Not here: scrolling and copying (scrolling-copying-and-the-mouse.md), driving panes from scripts (scripting/).
-->
```

Readers never see it: GitHub hides comments, and the site build strips it and fails on a page in `docs/site/` without one. The README carries one at the top.

Before adding to a page, read its scope. If the new material fits, add it there. If it doesn't, it goes on the page whose scope it fits, or on a new page linked from the one where readers will look for it. Never widen a scope just to make room for what you're adding.

A new page, or a changed scope comment, is a decision about the docs' structure. List it under "Worth knowing" in the pull request with the scope quoted, so Cyan can approve its size and boundaries before content piles into it.

## Keeping pages short as they change

Docs rot by accretion: each change adds its own caveat or section to the nearest page, nothing moves or comes out, and an overview turns into a reference manual. Avoid it this way:

- Revise, don't append. After your change, the page should read as if it had been written in one go. Rework the surrounding text and remove anything your change made wrong or redundant.
- Give each fact one home. State a caveat once, beside the feature it concerns, and link to it from elsewhere.
- Move detail down instead of compressing it. If a page gets long, move material to a linked page. Don't shorten it by dropping connecting words or packing sentences; terse prose is harder to read than longer plain prose.
- Write for a person first: plain words, the common case and a working example before options and edge cases. Mechanisms, performance notes and exact semantics belong in `openspec/specs/` or the code, not in the guide.
