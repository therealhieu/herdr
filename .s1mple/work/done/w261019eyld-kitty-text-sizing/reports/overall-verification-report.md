# W261019EYLD Overall Verification Report

## Scope

- **Scope:** `overall`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`, `G1-T4`, `G2-T1`, `G2-T2`, `G3-T1`, `G3-T2`
- **Change range:** `41461ad7..c0437ae0`
- **Report:** `.s1mple/work/active/w261019eyld-kitty-text-sizing/reports/overall-verification-report.md`

## Status

`Failed`

The behavior is correct and every check passes: `just lint` is clean, `just test` runs 3953 tests with no failures, and two scratch probes confirmed persistence restore and the `A10` shortcut on the real core. The status is `Failed` only because of three Minor evidence gaps. Two decisions the ship relies on, `A10` and REQ6 persistence restore, have no committed regression test. The `V9` benchmark also does not run the new per-frame and per-byte code that `REQ7` describes. No Critical or Major defect was found.

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES1`, `DES2`, `DES3` | `V1`, `V4`, `V8` | `src/text_sizing/cell.rs:encode_cell`/`decode` (all six params round-trip); `src/text_sizing/inject.rs:place_block`; `src/text_sizing/present.rs:draw_block` writes `CUP` + top-left SGR + `ESC]66;s:w[:n:d][:v][:h];text ESC\`; `V8` records kitty-in-herdr multicells equal to plain kitty after the pane offset | `Met` |
| `REQ2` | `DES2` | `V2`, `V3`, `V8` | `src/text_sizing/tracker.rs:parse_metadata` (kitty clamping); `src/text_sizing/inject.rs:place_block` (discard `s·w > cols`/`s > rows`, `CR`+`IND` wrap, `cols − s·w` back-up, `IND`×(s−1)+`CUU`, top row last); `src/pane/terminal.rs:write_pty_bytes_with_ordered_responses` `TextSizing` arm runs after the core consumes bytes to `end_offset`; empty text dropped in `parse_body` | `Met` |
| `REQ3` | `DES2`, `DES4`, `DES5` | `V3`, `V6`, `V8` | `src/pane/terminal.rs` observes only when `text_sizing::is_active()`, otherwise `reset()`; `src/text_sizing.rs:RegistryState`/`ClientRegistration`; `src/server/client_transport.rs:handle_client_handshake` guard lives until `client_read_loop_with_endpoint_controls` returns; `src/client/mod.rs:run_client_with_mode` sets host support from `text_sizing_enabled() && client_rendered_shell && text_sizing_profile_allowed()`; `V8` probe with the switch off gives 2/1/1 | `Met` |
| `REQ4` | `DES1`, `DES3` | `V4`, `V5`, `V8` | `src/text_sizing/present.rs:Presenter::prepare`/`finish`/`write_into`/`commit` on the final composed frame; `write_fallback_top_row`; `src/client/state.rs:try_present_frame_to` commits only after a successful write; `present_surface_patch` falls back while blocks exist; `src/server/render_stream.rs:prepare_frame` applies the fallback for direct-terminal clients | `Met` |
| `REQ5` | `DES2`, `DES3` | `V3`, `V4`, `V8` | Core overwrite/erase breaks the placeholder set (`text_sizing_overwriting_or_erasing_breaks_blocks`); `present.rs:block_is_intact` then writes the fallback and `pre` erases the host block; lower-row displacement not emulated (`A4`, recorded deviation) | `Met` |
| `REQ6` (text reads) | `DES1`, `DES6` | `V1`, `V7` | `src/text_sizing/text.rs:decode_text` applied in `src/pane.rs` `visible_text`, `visible_ansi`, `recent_*_snapshot` (via `decode_snapshot_text`), `extract_selection`; `src/terminal/runtime.rs` wraps `PaneRuntime`, so API reads go through it; `detection_text`/`recent_unwrapped_ansi`/`snapshot_history` stay raw | `Met` |
| `REQ6` (persistence restore) | `DES1`, `DES6` | none planned | `src/pane.rs:snapshot_history` → raw `recent_unwrapped_ansi`; `src/pane/terminal.rs:seed_history_ansi` writes it back. A scratch probe confirmed it works but nothing is committed (`OVR-VER2`) | `Met` |
| `REQ7` | `DES7` | `V9`, `V10` | `PaneTerminal::render` and dirty-patch collection are unchanged in the diff; tracker work is gated by `is_active()`; presenter returns early without placeholders or host blocks. `V9` does not run the new paths (`OVR-VER3`) | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `just test-one text_sizing::cell` | 6 run, 6 passed | `Passed` |
| `V2` | `just test-one text_sizing::tracker` | 6 run, 6 passed | `Passed` |
| `V3` | `just test-one text_sizing_` | 15 run, 15 passed (9 `PaneTerminal` cases plus client, server, handshake, endpoint, config) | `Passed` |
| `V4` | `just test-one text_sizing::present` | 6 run, 6 passed | `Passed` |
| `V5` | `just test-one text_sizing_client` | 2 run, 2 passed | `Passed` |
| `V6` | `just test-one text_sizing` and `just test-one endpoint` | 36 passed; 283 passed (frozen hello fixture included) | `Passed` |
| `V7` | `just test-one text_sizing::text` | 2 run, 2 passed | `Passed` |
| `V8` | Recorded orchestrator evidence (ship file, Evidence → Verification Cases) | Probe 2/3/3 with sizing and 2/1/1 with `text_sizing = false`, matching plain kitty. `so` `tour.md` multicells match plain kitty. Scroll, tab switch, pane split and popup left no stale cells. Herdr scrollback viewport and host window resize were not run live; `V4` presenter cases cover them | `Passed` |
| `V9` | Recorded orchestrator evidence | Scaling unchanged on base and head (15-pane ratios 1.02→1.01 and 1.11→1.08). `native_file_render_scale_profile` panic is pre-existing on `41461ad7`. The profile covers server surface and client compose only (`OVR-VER3`) | `Passed` |
| `V10` | `just lint && just test` | `cargo fmt --check` and clippy `-D warnings` clean; nextest 3953 run, 3953 passed, 14 skipped; maintenance, UI hot-path architecture, integration-asset, and docs-contract tests pass | `Passed` |
| Frozen protocol | `git diff 41461ad7..HEAD -- tests/fixtures src/protocol/wire.rs` | Empty: no `PROTOCOL_VERSION`, codec, or fixture change; `EndpointClientHello.text_sizing` is `serde(default, skip_serializing_if = "is_false")` | `Passed` |
| No `unwrap()` in production | Scan of non-test code in `src/text_sizing*` and of the diff in upstream files | None; only `unwrap_or`/`unwrap_or_else`/`unwrap_or_default` | `Passed` |
| Compile gating | Diff scan for `cfg` | Only `#[cfg(unix)]`/`#[cfg(not(unix))]`/`#[cfg(any(unix, test))]` in `src/client/handshake.rs`, matching the existing `direct_graphics_profile_allowed` pattern; no `cfg!` or `target_os` in core modules | `Passed` |
| No s1mple IDs in code | Diff scan of `src`, `crates`, `docs/next` for `REQ`/`DES`/`V<n>`/`A<n>`/work IDs | None | `Passed` |
| Persistence probe (scratch) | Temporary detached worktree (removed afterwards); test fed `ab` + `s=2;Hi` + an 18-emoji `w=2:n=1:d=2` block (62 marks) into `GhosttyPaneTerminal`, then `recent_unwrapped_ansi(usize::MAX)` → `seed_history_ansi` into a fresh core | All 10 placeholder cells, including the 62-mark top-left, decode identically before and after restore | `Passed` |
| `A10` shortcut probe (scratch) | Same worktree: `w=1;∼`, `;ab`, `w=1;漢`, `s=2;ab` | `∼`, `a`, `b` stored as plain cells; `漢` (width 2) became a 1×1 block; `s=2;ab` split into two 2×2 blocks (nonces 1 and 2) | `Passed` |

