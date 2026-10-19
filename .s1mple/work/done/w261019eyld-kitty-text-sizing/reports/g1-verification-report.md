# W261019EYLD G1 Verification Report

## Scope

- **Scope:** `G1`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`, `G1-T4`
- **Change range:** `c1e0a4a8..7d0e37e5`
- **Report:** `.s1mple/work/active/w261019eyld-kitty-text-sizing/reports/g1-verification-report.md`

## Status

`Failed`

Every assigned row is met and every check passes. One Minor finding (`G1-VER1`) needs remediation.

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1`, `REQ6` | `DES1` | `V1` | `src/text_sizing/cell.rs`: `BASE`, `BlockMeta`, `Placeholder`, `encode_cell`, `decode`, `is_placeholder`, `is_mark`. Marks are `U+E0100 + v` (`v < 240`), 4 position marks, 8-mark header, base-120 text digits, at most 64 marks (equal to Ghostty's 64 appended codepoints, `vendor/libghostty-vt/src/terminal/page.zig:92` and `:1540`), text capped at 24 codepoints. `DES4` state exists and compiles: `src/text_sizing.rs` `is_active`, `set_active_for_test`, `set_host_supported`/`host_supported`, `RegistryState`, `ClientRegistration` (wiring and tests belong to `G3-T1`). | `Met` |
| `REQ1`, `REQ2` | `DES2` | `V2` | `src/text_sizing/tracker.rs`: `TextSizingTracker::observe`/`drain_pending`, `TextSizingCommand { end_offset, params, text }`, `parse_metadata` (clamps `s` 1..7, `w` 0..7, `n`/`d` 0..15), `MAX_BODY_BYTES` cutoff to `SkipOsc`, `is_control` drops C0/C1, empty text yields no command. | `Met` |
| `REQ2`, `REQ3`, `REQ5`, `REQ7` | `DES2`, `DES7` | `V3` | `src/text_sizing/inject.rs`: `apply`, `writes_plain_text` (`A10` shortcuts), `place_block` (discard `s·w > cols` or `s > rows`, pending wrap as `cursor_x + 1`, `CR`+`IND` wrap or back up to `cols − s·w`, `IND`×(s−1) + `CUU` to make room, lower rows first, top row last, DECAWM toggle restored). `crates/ghostty-vt/src/lib.rs`: `cursor_x`, `cursor_pending_wrap`, `MODE_WRAPAROUND`. `src/pane/terminal.rs:1413` observes only when `text_sizing::is_active()` (one relaxed atomic load when inactive, no allocation); `OrderedColorOrC1Event::TextSizing` arm injects after the core consumes bytes up to `end_offset`, so a CPR later in the same write is answered from the moved cursor. No change to `PaneTerminal::render` or dirty-patch collection. | `Met` |
| `REQ6` | `DES6` | `V7` | `src/text_sizing/text.rs`: `decode_text` (returns input unchanged without `U+10EEED`). `src/pane.rs`: `visible_text`, `visible_ansi`, `recent_text_snapshot`, `recent_ansi_snapshot`, `recent_unwrapped_text_snapshot`, `recent_unwrapped_ansi_snapshot` (via `decode_snapshot_text`), `extract_selection`. `detection_text`, `recent_unwrapped_ansi`, and `snapshot_history` stay raw. `src/terminal/runtime.rs` and `src/app/api_helpers.rs` reach text through these `PaneRuntime` wrappers. | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `just test-one text_sizing::cell` | 5 tests run: 5 passed | `Passed` |
| `V2` | `just test-one text_sizing::tracker` | 5 tests run: 5 passed | `Passed` |
| `V3` | `just test-one text_sizing_` | 8 tests run: 8 passed | `Passed` |
| `V7` | `just test-one text_sizing::text` | 2 tests run: 2 passed | `Passed` |
| Format | `cargo fmt --check` | No diff | `Passed` |
| Lint | `cargo clippy --all-targets --locked -- -D warnings` | Finished, no warnings | `Passed` |
| Group tests | `just test-one text_sizing` | 20 tests run: 20 passed | `Passed` |
| Regression | `just test` | nextest 3937 passed, 14 skipped; maintenance, bun, and docs-contract tests pass | `Passed` |
| Production `unwrap()` | Review of new non-test code | None; only `unwrap_or`/`unwrap_or_else` | `Passed` |

## Integration

Not applicable — group scope.

## Findings

### G1-VER1 — Minor — correctness — Tracker keeps partial OSC state while sizing is inactive

- **Builds:** `REQ2`, `REQ3`, `DES2`, `G1-T3`
- **Evidence:** `src/pane/terminal.rs:1413`

  ```rust
  let text_sizing_commands = if crate::text_sizing::is_active() {
      core.text_sizing_tracker.observe(bytes);
      core.text_sizing_tracker.drain_pending()
  } else {
      Vec::new()
  };
  ```

  When `is_active()` turns false, the tracker stops seeing bytes but keeps its `state` and `body`. Example: sizing is active while `ESC ] 66 ; s=2 ; Hel` arrives, a client detach turns it off for the rest of that OSC and later output, then sizing turns on again. The tracker resumes in `OscBody` with the stale `66;s=2;Hel` body, appends unrelated later bytes up to the next `BEL`, and injects a spurious block at the cursor. The Ghostty core already finished that OSC, so the two parsers disagree. The registry is server-wide and is changed by client attach and detach, which run independently of the PTY thread (`DES4`, `A5`). So a mid-sequence flip is possible.
- **Impact:** After a client attach or detach, a pane can show a corrupted block or unexpected cursor movement. It is rare, but it breaks the guarantee that OSC 66 is not interpreted while sizing is off (`REQ3`).
- **Resolution:** Do not let tracker state survive an inactive period. For example, reset the tracker to its default state whenever `is_active()` is false, or always observe bytes and drop the drained commands when inactive. Either way, a sequence that is only partly seen while active never produces a command.
- **Proof to close:** A `text_sizing_*` test in `src/pane/terminal.rs` feeds a partial OSC 66 while active. It then feeds the rest plus unrelated text with a later `BEL` while inactive, and a further chunk while active again. It asserts no placeholder cells and an unchanged CPR column. `just test-one text_sizing` passes.

## Test Coverage

- **Covered:** The cell contract round-trips ASCII, `∼`, CJK, and emoji with a skin tone. Tests cover the 24-codepoint and 56-text-mark truncation at a codepoint, and malformed graphemes (wrong base, out-of-range mark, foreign char, missing header, short, unterminated digit). The tracker tests cover `BEL`/`ST` terminators, splits across `observe` calls with a relative `end_offset`, kitty clamping, OSC 0/660/6, DCS, oversized bodies, and C0/C1 removal. At the real `PaneTerminal` level, tests cover the kitty probe columns (3/3/2 and empty text gives 1) and inactive no-op. They also cover placement with grapheme clustering on and off, scroll-to-fit on the last row, discarding blocks larger than the screen, DECAWM-off back-up with DECAWM still off afterwards, wrap when the block does not fit, and print or `ECH` over a block breaking it. Text decode has a 2×2 block test and a same-allocation passthrough test.
- **Missing:** No test covers tracker state across an inactive period (`G1-VER1`). The `PaneRuntime` read wrappers are not exercised end to end. They are thin one-line wraps, so this is acceptable. No test proves that a persistence round trip (`snapshot_history` → `seed_history_ansi`) keeps placeholders intact (`REQ6`, second clause). No assigned `V` requires it.
- **Low-value:** None.

## Scope Changes

The implementer reported these deviations. They are documented and accepted as non-blocking for G1:

- `place_block` turns DECAWM on while writing and restores it afterwards (`src/text_sizing/inject.rs`, end of `place_block`). With DECAWM off and mode 2027 off, Ghostty would attach marks printed at the last column to the left cell. `text_sizing_backs_up_when_autowrap_is_off` checks that the final state matches, for both clustering modes. `terminal.write` cannot fail, and `mode_set` errors are ignored on both calls, so the original mode is always written back.
- Pending wrap is treated as `cursor_x + 1`, and DECSLRM is ignored.
- Blocks are discarded by comparing against screen rows, not scroll-region height. With a scroll region shorter than `s`, `CUU`/`CUD` clamp at the margins and the block is written broken, but nothing crashes. IRM is not neutralized.
- `w=0` graphemes are stored with explicit width 1 or 2.
- Kitty lower-row displacement is not emulated. This is planned in `A4`.

## Blockers and Unverified Items

- Persistence restoring blocks (`REQ6`) is not verified by any G1 check. It relies on the core's ANSI history keeping every grapheme codepoint. Overall verification or `V8` should confirm it.
- `REQ7` scaling (`V9`, `just bench-render-scale`) is outside G1. Code review shows that only one relaxed atomic load is added to the PTY path while sizing is inactive.
