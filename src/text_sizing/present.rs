//! Client presentation of sized text blocks.
//!
//! Runs on the final composed frame, before the frame encoder. A block whose
//! placeholder cells are all present ("intact") is drawn by a host that can
//! size text: its cells become `skip` cells and one OSC 66 is written after
//! the frame. Every other placeholder cell gets fallback text: the block text
//! unscaled in its top row, spaces elsewhere.
//!
//! Blocks drawn on the host that are not identical in the next frame are
//! erased with `ECH` before the frame is written, so the encoder never writes
//! into a host block. Skipped cells keep their placeholder symbols, so the
//! encoder's stored frame differs from any later non-placeholder cell and
//! rewrites a stale block region.

use std::io::Write;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::cell::{decode, is_placeholder, BlockMeta, Placeholder};
use crate::protocol::{render_ansi, CellData, FrameData};

const SYNC_OUTPUT_START: &[u8] = b"\x1b[?2026h";

/// One block drawn, or to be drawn, by the host terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HostBlock {
    x: u16,
    y: u16,
    nonce: u16,
    meta: BlockMeta,
    /// Foreground, background, and modifier of the top-left cell.
    style: (u32, u32, u16),
}

/// Tracks the sized blocks drawn on the host terminal.
#[derive(Debug, Default)]
pub(crate) struct Presenter {
    host_blocks: Vec<HostBlock>,
}

/// Intact blocks of one prepared frame.
#[derive(Debug, Default)]
pub(crate) struct Pass {
    blocks: Vec<HostBlock>,
    width: u16,
    height: u16,
}

impl Presenter {
    /// Marks intact blocks as `skip` cells when the host can size text and
    /// replaces every other placeholder cell with fallback text.
    ///
    /// Without host blocks or placeholder cells this is one scan of cell
    /// symbol prefixes and allocates nothing: `Vec::new` reserves no memory.
    pub(crate) fn prepare(&self, frame: &mut FrameData, host_supported: bool) -> Pass {
        let mut pass = Pass {
            blocks: Vec::new(),
            width: frame.width,
            height: frame.height,
        };
        if self.host_blocks.is_empty() && !has_placeholder(frame) {
            return pass;
        }
        present_blocks(frame, host_supported.then_some(&mut pass.blocks));
        pass
    }

    /// Returns the bytes to write before (`pre`) and after (`post`) the
    /// encoded frame. `full` means the encoder redraws the whole frame.
    pub(crate) fn finish(&self, pass: &Pass, full: bool) -> (Vec<u8>, Vec<u8>) {
        let mut pre = Vec::new();
        for block in self
            .host_blocks
            .iter()
            .filter(|block| full || !pass.blocks.contains(block))
        {
            if pre.is_empty() {
                pre.extend_from_slice(b"\x1b[0m");
            }
            erase_block(&mut pre, block, pass.width, pass.height);
        }

        let mut post = Vec::new();
        for block in pass
            .blocks
            .iter()
            .filter(|block| full || !self.host_blocks.contains(block))
        {
            if post.is_empty() {
                post.extend_from_slice(b"\x1b7");
            }
            draw_block(&mut post, block);
        }
        if !post.is_empty() {
            post.extend_from_slice(b"\x1b8");
        }
        (pre, post)
    }

    /// Inserts this pass's `pre` bytes after the encoded frame's leading
    /// synchronized-output start and its `post` bytes before the final
    /// synchronized-output end.
    pub(crate) fn write_into(&self, pass: &Pass, full: bool, bytes: &mut Vec<u8>) {
        if self.host_blocks.is_empty() && pass.blocks.is_empty() {
            return;
        }
        let (pre, post) = self.finish(pass, full);
        if !post.is_empty() {
            let at = render_ansi::final_sync_output_end(bytes).unwrap_or(bytes.len());
            bytes.splice(at..at, post);
        }
        if !pre.is_empty() {
            let at = if bytes.starts_with(SYNC_OUTPUT_START) {
                SYNC_OUTPUT_START.len()
            } else {
                0
            };
            bytes.splice(at..at, pre);
        }
    }