## Integration

The groups integrate correctly. The shared cell contract, the handoffs between client and server, and the final scope all trace to code and tests.

**Placeholder cell contract.** One module, `src/text_sizing/cell.rs`, owns the contract, and every producer and consumer uses it:

```text
inject::place_block ──encode_cell──► core grid ──CellData.symbol──► client compose
                                         │                               │
                                         ├─ decode_text (text reads)      ├─ Presenter::prepare (decode, intact check)
                                         └─ snapshot_history (raw)        └─ render_stream::prepare_frame → apply_fallback
```

- The server writes cells with `encode_cell`. The client presenter, the server direct-terminal fallback, and `text::decode_text` all read them with `decode`/`is_placeholder`.
- Control characters are rejected in three places: when encoding, when decoding, and again in `present.rs:draw_block`. A pane that prints a forged placeholder therefore cannot inject escapes into the host (`forged_control_text_never_reaches_the_host`, `block_text_never_holds_control_characters`).
- The real Ghostty core keeps all 64 suffix codepoints (`vendor/libghostty-vt/src/terminal/page.zig:92` `grapheme_max_len = 64`). A 62-mark top-left survives the core and history restore (scratch probe).

**Host support handoff.** The client computes `set_host_supported(...)` in `src/client/mod.rs:213` before any `do_handshake` call. Every handshake path (`mod.rs:291`, `endpoint/supervisor.rs:486`, `terminal_sessions.rs:93`, `handshake.rs:163`/`:456`) builds the same hello with `text_sizing: host_supported()`. `ClientState::try_present_frame_to` reads the same flag for `Presenter::prepare`.

