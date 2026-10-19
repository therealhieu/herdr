# W261019EYLD G2 Verification Report

## Scope

- **Scope:** `G2`
- **Tasks:** `G2-T1`, `G2-T2`
- **Change range:** `be21a1d3..de1ef1da`
- **Report:** `.s1mple/work/active/w261019eyld-kitty-text-sizing/reports/g2-verification-report.md`

## Status

`Failed`

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES3` | `V4` | `src/text_sizing/present.rs:Presenter::prepare`, `present_blocks`, `block_is_intact`, `draw_block` (`CUP` + top-left SGR + `ESC]66;s=..:w=..[:n:d][:v][:h];text ESC\`, `w` always explicit, values from decoded `BlockMeta`) | `Met` |
| `REQ4` | `DES3` | `V4` | `src/text_sizing/present.rs:Presenter::finish` (erase stale/all-on-full blocks with `CSI 0 m` + `CUP` + `ECH`, clamped to the new size), `write_fallback_top_row`, `present_blocks` (non-intact placeholders → top-row text, spaces elsewhere) | `Met` |
| `REQ5` | `DES3` | `V4` | `src/text_sizing/present.rs:present_blocks` (a block with any missing cell is never drawn on the host); remaining cells follow `A8` (top-row text), which conflicts with REQ5's "remaining cells render blank" — see `G2-VER2` | `Met` |
| `REQ1` | `DES3` | `V5` | `src/client/state.rs:ClientState::try_present_frame_to` (`prepare` after `bold_color::apply`, `write_into` inserts `pre` after `CSI ?2026h` and `post` before the final `CSI ?2026l`) | `Met` |
| `REQ4` | `DES3` | `V5` | `src/client/state.rs:try_present_frame_to` (`text_sizing.commit` only after `write_composed_output` succeeds); `present_surface_patch` returns `Ok(false)` while host blocks exist or a patch row holds a placeholder; `src/server/render_stream.rs:ClientRenderState::prepare_frame` (`apply_fallback` before `is_current`/`encode`) | `Met` |
| `REQ7` | `DES7` | `V5` | `src/text_sizing/present.rs:Presenter::prepare` (one prefix scan, no allocation when no placeholder and no host blocks), `write_into` early return; no change to `PaneTerminal::render` or server render loop; patch fallback recomposes and diffs (not a full repaint) | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V4` | `just test-one text_sizing::present` | 5 tests run: 5 passed | `Passed` |
| `V5` | `just test-one text_sizing_client` | 2 tests run: 2 passed | `Passed` |
| Format | `cargo fmt --check` | No diff | `Passed` |
| Clippy | `cargo clippy --all-targets --locked -- -D warnings` | Finished, no warnings | `Passed` |
| Group tests | `just test-one text_sizing` | 29 tests run: 29 passed | `Passed` |
| UI hot path | `just ui-hot-path-architecture-test` | Ran 6 tests, OK | `Passed` |
| Regression | `just test` | nextest 3946 passed, 14 skipped; maintenance suites OK | `Passed` |
| Host-output safety | Code review of every byte sink fed by decoded block text | Decoded text reaches host output unfiltered; see `G2-VER1` | `Failed` |

## Integration

Not applicable — group scope

## Findings

### G2-VER1 — Critical — Security — Forged placeholder cells inject arbitrary escape sequences into the host terminal

