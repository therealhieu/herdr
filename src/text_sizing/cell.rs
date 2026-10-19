//! Placeholder cell contract for sized text blocks.
//!
//! Every cell of a block holds one grapheme: the base [`BASE`] followed by
//! zero-width marks from the Variation Selectors Supplement (`U+E0100 + v`,
//! `v < 240`). Each cell carries `row`, `col`, `nonce_hi`, and `nonce_lo`;
//! the top-left cell (`row == col == 0`) adds `scale * 8 + width`, the
//! fractional numerator and denominator, `vertical * 3 + horizontal`, and the
//! block text as base-120 digits (a digit below 120 ends a codepoint, digits
//! `120..240` continue it).

/// Base codepoint of every placeholder cell (Plane 16 private use, width 1).
pub(crate) const BASE: char = '\u{10EEED}';
const MARK_FIRST: u32 = 0xE0100;
const MARK_VALUES: u32 = 240;
/// Ghostty stores at most this many codepoints after a cell's base.
pub(crate) const MAX_MARKS: usize = 64;
const POSITION_MARKS: usize = 4;
const HEADER_MARKS: usize = 8;
/// Marks left for the block text in the top-left cell.
pub(crate) const MAX_TEXT_MARKS: usize = MAX_MARKS - HEADER_MARKS;
/// Kitty's limit on codepoints stored in one multicell block.
pub(crate) const MAX_TEXT_CODEPOINTS: usize = 24;
const TEXT_DIGIT_BASE: u32 = 120;
const MAX_TEXT_DIGITS: usize = 3;
/// Number of distinct block nonces.
pub(crate) const NONCE_LIMIT: u16 = (MARK_VALUES * MARK_VALUES) as u16;
pub(crate) const MAX_SCALE: u8 = 7;
pub(crate) const MAX_WIDTH: u8 = 7;
pub(crate) const MAX_FRACTION: u8 = 15;
const MAX_ALIGNMENT: u8 = 2;

/// Sizing parameters and text of one block, stored in its top-left cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockMeta {
    /// Kitty `s`: the block spans `scale` rows.
    pub scale: u8,
    /// Kitty `w`: cells per scaled column; the block spans `scale * width` columns.
    pub width: u8,
    /// Kitty `n`: fractional scale numerator.
    pub numerator: u8,
    /// Kitty `d`: fractional scale denominator.
    pub denominator: u8,
    /// Kitty `v`: vertical alignment.
    pub vertical: u8,
    /// Kitty `h`: horizontal alignment.
    pub horizontal: u8,
    pub text: String,
}

impl BlockMeta {
    pub(crate) fn columns(&self) -> u16 {
        u16::from(self.scale) * u16::from(self.width)
    }

    pub(crate) fn rows(&self) -> u16 {
        u16::from(self.scale)
    }
}

/// One decoded placeholder cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Placeholder {
    pub row: u8,
    pub col: u8,
    pub nonce: u16,
    /// Present only on the top-left cell.
    pub meta: Option<BlockMeta>,
}

/// Returns whether `grapheme` starts with the placeholder base.
pub(crate) fn is_placeholder(grapheme: &str) -> bool {
    grapheme.starts_with(BASE)
}

