use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Widget, Wrap},
};
use ratatui_retained::{Region, Stack, footprint};
use std::{collections::HashMap, env, path::Path, sync::Arc};

use ship_core::{
    id::IdOf,
    model::{Pane, PaneStatus, Tab},
    screen::{Cursor, Screen, Scrolled, Size},
};

use super::{
    Local,
    observer::{Observer, Selected},
    panes,
    sidebar::Sidebar,
};
use crate::keymap::ModeName;

/// What the status line shows besides the selection.
pub(super) struct Status<'a> {
    /// The active mode, outside normal.
    pub mode: Option<&'a ModeName>,
    /// The last error, or "not available yet".
    pub message: Option<&'a str>,
    /// The mode's `chord label` hints, in the file's order.
    pub hints: Vec<&'a str>,
}

/// `area` split into the sidebar's columns and the pane area beside them,
/// for drawing, hit-testing and the size the server gives panes.
pub(super) fn split(area: Rect, sidebar_width: u16) -> (Rect, Rect) {
    let width = if sidebar_width == 0 {
        0
    } else {
        Sidebar::clamp(sidebar_width, area.width)
    };
    (
        Rect { width, ..area },
        Rect {
            x: area.x + width,
            width: area.width - width,
            ..area
        },
    )
}

/// The area a terminal of `cols` by `rows` leaves for the tab. The server
/// sizes panes to it. The status line overlays the tab, so it takes none.
pub(super) fn tab_area(cols: u16, rows: u16, sidebar_width: u16) -> Size {
    let (_, tab) = split(Rect::new(0, 0, cols, rows), sidebar_width);
    Size {
        cols: tab.width,
        rows: tab.height,
    }
}

/// The frame's layers, bottom to top.
const BASE: usize = 0;
const SCREENS: usize = 1;
const OVERLAY: usize = 2;

/// What the retained frame remembers between draws to work out what
/// changed.
#[derive(Default)]
pub(super) struct Drawn {
    /// Whether anything besides pane screens may have changed since the
    /// last draw. Set by every input but a screen.
    pub chrome: bool,
    /// Each pane's shown rect and scroll position. The filler and the
    /// scroll indicator depend on them, so a change redraws the chrome.
    shown: Vec<(IdOf<Pane>, Rect, Option<Scrolled>)>,
    /// Each pane's shown rect and screen as last painted.
    screens: HashMap<IdOf<Pane>, (Rect, Arc<Screen>)>,
    /// The cells the base and the overlay paint, measured when first needed
    /// after the chrome changes.
    footprints: [Option<Region>; 2],
}

/// The client's frame as a [`Stack`] over its state: the sidebar and pane
/// chrome, the pane screens, then the scroll indicators, status line and
/// menu over them. Only the screens report damage finer than the whole
/// frame; any other change redraws everything.
pub(super) struct Layers<'a> {
    pub local: &'a mut Local,
    pub area: Rect,
}

