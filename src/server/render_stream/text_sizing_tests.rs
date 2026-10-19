use super::*;

#[test]
fn text_sizing_direct_terminal_frames_carry_fallback_text_only() {
    use crate::text_sizing::cell::{encode_cell, BlockMeta, BASE};

    let block = BlockMeta {
        scale: 2,
        width: 1,
        numerator: 0,
        denominator: 0,
        vertical: 0,
        horizontal: 0,
        text: "H".to_string(),
    };
    let buffer = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 6, 2));
    let mut frame = FrameData::from_ratatui_buffer(&buffer, None);
    for (row, col) in [(0u8, 0u8), (0, 1), (1, 0), (1, 1)] {
        let index = usize::from(row) * 6 + 1 + usize::from(col);
        let origin = (row, col) == (0, 0);
        frame.cells[index].symbol = encode_cell(row, col, 7, origin.then_some(&block));
    }

    let mut state = ClientRenderState::new(RenderEncoding::TerminalAnsi);
    let Some(PreparedRender::TerminalAnsi {
        message: ServerMessage::Terminal(terminal),
        frame,
        ..
    }) = state.prepare_frame(frame)
    else {
        panic!("expected a terminal frame");
    };
    assert_eq!(frame.cells[1].symbol, "H");
    let bytes = String::from_utf8_lossy(&terminal.bytes);
    assert!(bytes.contains("H "), "{bytes:?}");
    assert!(!bytes.contains(BASE), "{bytes:?}");
    assert!(!bytes.contains("\x1b]66;"), "{bytes:?}");
}
