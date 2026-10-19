//! Replaces placeholder cells with block text in text read from a pane.

use super::cell::{decode, is_mark, BASE};

/// Replaces each block's top-left placeholder with the block text and drops
/// every other placeholder. Text without placeholders is returned unchanged.
pub(crate) fn decode_text(text: String) -> String {
    if !text.contains(BASE) {
        return text;
    }
    let mut decoded = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut grapheme = String::new();
    while let Some(ch) = chars.next() {
        if ch != BASE {
            decoded.push(ch);
            continue;
        }
        grapheme.clear();
        grapheme.push(ch);
        while let Some(&mark) = chars.peek() {
            if !is_mark(mark) {
                break;
            }
            grapheme.push(mark);
            chars.next();
        }
        if let Some(meta) = decode(&grapheme).and_then(|cell| cell.meta) {
            decoded.push_str(&meta.text);
        }
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_sizing::cell::{encode_cell, BlockMeta};

    #[test]
    fn replaces_block_cells_with_block_text() {
        let meta = BlockMeta {
            scale: 2,
            width: 1,
            numerator: 0,
            denominator: 0,
            vertical: 0,
            horizontal: 0,
            text: "H".to_string(),
        };
        let top = format!(
            "a{}{}b",
            encode_cell(0, 0, 5, Some(&meta)),
            encode_cell(0, 1, 5, None)
        );
        let bottom = format!(
            "a{}{}b",
            encode_cell(1, 0, 5, None),
            encode_cell(1, 1, 5, None)
        );

        let decoded = decode_text(format!("{top}\n{bottom}"));

        assert_eq!(decoded, "aHb\nab");
        assert!(!decoded.contains(BASE));
    }

    #[test]
    fn returns_text_without_placeholders_unchanged() {
        let text = String::from("plain \u{1f44d} text");
        let pointer = text.as_ptr();
        let decoded = decode_text(text);
        assert_eq!(decoded, "plain \u{1f44d} text");
        assert_eq!(decoded.as_ptr(), pointer);
    }
}
