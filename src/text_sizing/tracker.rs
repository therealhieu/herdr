//! Finds kitty text sizing commands (`ESC ] 66 ; metadata ; text ST`) in pane
//! output so they can be applied in order with the bytes the core consumes.

use super::cell::{MAX_FRACTION, MAX_SCALE, MAX_WIDTH};

const PREFIX: &[u8] = b"66;";
const MAX_TEXT_BYTES: usize = 4096;
const MAX_METADATA_BYTES: usize = 256;
const MAX_BODY_BYTES: usize = PREFIX.len() + MAX_METADATA_BYTES + 1 + MAX_TEXT_BYTES;
const MAX_ALIGNMENT: u8 = 3;

/// Kitty OSC 66 metadata after clamping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSizingParams {
    pub scale: u8,
    pub width: u8,
    pub numerator: u8,
    pub denominator: u8,
    pub vertical: u8,
    pub horizontal: u8,
}

impl Default for TextSizingParams {
    fn default() -> Self {
        Self {
            scale: 1,
            width: 0,
            numerator: 0,
            denominator: 0,
            vertical: 0,
            horizontal: 0,
        }
    }
}

/// One complete OSC 66 command found in the observed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextSizingCommand {
    /// Offset just past the terminator, relative to the `observe` call that completed it.
    pub end_offset: usize,
    pub params: TextSizingParams,
    /// Text with C0 and C1 controls removed; never empty.
    pub text: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Escape,
    OscBody,
    OscEscape,
    SkipOsc,
    SkipOscEscape,
    IgnoreString,
    IgnoreStringEscape,
}

#[derive(Debug, Default)]
pub(crate) struct TextSizingTracker {
    state: State,
    body: Vec<u8>,
    pending: Vec<TextSizingCommand>,
}

impl TextSizingTracker {
    pub(crate) fn observe(&mut self, bytes: &[u8]) {
        let mut cursor = 0;
        while cursor < bytes.len() {
            if matches!(self.state, State::Ground | State::IgnoreString) {
                let Some(offset) = bytes[cursor..].iter().position(|&byte| byte == 0x1b) else {
                    break;
                };
                cursor += offset;
            }
            let byte = bytes[cursor];
            cursor += 1;
            self.state = match self.state {
                State::Ground => {
                    if byte == 0x1b {
                        State::Escape
                    } else {
                        State::Ground
                    }
                }
                State::Escape => self.after_escape(byte),
                State::OscBody => match byte {
                    0x07 => {
                        self.finalize(cursor);
                        State::Ground
                    }
                    0x1b => State::OscEscape,
                    0x18 | 0x1a => State::Ground,
                    _ => {
                        self.body.push(byte);
                        if self.body.len() > MAX_BODY_BYTES || !self.body_may_match() {
                            State::SkipOsc
                        } else {
                            State::OscBody
                        }
                    }
                },
                State::OscEscape => {
                    if byte == b'\\' {
                        self.finalize(cursor);
                        State::Ground
                    } else {
                        // ESC aborts the string and starts a new sequence.
                        self.after_escape(byte)
                    }
                }
                State::SkipOsc => match byte {
                    0x07 | 0x18 | 0x1a => State::Ground,
                    0x1b => State::SkipOscEscape,
                    _ => State::SkipOsc,
                },
                State::SkipOscEscape | State::IgnoreStringEscape => {
                    if byte == b'\\' {
                        State::Ground
                    } else {
                        self.after_escape(byte)
                    }
                }
                State::IgnoreString => {
                    if byte == 0x1b {
                        State::IgnoreStringEscape
                    } else {
                        State::IgnoreString
                    }
                }
            };
        }
    }

    pub(crate) fn drain_pending(&mut self) -> Vec<TextSizingCommand> {
        std::mem::take(&mut self.pending)
    }

    /// Forgets any partly seen sequence and undrained commands, so bytes the
    /// tracker did not observe can never complete a command.
    pub(crate) fn reset(&mut self) {
        self.state = State::Ground;
        self.body.clear();
        self.pending.clear();
    }

    fn after_escape(&mut self, byte: u8) -> State {
        match byte {
            b']' => {
                self.body.clear();
                State::OscBody
            }
            b'P' | b'_' | b'^' | b'X' => State::IgnoreString,
            0x1b => State::Escape,
            _ => State::Ground,
        }
    }

    /// Whether the body collected so far can still be an OSC 66 body.
    fn body_may_match(&self) -> bool {
        let len = self.body.len().min(PREFIX.len());
        self.body[..len] == PREFIX[..len]
    }

    fn finalize(&mut self, end_offset: usize) {
        if let Some((params, text)) = parse_body(&self.body) {
            self.pending.push(TextSizingCommand {
                end_offset,
                params,
                text,
            });
        }
        self.body.clear();
    }
}

fn parse_body(body: &[u8]) -> Option<(TextSizingParams, String)> {
    let rest = body.strip_prefix(PREFIX)?;
    let separator = rest.iter().position(|&byte| byte == b';')?;
    let (metadata, text) = (&rest[..separator], &rest[separator + 1..]);
    if metadata.len() > MAX_METADATA_BYTES || text.len() > MAX_TEXT_BYTES {
        return None;
    }
    let params = parse_metadata(metadata);
    let text: String = String::from_utf8_lossy(text)
        .chars()
        .filter(|&ch| !is_control(ch))
        .collect();
    (!text.is_empty()).then_some((params, text))
}

