//! Terminal mouse events to selections, folds and pane input.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ship_core::{
    geometry::TabGeometry,
    id::IdOf,
    model::{NodeId, Pane, Tab},
    protocol::{InputFrame, MouseInput},
};

use super::sidebar::Sidebar;

/// What a mouse event does.
pub(super) enum Clicked {
    /// Select this pane, or land on this tab as the keyboard does.
    Select(NodeId),
    /// Fold or unfold a sidebar row.
    Fold(IdOf<Tab>),
    Frame(InputFrame),
}

/// Which pane a press started in, so drags and the release go to it even
/// past its edges. Cleared on release.
#[derive(Default)]
pub(super) struct Mouse {
    held: Option<IdOf<Pane>>,
}

impl Mouse {
    /// `panes` is the pane area as drawn, `sidebar` the sidebar's rendered rows.
    pub fn route(
        &mut self,
        event: MouseEvent,
        panes: Rect,
        geometry: Option<&TabGeometry>,
        selected: Option<IdOf<Pane>>,
        sidebar: &Sidebar,
        busy: bool,
    ) -> Option<Clicked> {
        let at = Position::new(event.column, event.row);
        let offset = |rect: Rect| Rect {
            x: panes.x.saturating_add(rect.x),
            y: panes.y.saturating_add(rect.y),
            ..rect
        };
        let content = |id| Some(offset(geometry?.panes.get(&id)?.content));
        let down = matches!(event.kind, MouseEventKind::Down(_));
        if down {
            self.held = None;
        }
        match event.kind {
            MouseEventKind::Up(_) => {
                let id = self.held.take()?;
                Some(Clicked::Frame(InputFrame::Mouse(input(
                    event,
                    id,
                    content(id)?,
                ))))
            }
            MouseEventKind::Drag(_) => {
                let id = self.held?;
                Some(Clicked::Frame(InputFrame::Mouse(input(
                    event,
                    id,
                    content(id)?,
                ))))
            }
            MouseEventKind::Down(MouseButton::Left) if !panes.contains(at) => {
                let row = sidebar.row_at(at)?;
                Some(match row.marker {
                    true => Clicked::Fold(row.tab),
                    false => Clicked::Select(NodeId::Tab(row.tab)),
                })
            }
            _ if !panes.contains(at) => None,
            kind => {
                let (&id, pane) = geometry?
                    .panes
                    .iter()
                    .find(|(_, pane)| offset(pane.frame).contains(at))?;
                if down && (busy || Some(id) != selected) {
                    return Some(Clicked::Select(NodeId::Pane(id)));
                }
                // The wheel reaches any pane; other input only the selected one.
                let wheel = matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown);
                let content = offset(pane.content);
                if !content.contains(at) || (!wheel && Some(id) != selected) {
                    return None;
                }
                if down {
                    self.held = Some(id);
                }
                Some(Clicked::Frame(InputFrame::Mouse(input(event, id, content))))
            }
        }
    }
}

/// `event` for `pane`, relative to its `content`, negative above or left.
fn input(event: MouseEvent, pane: IdOf<Pane>, content: Rect) -> MouseInput {
    MouseInput {
        pane,
        kind: event.kind,
        modifiers: event.modifiers,
        x: i32::from(event.column) - i32::from(content.x),
        y: i32::from(event.row) - i32::from(content.y),
    }
}
