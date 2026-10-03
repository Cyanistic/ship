use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SplitPane,
    ClosePane,
    FocusLeft,
    FocusDown,
    FocusUp,
    FocusRight,
    Quit,
    /// Prefix mode entered, consume the key.
    EnterPrefix,
    /// Key not handled by prefix, forward to pane.
    Forward(KeyEvent),
}

pub struct PrefixState {
    prefix_key: char,
    pub active: bool,
}

impl PrefixState {
    pub fn new() -> Self {
        let prefix_key = std::env::var("RATATUI_GHOSTTY_PREFIX_KEY")
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or('b');

        Self {
            prefix_key,
            active: false,
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> Action {
        if self.active {
            self.active = false;
            return self.dispatch_prefix(event);
        }

        if event.modifiers.contains(KeyModifiers::CONTROL)
            && event.code == KeyCode::Char(self.prefix_key)
        {
            self.active = true;
            return Action::EnterPrefix;
        }

        Action::Forward(event)
    }

    fn dispatch_prefix(&self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::Char('c') => Action::NewTab,
            KeyCode::Char('&') => Action::CloseTab,
            KeyCode::Char('n') => Action::NextTab,
            KeyCode::Char('p') => Action::PrevTab,
            KeyCode::Char('%') => Action::SplitPane,
            KeyCode::Char('x') => Action::ClosePane,
            KeyCode::Char('h') | KeyCode::Left => Action::FocusLeft,
            KeyCode::Char('j') | KeyCode::Down => Action::FocusDown,
            KeyCode::Char('k') | KeyCode::Up => Action::FocusUp,
            KeyCode::Char('l') | KeyCode::Right => Action::FocusRight,
            KeyCode::Char('[') => Action::Forward(event),
            KeyCode::Char('q') => Action::Quit,
            _ => Action::Forward(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn prefix_activates_on_ctrl_b() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: false,
        };
        assert_eq!(state.handle_key(ctrl('b')), Action::EnterPrefix);
        assert!(state.active);
    }

    #[test]
    fn non_prefix_key_forwards() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: false,
        };
        let ev = key(KeyCode::Char('a'));
        assert_eq!(state.handle_key(ev), Action::Forward(ev));
    }

    #[test]
    fn prefix_then_c_creates_tab() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('c'))), Action::NewTab);
        assert!(!state.active);
    }

    #[test]
    fn prefix_then_ampersand_closes_tab() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('&'))), Action::CloseTab);
    }

    #[test]
    fn prefix_then_n_next_tab() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('n'))), Action::NextTab);
    }

    #[test]
    fn prefix_then_p_prev_tab() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('p'))), Action::PrevTab);
    }

    #[test]
    fn prefix_then_percent_splits() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('%'))), Action::SplitPane);
    }

    #[test]
    fn prefix_then_x_closes_pane() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('x'))), Action::ClosePane);
    }

    #[test]
    fn prefix_then_q_quits() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        assert_eq!(state.handle_key(key(KeyCode::Char('q'))), Action::Quit);
    }

    #[test]
    fn prefix_hjkl_focus() {
        for (c, expected) in [
            ('h', Action::FocusLeft),
            ('j', Action::FocusDown),
            ('k', Action::FocusUp),
            ('l', Action::FocusRight),
        ] {
            let mut state = PrefixState {
                prefix_key: 'b',
                active: true,
            };
            assert_eq!(state.handle_key(key(KeyCode::Char(c))), expected);
        }
    }

    #[test]
    fn prefix_arrows_focus() {
        for (code, expected) in [
            (KeyCode::Left, Action::FocusLeft),
            (KeyCode::Down, Action::FocusDown),
            (KeyCode::Up, Action::FocusUp),
            (KeyCode::Right, Action::FocusRight),
        ] {
            let mut state = PrefixState {
                prefix_key: 'b',
                active: true,
            };
            assert_eq!(state.handle_key(key(code)), expected);
        }
    }

    #[test]
    fn prefix_unknown_key_forwards() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        let ev = key(KeyCode::Char('z'));
        assert_eq!(state.handle_key(ev), Action::Forward(ev));
        assert!(!state.active);
    }

    #[test]
    fn prefix_bracket_forwards_noop() {
        let mut state = PrefixState {
            prefix_key: 'b',
            active: true,
        };
        let ev = key(KeyCode::Char('['));
        assert_eq!(state.handle_key(ev), Action::Forward(ev));
    }

    #[test]
    fn custom_prefix_key() {
        let mut state = PrefixState {
            prefix_key: 'a',
            active: false,
        };
        // C-b should NOT activate with prefix_key='a'
        let ev = ctrl('b');
        assert_eq!(state.handle_key(ev), Action::Forward(ev));
        // C-a should activate
        assert_eq!(state.handle_key(ctrl('a')), Action::EnterPrefix);
        assert!(state.active);
    }
}