    /// Records the pass's blocks as drawn; call only after its bytes were
    /// written to the host.
    pub(crate) fn commit(&mut self, pass: Pass) {
        self.host_blocks = pass.blocks;
    }

    /// Returns whether the host currently shows sized blocks.
    pub(crate) fn has_host_blocks(&self) -> bool {
        !self.host_blocks.is_empty()
    }
}

/// Replaces every placeholder cell with fallback text, for outputs that never
/// draw sized text. Without placeholder cells this only scans symbol
/// prefixes and allocates nothing.
pub(crate) fn apply_fallback(frame: &mut FrameData) {
    if has_placeholder(frame) {
        present_blocks(frame, None);
    }
}

/// Returns whether any cell holds a placeholder.
pub(crate) fn cells_have_placeholder(cells: &[CellData]) -> bool {
    cells.iter().any(|cell| is_placeholder(&cell.symbol))
}

fn has_placeholder(frame: &FrameData) -> bool {
    cells_have_placeholder(&frame.cells)
}

/// Collects intact blocks into `intact` (marking their cells `skip`) when it
/// is given, and writes fallback text into every other placeholder cell.
fn present_blocks(frame: &mut FrameData, mut intact: Option<&mut Vec<HostBlock>>) {
    let width = usize::from(frame.width);
    if width == 0 {
        return;
    }
    for index in 0..frame.cells.len() {
        let cell = &frame.cells[index];
        if cell.skip || !is_placeholder(&cell.symbol) {
            continue;
        }
        let Some(Placeholder {
            row: 0,
            col: 0,
            nonce,
            meta: Some(meta),
        }) = decode(&cell.symbol)
        else {
            continue;
        };
        let style = (cell.fg, cell.bg, cell.modifier);
        let (Ok(x), Ok(y)) = (u16::try_from(index % width), u16::try_from(index / width)) else {
            continue;
        };
        match intact.as_deref_mut() {
            Some(blocks) if block_is_intact(frame, x, y, nonce, &meta) => {
                for (dr, dc) in block_cells(&meta) {
                    if let Some(index) = cell_index(frame, x + dc, y + dr) {
                        frame.cells[index].skip = true;
                    }
                }
                blocks.push(HostBlock {
                    x,
                    y,
                    nonce,
                    meta,
                    style,
                });
            }
            _ => write_fallback_top_row(frame, x, y, nonce, &meta),
        }
    }
    for cell in &mut frame.cells {
        if !cell.skip && is_placeholder(&cell.symbol) {
            cell.symbol.clear();
            cell.symbol.push(' ');
        }
    }
}

fn block_cells(meta: &BlockMeta) -> impl Iterator<Item = (u16, u16)> {
    let columns = meta.columns();
    (0..meta.rows()).flat_map(move |dr| (0..columns).map(move |dc| (dr, dc)))
}

fn cell_index(frame: &FrameData, x: u16, y: u16) -> Option<usize> {
    (x < frame.width && y < frame.height)
        .then(|| usize::from(y) * usize::from(frame.width) + usize::from(x))
}

/// Returns whether the cell at `(x, y)` is the block's `(row, col)` cell.
fn is_block_cell(frame: &FrameData, x: u16, y: u16, nonce: u16, row: u16, col: u16) -> bool {
    cell_index(frame, x, y)
        .and_then(|index| decode(&frame.cells[index].symbol))
        .is_some_and(|cell| {
            cell.nonce == nonce && u16::from(cell.row) == row && u16::from(cell.col) == col
        })
}

fn block_is_intact(frame: &FrameData, x: u16, y: u16, nonce: u16, meta: &BlockMeta) -> bool {
    let fits = u32::from(x) + u32::from(meta.columns()) <= u32::from(frame.width)
        && u32::from(y) + u32::from(meta.rows()) <= u32::from(frame.height);
    fits && block_cells(meta)
        .skip(1)
        .all(|(dr, dc)| is_block_cell(frame, x + dc, y + dr, nonce, dr, dc))
}