**Registration handoff.** The server copies `hello.text_sizing` into `text_sizing_supported`; direct-terminal hellos stay `false`. `ClientRegistration` then updates `REGISTRY` and mirrors `ACTIVE`, which the PTY path reads in `GhosttyPaneTerminal::process_pty_bytes`. The guard is a local in `handle_client_handshake`, so every return drops it: early handshake exits, failed welcome writes, and the end of the read loop.

**Final scope.** All 24 changed files belong to the planned groups. Upstream files take small, additive edits: one field each in `ClientState` and `GhosttyPaneCore`, one event arm, one hello field, one config field, one `pub(crate)`, and the wrapper calls in `pane.rs`.

One accepted design consequence, not a defect: while sizing is active, an upstream-protocol client attached at the same time receives raw placeholder cells. `A5` counts it as unsupported, and the non-goals exclude upstream clients.

## Findings

### OVR-VER1 — Minor — test coverage — `A10` plain-text shortcut has no regression test

- **Builds:** `REQ2`, `REQ7`, `DES2`, `G1-T3`
- **Evidence:** `src/text_sizing/inject.rs:writes_plain_text` holds the shortcut, and no test checks that it creates no placeholders. The only related assertion, `src/pane/terminal.rs` `text_sizing_cursor_matches_kitty_probe` with `(&b"\r\x1b]66;w=1; \x07"[..], 2)`, checks only the cursor column. A 1×1 block would give the same column, so the test passes whether or not the shortcut works. No test covers the `s=1`/`w=0` plain path either.
- **Impact:** OpenTUI writes `w=1` for every box-drawing glyph. If the shortcut regressed, those glyphs would silently become blocks. `present_surface_patch` would then fall back to full composition for every frame, and unsupported hosts would get fallback rendering. All current tests would stay green. The scratch probe shows the code works today.
- **Resolution:** Add a `text_sizing_*` `PaneTerminal` test. It should assert that `w=1;∼` and `;ab` leave plain cells and no placeholders, that `w=1;漢` and a fractional `s=1` command create blocks, and that a multi-grapheme `w=0` command splits into one block per grapheme.
- **Proof to close:** `just test-one text_sizing_` passes with the new test, and the test fails when `writes_plain_text` returns `false`.

### OVR-VER2 — Minor — traceability — REQ6 persistence restore has no verification case or test

