# Ship

Ship is a compact Rust terminal workspace for coding-agent work. A server owns sessions, recursive tabs and panes; clients attach to observe and drive them, and work keeps running after the UI closes. The goal is to recover the daily-use value of Herdr with far less owned code. Ship is Cyan's side project, and Cyan is its primary user.

"Workspace" describes the product only. A workspace entity was considered and dropped; the session is the top-level container.

## Which documents authorize work

A locked openspec paper, meaning one whose status reads "Locked" with Cyan's recorded approval, is the implementation contract. Everything else is evidence or direction: the product brief, `docs/planning/`, `docs/research/`, `.scratch/`, `experiments/`, and archived or superseded drafts. Earlier directions in those documents (structdiff and JSON Patch screen diffs) are history, even where the text describes them in detail.

Questions those documents mark as open stay open. When work depends on one, bring it to Cyan with a recommendation and leave it unsettled in the code until Cyan decides.

## Compactness

Own the domain, rent the mechanisms. Ship owns session/tab/pane behavior and how the pieces compose. Serialization, terminal emulation, PTYs, transport, clipboard, audio and similar solved capabilities come from libraries; look for a dependency before writing a mechanism.

Implementation Rust has a budget of roughly 20,000 lines. It is a conservative ceiling that leaning on dependencies should beat, and it shapes design without overriding working behavior or clear boundaries. Only implementation Rust counts; tests, integrations, scripts and docs fall outside it. Get small by expressing each invariant once through Rust types, derives and macros, not through opaque compression or generic frameworks built for hypothetical features.

Ship starts with zero authored test code. Verify work with builds, formatting, Clippy, real workflows and temporary probes. Propose a permanent test only after encountering flaky behavior, and add it once Cyan agrees on its coverage. When reporting size, give implementation Rust and tests as separate counts.

## Client/server boundary

Keep the boundary crossable by a network later. The client reaches the server only through explicit commands, responses and events, never through server internals. Live resources (PTYs, child processes, Ghostty objects, tasks, connections) stay private to the server; serialize descriptions and screen contents, never handles.

Shared state is server-authoritative and distinct from client presentation state such as selection, focus, viewing position and sidebar expansion, which stay client-local. Clients render the state the server publishes rather than predicting the result of their own commands.

## Performance

Ship should feel at least as snappy as tmux, Zellij and Herdr, and ideally snappier.

Profile before consequential architecture decisions and before anything that looks heavy. Intuition about terminal data misleads: diffs looked cheaper, yet zstd-compressed full snapshots measured smaller and faster because of how terminal rendering changes the screen. For everything else, judge by feel in real use. A workflow that suddenly feels slow is a bug even when it works.

## Platforms

Linux is Cyan's primary environment. Linux and macOS are priority platforms; Windows and others are best effort. Choose cross-platform dependencies, validate Linux and macOS deliberately, and report any check not run on a platform as unverified there. Add platform-specific code when a concrete platform need appears.

## Reference multiplexers

Herdr, Zellij and tmux show what established multiplexers do and what users expect. Use them to ground behavior and to check whether an idea is settled practice. They are inspiration, not templates: Ship's structure and command surface are its own.
