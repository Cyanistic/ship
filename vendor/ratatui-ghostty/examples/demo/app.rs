use std::sync::Arc;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui::layout::Rect;
use ratatui_ghostty::colors::HostColors;

use crate::keybinds::{Action, PrefixState};
use crate::layout::tall_layout;
use ratatui_ghostty::session::{SessionConfig, SessionEvent, SessionHandle, SessionIo};

pub struct Tab {
    pub panes: Vec<SessionHandle>,
    pub titles: Vec<String>,
    pub rects: Vec<Rect>,
    pub active: usize,
}

impl Tab {
    fn new_with_shell(
        cols: u16,
        rows: u16,
        shell: &str,
        colors: &HostColors,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let pane = spawn_pane(cols, rows, shell, colors, wake)?;
        Ok(Self {
            panes: vec![pane],
            titles: vec![String::new()],
            rects: vec![Rect::new(0, 0, cols, rows)],
            active: 0,
        })
    }
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub prefix: PrefixState,
    pub running: bool,
    shell: String,
    colors: HostColors,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl App {
    pub fn new(
        cols: u16,
        rows: u16,
        colors: HostColors,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
        let tab = Tab::new_with_shell(cols, rows, &shell, &colors, Arc::clone(&wake))?;
        Ok(Self {
            tabs: vec![tab],
            active_tab: 0,
            prefix: PrefixState::new(),
            running: true,
            shell,
            colors,
            wake,
        })
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn active_pane(&self) -> Option<&SessionHandle> {
        let tab = self.active_tab();
        tab.panes.get(tab.active)
    }

    /// Drain pane events and handle title updates. Returns true if any pane is dirty.
    pub fn drain_and_check_dirty(&mut self) -> bool {
        let mut any_dirty = false;
        for tab in &mut self.tabs {
            for (i, pane) in tab.panes.iter().enumerate() {
                while let Some(ev) = pane.poll_event() {
                    match ev {
                        SessionEvent::TitleChanged(t) => {
                            if i < tab.titles.len() {
                                tab.titles[i] = t;
                            }
                        }
                        SessionEvent::Exited
                        | SessionEvent::Bell
                        | SessionEvent::CwdChanged(_)
                        | SessionEvent::Error(_) => {}
                    }
                }
                if pane.is_dirty() {
                    any_dirty = true;
                }
            }
        }
        self.garbage_collect();
        any_dirty
    }

    fn garbage_collect(&mut self) {
        for tab in &mut self.tabs {
            let alive: Vec<bool> = tab.panes.iter().map(|p| p.is_alive()).collect();
            let mut iter = alive.iter();
            tab.panes.retain(|_| *iter.next().unwrap());
            let mut iter = alive.iter();
            tab.titles.retain(|_| *iter.next().unwrap());
            let mut iter = alive.iter();
            tab.rects.retain(|_| *iter.next().unwrap());
            if tab.active >= tab.panes.len() && !tab.panes.is_empty() {
                tab.active = tab.panes.len() - 1;
            }
        }
        self.tabs.retain(|t| !t.panes.is_empty());
        if self.tabs.is_empty() {
            self.running = false;
            return;
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn handle_action(&mut self, action: Action, pane_area: Rect) {
        let old_focus = self.focus_key();

        match action {
            Action::EnterPrefix => {}
            Action::Forward(event) => {
                if let Some(pane) = self.active_pane() {
                    pane.send_key(event);
                }
            }
            Action::NewTab => {
                if let Ok(tab) = Tab::new_with_shell(
                    pane_area.width,
                    pane_area.height,
                    &self.shell,
                    &self.colors,
                    Arc::clone(&self.wake),
                ) {
                    self.tabs.push(tab);
                    self.active_tab = self.tabs.len() - 1;
                    self.resize_all(pane_area);
                }
            }
            Action::CloseTab => {
                if !self.tabs.is_empty() {
                    self.tabs.remove(self.active_tab);
                    if self.tabs.is_empty() {
                        self.running = false;
                        return;
                    }
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                    if let Some(pane) = self.active_pane() {
                        pane.send_focus(true);
                    }
                }
            }
            Action::NextTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
            }
            Action::PrevTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
                }
            }
            Action::SplitPane => {
                if let Ok(pane) = spawn_pane(
                    pane_area.width,
                    pane_area.height,
                    &self.shell,
                    &self.colors,
                    Arc::clone(&self.wake),
                ) {
                    let tab = self.active_tab_mut();
                    tab.panes.push(pane);
                    tab.titles.push(String::new());
                    tab.rects.push(Rect::default());
                    tab.active = tab.panes.len() - 1;
                }
                self.resize_all(pane_area);
            }
            Action::ClosePane => {
                let tab = self.active_tab_mut();
                if !tab.panes.is_empty() {
                    tab.panes.remove(tab.active);
                    tab.titles.remove(tab.active);
                    tab.rects.remove(tab.active);
                    if tab.panes.is_empty() {
                        self.tabs.remove(self.active_tab);
                        if self.tabs.is_empty() {
                            self.running = false;
                            return;
                        }
                        if self.active_tab >= self.tabs.len() {
                            self.active_tab = self.tabs.len() - 1;
                        }
                    } else if tab.active >= tab.panes.len() {
                        tab.active = tab.panes.len() - 1;
                    }
                }
                self.resize_all(pane_area);
                if let Some(pane) = self.active_pane() {
                    pane.send_focus(true);
                }
            }
            Action::FocusLeft | Action::FocusUp => {
                let tab = self.active_tab_mut();
                if tab.active > 0 {
                    tab.active -= 1;
                }
            }
            Action::FocusRight | Action::FocusDown => {
                let tab = self.active_tab_mut();
                if tab.active + 1 < tab.panes.len() {
                    tab.active += 1;
                }
            }
            Action::Quit => {
                self.running = false;
            }
        }

        self.update_focus(old_focus);
    }

    fn focus_key(&self) -> (usize, usize) {
        (self.active_tab, self.tabs[self.active_tab].active)
    }

    fn update_focus(&self, old: (usize, usize)) {
        let new = self.focus_key();
        if old == new {
            return;
        }
        let old_valid = self.tabs.get(old.0).is_some_and(|t| old.1 < t.panes.len());
        let new_valid = self.tabs.get(new.0).is_some_and(|t| new.1 < t.panes.len());
        if !old_valid || !new_valid {
            return;
        }
        self.tabs[old.0].panes[old.1].send_focus(false);
        self.tabs[new.0].panes[new.1].send_focus(true);
    }

    pub fn pane_index_at(&self, col: u16, row: u16) -> Option<usize> {
        let tab = self.active_tab();
        tab.rects
            .iter()
            .position(|r| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height)
    }

    pub fn set_focus(&mut self, pane_index: usize) {
        let old = self.focus_key();
        let tab = self.active_tab_mut();
        if pane_index < tab.panes.len() {
            tab.active = pane_index;
        }
        self.update_focus(old);
    }

    pub fn resize_all(&mut self, area: Rect) {
        for tab in &mut self.tabs {
            let rects = tall_layout(area, tab.panes.len());
            for (pane, rect) in tab.panes.iter().zip(rects.iter()) {
                pane.send_resize(rect.width, rect.height);
            }
            tab.rects = rects;
        }
    }
}

fn spawn_pane(
    cols: u16,
    rows: u16,
    shell: &str,
    colors: &HostColors,
    wake: Arc<dyn Fn() + Send + Sync>,
) -> anyhow::Result<SessionHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", "xterm-256color");
    let _child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let writer = pair.master.take_writer()?;
    let reader = pair.master.try_clone_reader()?;
    let master = pair.master;

    let io = SessionIo {
        reader: Box::new(reader),
        writer: Box::new(writer),
        resizer: Box::new(move |cols, rows| {
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| e.into())
        }),
    };

    let config = SessionConfig {
        colors: Some(colors.clone()),
        ..SessionConfig::default()
    };

    SessionHandle::spawn(config, io, cols, rows, move || wake()).map_err(|e| anyhow::anyhow!(e))
}
