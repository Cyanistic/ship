//! Temporary OSC 7501, OSC 133 prompt and RIS scanner ahead of libghostty.
//! Delete when libghostty-rs exposes program status and semantic prompts.

use std::str::from_utf8;

use base64::{
    Engine,
    alphabet::STANDARD,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};

pub(crate) const QUERY_REPLY: &str =
    "?:states=idle,working,done,blocked,error:kinds=permission,question,auth";
const MAX_SEQUENCE: usize = 4096;

/// Mirrors GhosttyTerminalProgramStatus; an empty id addresses the root.
#[derive(Clone, Debug)]
pub struct ProgramStatus {
    pub state: ProgramState,
    pub kind: Option<ProgramKind>,
    pub progress: Option<u8>,
    pub id: String,
    pub app: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramState {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramKind {
    Permission,
    Question,
    Auth,
}

pub(crate) enum Found {
    Query(Terminator),
    Report(ProgramStatus),
    PromptStart,
    Reset,
}

#[derive(Clone, Copy)]
pub(crate) enum Terminator {
    Bel,
    St,
}

#[derive(Default)]
pub(crate) struct Scanner {
    state: Scan,
    sequence: Vec<u8>,
    oversized: bool,
}

#[derive(Default)]
enum Scan {
    #[default]
    Ground,
    Escape,
    Osc,
    OscEscape,
}

impl Scanner {
    pub fn scan(&mut self, bytes: &[u8], mut found: impl FnMut(Found)) {
        for &byte in bytes {
            match self.state {
                Scan::Ground => {
                    if byte == 0x1b {
                        self.state = Scan::Escape;
                    }
                }
                Scan::Escape => {
                    self.state = match byte {
                        b']' => {
                            self.sequence.clear();
                            self.oversized = false;
                            Scan::Osc
                        }
                        b'c' => {
                            found(Found::Reset);
                            Scan::Ground
                        }
                        0x1b => Scan::Escape,
                        _ => Scan::Ground,
                    };
                }
                Scan::Osc | Scan::OscEscape => {
                    let st = matches!(self.state, Scan::OscEscape) && byte == b'\\';
                    if byte == 7 || st {
                        // Count OSC and the terminator too. ESC for ST was buffered.
                        if !self.oversized && self.sequence.len() + 3 <= MAX_SEQUENCE {
                            if st {
                                self.sequence.pop();
                            }
                            self.finish(
                                if st { Terminator::St } else { Terminator::Bel },
                                &mut found,
                            );
                        }
                        self.state = Scan::Ground;
                    } else {
                        if self.sequence.len() < MAX_SEQUENCE {
                            self.sequence.push(byte);
                        } else {
                            self.oversized = true;
                        }
                        self.state = if byte == 0x1b {
                            Scan::OscEscape
                        } else {
                            Scan::Osc
                        };
                    }
                }
            }
        }
    }

    fn finish(&self, terminator: Terminator, found: &mut impl FnMut(Found)) {
        if let Some(body) = self.sequence.strip_prefix(b"7501;") {
            if body.trim_ascii() == b"?" {
                found(Found::Query(terminator));
            } else if let Some(report) = report(body) {
                found(Found::Report(report));
            }
        } else if self.sequence == b"133;A" || self.sequence.starts_with(b"133;A;") {
            found(Found::PromptStart);
        }
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.+-".contains(&b))
}

fn text(value: &str, encoded: usize, decoded: usize) -> Option<String> {
    if value.len() > encoded {
        return None;
    }
    let decoder = GeneralPurpose::new(
        &STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
    let text = String::from_utf8(decoder.decode(value).ok()?).ok()?;
    (text.len() <= decoded && !text.chars().any(char::is_control)).then_some(text)
}

/// Check every pair before applying anything, including overwritten fields.
fn report(body: &[u8]) -> Option<ProgramStatus> {
    let mut state = None;
    let mut kind = None;
    let mut progress = None;
    let mut id = String::new();
    let mut app = None;
    let mut title = None;
    let mut message = None;
    for pair in body.split(|&b| b == b':') {
        let Some(at) = pair.iter().position(|&b| b == b'=') else {
            continue;
        };
        let key = pair[..at].trim_ascii();
        let value = pair[at + 1..].trim_ascii();
        if key.len() > 16 {
            return None;
        }
        if key.is_empty() || !key.iter().all(u8::is_ascii_lowercase) {
            continue;
        }
        // Invalid ids must never fall back to the root; invalid apps drop the report.
        if key == b"app" && !identifier(from_utf8(value).ok()?) {
            return None;
        }
        if key == b"id" {
            let id = from_utf8(value).ok()?;
            if id.len() > 128 || id.split('/').count() > 8 || !id.split('/').all(identifier) {
                return None;
            }
        }
        if (key == b"title" && value.len() > 256) || (key == b"msg" && value.len() > 2732) {
            return None;
        }
        if !value
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"_.,+/=-".contains(b))
        {
            continue;
        }
        let value = from_utf8(value).ok()?;
        match key {
            b"state" => {
                state = match value {
                    "idle" => Some(ProgramState::Idle),
                    "working" => Some(ProgramState::Working),
                    "done" => Some(ProgramState::Done),
                    "blocked" => Some(ProgramState::Blocked),
                    "error" => Some(ProgramState::Error),
                    "clear" => Some(ProgramState::Clear),
                    _ => None,
                }
            }
            b"kind" => {
                kind = match value {
                    "permission" => Some(ProgramKind::Permission),
                    "question" => Some(ProgramKind::Question),
                    "auth" => Some(ProgramKind::Auth),
                    _ => None,
                }
            }
            b"progress" => {
                progress = value
                    .bytes()
                    .all(|b| b.is_ascii_digit())
                    .then(|| value.parse::<u8>().ok().filter(|&n| n <= 100))
                    .flatten();
            }
            b"id" => id = value.to_owned(),
            b"app" => app = Some(value.to_owned()),
            b"title" => title = Some(text(value, 256, 192)?),
            b"msg" => message = Some(text(value, 2732, 2048)?),
            _ => {}
        }
    }
    let state = state?;
    Some(ProgramStatus {
        state,
        kind: kind.filter(|_| state == ProgramState::Blocked),
        progress: progress
            .filter(|_| matches!(state, ProgramState::Working | ProgramState::Blocked)),
        id,
        app,
        title,
        message,
    })
}
