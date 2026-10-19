//! Client-side bold text recoloring, mirroring Ghostty's `bold-color`.
//!
//! Runs on composed cells right before the blit encoder, so the encoder's committed frame and
//! its diffs already carry the recolored values.

use crate::config::BoldColorConfig;
use crate::protocol::CellData;

const BOLD: u16 = ratatui::style::Modifier::BOLD.bits();
const NAMED: u32 = 0x00;
const INDEXED: u32 = 0x01;
const RGB: u32 = 0x02;
/// Distance from a base palette color (0-7) to its bright variant (8-15). Named colors are
/// packed one above their palette index, so the offset is the same for both encodings.
const BRIGHT_OFFSET: u32 = 8;

/// Recolors the foreground of bold cells: default fg becomes the configured RGB color
/// (`Rgb` only), and palette colors 0-7 become their bright variants 8-15.
pub(super) fn apply(cells: &mut [CellData], rule: BoldColorConfig) {
    let default_fg = match rule {
        BoldColorConfig::Bright => None,
        BoldColorConfig::Rgb(r, g, b) => {
            Some((RGB << 24) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
        }
    };
    for cell in cells.iter_mut().filter(|cell| cell.modifier & BOLD != 0) {
        if let Some(fg) = bold_fg(cell.fg, default_fg) {
            cell.fg = fg;
        }
    }
}

fn bold_fg(fg: u32, default_fg: Option<u32>) -> Option<u32> {
    let low = fg & 0xFF;
    match fg >> 24 {
        // Named: 0x00 is the default color, 0x01-0x08 are palette 0-7.
        NAMED if low == 0x00 => default_fg,
        NAMED if low <= 0x08 => Some(fg + BRIGHT_OFFSET),
        INDEXED if low < 8 => Some(fg + BRIGHT_OFFSET),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::protocol::{color_to_u32, FrameData, PaneSurfacePatchRow};
    use ratatui::style::{Color, Modifier};

    const YELLOW: BoldColorConfig = BoldColorConfig::Rgb(248, 222, 166);

    fn cell(fg: Color, modifier: Modifier) -> CellData {
        CellData {
            symbol: "B".into(),
            fg: color_to_u32(fg),
            bg: color_to_u32(Color::Reset),
            modifier: modifier.bits(),
            skip: false,
            hyperlink: None,
        }
    }

    fn frame(cells: Vec<CellData>) -> FrameData {
        FrameData {
            width: cells.len() as u16,
            height: 1,
            cells,
            cursor: None,
            hyperlinks: Vec::new(),
            graphics: Vec::new(),
        }
    }

    fn recolored(rule: BoldColorConfig, fg: Color, modifier: Modifier) -> u32 {
        let mut cells = [cell(fg, modifier)];
        apply(&mut cells, rule);
        cells[0].fg
    }

    #[test]
    fn bold_color_rule_matches_ghostty() {
        let bold = Modifier::BOLD;
        let cases = [
            (YELLOW, Color::Reset, bold, Color::Rgb(248, 222, 166)),
            (YELLOW, Color::Red, bold, Color::LightRed),
            (YELLOW, Color::Black, bold, Color::DarkGray),
            (YELLOW, Color::Gray, bold, Color::White),
            (YELLOW, Color::Indexed(1), bold, Color::Indexed(9)),
            (YELLOW, Color::Indexed(8), bold, Color::Indexed(8)),
            (YELLOW, Color::Indexed(200), bold, Color::Indexed(200)),
            (YELLOW, Color::Rgb(1, 2, 3), bold, Color::Rgb(1, 2, 3)),
            (YELLOW, Color::LightRed, bold, Color::LightRed),
            (YELLOW, Color::Reset, Modifier::ITALIC, Color::Reset),
            (YELLOW, Color::Red, Modifier::empty(), Color::Red),
            (BoldColorConfig::Bright, Color::Reset, bold, Color::Reset),
            (BoldColorConfig::Bright, Color::Red, bold, Color::LightRed),
            (
                BoldColorConfig::Bright,
                Color::Indexed(1),
                bold,
                Color::Indexed(9),
            ),
        ];
        for (rule, fg, modifier, expected) in cases {
            assert_eq!(
                recolored(rule, fg, modifier),
                color_to_u32(expected),
                "{rule:?} {fg:?} {modifier:?}"
            );
        }
    }

    #[test]
    fn bold_color_recolors_presented_frame() {
        let mut state = ClientState::test_new();
        state.bold_color = Some(YELLOW);
        let mut output = Vec::new();

        assert!(state
            .try_present_frame_to(&mut output, frame(vec![cell(Color::Reset, Modifier::BOLD)])));

        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("38;2;248;222;166"), "{output:?}");
    }

    #[test]
    fn bold_color_unset_keeps_presented_frame_bytes() {
        let frame = frame(vec![
            cell(Color::Reset, Modifier::BOLD),
            cell(Color::Red, Modifier::BOLD),
        ]);
        let baseline = render_ansi::BlitEncoder::new().encode(&frame, false).bytes;
        let mut state = ClientState::test_new();
        let mut output = Vec::new();

        assert!(state.try_present_frame_to(&mut output, frame));

        assert_eq!(output, baseline);
    }

    #[test]
    fn bold_color_recolors_surface_patch() {
        let mut state = ClientState::test_new();
        state.bold_color = Some(YELLOW);
        let mut output = Vec::new();
        assert!(state.try_present_frame_to(
            &mut output,
            frame(vec![cell(Color::Reset, Modifier::empty())])
        ));

        let patch = shell::ClientComposedSurfacePatch {
            rows: vec![PaneSurfacePatchRow {
                x: 0,
                y: 0,
                cells: vec![cell(Color::Reset, Modifier::BOLD)],
            }],
            cursor: None,
        };
        assert!(state.present_surface_patch(patch).expect("patch presented"));

        let expected = frame(vec![cell(Color::Rgb(248, 222, 166), Modifier::BOLD)]);
        assert!(state.blit_encoder.is_current(&expected));
    }
}
