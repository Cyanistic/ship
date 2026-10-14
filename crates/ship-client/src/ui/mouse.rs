//! Terminal mouse events to gestures, selections, folds and pane input.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ship_core::{
    geometry::TabGeometry,
    id::IdOf,
    layout::Axis,
    model::{NodeId, Pane, Tab},
    protocol::{InputFrame, MouseInput},
};

use super::{
    Scene,
    lane::{Edge, EdgeAt},
    sidebar::{Hit, Row},
};

pub(super) enum Clicked {
    Select(NodeId),
    Fold(IdOf<Tab>),
    Frame(InputFrame),
    Resize(Vec<EdgeAt>),
    Width(u16),
}

#[derive(Default)]
pub(super) struct Mouse {
    /// Program drags and releases follow the pane that received the press.
    held: Option<IdOf<Pane>>,
    gesture: Option<Gesture>,
}

enum Gesture {
    Pressed { at: Position, row: Row, moved: bool },
    Border(Vec<Grab>),
    Divider { at: Position, moved: bool },
}

struct Grab {
    edge: Edge,
    last: u16,
}

impl Mouse {
    pub fn dragging(&self) -> bool {
        self.gesture.is_some()
    }

    pub fn held(&self) -> impl Iterator<Item = Edge> + '_ {
        match &self.gesture {
            Some(Gesture::Border(grabs)) => grabs.as_slice(),
            _ => &[],
        }
        .iter()
        .map(|grab| grab.edge)
    }

    pub fn route(&mut self, event: MouseEvent, scene: &Scene, busy: bool) -> Option<Clicked> {
        let at = Position::new(event.column, event.row);
        let relative = Position::new(
            at.x.saturating_sub(scene.panes.x),
            at.y.saturating_sub(scene.panes.y),
        );
        let offset = |rect: Rect| Rect {
            x: scene.panes.x.saturating_add(rect.x),
            y: scene.panes.y.saturating_add(rect.y),
            ..rect
        };
        let content = |id| Some(offset(scene.geometry?.panes.get(&id)?.content));
        let down = matches!(event.kind, MouseEventKind::Down(_));
        if down {
            self.held = None;
            self.gesture = None;
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
        ) && let Some(mut gesture) = self.gesture.take()
        {
            let release = matches!(event.kind, MouseEventKind::Up(_));
            let clicked = match &mut gesture {
                Gesture::Pressed {
                    at: origin,
                    row,
                    moved,
                } => {
                    *moved |= at != *origin;
                    if release && !*moved {
                        Some(if row.marker {
                            Clicked::Fold(row.tab)
                        } else {
                            Clicked::Select(NodeId::Tab(row.tab))
                        })
                    } else {
                        None
                    }
                }
                Gesture::Divider { at: origin, moved } => {
                    *moved |= at != *origin;
                    (*moved && scene.sidebar.shown)
                        .then_some(Clicked::Width(at.x.saturating_add(1)))
                }
                Gesture::Border(grabs) => {
                    if grabs.iter().any(|grab| {
                        !scene
                            .geometry
                            .is_some_and(|geometry| geometry.panes.contains_key(&grab.edge.pane))
                    }) {
                        return None;
                    }
                    let positions: Vec<_> = grabs
                        .iter_mut()
                        .filter_map(|grab| {
                            let to = match grab.edge.axis {
                                Axis::Horizontal => relative.x,
                                Axis::Vertical => relative.y,
                            };
                            if to == grab.last {
                                return None;
                            }
                            grab.last = to;
                            Some(EdgeAt {
                                edge: grab.edge,
                                to,
                            })
                        })
                        .collect();
                    (!positions.is_empty()).then_some(Clicked::Resize(positions))
                }
            };
            if !release {
                self.gesture = Some(gesture);
            }
            return clicked;
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
            MouseEventKind::Down(MouseButton::Left) if !scene.panes.contains(at) => {
                self.gesture = Some(match scene.sidebar.hit(at)? {
                    Hit::Row(row) => Gesture::Pressed {
                        at,
                        row,
                        moved: false,
                    },
                    Hit::Divider => Gesture::Divider { at, moved: false },
                });
                None
            }
            _ if !scene.panes.contains(at) => None,
            kind => {
                let geometry = scene.geometry?;
                if kind == MouseEventKind::Down(MouseButton::Left) {
                    let edges = edges_at(geometry, relative);
                    if !edges.is_empty() {
                        self.gesture = Some(Gesture::Border(
                            edges
                                .into_iter()
                                .map(|edge| Grab {
                                    last: match edge.axis {
                                        Axis::Horizontal => relative.x,
                                        Axis::Vertical => relative.y,
                                    },
                                    edge,
                                })
                                .collect(),
                        ));
                        return None;
                    }
                }
                let (&id, pane) = geometry
                    .panes
                    .iter()
                    .find(|(_, pane)| offset(pane.frame).contains(at))?;
                if down && (busy || Some(id) != scene.selected) {
                    return Some(Clicked::Select(NodeId::Pane(id)));
                }
                let wheel = matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown);
                let content = offset(pane.content);
                if !content.contains(at) || (!wheel && Some(id) != scene.selected) {
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

/// Shared frame cells, never outer edges or content. Layout order breaks ties
/// between the panes ending at a junction, with at most one edge per axis.
fn edges_at(geometry: &TabGeometry, at: Position) -> Vec<Edge> {
    if geometry.panes.len() < 2
        || geometry
            .panes
            .values()
            .any(|pane| pane.content.contains(at))
        || geometry
            .panes
            .values()
            .filter(|pane| pane.frame.contains(at))
            .count()
            < 2
    {
        return Vec::new();
    }
    [Axis::Horizontal, Axis::Vertical]
        .into_iter()
        .filter_map(|axis| {
            geometry
                .panes
                .iter()
                .find(|(_, pane)| {
                    pane.frame.contains(at)
                        && match axis {
                            Axis::Horizontal => {
                                pane.frame.right().saturating_sub(1) == at.x
                                    && geometry.panes.values().any(|other| {
                                        other.frame.contains(at)
                                            && other.frame.x == at.x
                                            && other.frame.x > pane.frame.x
                                    })
                            }
                            Axis::Vertical => {
                                pane.frame.bottom().saturating_sub(1) == at.y
                                    && geometry.panes.values().any(|other| {
                                        other.frame.contains(at)
                                            && other.frame.y == at.y
                                            && other.frame.y > pane.frame.y
                                    })
                            }
                        }
                })
                .map(|(pane, _)| Edge { pane: *pane, axis })
        })
        .collect()
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