/// Encodes one placeholder cell. `meta` is written only for the top-left
/// cell; its text is truncated at a codepoint to fit the mark limit and
/// kitty's codepoint limit.
pub(crate) fn encode_cell(row: u8, col: u8, nonce: u16, meta: Option<&BlockMeta>) -> String {
    let nonce = nonce % NONCE_LIMIT;
    let mut marks: Vec<u32> = Vec::with_capacity(MAX_MARKS);
    marks.extend([
        u32::from(row),
        u32::from(col),
        u32::from(nonce) / MARK_VALUES,
        u32::from(nonce) % MARK_VALUES,
    ]);
    if let Some(meta) = meta {
        let scale = meta.scale.clamp(1, MAX_SCALE);
        let width = meta.width.clamp(1, MAX_WIDTH);
        marks.extend([
            u32::from(scale) * 8 + u32::from(width),
            u32::from(meta.numerator.min(MAX_FRACTION)),
            u32::from(meta.denominator.min(MAX_FRACTION)),
            u32::from(alignment(meta.vertical)) * 3 + u32::from(alignment(meta.horizontal)),
        ]);
        let mut digits = [0u32; MAX_TEXT_DIGITS];
        for ch in meta.text.chars().take(MAX_TEXT_CODEPOINTS) {
            let count = text_digits(u32::from(ch), &mut digits);
            if marks.len() + count > MAX_MARKS {
                break;
            }
            marks.extend_from_slice(&digits[MAX_TEXT_DIGITS - count..]);
        }
    }

    let mut encoded = String::with_capacity(4 * (marks.len() + 1));
    encoded.push(BASE);
    encoded.extend(marks.into_iter().filter_map(mark_char));
    encoded
}

/// Decodes a placeholder grapheme, or returns `None` when it is malformed.
pub(crate) fn decode(grapheme: &str) -> Option<Placeholder> {
    let mut chars = grapheme.chars();
    if chars.next()? != BASE {
        return None;
    }
    let mut marks = [0u8; MAX_MARKS];
    let mut len = 0;
    for ch in chars {
        let value = u32::from(ch).checked_sub(MARK_FIRST)?;
        if value >= MARK_VALUES || len == MAX_MARKS {
            return None;
        }
        marks[len] = u8::try_from(value).ok()?;
        len += 1;
    }
    let marks = &marks[..len];
    if marks.len() < POSITION_MARKS {
        return None;
    }
    let (row, col) = (marks[0], marks[1]);
    if row >= MAX_SCALE || u16::from(col) >= u16::from(MAX_SCALE) * u16::from(MAX_WIDTH) {
        return None;
    }
    let nonce = u16::from(marks[2]) * MARK_VALUES as u16 + u16::from(marks[3]);
    if row != 0 || col != 0 {
        return (marks.len() == POSITION_MARKS).then_some(Placeholder {
            row,
            col,
            nonce,
            meta: None,
        });
    }

    if marks.len() <= HEADER_MARKS {
        return None;
    }
    let (scale, width) = (marks[4] / 8, marks[4] % 8);
    let (numerator, denominator) = (marks[5], marks[6]);
    let (vertical, horizontal) = (marks[7] / 3, marks[7] % 3);
    if !(1..=MAX_SCALE).contains(&scale)
        || width == 0
        || numerator > MAX_FRACTION
        || denominator > MAX_FRACTION
        || vertical > MAX_ALIGNMENT
    {
        return None;
    }
    let text = decode_text_digits(&marks[HEADER_MARKS..])?;
    Some(Placeholder {
        row,
        col,
        nonce,
        meta: Some(BlockMeta {
            scale,
            width,
            numerator,
            denominator,
            vertical,
            horizontal,
            text,
        }),
    })
}

/// Kitty treats alignment value 3 like the default (0).
fn alignment(value: u8) -> u8 {
    if value > MAX_ALIGNMENT {
        0
    } else {
        value
    }
}

/// Writes `value` as base-120 digits right-aligned into `digits`; returns the
/// digit count. Every digit but the last is offset by 120 to mark continuation.
fn text_digits(value: u32, digits: &mut [u32; MAX_TEXT_DIGITS]) -> usize {
    let mut remaining = value;
    let mut count = 0;
    loop {
        let digit = remaining % TEXT_DIGIT_BASE;
        let offset = if count == 0 { 0 } else { TEXT_DIGIT_BASE };
        digits[MAX_TEXT_DIGITS - 1 - count] = digit + offset;
        count += 1;
        remaining /= TEXT_DIGIT_BASE;
        if remaining == 0 || count == MAX_TEXT_DIGITS {
            return count;
        }
    }
}

