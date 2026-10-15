//! Differential check: drives the model with random edits and, after every
//! frame, requires that
//! 1. the retained buffer equals a from-scratch render of the same layers, and
//! 2. a VT parser fed our bytes shows the same screen and cursor as one fed a
//!    plain ratatui `Terminal::draw` of that from-scratch render.
//!
//! Usage: check [seeds] [steps]

mod common;

use std::panic::{self, AssertUnwindSafe};
use std::{env, process};

use common::{Model, Popup, Sink};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::{TerminalOptions, Viewport};
use ratatui_retained::Ui;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const GLYPHS: &[&str] = &[
    "a", "Z", " ", "é", "コ", "漢", "❤️", "👍", "─", "e\u{301}", "\t",
];
const COLORS: &[Color] = &[
    Color::Reset,
    Color::Red,
    Color::Green,
    Color::Indexed(208),
    Color::Rgb(10, 200, 90),
];

fn style(rng: &mut Rng) -> Style {
    let mut s = Style::new().fg(*rng.pick(COLORS));
    if rng.below(3) == 0 {
        s = s.bg(*rng.pick(COLORS));
    }
    if rng.below(4) == 0 {
        s = s.add_modifier(*rng.pick(&[
            Modifier::BOLD,
            Modifier::REVERSED,
            Modifier::UNDERLINED,
            Modifier::ITALIC,
        ]));
    }
    s
}

fn edit_rows(rng: &mut Rng, rows: &mut Vec<Line<'static>>) {
    if rows.is_empty() {
        rows.push(Line::default());
    }
    let n = rows.len();
    match rng.below(10) {
        0..=5 => {
            let r = rng.below(n);
            let glyph = rng.pick(GLYPHS).to_string();
            let st = style(rng);
            rows[r].spans.push(Span::styled(glyph, st));
        }
        6 => {
            let r = rng.below(n);
            rows[r].spans.pop();
        }
        7 => rows.rotate_left(1),
        8 => {
            let r = rng.below(n);
            rows[r] = Line::default();
        }
        _ => {
            if rng.below(2) == 0 {
                rows.pop();
            } else {
                rows.push(Line::raw("new row"));
            }
        }
    }
}

fn step(rng: &mut Rng, m: Model, size: Rect) -> Option<Rect> {
    match rng.below(40) {
        0..=14 => m.pane.update(|rows| edit_rows(rng, rows)),
        15..=17 => m.pane2.update(|rows| edit_rows(rng, rows)),
        18 => m.split.update(|s| *s = !*s),
        19 => m.sidebar_w.set(rng.below(size.width as usize / 2) as u16),
        20 => m.selected.set(rng.below(14)),
        21 => m.tabs.update(|t| {
            let i = rng.below(t.len());
            // Names of varying length change the sidebar's footprint.
            t[i] = (0..rng.below(20)).map(|_| *rng.pick(GLYPHS)).collect();
        }),
        22 => m
            .title
            .set((0..rng.below(30)).map(|_| *rng.pick(GLYPHS)).collect()),
        23..=24 => m.status.set(format!(
            "{} {}",
            rng.pick(&["NORMAL", "PREFIX", "コピー"]),
            rng.below(1000)
        )),
        25..=27 => {
            let popup = if m.popup.with_untracked(Option::is_some) && rng.below(2) == 0 {
                None
            } else {
                let items = (0..1 + rng.below(6))
                    .map(|i| format!("item {i}{}", rng.pick(GLYPHS)))
                    .collect();
                Some(Popup {
                    x: rng.below(size.width as usize) as u16,
                    y: rng.below(size.height as usize) as u16,
                    items,
                })
            };
            m.popup.set(popup);
        }
        28..=29 => {
            let tint = if rng.below(3) == 0 {
                None
            } else {
                let x = rng.below(size.width as usize) as u16;
                let y = rng.below(size.height as usize) as u16;
                Some(Rect::new(
                    x,
                    y,
                    rng.below(40) as u16,
                    rng.below(4) as u16 + 1,
                ))
            };
            m.tint.set(tint);
        }
        30..=33 => {
            let c = if rng.below(5) == 0 {
                None
            } else {
                Some(Position::new(
                    rng.below(size.width as usize) as u16,
                    rng.below(size.height as usize) as u16,
                ))
            };
            m.cursor.set(c);
        }
        34 => {
            return Some(Rect::new(
                0,
                0,
                20 + rng.below(100) as u16,
                5 + rng.below(40) as u16,
            ));
        }
        _ => {}
    }
    None
}