- **Builds:** `DES3`, `G2-T1`, `G2-T2` (`REQ1`, `REQ4`)
- **Evidence:** Placeholders are ordinary printed graphemes, so any pane output can create them: `src/text_sizing/inject.rs:130`–`145` builds them with `encode_cell` and writes them with `terminal.write(out.as_bytes())`. Decoding accepts every codepoint, including C0, `DEL`, and C1:

  ```text
  src/text_sizing/cell.rs:218   text.push(char::from_u32(value * TEXT_DIGIT_BASE + mark)?);
  ```

  The presenter then writes that text to the host without filtering, in two sinks:

  ```text
  src/text_sizing/present.rs:289      let _ = write!(out, ";{}\x1b\\", meta.text);       // OSC 66 payload (host path)
  src/text_sizing/present.rs:245-247  symbol.clear(); symbol.push_str(grapheme);         // fallback cell symbol
  src/protocol/render_ansi.rs:987     let _ = writer.write_all(cell.symbol.as_bytes());  // encoder writes symbols raw
  ```

  Example: a pane prints `U+10EEED` + `U+E0100`×4 + `U+E0109` (s=1, w=1) + `U+E0100`×3 + one mark per ASCII byte of `ESC ] 52 ; c ; <base64> BEL`. On a supporting host, the 1×1 block is intact, and the client emits `ESC]66;s=1:w=1;ESC]52;c;…BEL ESC\`. An embedded `ESC \` also ends the OSC 66 early, so any sequence that follows reaches kitty. On every other host, `unicode-width` 0.2.2 gives control graphemes width 1. A forged `s=1:w=7` block (7 cells) therefore writes the control bytes one after another as fallback cell symbols. `src/server/render_stream.rs:125` exposes the same path to direct-terminal clients. Neither path checks `text_sizing::is_active()` or `host_supported()`, because `present_blocks` runs whenever a placeholder exists.
- **Impact:** Untrusted pane content, such as a `cat` of a hostile file or remote ssh output, can make herdr write raw escape sequences to the user's host terminal. Examples are OSC 52 clipboard writes (allowed by default in kitty), title changes, and CSI sequences that desynchronize herdr's diff state. This breaks the multiplexer isolation boundary. Once this lands it affects every user, including non-kitty hosts and users who never run an OSC 66 app.
- **Resolution:** Never emit control characters from decoded block text. Reject blocks whose text contains C0 (`U+0000`–`U+001F`), `DEL`, or C1 (`U+0080`–`U+009F`) when decoding (`cell.rs` `decode_text_digits` returns `None`). This also protects the G1 `text.rs` read sink. Alternatively, sanitize in `present.rs` before both sinks. The decoded form must not be able to contain `ESC`, `BEL`, or `ST`.
- **Proof to close:** A presenter test with a forged top-left cell whose text holds `ESC`, `BEL`, `ESC \`, and `U+009B` produces no OSC 66 and no fallback symbol containing those codepoints, on both the supported-host and fallback (`apply_fallback`) paths. `just test-one text_sizing` passes.

### G2-VER2 — Minor — Spec conflict — Blocks broken by an overwrite keep unscaled top-row text, while REQ5 requires blank remaining cells

- **Builds:** `REQ5`, `REQ4`, `DES3`, `V4`, `G2-T1`
- **Evidence:** `src/text_sizing/present.rs:188` (`_ => write_fallback_top_row(frame, x, y, nonce, &meta)`) runs for any non-intact block whose top-left survives. The test `incomplete_blocks_fall_back_to_unscaled_text` (`present.rs:397`–`399`, `:415`) replaces a lower-row cell and asserts that the top row still shows `H`. REQ5 says: "Writing over or erasing any cell of a block … removes the whole block; its remaining cells render blank". `A8` lists "broken" among the cases that show top-row text.
- **Impact:** Suppose an app erases only a lower row of a sized heading, for example `EL` on the line under an `s=2` H1. Kitty then clears the whole multicell, but herdr leaves the heading text unscaled in the top row. This is a leftover-cell divergence from plain kitty, which REQ4 also forbids. The decision record contradicts itself, so the implementer followed `A8` and `V4`.
- **Resolution:** Make the decisions consistent. Either amend REQ5 (and the known-deviation list) to accept top-row text for blocks broken by a write, or change the behavior so blocks broken by a write render blank, for example by invalidating the top-left on the server side. Then align `V4` and its test.
- **Proof to close:** The ship file's REQ5, `A8`, and `V4` agree, and `just test-one text_sizing::present` asserts the agreed behavior.

### G2-VER3 — Minor — Test gap — Server direct-terminal fallback is untested

- **Builds:** `DES3`, `G2-T2`
- **Evidence:** `src/server/render_stream.rs:124`–`125` (`crate::text_sizing::present::apply_fallback(&mut frame);`) is not covered by any test. The implementer's return lists this as a known limit. DES3 requires fallback-only presentation for direct-terminal ANSI clients.
- **Impact:** If someone reorders `apply_fallback` after `is_current`/`encode`, or removes it, direct-terminal clients would get raw `U+10EEED` + mark graphemes, and no test would fail.
- **Resolution:** Add a test that `ClientRenderState::prepare_frame` for a `TerminalAnsi` client turns a frame with a block into bytes holding the fallback text and no `U+10EEED`.
- **Proof to close:** The new test passes under `just test-one text_sizing`, or under a test name filter that the ship file records.

## Test Coverage

- **Covered:** These presenter cases are tested on synthetic frames:
  - an intact block becomes skip cells, and `post` holds the exact OSC 66 bytes
  - missing, clipped, and covered blocks fall back
  - a moved or removed block is erased with `ECH` for every old row
  - an unchanged block produces no output
  - a full redraw erases all old blocks and redraws, with clamping on a smaller frame
  - an unsupported host never skips cells or emits OSC 66

  Client wiring is tested once:
  - `pre` comes right after `CSI ?2026h`, and `post` comes before the final `CSI ?2026l`
  - a failed write keeps the host blocks, so the next frame erases them
  - the patch path falls back while host blocks exist and when a patch row holds a placeholder

  This matches the Test Strategy layers: presenter on synthetic frames, client wiring once.
- **Missing:**
  - Forged or control-character block text (`G2-VER1`).
  - The server direct-terminal fallback (`G2-VER3`).
  - The drawn host cursor on an intact block cell (documented as not shown).
  - Fractional `n`/`d` and `v`/`h` serialization in `draw_block`. Only `s`/`w` payloads are asserted.
- **Low-value:** None

## Scope Changes

- `src/client/mod.rs`: one-line field initialization (`text_sizing: Default::default()`). It is outside the planned files but required by the new `ClientState` field.
- `src/text_sizing.rs`: removed the stale `#[allow(dead_code)]` on `host_supported()`, which now has a caller. Justified.

No omissions from the G2 plan.

## Blockers and Unverified Items

- What kitty actually renders cannot be checked in CI. That covers host-terminal resize (kitty can shift alt-screen lines on shrink, which would move old blocks away from the coordinates `pre` erases), kitty's lower-row displacement on the write-failure and full-redraw paths, and visual alignment through scroll, tab switches, and popups. Manual live case `V8` covers these at overall verification. From code reading, top-down `write_all_cells` writes a block's top row before its lower rows, so kitty clears an orphaned multicell before any displacement can happen.
- `V9` (render-scale benchmark) is overall scope and was not run. The client pass runs once per frame per client and does not scale with pane count.