impl Layers<'_> {
    /// Notes the chrome's inputs that arrive with screens. Call before each
    /// draw.
    pub fn prepare(&mut self) {
        let scene = self.local.scene(self.area);
        let shown: Vec<_> = panes::placed(&scene)
            .map(|pane| {
                let scrolled = pane.screen.and_then(|screen| screen.scrolled);
                (pane.id, pane.shown, scrolled)
            })
            .collect();
        let drawn = &mut self.local.drawn;
        if drawn.shown != shown {
            drawn.shown = shown;
            drawn.chrome = true;
        }
        if drawn.chrome {
            drawn.footprints = [None, None];
        }
    }

    /// Where the terminal's cursor goes: the selected pane's cursor, unless
    /// an overlay covers it.
    pub fn cursor(&self) -> Option<(Position, Cursor)> {
        if self.local.menu.is_some() {
            return None;
        }
        let scene = self.local.scene(self.area);
        let cursor = panes::cursor(&scene)?;
        // A cursor under the status line would sit on top of it.
        let covered = self.status().is_some() && status_row(scene.panes).contains(cursor.0);
        (!covered).then_some(cursor)
    }

    fn status(&self) -> Option<Line<'_>> {
        let observer = &self.local.observer;
        let status = super::status(&self.local.keys, self.local.message.as_deref());
        status_line(observer.selected().as_ref(), observer.connected, &status)
    }

    /// The sidebar, then the pane chrome, or a hint when the tab has no
    /// panes or nothing is selected.
    fn base(&mut self, buf: &mut Buffer) {
        let local = &mut *self.local;
        let (sidebar_area, main) = split(self.area, local.sidebar.width());
        if local.sidebar.shown
            && let Some(replica) = &local.observer.replica
        {
            local.sidebar.draw(
                buf,
                sidebar_area,
                &replica.tabs,
                local.observer.record().and_then(|record| record.selection),
                local.mouse.gap(),
            );
        }
        let scene = local.scene(self.area);
        let observer = &local.observer;
        let selected = observer.selected();
        if has_panes(selected.as_ref()) {
            panes::chrome(buf, &scene, local.mouse.held());
        } else {
            Paragraph::new(hint(observer, selected.as_ref()))
                .centered()
                .wrap(Wrap { trim: true })
                .render(
                    Rect {
                        y: main.y + main.height / 2,
                        height: main.height - main.height / 2,
                        ..main
                    },
                    buf,
                );
        }
    }

    /// Scroll indicators, the status line when it has something to say, and
    /// the menu.
    fn overlay(&mut self, buf: &mut Buffer) {
        let scene = self.local.scene(self.area);
        if has_panes(self.local.observer.selected().as_ref()) {
            panes::scrolled(buf, &scene);
        }
        if let Some(line) = self.status() {
            let row = status_row(scene.panes);
            Clear.render(row, buf);
            line.render(row, buf);
        }
        if let Some(menu) = &mut self.local.menu {
            menu.draw(buf);
        }
    }

    /// Damages the rows of each screen that differ from the ones painted,
    /// and all of a pane that appeared, moved or went away.
    fn screens(&mut self, out: &mut Region) {
        let scene = self.local.scene(self.area);
        let placed: Vec<_> = panes::placed(&scene)
            .filter(|pane| !pane.shown.is_empty())
            .filter_map(|pane| Some((pane.id, pane.shown, Arc::clone(pane.screen?))))
            .collect();
        let painted = &mut self.local.drawn.screens;
        painted.retain(|id, (shown, _)| {
            let kept = placed.iter().any(|(placed, ..)| placed == id);
            if !kept {
                out.add(*shown);
            }
            kept
        });
        for (id, shown, screen) in placed {
            match painted.get(&id) {
                Some((old, painted)) if *old == shown => {
                    if !Arc::ptr_eq(painted, &screen) {
                        for (y, i) in (shown.top()..shown.bottom()).zip(0..) {
                            if painted.rows.get(i) != screen.rows.get(i) {
                                out.add(Rect::new(shown.x, y, shown.width, 1));
                            }
                        }
                    }
                }
                Some((old, _)) => {
                    out.add(*old);
                    out.add(shown);
                }
                None => out.add(shown),
            }
            painted.insert(id, (shown, screen));
        }
    }

    fn paint_screens(&self, want: &Region, buf: &mut Buffer) {
        let scene = self.local.scene(self.area);
        for pane in panes::placed(&scene) {
            let Some(screen) = pane.screen else {
                continue;
            };
            let shown = pane.shown;
            for (y, row) in (shown.top()..shown.bottom()).zip(&screen.rows) {
                let row_area = Rect::new(shown.x, y, shown.width, 1);
                if want.intersects(row_area) {
                    panes::paint_row(row, row_area, buf);
                }
            }
        }
    }
}

impl Stack for Layers<'_> {
    fn layers(&self) -> usize {
        3
    }

    fn area(&self, layer: usize) -> Option<Rect> {
        match layer {
            SCREENS => Some(split(self.area, self.local.sidebar.width()).1),
            _ => Some(self.area),
        }
    }

    fn damage(&mut self, layer: usize, area: Rect, out: &mut Region) {
        match layer {
            SCREENS => self.screens(out),
            _ if self.local.drawn.chrome => out.add(area),
            _ => {}
        }
    }

    fn extent(&self, layer: usize, area: Rect, want: Rect) -> Rect {
        if layer != SCREENS {
            return area;
        }
        // Screens paint whole rows of each pane they cross.
        let band = Rect::new(area.x, want.y, area.width, want.height);
        let scene = self.local.scene(self.area);
        panes::placed(&scene)
            .map(|pane| pane.shown.intersection(band))
            .filter(|rect| !rect.is_empty())
            .reduce(Rect::union)
            .unwrap_or_default()
    }

    fn touches(&mut self, layer: usize, area: Rect, damage: &Region) -> bool {
        let slot = match layer {
            BASE => 0,
            OVERLAY => 1,
            _ => return true,
        };
        if self.local.drawn.footprints[slot].is_none() {
            let mut all = Region::default();
            all.add(area);
            let cells = footprint(area, |buf| self.paint(layer, area, &all, buf));
            self.local.drawn.footprints[slot] = Some(cells);
        }
        self.local.drawn.footprints[slot]
            .as_ref()
            .is_some_and(|cells| cells.overlaps(damage))
    }

    fn paint(&mut self, layer: usize, _area: Rect, want: &Region, buf: &mut Buffer) {
        match layer {
            BASE => self.base(buf),
            SCREENS => self.paint_screens(want, buf),
            _ => self.overlay(buf),
        }
    }
}

