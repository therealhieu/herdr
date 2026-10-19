use super::*;
use tokio::sync::mpsc;

fn text_sizing_pane(cols: u16, rows: u16) -> (GhosttyPaneTerminal, mpsc::Sender<Bytes>) {
    let (tx, _rx) = mpsc::channel(16);
    let terminal = crate::ghostty::Terminal::new(cols, rows, 0).unwrap();
    (GhosttyPaneTerminal::new(terminal, tx.clone()).unwrap(), tx)
}

/// Writes `bytes` followed by a cursor position report request and
/// returns the reported 1-based `(row, col)`.
fn text_sizing_write_and_report(
    pane: &GhosttyPaneTerminal,
    tx: &mpsc::Sender<Bytes>,
    bytes: &[u8],
) -> (u16, u16) {
    let mut input = bytes.to_vec();
    input.extend_from_slice(b"\x1b[6n");
    let result = pane.process_pty_bytes(PaneId::from_raw(1), 0, &input, tx);
    let report = result
        .terminal_responses
        .last()
        .map(|reply| String::from_utf8_lossy(reply).into_owned())
        .unwrap_or_default();
    let body = report
        .strip_prefix("\x1b[")
        .and_then(|rest| rest.strip_suffix('R'))
        .unwrap_or_else(|| panic!("unexpected report {report:?}"));
    let (row, col) = body.split_once(';').unwrap();
    (row.parse().unwrap(), col.parse().unwrap())
}

/// Decoded placeholder cells keyed by `(row, col)` on the active screen.
fn text_sizing_cells(
    pane: &GhosttyPaneTerminal,
) -> std::collections::BTreeMap<(usize, usize), crate::text_sizing::cell::Placeholder> {
    let core = pane.core.lock().unwrap();
    let rows = core.terminal.screen_text_rows().unwrap();
    let mut cells = std::collections::BTreeMap::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.cells.iter().enumerate() {
            let grapheme: String = cell
                .graphemes
                .iter()
                .filter_map(|&cp| char::from_u32(cp))
                .collect();
            if crate::text_sizing::cell::is_placeholder(&grapheme) {
                let decoded = crate::text_sizing::cell::decode(&grapheme)
                    .unwrap_or_else(|| panic!("malformed placeholder at {x},{y}"));
                cells.insert((y, x), decoded);
            }
        }
    }
    cells
}

/// Top-left `(row, col, text)` of every block whose cells are all present.
fn text_sizing_intact_blocks(pane: &GhosttyPaneTerminal) -> Vec<(usize, usize, String)> {
    let cells = text_sizing_cells(pane);
    cells
        .iter()
        .filter_map(|(&(y, x), cell)| {
            let meta = cell.meta.as_ref()?;
            let intact = (0..usize::from(meta.rows())).all(|dr| {
                (0..usize::from(meta.columns())).all(|dc| {
                    cells.get(&(y + dr, x + dc)).is_some_and(|other| {
                        other.nonce == cell.nonce
                            && usize::from(other.row) == dr
                            && usize::from(other.col) == dc
                    })
                })
            });
            intact.then(|| (y, x, meta.text.clone()))
        })
        .collect()
}

#[test]
fn text_sizing_cursor_matches_kitty_probe() {
    let _active = crate::text_sizing::set_active_for_test(true);
    for (probe, expected_col) in [
        (&b"\r\x1b]66;w=2; \x07"[..], 3),
        (&b"\r\x1b]66;s=2; \x07"[..], 3),
        (&b"\r\x1b]66;w=1; \x07"[..], 2),
        (&b"\r\x1b]66;s=2;\x07"[..], 1),
    ] {
        let (pane, tx) = text_sizing_pane(20, 5);
        assert_eq!(
            text_sizing_write_and_report(&pane, &tx, probe),
            (1, expected_col),
            "probe {probe:?}"
        );
    }
}

#[test]
fn text_sizing_inactive_leaves_cursor_unchanged() {
    let _inactive = crate::text_sizing::set_active_for_test(false);
    for probe in [&b"\r\x1b]66;w=2; \x07"[..], &b"\r\x1b]66;s=2;H\x07"[..]] {
        let (pane, tx) = text_sizing_pane(20, 5);
        assert_eq!(text_sizing_write_and_report(&pane, &tx, probe), (1, 1));
        assert!(text_sizing_cells(&pane).is_empty());
    }
}