fn decode_text_digits(marks: &[u8]) -> Option<String> {
    let mut text = String::new();
    let mut value = 0u32;
    let mut digits = 0;
    for &mark in marks {
        let mark = u32::from(mark);
        digits += 1;
        if digits > MAX_TEXT_DIGITS {
            return None;
        }
        if mark >= TEXT_DIGIT_BASE {
            value = value * TEXT_DIGIT_BASE + (mark - TEXT_DIGIT_BASE);
        } else {
            text.push(char::from_u32(value * TEXT_DIGIT_BASE + mark)?);
            value = 0;
            digits = 0;
        }
    }
    (digits == 0 && !text.is_empty()).then_some(text)
}

fn mark_char(value: u32) -> Option<char> {
    char::from_u32(MARK_FIRST + value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(text: &str) -> BlockMeta {
        BlockMeta {
            scale: 2,
            width: 1,
            numerator: 1,
            denominator: 2,
            vertical: 2,
            horizontal: 1,
            text: text.to_string(),
        }
    }

    fn mark_count(encoded: &str) -> usize {
        encoded.chars().count() - 1
    }

    #[test]
    fn round_trips_meta_and_text() {
        for text in ["H", "\u{223c}", "\u{6f22}\u{5b57}", "\u{1f44d}\u{1f3fd}"] {
            let block = meta(text);
            let encoded = encode_cell(0, 0, 0x1234, Some(&block));
            assert!(is_placeholder(&encoded));
            assert_eq!(
                decode(&encoded),
                Some(Placeholder {
                    row: 0,
                    col: 0,
                    nonce: 0x1234,
                    meta: Some(block),
                }),
                "text {text:?}"
            );
        }
    }

    #[test]
    fn round_trips_non_origin_cells_without_meta() {
        let encoded = encode_cell(1, 13, NONCE_LIMIT - 1, None);
        assert_eq!(mark_count(&encoded), 4);
        assert_eq!(
            decode(&encoded),
            Some(Placeholder {
                row: 1,
                col: 13,
                nonce: NONCE_LIMIT - 1,
                meta: None,
            })
        );
    }

    #[test]
    fn truncates_text_to_kitty_codepoint_limit() {
        let encoded = encode_cell(0, 0, 7, Some(&meta(&"a".repeat(30))));
        let decoded = decode(&encoded).and_then(|cell| cell.meta);
        assert_eq!(decoded.map(|meta| meta.text), Some("a".repeat(24)));
    }

    #[test]
    fn truncates_text_at_a_codepoint_to_fit_the_mark_limit() {
        // Each emoji needs three base-120 digits: 18 fit into 56 text marks.
        let encoded = encode_cell(0, 0, 7, Some(&meta(&"\u{1f44d}".repeat(20))));
        assert!(mark_count(&encoded) <= MAX_MARKS);
        let decoded = decode(&encoded).and_then(|cell| cell.meta);
        assert_eq!(decoded.map(|meta| meta.text), Some("\u{1f44d}".repeat(18)));
    }

    #[test]
    fn rejects_malformed_graphemes() {
        let valid = encode_cell(0, 0, 9, Some(&meta("H")));
        let wrong_base = valid.replacen(BASE, "A", 1);
        let out_of_range = format!("{valid}\u{e01f0}");
        let foreign_char = format!("{valid}x");
        let missing_header = encode_cell(0, 0, 9, None);
        let short: String = valid.chars().take(4).collect();
        // Replace the final text digit with a continuation digit.
        let mut chars: Vec<char> = valid.chars().collect();
        chars.pop();
        chars.push(char::from_u32(MARK_FIRST + 130).expect("valid mark"));
        let unterminated_text: String = chars.into_iter().collect();

        for grapheme in [
            "",
            wrong_base.as_str(),
            out_of_range.as_str(),
            foreign_char.as_str(),
            missing_header.as_str(),
            short.as_str(),
            unterminated_text.as_str(),
        ] {
            assert_eq!(decode(grapheme), None, "grapheme {grapheme:?}");
        }
    }
}
