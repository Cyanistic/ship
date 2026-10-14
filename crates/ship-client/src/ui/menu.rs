//! One client-local overlay. While open, it consumes every terminal event.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, Tab},
};

use crate::keymap::{Action, MenuItem, Menus};

#[derive(Clone, Copy)]
pub(super) enum Target {
    Pane(IdOf<Pane>),
    Tab(IdOf<Tab>),
    Global,
}

impl Target {
    pub fn node(self) -> Option<NodeId> {
        match self {
            Self::Pane(pane) => Some(NodeId::Pane(pane)),
            Self::Tab(tab) => Some(NodeId::Tab(tab)),
            Self::Global => None,
        }
    }
}

pub(super) fn items(menus: &Menus, target: Target) -> &[MenuItem] {
    match target {
        Target::Pane(_) => &menus.pane,
        Target::Tab(_) => &menus.tab,
        Target::Global => &menus.global,
    }
}

pub(super) struct Menu {
    items: Vec<MenuItem>,
    state: ListState,
    area: Rect,
}

pub(super) enum Picked {
    Run(Action),
    Close,
    Open,
}

impl Menu {
    pub fn open(items: &[MenuItem], at: Position, screen: Rect) -> Option<Self> {
        if items.is_empty() || screen.is_empty() {
            return None;
        }
        let width = items
            .iter()
            .map(|item| Line::from(item.label()).width())
            .max()?
            .saturating_add(2)
            .min(usize::from(screen.width)) as u16;
        let height = items
            .len()
            .saturating_add(2)
            .min(usize::from(screen.height)) as u16;
        Some(Self {
            items: items.to_vec(),
            state: ListState::default().with_selected(Some(0)),
            area: Rect::new(
                at.x.clamp(screen.x, screen.right().saturating_sub(width)),
                at.y.clamp(screen.y, screen.bottom().saturating_sub(height)),
                width,
                height,
            ),
        })
    }

    pub fn handle(&mut self, event: &Event) -> Picked {
        let highlight = self.state.selected().unwrap_or_default();
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.modifiers != KeyModifiers::NONE {
                    return Picked::Open;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.state.select(Some(highlight.saturating_sub(1)));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.state
                            .select(Some((highlight + 1).min(self.items.len() - 1)));
                    }
                    KeyCode::Enter => {
                        return Picked::Run(self.items[highlight].action.clone());
                    }
                    KeyCode::Esc => return Picked::Close,
                    _ => {}
                }
            }
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseEventKind::Moved | MouseEventKind::Down(_)) =>
            {
                let at = Position::new(mouse.column, mouse.row);
                if !self.area.contains(at) && matches!(mouse.kind, MouseEventKind::Down(_)) {
                    return Picked::Close;
                }
                let inner = Block::new().borders(Borders::ALL).inner(self.area);
                if inner.contains(at) {
                    let index = self.state.offset() + usize::from(at.y - inner.y);
                    if let Some(item) = self.items.get(index) {
                        match mouse.kind {
                            MouseEventKind::Moved => self.state.select(Some(index)),
                            MouseEventKind::Down(MouseButton::Left) => {
                                return Picked::Run(item.action.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        Picked::Open
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let items = self
            .items
            .iter()
            .map(|item| ListItem::new(Line::from(item.label())));
        let list = List::new(items)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(Color::Cyan)),
            )
            .style(Style::new().fg(Color::Reset).bg(Color::Reset))
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan).bold());
        frame.render_widget(Clear, self.area);
        frame.render_stateful_widget(list, self.area, &mut self.state);
    }
}
