//! A pane's OSC 7501 records under revision 0.4's lifecycle rules.

use indexmap::IndexMap;
use ratatui_ghostty::program_status::{ProgramKind, ProgramState, ProgramStatus};
use ship_core::model::{BlockedKind, ROOT, ReportedState, StatusRecord};

const MAX_RECORDS: usize = 64;

/// Oldest update first, including repeated identical reports.
#[derive(Default)]
pub(crate) struct Records(IndexMap<String, StatusRecord>);

impl Records {
    pub fn apply(&mut self, report: ProgramStatus) -> bool {
        let state = match report.state {
            ProgramState::Idle => ReportedState::Idle,
            ProgramState::Working => ReportedState::Working,
            ProgramState::Done => ReportedState::Done,
            ProgramState::Blocked => ReportedState::Blocked,
            ProgramState::Error => ReportedState::Error,
            ProgramState::Clear => {
                let before = self.0.len();
                self.0.retain(|id, _| !within(id, &report.id));
                return before != self.0.len();
            }
        };
        self.0.shift_remove(&report.id);
        if self.0.len() == MAX_RECORDS {
            self.0.shift_remove_index(0);
        }
        self.0.insert(
            report.id,
            StatusRecord {
                state,
                kind: report.kind.map(|kind| match kind {
                    ProgramKind::Permission => BlockedKind::Permission,
                    ProgramKind::Question => BlockedKind::Question,
                    ProgramKind::Auth => BlockedKind::Auth,
                }),
                progress: report.progress,
                app: report.app,
                title: report.title.map(disarm),
                msg: report.message.map(disarm),
            },
        );
        true
    }

    pub fn prompt(&mut self) -> bool {
        let before = self.0.len();
        self.0.retain(|_, record| {
            !matches!(
                record.state,
                ReportedState::Working | ReportedState::Blocked
            )
        });
        before != self.0.len()
    }

    pub fn snapshot(&self) -> IndexMap<String, StatusRecord> {
        self.0.clone()
    }
}

fn within(id: &str, ancestor: &str) -> bool {
    ancestor == ROOT
        || id == ancestor
        || id
            .strip_prefix(ancestor)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn disarm(text: String) -> String {
    text.chars().filter(|c| !matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')).collect()
}