/// Lays the block text unscaled across the block's top-row cells that are
/// still present; a grapheme is written only when all its cells are present.
fn write_fallback_top_row(frame: &mut FrameData, x: u16, y: u16, nonce: u16, meta: &BlockMeta) {
    let columns = meta.columns();
    let mut col = 0u16;
    for grapheme in meta.text.graphemes(true) {
        let Ok(width) = u16::try_from(grapheme.width()) else {
            break;
        };
        if width == 0 {
            continue;
        }
        if col + width > columns {
            break;
        }
        let present = (col..col + width)
            .all(|dc| is_block_cell(frame, x.saturating_add(dc), y, nonce, 0, dc));
        if present {
            if let Some(index) = cell_index(frame, x.saturating_add(col), y) {
                let symbol = &mut frame.cells[index].symbol;
                symbol.clear();
                symbol.push_str(grapheme);
            }
        }
        col += width;
    }
}

fn erase_block(out: &mut Vec<u8>, block: &HostBlock, width: u16, height: u16) {
    if block.x >= width {
        return;
    }
    let count = block.meta.columns().min(width - block.x);
    for dr in 0..block.meta.rows() {
        let y = block.y.saturating_add(dr);
        if y >= height {
            break;
        }
        let _ = write!(out, "\x1b[{};{}H\x1b[{count}X", y + 1, block.x + 1);
    }
}