#[test]
fn text_sizing_places_placeholder_cells_with_and_without_grapheme_clustering() {
    let _active = crate::text_sizing::set_active_for_test(true);
    for clustering in [true, false] {
        let (pane, tx) = text_sizing_pane(10, 4);
        pane.core
            .lock()
            .unwrap()
            .terminal
            .mode_set(crate::ghostty::MODE_GRAPHEME_CLUSTER, clustering)
            .unwrap();
        let report = text_sizing_write_and_report(&pane, &tx, b"ab\x1b]66;s=2:w=1;H\x07");
        assert_eq!(report, (1, 5), "clustering {clustering}");
        assert_eq!(
            text_sizing_intact_blocks(&pane),
            vec![(0, 2, "H".to_string())],
            "clustering {clustering}"
        );
        let positions: Vec<_> = text_sizing_cells(&pane).into_keys().collect();
        assert_eq!(positions, vec![(0, 2), (0, 3), (1, 2), (1, 3)]);
    }
}

#[test]
fn text_sizing_block_on_last_row_scrolls_to_fit() {
    let _active = crate::text_sizing::set_active_for_test(true);
    let (pane, tx) = text_sizing_pane(10, 3);
    let report = text_sizing_write_and_report(&pane, &tx, b"top\r\n\r\n\x1b]66;s=2;H\x07");
    assert_eq!(report, (2, 3));
    assert_eq!(
        text_sizing_intact_blocks(&pane),
        vec![(1, 0, "H".to_string())]
    );
}

#[test]
fn text_sizing_discards_blocks_larger_than_the_screen() {
    let _active = crate::text_sizing::set_active_for_test(true);
    let (pane, tx) = text_sizing_pane(10, 3);
    let report = text_sizing_write_and_report(&pane, &tx, b"ab\x1b]66;w=7:s=7;H\x07");
    assert_eq!(report, (1, 3));
    assert!(text_sizing_cells(&pane).is_empty());
}

#[test]
fn text_sizing_backs_up_when_autowrap_is_off() {
    let _active = crate::text_sizing::set_active_for_test(true);
    for clustering in [true, false] {
        let (pane, tx) = text_sizing_pane(10, 3);
        pane.core
            .lock()
            .unwrap()
            .terminal
            .mode_set(crate::ghostty::MODE_GRAPHEME_CLUSTER, clustering)
            .unwrap();
        let report = text_sizing_write_and_report(&pane, &tx, b"\x1b[?7labcdefgh\x1b]66;w=3;H\x07");
        assert_eq!(report, (1, 10), "clustering {clustering}");
        assert_eq!(
            text_sizing_intact_blocks(&pane),
            vec![(0, 7, "H".to_string())],
            "clustering {clustering}"
        );
        // Autowrap stays off: the next character overwrites the last column.
        let report = text_sizing_write_and_report(&pane, &tx, b"Z");
        assert_eq!(report, (1, 10), "clustering {clustering}");
        assert!(text_sizing_intact_blocks(&pane).is_empty());
    }
}

#[test]
fn text_sizing_wraps_when_the_block_does_not_fit() {
    let _active = crate::text_sizing::set_active_for_test(true);
    let (pane, tx) = text_sizing_pane(10, 3);
    let report = text_sizing_write_and_report(&pane, &tx, b"abcdefghi\x1b]66;w=2;H\x07");
    assert_eq!(report, (2, 3));
    assert_eq!(
        text_sizing_intact_blocks(&pane),
        vec![(1, 0, "H".to_string())]
    );
}

#[test]
fn text_sizing_overwriting_or_erasing_breaks_blocks() {
    let _active = crate::text_sizing::set_active_for_test(true);
    let (pane, tx) = text_sizing_pane(10, 4);
    text_sizing_write_and_report(&pane, &tx, b"\x1b]66;s=2;H\x07");
    assert_eq!(text_sizing_intact_blocks(&pane).len(), 1);
    text_sizing_write_and_report(&pane, &tx, b"\x1b[2;2Hx");
    assert!(text_sizing_intact_blocks(&pane).is_empty());

    let (pane, tx) = text_sizing_pane(10, 4);
    text_sizing_write_and_report(&pane, &tx, b"\x1b]66;s=2;H\x07");
    text_sizing_write_and_report(&pane, &tx, b"\x1b[1;1H\x1b[1X");
    assert!(text_sizing_intact_blocks(&pane).is_empty());
}

