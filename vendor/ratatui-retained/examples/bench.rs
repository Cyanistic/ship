//! Per-frame cost of the retained renderer against a plain ratatui
//! `Terminal::draw` of the same layers, on a 200x60 Ship-shaped screen.
//!
//! Both paths run the same reactive flush and paint code; the plain path
//! paints every layer into a fresh buffer and diffs the whole screen.
//!
//! Usage: bench [frames] [workload prefix]

mod common;

use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

use common::{Model, Popup, Sink};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::{Terminal, TerminalOptions, Viewport};
use ratatui_retained::{Ui, reactive_scope};

const SIZE: Rect = Rect::new(0, 0, 200, 60);

#[derive(Clone, Copy)]
enum Workload {
    Typing,
    TypingUnderPopup,
    Scroll,
    SidebarSelect,
}

impl Workload {
    fn name(self) -> &'static str {
        match self {
            Self::Typing => "typing",
            Self::TypingUnderPopup => "typing under popup",
            Self::Scroll => "scroll (all rows)",
            Self::SidebarSelect => "sidebar select",
        }
    }

    fn setup(self, m: Model) {
        m.split.set(true);
        if let Self::TypingUnderPopup = self {
            m.popup.set(Some(Popup {
                x: 60,
                y: 4,
                items: (0..8).map(|i| format!("item {i}")).collect(),
            }));
        }
    }

    fn step(self, m: Model, i: usize) {
        match self {
            Self::Typing | Self::TypingUnderPopup => {
                let row = 5 + (i / 100) % 20;
                m.pane.update(|rows| {
                    rows[row]
                        .spans
                        .push(Span::styled("x", Style::new().fg(Color::Cyan)));
                    if i % 100 == 99 {
                        rows[row].spans.truncate(2);
                    }
                });
                m.cursor
                    .set(Some(Position::new(40 + (i % 100) as u16, row as u16 + 1)));
            }
            Self::Scroll => m.pane.update(|rows| rows.rotate_left(1)),
            Self::SidebarSelect => m.selected.set(i % 12),
        }
    }
}

struct Result {
    median: Duration,
    mean: Duration,
    bytes: usize,
}

fn measure(frames: usize, mut frame: impl FnMut(usize), sink: &Sink) -> Result {
    let mut times = Vec::with_capacity(frames);
    let mut bytes = 0;
    for i in 0..frames {
        let t = Instant::now();
        frame(i);
        times.push(t.elapsed());
        bytes += sink.take().len();
    }
    let mean = times.iter().sum::<Duration>() / frames as u32;
    times.sort();
    Result {
        median: times[frames / 2],
        mean,
        bytes: bytes / frames,
    }
}

fn setup(workload: Workload) -> (Ui<CrosstermBackend<Sink>>, Model, Sink) {
    let sink = Sink::default();
    let mut model = None;
    let mut ui = Ui::new(CrosstermBackend::new(sink.clone()), SIZE, |size| {
        let m = Model::new(58, 150);
        model = Some(m);
        m.view(size)
    });
    let m = model.unwrap();
    workload.setup(m);
    ui.draw().unwrap();
    sink.take();
    (ui, m, sink)
}

fn main() {
    let frames: usize = env::args().nth(1).map_or(5000, |s| s.parse().unwrap());
    println!(
        "{:<20} {:>22} {:>22} {:>16}",
        "workload", "plain ratatui", "retained", "bytes/frame"
    );
    println!(
        "{:<20} {:>22} {:>22} {:>16}",
        "", "median / mean", "median / mean", "plain / retained"
    );
    let only = env::args().nth(2);
    for workload in [
        Workload::Typing,
        Workload::TypingUnderPopup,
        Workload::Scroll,
        Workload::SidebarSelect,
    ] {
        if only
            .as_deref()
            .is_some_and(|o| !workload.name().starts_with(o))
        {
            continue;
        }
        let plain = {
            let (mut ui, m, _) = setup(workload);
            let sink = Sink::default();
            let mut term = Terminal::with_options(
                CrosstermBackend::new(sink.clone()),
                TerminalOptions {
                    viewport: Viewport::Fixed(SIZE),
                },
            )
            .unwrap();
            measure(
                frames,
                |i| {
                    workload.step(m, i);
                    reactive_scope::flush();
                    let cursor = ui.cursor();
                    term.draw(|f| {
                        ui.render_full(f.buffer_mut());
                        if let Some(c) = cursor {
                            f.set_cursor_position(c);
                        }
                    })
                    .unwrap();
                },
                &sink,
            )
        };
        let retained = {
            let (mut ui, m, sink) = setup(workload);
            measure(
                frames,
                |i| {
                    workload.step(m, i);
                    black_box(ui.draw().unwrap());
                },
                &sink,
            )
        };
        println!(
            "{:<20} {:>10.1?} / {:>9.1?} {:>10.1?} / {:>9.1?} {:>8} / {:>6}",
            workload.name(),
            plain.median,
            plain.mean,
            retained.median,
            retained.mean,
            plain.bytes,
            retained.bytes
        );
    }
}