- **Builds:** `REQ6`, `DES6`, `G1-T4`
- **Evidence:** `REQ6` says "session persistence restores blocks", but the Traceability table maps `REQ6` only to `V1` and `V7`. Neither one, nor `V8`, exercises `snapshot_history` → `seed_history_ansi`. The behavior works: the scratch probe matched all 10 placeholder cells, including a 62-mark top-left, before and after restore.
- **Impact:** Restore depends on two unpinned behaviors: the Ghostty ANSI formatter emitting every suffix codepoint, and re-clustering on seed. A vendor update could break either one without failing a test.
- **Resolution:** Add a `PaneTerminal` test that writes a block, takes `recent_unwrapped_ansi(usize::MAX)`, seeds a fresh terminal with `seed_history_ansi`, and asserts the block is intact with the same metadata. Include a near-limit text block.
- **Proof to close:** `just test-one text_sizing_` passes with the new restore test.

### OVR-VER3 — Minor — performance evidence — `V9` does not exercise the new per-frame and per-byte paths

- **Builds:** `REQ7`, `DES7`, `V9`, `G2-T2`
- **Evidence:** `just bench-render-scale` times only `client_shell::render_pane_surface` (server) and `ClientShellState::compose` (client) (`src/server/render_scale_benchmark.rs:85`–`119`). It does not run any of the code this work added: `ClientState::try_present_frame_to` (`Presenter::prepare` scans every frame cell), `ClientRenderState::prepare_frame` for direct-terminal clients (`apply_fallback` scans every cell per client per frame, `src/server/render_stream.rs:124`), or the PTY `TextSizingTracker`. The benchmark uses only `RenderEncoding::SemanticFrame` (`:402`). The recorded unchanged ratios therefore prove only that untouched code is unchanged. In addition, `REQ7` says "No new work runs in the server render loop", yet the direct-terminal fanout now does an O(cells) scan.
- **Impact:** `REQ7` holds by construction. Presenter and fallback costs grow with frame size, not pane count, and the tracker grows only with bytes per pane. Still, the evidence does not measure what `V9` claims to prove, and the `REQ7` wording does not match the planned `DES3` server fanout work.
- **Resolution:** Make one of these observable changes. (a) Add a profile, or a deterministic operation-count test, covering `Presenter::prepare` plus `apply_fallback` at 1 and 15+ panes, and the tracker with sizing active. (b) Amend the `REQ7`/`V9` text to state the frame-size bound, name the direct-terminal fanout scan, and record that the bench covers only server surface and client compose.
- **Proof to close:** The new profile or test output is recorded under Evidence → `V9`, or the ship file states the amended scope and the reason for it.

## Test Coverage

- **Covered:**
  - The cell contract: round-trip, limits, malformed input, and forged control text.
  - Tracker framing: terminators, split input, clamping, oversized bodies, foreign strings, and reset while inactive.
  - Kitty cursor and CPR on the real core with grapheme clustering on and off: wrap, DECAWM back-up, scroll-to-fit, and discard.
  - Breaking a block by overwrite or erase.
  - Presenter cases: intact, missing, clipped, covered, moved, removed, unchanged, full redraw, resize clamp, and unsupported host.
  - Client sync-frame insertion and the failed-write commit rule; patch fallback.
  - Direct-terminal fallback.
  - The registry state machine; host profile matrix; hello compatibility and fixture omission; config default and opt-out.
  - Text decode and the unchanged-allocation fast path.
- **Missing:**
  - The `A10` shortcut and multi-grapheme `w=0` splitting (`OVR-VER1`).
  - Persistence restore (`OVR-VER2`).
  - `ClientRegistration` drop updating `ACTIVE`, which is untested but is a 5-line guard over the tested `RegistryState`.
  - Performance of the new presenter, fallback, and tracker paths (`OVR-VER3`).
- **Low-value:** None.

## Scope Changes

- The `REQ5` wording was aligned with the `A8` fallback (`G2-VER2`, ship-file clarification `24a0cf48`). Behavior did not change.
- Kitty lower-row displacement is not emulated (`A4`, planned deviation).
- No unplanned additions or omissions.

## Blockers and Unverified Items

- `V8` and `V9` were not rerun, as instructed. The judgment above relies on the recorded orchestrator evidence.
- Two `REQ4` actions were not exercised live on kitty: herdr scrollback viewport scrolling and host window resize. They rely on the frame-level `V4` presenter cases.
