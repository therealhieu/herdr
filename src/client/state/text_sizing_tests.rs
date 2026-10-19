use super::*;
use crate::protocol::{CellData, FrameData, PaneSurfacePatchRow};
use crate::text_sizing::cell::{encode_cell, BlockMeta};

/// Sets the process-wide host support flag and clears it on drop.
struct HostSupported;

impl HostSupported {
    fn enable() -> Self {
        crate::text_sizing::set_host_supported(true);
        Self
    }
}

impl Drop for HostSupported {
    fn drop(&mut self) {
        crate::text_sizing::set_host_supported(false);
    }
}

fn cell(symbol: String) -> CellData {
    CellData {
        symbol,
        fg: 0,
        bg: 0,
        modifier: 0,
        skip: false,
        hyperlink: None,
    }
}

/// A 4x3 frame with a 2x2 block (`s=2`, text `H`) at column 1 when `block`.
fn frame(block: bool) -> FrameData {
    let meta = BlockMeta {
        scale: 2,
        width: 1,
        numerator: 0,
        denominator: 0,
        vertical: 0,
        horizontal: 0,
        text: "H".to_string(),
    };
    let mut cells = vec![cell(" ".to_string()); 12];
    if block {
        for (dr, dc) in [(0u8, 0u8), (0, 1), (1, 0), (1, 1)] {
            let origin = (dr, dc) == (0, 0);
            cells[usize::from(dr) * 4 + usize::from(dc) + 1] =
                cell(encode_cell(dr, dc, 3, origin.then_some(&meta)));
        }
    }
    FrameData {
        cells,
        width: 4,
        height: 3,
        cursor: None,
        hyperlinks: Vec::new(),
        graphics: Vec::new(),
    }
}

#[test]
fn text_sizing_client_writes_block_bytes_inside_the_synchronized_frame() {
    let _host = HostSupported::enable();
    let mut state = ClientState::test_new();
    let mut output = Vec::new();
    assert!(state.try_present_frame_to(&mut output, frame(true)));
    let text = String::from_utf8_lossy(&output).into_owned();
    let draw = text.find("\x1b]66;s=2:w=1;H\x1b\\").expect("block drawn");
    let sync_end = text.rfind("\x1b[?2026l").expect("sync end");
    assert!(draw < sync_end, "{text:?}");
    assert!(!text.contains('\u{10EEED}'), "{text:?}");

    // A failed write keeps the drawn block so a later frame still erases it.
    assert!(!state.try_present_frame_to(&mut &mut [][..], frame(false)));
    let mut output = Vec::new();
    assert!(state.try_present_frame_to(&mut output, frame(false)));
    let text = String::from_utf8_lossy(&output).into_owned();
    assert!(
        text.starts_with("\x1b[?2026h\x1b[0m\x1b[1;2H\x1b[2X\x1b[2;2H\x1b[2X"),
        "{text:?}"
    );
    assert!(!text.contains("\x1b]66;"), "{text:?}");
}

#[test]
fn text_sizing_client_patch_falls_back_while_blocks_exist() {
    let _host = HostSupported::enable();
    let mut state = ClientState::test_new();
    assert!(state.try_present_frame_to(&mut Vec::new(), frame(true)));
    let patch = |symbol: &str| shell::ClientComposedSurfacePatch {
        rows: vec![PaneSurfacePatchRow {
            x: 0,
            y: 2,
            cells: vec![cell(symbol.to_string())],
        }],
        cursor: None,
    };
    assert!(!state.present_surface_patch(patch("x")).expect("patch"));

    assert!(state.try_present_frame_to(&mut Vec::new(), frame(false)));
    let placeholder = encode_cell(1, 0, 9, None);
    assert!(!state
        .present_surface_patch(patch(&placeholder))
        .expect("patch"));
}