fn draw_block(out: &mut Vec<u8>, block: &HostBlock) {
    let meta = &block.meta;
    let (fg, bg, modifier) = block.style;
    let _ = write!(
        out,
        "\x1b[{};{}H{}\x1b]66;s={}:w={}",
        block.y + 1,
        block.x + 1,
        render_ansi::build_sgr(fg, bg, modifier),
        meta.scale,
        meta.width
    );
    if meta.numerator != 0 || meta.denominator != 0 {
        let _ = write!(out, ":n={}:d={}", meta.numerator, meta.denominator);
    }
    if meta.vertical != 0 {
        let _ = write!(out, ":v={}", meta.vertical);
    }
    if meta.horizontal != 0 {
        let _ = write!(out, ":h={}", meta.horizontal);
    }
    out.push(b';');
    // Decoding already rejects control characters; never let one end or
    // escape the OSC payload.
    for ch in meta.text.chars().filter(|ch| !ch.is_control()) {
        let mut utf8 = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
    }
    out.extend_from_slice(b"\x1b\\");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::CellData;
    use crate::text_sizing::cell::{encode_cell, BlockMeta};

    const STYLE_FG: u32 = 0x0200_ff00;

    fn blank_frame(width: u16, height: u16) -> FrameData {
        let cell = CellData {
            symbol: " ".to_string(),
            fg: 0,
            bg: 0,
            modifier: 0,
            skip: false,
            hyperlink: None,
        };
        FrameData {
            cells: vec![cell; usize::from(width) * usize::from(height)],
            width,
            height,
            cursor: None,
            hyperlinks: Vec::new(),
            graphics: Vec::new(),
        }
    }

    fn meta(scale: u8, width: u8, text: &str) -> BlockMeta {
        BlockMeta {
            scale,
            width,
            numerator: 0,
            denominator: 0,
            vertical: 0,
            horizontal: 0,
            text: text.to_string(),
        }
    }

    fn cell_mut(frame: &mut FrameData, x: u16, y: u16) -> &mut CellData {
        let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
        &mut frame.cells[index]
    }

    fn symbol(frame: &FrameData, x: u16, y: u16) -> &str {
        let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
        &frame.cells[index].symbol
    }

    /// Writes the block's placeholder cells; cells outside the frame are dropped.
    fn place(frame: &mut FrameData, x: u16, y: u16, nonce: u16, block: &BlockMeta) {
        for dr in 0..block.rows() {
            for dc in 0..block.columns() {
                let (cx, cy) = (x + dc, y + dr);
                if cx >= frame.width || cy >= frame.height {
                    continue;
                }
                let origin = (dr, dc) == (0, 0);
                let cell = cell_mut(frame, cx, cy);
                cell.symbol = encode_cell(dr as u8, dc as u8, nonce, origin.then_some(block));
                cell.fg = STYLE_FG;
            }
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// Presents `frame` like the client does and returns the frame and bytes.
    fn present(
        presenter: &mut Presenter,
        mut frame: FrameData,
        host_supported: bool,
        full: bool,
    ) -> (FrameData, String, String) {
        let pass = presenter.prepare(&mut frame, host_supported);
        let (pre, post) = presenter.finish(&pass, full);
        presenter.commit(pass);
        (frame, text(&pre), text(&post))
    }

    #[test]
    fn intact_block_is_skipped_and_drawn_after_the_frame() {
        let mut presenter = Presenter::default();
        let mut frame = blank_frame(6, 4);
        place(&mut frame, 1, 1, 7, &meta(2, 1, "H"));

        let (frame, pre, post) = present(&mut presenter, frame, true, false);

        for (x, y) in [(1u16, 1u16), (2, 1), (1, 2), (2, 2)] {
            let index = usize::from(y) * 6 + usize::from(x);
            assert!(frame.cells[index].skip, "cell ({x},{y}) not skipped");
        }
        assert!(!frame.cells[0].skip);
        assert_eq!(pre, "");
        assert_eq!(
            post,
            "\x1b7\x1b[2;2H\x1b[0;38;2;0;255;0;49m\x1b]66;s=2:w=1;H\x1b\\\x1b8"
        );
    }

    #[test]
    fn incomplete_blocks_fall_back_to_unscaled_text() {
        let block = meta(2, 2, "Hi");
        let mut missing = blank_frame(8, 4);
        place(&mut missing, 1, 1, 7, &block);
        cell_mut(&mut missing, 4, 2).symbol = "x".to_string();
        let mut clipped = blank_frame(8, 4);
        place(&mut clipped, 5, 2, 7, &block);
        let mut covered = blank_frame(8, 4);
        place(&mut covered, 1, 1, 7, &block);
        cell_mut(&mut covered, 2, 2).symbol = "#".to_string();

        for (name, frame, x, y) in [
            ("missing", missing, 1, 1),
            ("clipped", clipped, 5, 2),
            ("covered", covered, 1, 1),
        ] {
            let mut presenter = Presenter::default();
            let (frame, pre, post) = present(&mut presenter, frame, true, false);
            assert_eq!((pre.as_str(), post.as_str()), ("", ""), "{name}");
            assert!(frame.cells.iter().all(|cell| !cell.skip), "{name}");
            assert_eq!(symbol(&frame, x, y), "H", "{name}");
            assert_eq!(symbol(&frame, x + 1, y), "i", "{name}");
            assert_eq!(symbol(&frame, x + 2, y), " ", "{name}");
            assert!(
                frame
                    .cells
                    .iter()
                    .all(|cell| !crate::text_sizing::cell::is_placeholder(&cell.symbol)),
                "{name}"
            );
        }
    }

    #[test]
    fn moved_or_removed_block_is_erased_before_the_frame() {
        let block = meta(2, 1, "H");
        let mut presenter = Presenter::default();
        let mut first = blank_frame(8, 5);
        place(&mut first, 1, 1, 7, &block);
        present(&mut presenter, first, true, false);

        let mut moved = blank_frame(8, 5);
        place(&mut moved, 4, 2, 7, &block);
        let (_, pre, post) = present(&mut presenter, moved, true, false);
        assert_eq!(pre, "\x1b[0m\x1b[2;2H\x1b[2X\x1b[3;2H\x1b[2X");
        assert!(post.contains("\x1b[3;5H"), "{post:?}");
        assert!(post.contains("\x1b]66;s=2:w=1;H\x1b\\"), "{post:?}");

        let (_, pre, post) = present(&mut presenter, blank_frame(8, 5), true, false);
        assert_eq!(pre, "\x1b[0m\x1b[3;5H\x1b[2X\x1b[4;5H\x1b[2X");
        assert_eq!(post, "");
    }

    #[test]
    fn unchanged_block_writes_nothing_until_a_full_redraw() {
        let block = meta(2, 1, "H");
        let mut presenter = Presenter::default();
        let mut frame = blank_frame(8, 5);
        place(&mut frame, 1, 1, 7, &block);
        present(&mut presenter, frame.clone(), true, false);

        let (_, pre, post) = present(&mut presenter, frame.clone(), true, false);
        assert_eq!((pre.as_str(), post.as_str()), ("", ""));

        let (_, pre, post) = present(&mut presenter, frame, true, true);
        assert_eq!(pre, "\x1b[0m\x1b[2;2H\x1b[2X\x1b[3;2H\x1b[2X");
        assert!(post.contains("\x1b[2;2H"), "{post:?}");
        assert!(post.contains("\x1b]66;s=2:w=1;H\x1b\\"), "{post:?}");

        // A full redraw at a smaller size clamps erases to the new frame.
        let (_, pre, _) = present(&mut presenter, blank_frame(2, 2), true, true);
        assert_eq!(pre, "\x1b[0m\x1b[2;2H\x1b[1X");
    }

    #[test]
    fn forged_control_text_never_reaches_the_host() {
        // A 1x7 block whose top-left text is `ESC ] 52 ; c ; Q BEL ESC \ CSI(C1)`.
        let mut origin = String::from(crate::text_sizing::cell::BASE);
        let payload = [0x1b, 93, 53, 50, 59, 99, 59, 81, 0x07, 0x1b, 92, 121, 35];
        let marks = [0, 0, 0, 7, 15, 0, 0, 0].into_iter().chain(payload);
        origin.extend(marks.filter_map(|value| char::from_u32(0xE0100 + value)));
        let mut frame = blank_frame(8, 1);
        place(&mut frame, 0, 0, 7, &meta(1, 7, "x"));
        cell_mut(&mut frame, 0, 0).symbol = origin;

        let mut presenter = Presenter::default();
        let (supported, pre, post) = present(&mut presenter, frame.clone(), true, true);
        assert_eq!((pre.as_str(), post.as_str()), ("", ""));
        let mut fallback = frame;
        apply_fallback(&mut fallback);
        for frame in [supported, fallback] {
            for cell in &frame.cells {
                assert!(!cell.skip);
                assert!(!cell.symbol.chars().any(char::is_control), "{cell:?}");
                assert!(!is_placeholder(&cell.symbol), "{cell:?}");
            }
        }

        // The payload writer drops control characters on its own.
        let block = HostBlock {
            x: 0,
            y: 0,
            nonce: 7,
            meta: meta(1, 1, "a\u{1b}\\\u{7}b\u{9b}"),
            style: (0, 0, 0),
        };
        let mut out = Vec::new();
        draw_block(&mut out, &block);
        let expected = format!(
            "\x1b[1;1H{}\x1b]66;s=1:w=1;a\\b\x1b\\",
            render_ansi::build_sgr(0, 0, 0)
        );
        assert_eq!(text(&out), expected);
    }

    #[test]
    fn unsupported_host_never_skips_or_emits_sized_text() {
        let mut presenter = Presenter::default();
        let mut frame = blank_frame(6, 4);
        place(&mut frame, 1, 1, 7, &meta(2, 1, "H"));

        let (frame, pre, post) = present(&mut presenter, frame, false, true);

        assert!(frame.cells.iter().all(|cell| !cell.skip));
        assert_eq!((pre.as_str(), post.as_str()), ("", ""));
        assert_eq!(symbol(&frame, 1, 1), "H");
        assert_eq!(symbol(&frame, 2, 1), " ");
        assert_eq!(symbol(&frame, 1, 2), " ");
    }

    #[test]
    fn frames_without_placeholders_take_the_allocation_free_path() {
        let mut presenter = Presenter::default();
        let mut frame = blank_frame(6, 4);
        cell_mut(&mut frame, 2, 1).symbol = "界".to_string();
        let original = frame.clone();

        let pass = presenter.prepare(&mut frame, true);
        let (pre, post) = presenter.finish(&pass, true);
        assert_eq!(pass.blocks.capacity(), 0);
        assert_eq!((pre.capacity(), post.capacity()), (0, 0));
        let mut bytes = b"frame".to_vec();
        presenter.write_into(&pass, true, &mut bytes);
        assert_eq!(bytes, b"frame");
        presenter.commit(pass);
        assert!(!presenter.has_host_blocks());

        apply_fallback(&mut frame);
        assert_eq!(frame, original);
    }

    /// A fixed 146x32 frame composed from one pane, or from a 5x3 grid of
    /// pane regions separated by borders.
    fn composed_frame(panes: usize) -> FrameData {
        let (width, height) = (146u16, 32u16);
        let mut frame = blank_frame(width, height);
        let line: Vec<String> = "fn render(frame: &mut Frame) -> 界面 { draw(); } // ok "
            .graphemes(true)
            .map(str::to_string)
            .collect();
        for y in 0..height {
            for x in 0..width {
                let border = panes > 1 && (x % 29 == 28 || y % 11 == 10);
                let symbol = if border {
                    if y % 11 == 10 { "─" } else { "│" }.to_string()
                } else {
                    line[(usize::from(x) + usize::from(y) * 7) % line.len()].clone()
                };
                let cell = cell_mut(&mut frame, x, y);
                cell.symbol = symbol;
                cell.fg = u32::from(x % 8);
            }
        }
        frame
    }

    fn percentiles(mut samples: Vec<u128>) -> (u128, u128) {
        samples.sort_unstable();
        (
            samples[samples.len() / 2],
            samples[samples.len() * 95 / 100],
        )
    }

    #[test]
    #[ignore = "non-gating release render scaling profile"]
    fn text_sizing_present_render_scale_profile() {
        const SAMPLES: usize = 400;
        for panes in [1, 15] {
            let plain = composed_frame(panes);
            let mut sized = plain.clone();
            for (x, y, block) in [
                (2, 1, meta(2, 2, "Hi")),
                (40, 4, meta(3, 1, "X")),
                (90, 20, meta(2, 1, "界")),
            ] {
                place(&mut sized, x, y, 1 + x, &block);
            }
            let cells = plain.cells.len();

            for (label, template) in [("none", &plain), ("intact_blocks", &sized)] {
                let fast_path = label == "none";
                let mut presenter = Presenter::default();
                // Settle the host state so every sample is a steady-state frame.
                let pass = presenter.prepare(&mut template.clone(), true);
                presenter.commit(pass);
                let mut samples = Vec::with_capacity(SAMPLES);
                for _ in 0..SAMPLES + 20 {
                    let mut frame = template.clone();
                    let started = std::time::Instant::now();
                    let pass = presenter.prepare(&mut frame, true);
                    let bytes = presenter.finish(&pass, false);
                    presenter.commit(pass);
                    samples.push(started.elapsed().as_nanos());
                    if fast_path {
                        assert_eq!(frame, plain, "fast path must not touch the frame");
                        assert_eq!((bytes.0.capacity(), bytes.1.capacity()), (0, 0));
                    }
                    std::hint::black_box((frame, bytes));
                }
                let (median, p95) = percentiles(samples.split_off(20));
                eprintln!(
                    "text_sizing presenter panes={panes} cells={cells} blocks={label} \
                     median_ns={median} p95_ns={p95}"
                );
            }

            let mut samples = Vec::with_capacity(SAMPLES);
            for _ in 0..SAMPLES + 20 {
                let mut frame = plain.clone();
                let started = std::time::Instant::now();
                apply_fallback(&mut frame);
                samples.push(started.elapsed().as_nanos());
                assert_eq!(frame, plain, "fast path must not touch the frame");
                std::hint::black_box(frame);
            }
            let (median, p95) = percentiles(samples.split_off(20));
            eprintln!(
                "text_sizing apply_fallback panes={panes} cells={cells} placeholders=none \
                 median_ns={median} p95_ns={p95}"
            );
        }
    }
}