#[tokio::test]
async fn text_sizing_blocks_survive_history_snapshot_and_restore() {
    let _active = crate::text_sizing::set_active_for_test(true);
    // 18 four-byte emoji fill the top-left cell to 62 of its 64 marks.
    let long_text = "\u{1f600}".repeat(18);
    let input = format!("ab\x1b]66;s=2;Hi\x07\x1b]66;w=2:n=1:d=2;{long_text}\x07");
    let (source, _source_rx) = crate::pane::PaneRuntime::test_with_channel(20, 5);
    source.test_process_pty_bytes(input.as_bytes());

    let expected_blocks = vec![
        (0, 2, "H".to_string()),
        (0, 4, "i".to_string()),
        (0, 6, long_text.clone()),
    ];
    let source_cells = text_sizing_cells(&source.terminal.ghostty);
    assert_eq!(source_cells.len(), 10);
    assert_eq!(
        text_sizing_intact_blocks(&source.terminal.ghostty),
        expected_blocks
    );

    // Persistence stores raw placeholders so a restore can rebuild blocks.
    let history = source.snapshot_history().unwrap();
    assert!(history.contains(crate::text_sizing::cell::BASE));
    assert!(!history.contains(long_text.as_str()));

    let (restored, _restored_rx) = crate::pane::PaneRuntime::test_with_channel(20, 5);
    restored.terminal.ghostty.seed_history_ansi(&history);

    assert_eq!(text_sizing_cells(&restored.terminal.ghostty), source_cells);
    assert_eq!(
        text_sizing_intact_blocks(&restored.terminal.ghostty),
        expected_blocks
    );
    let decoded_line = format!("abHi{long_text}");
    for text in [
        restored.visible_text(),
        restored.recent_unwrapped_text_snapshot(usize::MAX).text,
    ] {
        assert_eq!(text.lines().next(), Some(decoded_line.as_str()));
        assert!(!text.contains(crate::text_sizing::cell::BASE));
    }
}

#[test]
fn text_sizing_partial_command_does_not_survive_an_inactive_period() {
    let chunks: [&[u8]; 3] = [b"\r\x1b]66;s=2;Hel", b"lo\x07abc", b"def\x07"];
    let feed = |pane: &GhosttyPaneTerminal, tx: &mpsc::Sender<Bytes>, chunk: &[u8]| {
        pane.process_pty_bytes(PaneId::from_raw(1), 0, chunk, tx);
    };

    let (reference, reference_tx) = text_sizing_pane(20, 5);
    let expected = {
        let _inactive = crate::text_sizing::set_active_for_test(false);
        feed(&reference, &reference_tx, chunks[0]);
        feed(&reference, &reference_tx, chunks[1]);
        text_sizing_write_and_report(&reference, &reference_tx, chunks[2])
    };

    let (pane, tx) = text_sizing_pane(20, 5);
    {
        let _active = crate::text_sizing::set_active_for_test(true);
        feed(&pane, &tx, chunks[0]);
    }
    {
        let _inactive = crate::text_sizing::set_active_for_test(false);
        feed(&pane, &tx, chunks[1]);
    }
    let _active = crate::text_sizing::set_active_for_test(true);
    assert_eq!(
        text_sizing_write_and_report(&pane, &tx, chunks[2]),
        expected
    );
    assert!(text_sizing_cells(&pane).is_empty());
}

#[test]
fn text_sizing_unsized_commands_write_plain_text() {
    let _active = crate::text_sizing::set_active_for_test(true);
    let first_row = |pane: &GhosttyPaneTerminal| -> String {
        let core = pane.core.lock().unwrap();
        let rows = core.terminal.screen_text_rows().unwrap();
        rows[0]
            .cells
            .iter()
            .flat_map(|cell| cell.graphemes.iter().filter_map(|&cp| char::from_u32(cp)))
            .collect()
    };
    for (probe, expected_col, text) in [
        ("\x1b]66;w=1;\u{223c}\x07", 2, "\u{223c}"),
        ("\x1b]66;w=1;\u{2502}\x07", 2, "\u{2502}"),
        ("\x1b]66;;ab\x07", 3, "ab"),
        ("\x1b]66;s=1:w=0;ab\x07", 3, "ab"),
    ] {
        let (pane, tx) = text_sizing_pane(10, 4);
        let report = text_sizing_write_and_report(&pane, &tx, probe.as_bytes());
        assert_eq!(report, (1, expected_col), "probe {probe:?}");
        assert!(text_sizing_cells(&pane).is_empty(), "probe {probe:?}");
        assert_eq!(first_row(&pane).trim_end(), text, "probe {probe:?}");
    }

    for (probe, expected_col, expected_blocks) in [
        (
            "\x1b]66;w=1;\u{6f22}\x07",
            2,
            vec![(0, 0, "\u{6f22}".to_string())],
        ),
        ("\x1b]66;w=1;ab\x07", 2, vec![(0, 0, "ab".to_string())]),
        (
            "\x1b]66;n=1:d=2:w=1;a\x07",
            2,
            vec![(0, 0, "a".to_string())],
        ),
        (
            "\x1b]66;n=1:d=2;ab\x07",
            3,
            vec![(0, 0, "a".to_string()), (0, 1, "b".to_string())],
        ),
        (
            "\x1b]66;s=2;ab\x07",
            5,
            vec![(0, 0, "a".to_string()), (0, 2, "b".to_string())],
        ),
    ] {
        let (pane, tx) = text_sizing_pane(10, 4);
        let report = text_sizing_write_and_report(&pane, &tx, probe.as_bytes());
        assert_eq!(report, (1, expected_col), "probe {probe:?}");
        assert_eq!(
            text_sizing_intact_blocks(&pane),
            expected_blocks,
            "probe {probe:?}"
        );
    }
}
