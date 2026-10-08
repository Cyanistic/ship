# Screen cost per workload

Evidence gathered on 2026-10-07 on macOS (Darwin arm64), loopback only, while tuning the attach stream after pane-terminals slice 7. Nothing here was run on Linux, so every result is unverified there. This note records a deferred idea, row-level screen updates, so it can be picked up if performance starts to feel like a problem. It is not an approved protocol change.

## Summary

Ship pays about the same CPU per published screen however little of it changed, because every screen is a full snapshot that is captured, converted, serialized, compressed, parsed and rebuilt. Zellij's cost follows how much changed. So Ship is close to Zellij when most of the screen changes, and about 3x worse when one line changes, which is what agent spinners and status lines do all day.

The deferred fix is **dirty rows on the wire with a persistent client buffer**: the server sends only rows that changed, each row whole, and the client patches them into the screen it kept. Ghostty already reports which rows changed, so finding them costs nothing. Scrolling output still changes every row and gains nothing.

The internal half, rendering only Ghostty's dirty rows inside ratatui-ghostty, was tried separately; see "Incremental render" below.

## Details

### Workload CPU, before any of these changes

Multiplexer CPU (server and client, percent of one core), 10 s per workload, 120×40 client, one run each. Workloads came from a small generator script: `tick` rewrites one short line at 60 Hz, `spinner` a progress bar line at 30 Hz, `matrix` is cmatrix-style rain (about 180 cells per frame at 30 Hz), `full` gives every cell a new 256-colour at 30 Hz, `flood` writes a large file as fast as possible.

| workload | Ship | Herdr | Zellij |
|---|---|---|---|
| tick | 13.3% | 20.5% | 5.3% |
| spinner | 9.3% | 13.2% | 3.3% |
| htop (`-d 5`) | 0.9% | 2.4% | 0.8% |
| matrix | 12.8% | 13.5% | 9.6% |
| full colour | 27.5% | 24.2% | 25.8% |
| flood | 158% | 104% | 209% |
| nvim Ctrl-E scroll, 60/s | ~17% | 15.3% | 4.6% |