fn has_panes(selected: Option<&Selected>) -> bool {
    selected
        .and_then(|selected| selected.tab.geometry.as_ref())
        .is_some_and(|geometry| !geometry.panes.is_empty())
}

/// The status line's row, over the pane area's last.
fn status_row(main: Rect) -> Rect {
    Rect {
        y: main.bottom().saturating_sub(1),
        height: main.height.min(1),
        ..main
    }
}

/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · alt-right to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane: ship pane create --tab <tab-id>`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String {
    let (Some(replica), Some(_)) = (&observer.replica, observer.record()) else {
        return "connecting".into();
    };
    match (selected, replica.tabs.len()) {
        (Some(selected), _) => format!("no pane: ship pane create --tab {}", selected.tab.id),
        (None, 0) => "no tabs · ship tab create".into(),
        (None, 1) => "1 tab · alt-right to open one".into(),
        (None, count) => format!("{count} tabs · alt-right to open one"),
    }
}

/// Herdr's mode bar: the mode's name as a pill, then the selected pane's exit
/// status, the connection and the last message, then the hints, which are cut
/// first when the row is full. `None` when there is nothing to say.
fn status_line<'a>(
    selected: Option<&Selected>,
    connected: bool,
    status: &Status<'a>,
) -> Option<Line<'a>> {
    let pill = |text: String, color| Span::styled(text, Style::new().black().bg(color).bold());
    let mut spans = Vec::new();
    if let Some(mode) = status.mode {
        spans.push(pill(format!(" {} ", mode.0.to_uppercase()), Color::Cyan));
    }
    if let Some(PaneStatus::Exited(status)) = selected
        .and_then(|selected| selected.pane)
        .map(|pane| &pane.status)
    {
        let code = status.signal.clone().unwrap_or(status.code.to_string());
        spans.push(Span::from(format!(" exited ({code})")).yellow());
    }
    if !connected {
        spans.push(Span::from(" disconnected, reconnecting").red());
    }
    // A config error spans several lines; its first says where it is.
    if let Some(message) = status.message.and_then(|message| message.lines().next()) {
        spans.push(Span::from(format!(" {message}")).red());
    }
    for hint in &status.hints {
        let (chord, label) = hint.split_once(' ').unwrap_or((hint, ""));
        spans.push(Span::from(format!("  {chord}")).cyan().bold());
        spans.push(Span::from(format!(" {label}")).gray());
    }
    // Herdr's panel background, a step up from the terminal's.
    (!spans.is_empty()).then(|| Line::from(spans).bg(Color::Black))
}

/// The pane's name, else its program's title, else its command's file name.
pub(super) fn pane_label(pane: &Pane) -> &str {
    pane.name
        .get()
        .or(pane.title.as_deref())
        .or_else(|| {
            let program = Path::new(pane.command.first()?).file_name()?;
            program.to_str()
        })
        .unwrap_or_default()
}

/// The tab's name, else its first pane's directory's last component, else
/// that pane's label, else its 1-based `position` among its siblings.
pub(super) fn tab_label(tab: &Tab, position: usize) -> String {
    tab.name
        .get()
        .map(str::to_owned)
        .or_else(|| {
            tab.panes().next().map(|pane| {
                pane.dir
                    .as_deref()
                    .map_or_else(|| pane_label(pane).to_owned(), last_component)
            })
        })
        .unwrap_or_else(|| (position + 1).to_string())
}

/// `~` for the home directory, else the path's last component, else the path
/// itself, as for `/`.
fn last_component(dir: &str) -> String {
    let path = Path::new(dir);
    if env::home_dir().is_some_and(|home| home == path) {
        return "~".to_owned();
    }
    path.file_name()
        .map_or(dir, |name| name.to_str().unwrap_or(dir))
        .to_owned()
}
