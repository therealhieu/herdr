//! Writes OSC 66 blocks into the pane core as placeholder cells, placing them
//! and moving the cursor the way kitty does.
//!
//! Only relative cursor movement is used, so origin mode and scroll margins
//! cannot offset the block.

use std::fmt::Write as _;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::cell::{encode_cell, BlockMeta, NONCE_LIMIT};
use super::tracker::{TextSizingCommand, TextSizingParams};
use crate::ghostty::{Terminal, MODE_WRAPAROUND};

/// Applies one OSC 66 command at the current cursor position.
pub(crate) fn apply(terminal: &mut Terminal, command: &TextSizingCommand, nonce: &mut u16) {
    let params = command.params;
    if writes_plain_text(params, &command.text) {
        terminal.write(command.text.as_bytes());
        return;
    }
    if params.width > 0 {
        place_block(
            terminal,
            block_meta(params, params.width, &command.text),
            nonce,
        );
        return;
    }
    for grapheme in command.text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme).min(2);
        let Ok(width) = u8::try_from(width) else {
            continue;
        };
        if width == 0 {
            continue;
        }
        place_block(terminal, block_meta(params, width, grapheme), nonce);
    }
}

/// Commands that size nothing are written as ordinary text: unscaled text
/// with automatic width, and a single width-1 codepoint pinned to width 1.
fn writes_plain_text(params: TextSizingParams, text: &str) -> bool {
    let fractional = params.numerator > 0 && params.denominator > params.numerator;
    if params.scale != 1 || fractional {
        return false;
    }
    if params.width == 0 {
        return true;
    }
    let mut chars = text.chars();
    params.width == 1
        && matches!((chars.next(), chars.next()), (Some(ch), None) if ch.width() == Some(1))
}

fn block_meta(params: TextSizingParams, width: u8, text: &str) -> BlockMeta {
    BlockMeta {
        scale: params.scale,
        width,
        numerator: params.numerator,
        denominator: params.denominator,
        vertical: params.vertical,
        horizontal: params.horizontal,
        text: text.to_string(),
    }
}

/// Places one block like kitty's `handle_fixed_width_multicell_command`.
fn place_block(terminal: &mut Terminal, meta: BlockMeta, nonce: &mut u16) {
    let (Ok(cols), Ok(rows), Ok(cursor_x), Ok(pending_wrap)) = (
        terminal.cols(),
        terminal.rows(),
        terminal.cursor_x(),
        terminal.cursor_pending_wrap(),
    ) else {
        return;
    };
    let (width, height) = (meta.columns(), meta.rows());
    if width == 0 || width > cols || height > rows {
        return;
    }
    let wraparound = terminal.mode_get(MODE_WRAPAROUND).unwrap_or(true);
    // A pending wrap means the cursor logically sits past the last column.
    let cursor_x = if pending_wrap {
        cursor_x.saturating_add(1)
    } else {
        cursor_x
    };

    let mut out = String::new();
    let x = if cursor_x.saturating_add(width) <= cols {
        cursor_x
    } else if wraparound {
        // Carriage return plus index: next line, scrolling at the bottom.
        out.push_str("\r\x1bD");
        0
    } else {
        cols - width
    };

    // Make room for the lower rows, scrolling when the block reaches the
    // bottom, then return to the top row.
    if height > 1 {
        for _ in 1..height {
            out.push_str("\x1bD");
        }
        let _ = write!(out, "\x1b[{}A", height - 1);
    }

    let block_nonce = *nonce;
    *nonce = (*nonce + 1) % NONCE_LIMIT;
    // Lower rows first, the top row last, so the final cursor and pending
    // wrap state equal printing `width` ordinary cells on the original row.
    let mut current_row = 0u16;
    for row in (0..height).rev() {
        if row > current_row {
            let _ = write!(out, "\x1b[{}B", row - current_row);
        } else if row < current_row {
            let _ = write!(out, "\x1b[{}A", current_row - row);
        }
        current_row = row;
        out.push('\r');
        if x > 0 {
            let _ = write!(out, "\x1b[{x}C");
        }
        for col in 0..width {
            let top_left = row == 0 && col == 0;
            out.push_str(&encode_cell(
                u8::try_from(row).unwrap_or(u8::MAX),
                u8::try_from(col).unwrap_or(u8::MAX),
                block_nonce,
                top_left.then_some(&meta),
            ));
        }
    }

    // Without autowrap, Ghostty attaches marks printed at the last column to
    // the cell on its left; autowrap's pending-wrap state keeps them on the
    // cell just printed. The final cursor position is the same either way.
    if !wraparound {
        let _ = terminal.mode_set(MODE_WRAPAROUND, true);
    }
    terminal.write(out.as_bytes());
    if !wraparound {
        let _ = terminal.mode_set(MODE_WRAPAROUND, false);
    }
}
