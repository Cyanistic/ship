//! The sidebar's frame: its width, the right-hand divider, the heading and
//! the split between the tabs section on top and the agents section below.

mod agents;
pub(super) mod tabs;

use std::cmp::Ordering;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ship_core::{command::Direction, id::IdOf, model::Pane};

pub(super) use self::{
    agents::Agents,
    tabs::{Gap, Tabs, movable},
};
use super::observer::Observer;
use crate::keymap::Token;

const MIN_WIDTH: u16 = 10;
const MIN_PANES: u16 = 20;
const BUTTON: &str = "≡";
/// The heading and the blank row under it, above the tab rows.
const HEADING: u16 = 2;
/// The fewest rows either section keeps when the split moves.
const MIN_ROWS: u16 = 3;

pub(super) struct Sidebar {
    pub shown: bool,
    configured: u16,
    chosen: Option<u16>,
    /// The dragged split: the divider row's offset as a share of the
    /// sidebar's height, until the client exits. Half when unset.
    split: Option<f32>,
    area: Rect,
    pub tabs: Tabs,
    pub agents: Agents,
}

/// A wheel step.
#[derive(Clone, Copy)]
pub(super) enum Scroll {
    Up,
    Down,
}

pub(super) enum Hit {
    Heading,
    Button,
    /// The right-hand border, which resizes the width.
    Divider,
    /// The row between the sections, which moves the split.
    Split,
    Tabs(tabs::Hit),
    /// An agent's rows.
    Agent(IdOf<Pane>),
}

impl Sidebar {
    pub fn new(width: u16) -> Self {
        Self {
            shown: true,
            configured: width,
            chosen: None,
            split: None,
            area: Rect::ZERO,
            tabs: Tabs::default(),
            agents: Agents::default(),
        }
    }

    pub fn width(&self) -> u16 {
        if self.shown {
            self.chosen.unwrap_or(self.configured).max(MIN_WIDTH)
        } else {
            0
        }
    }

    pub fn configure(&mut self, width: u16) {
        if self.configured != width {
            self.configured = width;
            self.chosen = None;
        }
    }

    pub fn set_width(&mut self, width: u16, cols: u16) {
        self.chosen = Some(Self::clamp(width, cols));
    }

    /// Left and right only; `nudge_split` takes up and down.
    pub fn nudge(&mut self, direction: Direction, amount: u16, cols: u16) {
        if self.shown {
            let width = Self::clamp(self.width(), cols);
            self.set_width(
                match direction {
                    Direction::Left => width.saturating_sub(amount),
                    Direction::Right => width.saturating_add(amount),
                    Direction::Up | Direction::Down => return,
                },
                cols,
            );
        }
    }

    /// The pane area's minimum wins when both minimums cannot fit.
    pub fn clamp(width: u16, cols: u16) -> u16 {
        let max = cols.saturating_sub(MIN_PANES);
        width.clamp(MIN_WIDTH.min(max), max)
    }

    /// The divider row between the sections, from the last drawn area.
    fn split_row(&self) -> u16 {
        let height = self.area.height;
        let offset = self.split.map_or(height / 2, |share| {
            (share * f32::from(height)).round() as u16
        });
        self.area.y + split_clamp(offset, height)
    }

    /// Move the divider row to `y`, within the split's limits.
    pub fn set_split(&mut self, y: u16) {
        let height = self.area.height;
        if self.shown && height > 0 {
            let offset = split_clamp(y.saturating_sub(self.area.y), height);
            self.split = Some(f32::from(offset) / f32::from(height));
        }
    }

    /// Up and down only; `nudge` takes left and right.
    pub fn nudge_split(&mut self, direction: Direction, amount: u16) {
        let row = self.split_row();
        self.set_split(match direction {
            Direction::Up => row.saturating_sub(amount),
            Direction::Down => row.saturating_add(amount),
            Direction::Left | Direction::Right => return,
        });
    }

    /// Sent to the section under `at`.
    pub fn scroll(&mut self, at: Position, scroll: Scroll) {
        if !self.shown || !self.area.contains(at) {
            return;
        }
        match at.y.cmp(&self.split_row()) {
            Ordering::Less => self.tabs.scroll(scroll),
            Ordering::Equal => {}
            Ordering::Greater => self.agents.scroll(scroll),
        }
    }

    pub fn hit(&self, at: Position) -> Option<Hit> {
        if !self.shown || !self.area.contains(at) {
            return None;
        }
        if at.x == self.area.right().saturating_sub(1) {
            return Some(Hit::Divider);
        }
        if at.y == self.area.y {
            return Some(if at.x == self.area.right().saturating_sub(2) {
                Hit::Button
            } else {
                Hit::Heading
            });
        }
        let split = self.split_row();
        match at.y.cmp(&split) {
            Ordering::Less => self.tabs.hit(at).map(Hit::Tabs),
            Ordering::Equal => Some(Hit::Split),
            Ordering::Greater => self.agents.hit(at).map(Hit::Agent),
        }
    }

    /// The heading, tabs on top with rollup icons, the divider row, and
    /// agents below. The tabs and selection come from `observer`.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        observer: &Observer,
        rows: &[Vec<Token>],
        gap: Option<&Gap>,
    ) {
        let Some(replica) = &observer.replica else {
            return;
        };
        self.area = area;
        if area.is_empty() {
            return;
        }
        // Herdr's ANSI `terminal` palette: a dark-gray divider and bold header,
        // gray rows and a dark-gray band behind the selected row.
        let divider = Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::new().dark_gray());
        let content = divider.inner(area);
        frame.render_widget(divider, area);
        frame.render_widget(
            Paragraph::new(" tabs").dark_gray().bold(),
            Rect {
                height: 1,
                ..content
            },
        );
        frame.render_widget(
            Paragraph::new(BUTTON).dark_gray(),
            Rect::new(content.right().saturating_sub(1), content.y, 1, 1).intersection(content),
        );
        let split = self.split_row();
        let top = content
            .y
            .saturating_add(content.height.min(HEADING))
            .min(split);
        self.tabs.draw(
            frame,
            Rect {
                y: top,
                height: split - top,
                ..content
            },
            &replica.tabs,
            observer.record().and_then(|record| record.selection),
            gap,
        );
        let label = " agents ";
        frame.render_widget(
            Line::from(vec![
                Span::from("─"),
                Span::from(label).bold(),
                Span::from("─".repeat(usize::from(content.width))),
            ])
            .dark_gray(),
            Rect {
                y: split,
                height: 1,
                ..content
            }
            .intersection(content),
        );
        let below = split.saturating_add(1).min(content.bottom());
        self.agents.draw(
            frame,
            Rect {
                y: below,
                height: content.bottom() - below,
                ..content
            },
            &replica.tabs,
            rows,
        );
    }
}

/// The divider row's offset in a sidebar `height` rows tall: 10–90% of it,
/// leaving at least `MIN_ROWS` tab rows under the heading and agent rows
/// under the divider. The agents' minimum wins when both cannot fit.
fn split_clamp(offset: u16, height: u16) -> u16 {
    let min = height.div_ceil(10).max(HEADING + MIN_ROWS);
    let max = (u32::from(height) * 9 / 10) as u16;
    let max = max.min(height.saturating_sub(MIN_ROWS + 1));
    offset.clamp(min.min(max), max)
}