The flood row isn't comparable: throughput wasn't measured, and Herdr wrote far less to its terminal (145 KB/s against Ship's 700 KB/s).

Setting the publish interval to 16 ms (60 fps) instead of 4 ms saved about 2 points on the scroll test and tripled its latency to about 18 ms, matching Herdr. The cost is per screen, not per frame interval.

### What Zellij does

1. Its emulator marks a line dirty whenever it writes to it (`zellij-server/src/panes/grid.rs`, `output_buffer.update_line(cursor.y)`), and rendering only visits dirty lines (`output/mod.rs`, `changed_chunks_in_viewport`). Scrolling marks every line (`update_all_lines`, with a TODO to narrow it).
2. The server writes those lines as ANSI text for each client, and the client copies the string to stdout (`zellij-client/src/lib.rs`, `ClientInstruction::Render`). This depends on the server drawing each client's whole UI, which Ship's client/server boundary rules out.

### Server-rendering probe

A throwaway build had each attach stream diff the pane's ratatui buffer against the last one it sent and send the diff as ANSI text inside the existing JSON message; the client wrote it to stdout. One pane, no chrome, one client, so it is a best case.

| workload | Ship | probe | Zellij |
|---|---|---|---|
| tick | 14.7% | 8.1% | 5.3% |
| spinner | 9.6% | 4.9% | 3.3% |
| matrix | 12.8% | 7.7% | 9.6% |
| nvim scroll | ~21% | ~13.6% | 4.6% |
| full colour | 27.9% | 21.4% | 25.8% |

The scroll numbers in this table use bench_scroll's own CPU measurement, which reads higher than the earlier ~17%.

Almost all of the saving was on the client (tick: 6.3% → 1.5%). The probe server alone used 6.2% on the tick, more than Zellij's total, because it still built the whole grid out of Ghostty on every frame. The probe changed two things at once (fewer conversion steps, and only changes on the wire), so it doesn't say how much each contributed.

### Ghostty dirty rows

libghostty-vt's render state tracks a global dirty state (`Clean`, `Partial`, `Full`) and a per-row dirty flag: `snapshot.dirty()`, `row.dirty()`, `row.set_dirty(false)`, `snapshot.set_dirty(..)` in `libghostty-vt/src/render.rs`. The caller must clear both layers. Logged per render on the 39-row pane:

| workload | dirty rows per render |
|---|---|
| tick | 1 (368 of 373 renders) |
| spinner | 1 (185 of 192) |
| htop | 1–2, or about 33 on a list refresh |
| matrix | 13–36, mostly about 33 |
| streaming output, 30 lines/s | 39, reported `Full` (146 of 189) |

### Deferred: dirty rows on the wire

```text
server: "row 38 is now [cells]"            only rows that changed, each row whole
client: overwrite row 38 in its kept screen, then draw
attach, resize, Full: every row, as today
```

- Each message is still state ("row 38 is now X"), not a list of edits, so it can be applied without knowing the old row.
- It sits between structdiff's two strategies. Default structdiff treated the cell list as one unit and resent all of it on any change. Ordered structdiff aligned individual cells, which is quadratic and took about 0.5 s per 160×50 screen. Rows compared by position need no alignment.
- Ghostty renders more often than Ship publishes, so dirty rows must accumulate between publishes until the pane task takes them.
- Open: what each attach stream compares against with several clients or a slow one. The simplest answer is that each stream remembers what it last sent per pane, as the probe did.
- Scrolling still sends every row. A "shift up n, then these rows" message would fix it but adds scroll regions, multi-row shifts and client-side moves. Leave it out unless scrolling output feels slow.
- Expected outcome, not measured: near Zellij on spinners, typing and status lines; scrolling better but still behind.

## Incremental render (tried)

ratatui-ghostty re-rendered all cells on every PTY read, allocating a `String` per cell; on the tick that was about 93 renders a second of 4,680 cells each. A change in the vendored crate renders only Ghostty's dirty rows into the buffer it keeps, with every row on a resize or a `Full` frame. A check build that also did a full render after every incremental one found zero differing cells across about 67,000 renders, including flood, drag-resizes and nvim.

It cut CPU on non-scrolling workloads (tick 14.7% → 11.5%, spinner 9.2% → 7.1%) and typing latency (sh 3.9 → 3.2 ms, nvim insert 6.4 → 4.6 ms p50). It also doubled the screens per nvim Ctrl-E (1.00 → 2.00) and raised scroll latency from about 6 ms to 11 ms. With a faster capture, nvim's intermediate frame (the `^E` showcmd change) gets published and the real frame waits out the 4 ms gap. The same thing happened earlier when the capture buffer was reused. A burst allowance in the publish loop (two publishes back to back, same average rate) brought scroll latency back to about 6–7.5 ms at slightly higher scroll CPU (about 23.7% against 21.5%).

Cyan turned the burst down for its uneven frame spacing. What was kept instead is a settle delay: the first change after a quiet frame waits 1 ms before publishing, so both of nvim's frames land in one screen. Tokio's timer rounds that up to about 2 ms, so typing pays it (sh 3.2 → 5.4 ms, nvim insert 4.9 → 8.1 ms p50), but Ctrl-E is back to one screen, scroll latency 8.5 ms and scroll CPU 20.6%. Reusing the capture buffer on top no longer causes double screens; it brings scroll CPU to 16.9%, with full colour at 19.4% against 28.6% before any of this. Recorded as D58 in the pane-terminals program paper.

## Reproduction

The generator and bench scripts (`gen.py`, `bench_work.py`, `bench_scroll.py`, `gap_events.py`) and the probe trees lived in a temporary scratch directory and were not preserved in the repository.