fn parse_metadata(metadata: &[u8]) -> TextSizingParams {
    let mut params = TextSizingParams::default();
    for pair in metadata.split(|&byte| byte == b':') {
        let Some((&key, value)) = pair.split_first() else {
            continue;
        };
        let Some(value) = value.strip_prefix(b"=").and_then(parse_number) else {
            continue;
        };
        match key {
            b's' => params.scale = value.clamp(1, MAX_SCALE),
            b'w' => params.width = value.min(MAX_WIDTH),
            b'n' => params.numerator = value.min(MAX_FRACTION),
            b'd' => params.denominator = value.min(MAX_FRACTION),
            b'v' => params.vertical = value.min(MAX_ALIGNMENT),
            b'h' => params.horizontal = value.min(MAX_ALIGNMENT),
            _ => {}
        }
    }
    params
}

/// Parses a decimal value, saturating at `u8::MAX`.
fn parse_number(digits: &[u8]) -> Option<u8> {
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = digits.iter().fold(0u32, |value, &digit| {
        value
            .saturating_mul(10)
            .saturating_add(u32::from(digit - b'0'))
    });
    Some(u8::try_from(value).unwrap_or(u8::MAX))
}

fn is_control(ch: char) -> bool {
    let value = u32::from(ch);
    value < 0x20 || (0x7f..=0x9f).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observe_all(chunks: &[&[u8]]) -> Vec<Vec<TextSizingCommand>> {
        let mut tracker = TextSizingTracker::default();
        chunks
            .iter()
            .map(|chunk| {
                tracker.observe(chunk);
                tracker.drain_pending()
            })
            .collect()
    }

    fn command(end_offset: usize, params: TextSizingParams, text: &str) -> TextSizingCommand {
        TextSizingCommand {
            end_offset,
            params,
            text: text.to_string(),
        }
    }

    #[test]
    fn finds_commands_with_bel_and_st_terminators() {
        let bytes = b"ab\x1b]66;w=2;xy\x07cd\x1b]66;s=2;H\x1b\\e";
        let found = observe_all(&[bytes]).remove(0);
        assert_eq!(
            found,
            vec![
                command(
                    14,
                    TextSizingParams {
                        width: 2,
                        ..TextSizingParams::default()
                    },
                    "xy"
                ),
                command(
                    bytes.len() - 1,
                    TextSizingParams {
                        scale: 2,
                        ..TextSizingParams::default()
                    },
                    "H"
                ),
            ]
        );
    }

    #[test]
    fn finds_commands_split_across_observe_calls() {
        let found = observe_all(&[b"x\x1b]6", b"6;s=2:w=1;", b"H\x1b", b"\\tail"]);
        assert!(found[0].is_empty() && found[1].is_empty() && found[2].is_empty());
        assert_eq!(
            found[3],
            vec![command(
                1,
                TextSizingParams {
                    scale: 2,
                    width: 1,
                    ..TextSizingParams::default()
                },
                "H"
            )]
        );
    }

    #[test]
    fn clamps_metadata_like_kitty() {
        let found = observe_all(&[b"\x1b]66;s=9:w=12:n=20:d=99:v=5:h=4:x=1;T\x07"]).remove(0);
        assert_eq!(
            found[0].params,
            TextSizingParams {
                scale: 7,
                width: 7,
                numerator: 15,
                denominator: 15,
                vertical: 3,
                horizontal: 3,
            }
        );
        let found = observe_all(&[b"\x1b]66;s=0;T\x07"]).remove(0);
        assert_eq!(found[0].params.scale, 1);
    }

    #[test]
    fn ignores_other_strings_and_oversized_bodies() {
        let oversized = format!("\x1b]66;s=2;{}\x07", "a".repeat(MAX_TEXT_BYTES + 1));
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x1b]0;66;title\x07\x1b]660;s=2;x\x07\x1b]6;x\x1b\\");
        bytes.extend_from_slice(b"\x1bP]66;s=2;dcs\x1b\\");
        bytes.extend_from_slice(oversized.as_bytes());
        bytes.extend_from_slice(b"\x1b]66;;ok\x07");
        let found = observe_all(&[&bytes]).remove(0);
        assert_eq!(
            found,
            vec![command(bytes.len(), TextSizingParams::default(), "ok")]
        );
    }

    #[test]
    fn drops_control_characters_from_text() {
        let found =
            observe_all(&["\x1b]66;w=1;a\tb\u{85}c\u{7f}\x07\x1b]66;s=2;\n\u{9f}\x07".as_bytes()])
                .remove(0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "abc");
    }

    #[test]
    fn reset_forgets_a_partly_seen_command() {
        let mut tracker = TextSizingTracker::default();
        tracker.observe(b"\x1b]66;s=2;Hel");
        tracker.reset();
        tracker.observe(b"def\x07\x1b]66;;ok\x07");
        assert_eq!(
            tracker.drain_pending(),
            vec![command(13, TextSizingParams::default(), "ok")]
        );
    }
}