fn screens_match(a: &vt100::Screen, b: &vt100::Screen, size: Rect) -> Result<(), String> {
    for row in 0..size.height {
        for col in 0..size.width {
            let (ca, cb) = (a.cell(row, col).unwrap(), b.cell(row, col).unwrap());
            // A cell never written and one holding a space look the same.
            let shown = |c: &vt100::Cell| {
                let text = if c.has_contents() { c.contents() } else { " " };
                (
                    text.to_string(),
                    c.fgcolor(),
                    c.bgcolor(),
                    c.bold(),
                    c.italic(),
                    c.underline(),
                    c.inverse(),
                    c.is_wide(),
                    c.is_wide_continuation(),
                )
            };
            if shown(ca) != shown(cb) {
                return Err(format!("cell ({col},{row}): ours {ca:?}, ratatui {cb:?}"));
            }
        }
    }
    if a.hide_cursor() != b.hide_cursor() {
        return Err(format!(
            "cursor hidden: ours {}, ratatui {}",
            a.hide_cursor(),
            b.hide_cursor()
        ));
    }
    if !a.hide_cursor() && a.cursor_position() != b.cursor_position() {
        return Err(format!(
            "cursor: ours {:?}, ratatui {:?}",
            a.cursor_position(),
            b.cursor_position()
        ));
    }
    Ok(())
}

fn run(seed: u64, steps: usize) -> Result<(usize, usize, bool), String> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut size = Rect::new(0, 0, 80, 24);
    let mut model = None;
    let (sink_ours, sink_theirs) = (Sink::default(), Sink::default());
    let mut ui = Ui::new(CrosstermBackend::new(sink_ours.clone()), size, |sz| {
        let m = Model::new(30, 50);
        model = Some(m);
        m.view(sz)
    });
    let m = model.unwrap();
    let mut term = Terminal::with_options(
        CrosstermBackend::new(sink_theirs.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(size),
        },
    )
    .unwrap();
    let mut ours = vt100::Parser::new(size.height, size.width, 0);
    let mut theirs = vt100::Parser::new(size.height, size.width, 0);
    let (mut repainted, mut full) = (0, 0);

    for i in 0..steps {
        for _ in 0..1 + rng.below(3) {
            if let Some(new) = step(&mut rng, m, size) {
                size = new;
                ui.resize(size).unwrap();
                term.resize(size).unwrap();
                ours.screen_mut().set_size(size.height, size.width);
                theirs.screen_mut().set_size(size.height, size.width);
            }
        }
        let stats = ui.draw().unwrap();
        repainted += stats.repainted;
        full += size.area() as usize;

        let mut reference = Buffer::empty(size);
        ui.render_full(&mut reference);
        if ui.buffer() != &reference {
            let (ours_buf, theirs_buf) = (ui.buffer(), &reference);
            let at = ours_buf
                .content
                .iter()
                .zip(&theirs_buf.content)
                .position(|(a, b)| a != b)
                .map(|i| ours_buf.pos_of(i));
            return Err(format!("seed {seed} step {i}: buffer differs at {at:?}"));
        }

        let cursor = ui.cursor();
        term.draw(|f| {
            *f.buffer_mut() = reference.clone();
            if let Some(c) = cursor {
                f.set_cursor_position(c);
            }
        })
        .unwrap();
        let (a, b) = (sink_ours.take(), sink_theirs.take());
        if env::args().nth(3).is_some_and(|s| s.parse() == Ok(seed)) {
            eprintln!(
                "step {i} ours {:?}\nstep {i} theirs {:?}",
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            );
        }
        // vt100 panics on some valid input (plain ratatui's too). When the
        // oracle can't parse ratatui's bytes, stop this seed and count it.
        let parse = |p: &mut vt100::Parser, bytes: &[u8]| {
            panic::catch_unwind(AssertUnwindSafe(|| p.process(bytes))).is_ok()
        };
        if !parse(&mut theirs, &b) {
            return Ok((repainted, full, true));
        }
        if !parse(&mut ours, &a) {
            return Err(format!(
                "seed {seed} step {i}: vt100 panicked on our bytes only"
            ));
        }
        screens_match(ours.screen(), theirs.screen(), size)
            .map_err(|e| format!("seed {seed} step {i}: {e}"))?;
    }
    Ok((repainted, full, false))
}

fn main() {
    let mut args = env::args().skip(1);
    let seeds: u64 = args.next().map_or(200, |s| s.parse().unwrap());
    let steps: usize = args.next().map_or(500, |s| s.parse().unwrap());
    panic::set_hook(Box::new(|_| {}));
    let (mut failures, mut repainted, mut full, mut cut) = (0, 0, 0, 0);
    for seed in 0..seeds {
        // Each run gets a fresh reactive graph's worth of signals; scopes are
        // dropped with the Ui.
        match run(seed, steps) {
            Ok((r, f, oracle_gave_up)) => {
                repainted += r;
                full += f;
                cut += oracle_gave_up as usize;
            }
            Err(e) => {
                failures += 1;
                if failures <= 10 {
                    eprintln!("FAIL {e}");
                }
            }
        }
    }
    println!(
        "{} seeds x {} frames: {} failed, {} cut short by a vt100 panic; repainted {:.1}% of the cells a full redraw would",
        seeds,
        steps,
        failures,
        cut,
        100.0 * repainted as f64 / full.max(1) as f64
    );
    if failures > 0 {
        process::exit(1);
    }
}
