//! A Ship-shaped app shared by the checker and the bench: a sidebar, a
//! bordered pane, an optional second pane overlapping it, a status line, a
//! popup menu and a style-only tint (like a selection highlight).
#![allow(dead_code)]

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Widget};
use ratatui_retained::reactive_scope::{ReadSignal, Signal, create_signal};
use ratatui_retained::{View, rows, widget};

#[derive(Clone, Copy)]
pub struct Model {
    pub sidebar_w: Signal<u16>,
    pub tabs: Signal<Vec<String>>,
    pub selected: Signal<usize>,
    pub title: Signal<String>,
    pub pane: Signal<Vec<Line<'static>>>,
    pub split: Signal<bool>,
    pub pane2: Signal<Vec<Line<'static>>>,
    pub status: Signal<String>,
    pub popup: Signal<Option<Popup>>,
    pub tint: Signal<Option<Rect>>,
    pub cursor: Signal<Option<Position>>,
}

#[derive(Clone, PartialEq)]
pub struct Popup {
    pub x: u16,
    pub y: u16,
    pub items: Vec<String>,
}

impl Widget for &Popup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = Block::bordered()
            .title("menu")
            .style(Style::new().bg(Color::Blue));
        let lines: Vec<Line> = self.items.iter().map(|s| Line::raw(s.as_str())).collect();
        Paragraph::new(lines).block(block).render(area, buf);
    }
}

#[derive(PartialEq)]
pub struct Tint;

impl Widget for &Tint {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(
            area,
            Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED),
        );
    }
}

#[derive(PartialEq)]
pub struct Status(String);

impl Widget for &Status {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::raw(self.0.as_str())
            .style(Style::new().bg(Color::DarkGray))
            .render(area, buf);
    }
}

impl Model {
    pub fn new(pane_rows: usize, width: usize) -> Self {
        let line = |i: usize| -> Line<'static> {
            let text: String = (0..width)
                .map(|c| (b'a' + ((i + c) % 26) as u8) as char)
                .collect();
            Line::from(vec![
                Span::styled(format!("{i:>3} "), Style::new().fg(Color::Green)),
                Span::raw(text),
            ])
        };
        Self {
            sidebar_w: create_signal(24),
            tabs: create_signal((0..12).map(|i| format!("tab {i}")).collect()),
            selected: create_signal(0),
            title: create_signal("shell".into()),
            pane: create_signal((0..pane_rows).map(line).collect()),
            split: create_signal(false),
            pane2: create_signal((0..pane_rows).map(|i| line(i + 7)).collect()),
            status: create_signal("NORMAL".into()),
            popup: create_signal(None),
            tint: create_signal(None),
            cursor: create_signal(Some(Position::new(30, 3))),
        }
    }

    pub fn view(self, size: ReadSignal<Rect>) -> View {
        let m = self;
        let main = move || {
            let s = size.get();
            Rect::new(0, 0, s.width, s.height.saturating_sub(1))
        };
        let pane_outer = move || {
            let r = main();
            let sw = m.sidebar_w.get().min(r.width);
            Rect::new(sw, 0, r.width - sw, r.height)
        };
        let paint_line =
            |line: &Line<'static>, area: Rect, buf: &mut Buffer| line.render(area, buf);
        View {
            layers: vec![
                widget(
                    move || {
                        let r = main();
                        Some(Rect::new(0, 0, m.sidebar_w.get().min(r.width), r.height))
                    },
                    move || {
                        let selected = m.selected.get();
                        let lines: Vec<Line<'static>> = m.tabs.with(|tabs| {
                            tabs.iter()
                                .enumerate()
                                .map(|(i, t)| {
                                    let style = if i == selected {
                                        Style::new().add_modifier(Modifier::REVERSED)
                                    } else {
                                        Style::new()
                                    };
                                    Line::styled(t.clone(), style)
                                })
                                .collect()
                        });
                        Paragraph::new(lines)
                    },
                ),
                widget(
                    move || Some(pane_outer()),
                    move || Block::bordered().title(m.title.get_clone()),
                ),
                rows(
                    move || {
                        let r = pane_outer();
                        Some(Rect::new(
                            r.x + 1,
                            r.y + 1,
                            r.width.saturating_sub(2),
                            r.height.saturating_sub(2),
                        ))
                    },
                    m.pane.readonly(),
                    paint_line,
                ),
                rows(
                    move || {
                        if !m.split.get() {
                            return None;
                        }
                        let r = pane_outer();
                        let x = r.x + r.width / 2 - 3.min(r.width / 2);
                        Some(Rect::new(
                            x,
                            r.y + 2,
                            r.right().saturating_sub(x),
                            r.height.saturating_sub(5),
                        ))
                    },
                    m.pane2.readonly(),
                    paint_line,
                ),
                widget(
                    move || {
                        let s = size.get();
                        Some(Rect::new(
                            0,
                            s.height.saturating_sub(1),
                            s.width,
                            1.min(s.height),
                        ))
                    },
                    move || Status(m.status.get_clone()),
                ),
                widget(move || m.tint.get(), || Tint),
                widget(
                    move || {
                        m.popup.with(|p| {
                            p.as_ref()
                                .map(|p| Rect::new(p.x, p.y, 30, p.items.len() as u16 + 2))
                        })
                    },
                    move || {
                        m.popup.get_clone().unwrap_or(Popup {
                            x: 0,
                            y: 0,
                            items: vec![],
                        })
                    },
                ),
            ],
            cursor: Box::new(move || m.cursor.get()),
        }
    }
}

/// A writer whose bytes the caller can take out while a backend owns it.
#[derive(Clone, Default)]
pub struct Sink(pub std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl Sink {
    pub fn take(&self) -> Vec<u8> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

impl std::io::Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
