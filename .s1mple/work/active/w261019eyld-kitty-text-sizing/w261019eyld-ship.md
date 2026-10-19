# W261019EYLD kitty-text-sizing — Ship

## Table of Contents

- [1. Decisions](#1-decisions)
  - [Q&A Log](#qa-log)
  - [Requirements](#requirements)
  - [UI Mockups](#ui-mockups)
  - [UI Design](#ui-design)
  - [System Design](#system-design)
  - [Verification](#verification)
    - [Test Strategy](#test-strategy)
  - [Traceability](#traceability)
- [2. Goal](#2-goal)
  - [Status](#status)
  - [Sources](#sources)
  - [Reports](#reports)
  - [Todos](#todos)
  - [Execution Rules](#execution-rules)
  - [Group Cycle](#group-cycle)
  - [Group Scheduling](#group-scheduling)
- [3. Plan](#3-plan)
  - [G1 — Server placeholder model](#g1--server-placeholder-model)
  - [G2 — Client presentation](#g2--client-presentation)
  - [G3 — Capability negotiation and config](#g3--capability-negotiation-and-config)
- [4. Overall Verification and Harden](#4-overall-verification-and-harden)
- [5. Deliver](#5-deliver)
- [6. Evidence](#6-evidence)
- [Ship File Check](#ship-file-check)

## 1. Decisions

- **Prompt:** Make herdr support kitty's text sizing protocol (OSC 66) so `so` running in a herdr pane inside kitty draws 2× H1, 1.5× H2, and half-size scripts (request from the `so` project, evidence measured 2026-10-19 on kitty 0.49.2 / herdr 0.9.5).
- **Goal:** Inside herdr on a kitty host, the OSC 66 probe gives the same CPR columns as plain kitty, and sized text shows at the right place and size and stays aligned through scrolling, resizing, switching, and redraws; on hosts that cannot size text, OSC 66 is still dropped so apps fall back.
- **Scope:** OSC 66 parsing in the pane byte stream, a grid model for sized blocks, kitty-equal cursor/CPR, client re-emission to kitty, degraded rendering, capability gating, text extraction cleanup, config switch. · **Non-goals:** kitty's lower-row displacement rule (rule 4 below), wrapping OSC 66 inside the app's OSC 8 hyperlinks on the host, changing pane `TERM`, upstream-protocol clients rendering sized text, Windows/Linux verification, changes to `so`/OpenTUI.
- **UI:** `not needed — no new screen, component, or interaction; the change only reproduces app-drawn text on the host`.

Discussion context: [`w261019eyld-discuss-context.md`](w261019eyld-discuss-context.md) (commit `552bee34`). Spec: kitty `docs/text-sizing-protocol.rst`; behavior reference: kitty `kitty/screen.c` `handle_fixed_width_multicell_command`, `move_cursor_past_multicell`, `screen_handle_multicell_command`.

### Q&A Log

The user asked for a self-driven ship with execution, so no questions were asked; every decision below is an assumption grounded in code or the spec.

| Q | Question | Answer | Decides |
|---|---|---|---|
| `A1` | Patch libghostty-vt to store multicell blocks, or model them outside the core? | Model outside the core with marker ("placeholder") cells written into the core grid. The core cell model only has `narrow/wide/spacer_tail/spacer_head` (`vendor/libghostty-vt/src/terminal/page.zig:2213`), upstream Ghostty still has no OSC 66 implementation (ghostty-org/ghostty#10333 open), and a multicell core patch would be a large, long-lived Zig fork. Placeholders inherit scroll, reflow, erase, overwrite, alt-screen, and persistence from the core for free, like kitty Unicode image placeholders (`src/pane/terminal.rs:3213`). | `DES1`, `DES2` |
| `A2` | Where do block metadata and text live? | Inside the placeholder graphemes themselves (self-describing cells), not in a server side table. Cells already travel as `CellData.symbol` strings (`src/protocol/wire.rs:697`), so no frozen codec, `PROTOCOL_VERSION`, or fixture changes, no table GC, and blocks survive history persistence (`src/pane.rs:3575`). Ghostty allows 64 appended codepoints per cell (`page.zig:92`); kitty caps a multicell at 24 codepoints (`MAX_NUM_CODEPOINTS_PER_CELL`). | `DES1` |
| `A3` | How exactly is a block placed, and how does the cursor move? | Mirror kitty's `handle_fixed_width_multicell_command`: discard when `s·w > cols` or `s > rows`; wrap to the next line (DECAWM on) or back up to `cols − s·w` (DECAWM off) when it does not fit; scroll to make room for `s` rows; cursor ends `s·w` cells right on the original row. `w=0` splits text into graphemes, one block each with width = grapheme width. Empty text (after dropping C0/C1) is a no-op. | `DES2` |
| `A4` | Kitty's lower-row displacement rule ("writing into a lower row moves the cursor past the block") | Not emulated: it would need a cell lookup before every printed character in the PTY hot path. In herdr, writing into any block cell breaks the block. `so` erases each old block row with ECH before redrawing, so it does not depend on displacement. Recorded as a known deviation. | `DES2`, `REQ5` |
| `A5` | When is sizing "available" so CPR reflects it? | Server-wide flag: on while at least one attached endpoint client advertises host support; with no clients attached, the last value is kept (detach/reattach keeps behavior stable); off at server start. Clients that do not advertise (upstream clients, direct-terminal clients) count as unsupported. | `DES4`, `REQ3` |
| `A6` | How does a client decide its host can size text? | Host profile like `direct_graphics_profile_values` (`src/client/handshake.rs:58`): `TERM=xterm-kitty` or `KITTY_WINDOW_ID` set, not inside tmux/screen, stdin and stdout are terminals, and `terminal.text_sizing` is not false. Ghostty and WezTerm do not implement OSC 66, so they stay unsupported. | `DES4`, `DES5` |
| `A7` | How is the capability sent to the server? | New optional JSON field `text_sizing` on `EndpointClientHello`, `#[serde(default, skip_serializing_if = "is_false")]`, so the frozen hello fixture `tests/fixtures/endpoint-hello-v1.json` and older peers stay compatible (`AGENTS.md` Stable client endpoint contract). | `DES4` |
| `A8` | What does a client show for a block it cannot draw whole? | Its top-row cells show the block text unscaled (graphemes that fit, then spaces); every other block cell is a space with the block's style. Applies to clipped, covered (popups), partially scrolled, broken, and unsupported-host cases. | `DES3` |
| `A9` | How does the client keep kitty's screen consistent? | A per-client presenter runs on the final composed frame, before `BlitEncoder`: intact blocks become `skip` cells plus one OSC 66 after the frame; blocks drawn last frame but not identical now are erased first with `CUP` + `ECH` on every row; full redraws erase all previously drawn blocks. The patch fast path falls back to composition while blocks exist. | `DES3` |
| `A10` | Shortcuts for OSC 66 that changes nothing visible | `s=1` with no fractional scale and `w=0` is written as plain text; `s=1`, no fraction, `w=1`, and a single codepoint of width 1 (OpenTUI's width-pinned `∼ ▌ • │`) is also written as plain text. This keeps box-drawing-heavy TUIs from creating thousands of blocks. | `DES2` |
| `A11` | Text consumers | `herdr agent read` text and ANSI output, visible text, and copy selection replace placeholders with the block text at the top-left cell and drop other placeholder cells. Detection text and the persistence snapshot (`snapshot_history`) keep raw placeholders: detection ignores them, and persistence must restore blocks. | `DES6` |
| `A12` | Config switch name and default | `[terminal] text_sizing = true` (`Option<bool>`, default on), read by the client to compute its hello flag; it takes effect on the next attach. Documented in `docs/next/website/src/data/config-reference.json` because `scripts/test_config_reference_check.py` requires every key. | `DES5` |
| `A13` | Performance | The server render loop is unchanged. The PTY tracker scans only for `ESC` with `position` like the existing trackers (`src/pane/osc.rs:176`) and does nothing when sizing is off. The client pass is O(cells) once per presented frame per client (like `bold_color`). Profile with `just bench-render-scale` at 1 and 15+ panes. | `DES7` |

### Requirements

| ID | Requirement | Source |
|---|---|---|
| `REQ1` | With sizing available, OSC 66 text from a pane is drawn by kitty with the same `s`, `w`, `n`, `d`, `v`, `h` and at the same pane cell position as in plain kitty. | prompt, `A2`, `A9` |
| `REQ2` | With sizing available, cursor movement and CPR after OSC 66 match kitty: probe `w=2` and `s=2` each advance 2 columns, OpenTUI `w=1` advances 1; blocks wrap or back up, scroll to fit their height, and are discarded when larger than the screen; empty text is a no-op. | prompt, `A3`, `A10` |
| `REQ3` | When no attached client can size text, or the switch is off, OSC 66 is dropped as today: nothing is drawn and the cursor does not move. | prompt, `A5`, `A6`, `A12` |
| `REQ4` | Sized blocks stay aligned with no leftover or shifted cells through scrolling, pane resize, tab/pane/workspace switches, popups, and redraws; a block that cannot be drawn whole shows unscaled text in its top row and blanks elsewhere. | prompt, `A8`, `A9` |
| `REQ5` | Writing over or erasing any cell of a block (print, ECH, EL, ED, DCH, ICH, IL, DL) removes its scaled rendering; its remaining cells render with the `REQ4`/`A8` fallback (unscaled text in surviving top-row cells while the top-left cell survives, blanks elsewhere). Lower-row displacement is not emulated. | prompt, `A4`, `A8` |
| `REQ6` | `herdr agent read`, visible text, and copied selections show the block text instead of marker characters; session persistence restores blocks. | `A11` |
| `REQ7` | No new work runs in the server render loop, and PTY parsing and client presentation stay within the existing scaling behavior at 1 and 15+ panes. | `A13`, `AGENTS.md` Multiplicative performance paths |

### UI Mockups

Not applicable — UI is not needed.

### UI Design

Not applicable — no UI.

### System Design

| ID | Decision | Builds | Why |
|---|---|---|---|
| `DES1` | **Placeholder cell contract** in new `src/text_sizing/cell.rs`. Every cell of a block holds one grapheme: base `U+10EEED`, then marks from `U+E0100 + v` (`v` in `0..240`). Marks: `row`, `col`, `nonce_hi`, `nonce_lo`; the top-left cell (`row=col=0`) adds `s·8+w`, `n`, `d`, `v·3+h`, then the text as base-120 varint marks (digit `<120` ends a codepoint, `120..240` continues). At most 64 marks (8 header + 56 text); text is truncated at a codepoint to fit and to kitty's 24 codepoints. API: `encode_cell(..) -> String`, `decode(&str) -> Option<Placeholder>`, `is_placeholder(&str) -> bool` (first char check). | `REQ1`, `REQ4`, `REQ6` | Self-describing cells need no side table or wire change; marks have width 0 in both unicode-width and Ghostty and cluster onto the base in both 2027 on and off modes (`Terminal.zig:1220`, `:1462`). |
| `DES2` | **PTY injection** in new `src/text_sizing/tracker.rs` and `src/text_sizing/inject.rs`. A byte tracker (modeled on `DefaultColorEventTracker`, `src/pane/osc.rs:169`) emits `TextSizingCommand { end_offset, params, text }` for `ESC ] 66 ; meta ; text (BEL / ESC \)`, bodies ≤ 4096 + header, parsing keys `s,w,n,d,v,h` with kitty's clamping. `src/pane/terminal.rs` adds the tracker and a nonce counter to `GhosttyPaneCore`, observes bytes only when `text_sizing::is_active()`, and adds an `OrderedColorOrC1Event::TextSizing` arm: after the core consumes bytes up to `end_offset`, `inject::apply(&mut core.terminal, cmd, &mut nonce)` writes the synthesized bytes. Placement (per `A3`) uses relative moves only: `CR`+`CUF` for columns, `IND`×(s−1) then `CUU`(s−1) to make room, lower rows first, top row last so the final cursor and pending-wrap state equal printing `s·w` cells. Shortcuts per `A10` write plain text. Needs new `Terminal::cursor_x()` and `cursor_pending_wrap()` in `crates/ghostty-vt/src/lib.rs` (data ids 3 and 5 already bound). | `REQ1`, `REQ2`, `REQ3`, `REQ5` | Reuses the ordered-event write split that already orders herdr replies around core writes (`src/pane/terminal.rs:1471`), so CPR after OSC 66 is answered by the core from the moved cursor. |
| `DES3` | **Client presenter** in new `src/text_sizing/present.rs`: `Presenter { host_blocks }` with `prepare(&mut FrameData, host_supported) -> Pass` and `finish(pass, encoded_full) -> (pre, post)`, plus `commit()`. On the composed frame: decode placeholders; a block is intact when every cell at `(x+dc, y+dr)` decodes with the top-left's nonce and its own `(dr, dc)` and fits the frame. Intact blocks on a supporting host get `skip = true`; all other placeholder cells get fallback symbols (`A8`). `pre` (inserted after `CSI ?2026h`) resets SGR and erases every stale host block with `CUP` + `ECH(bw)` per row (all previous blocks on a full redraw); `post` (inserted before the final `CSI ?2026l`, inside `ESC 7`/`ESC 8`) draws new or changed blocks with `CUP`, the top-left cell's SGR, and `ESC ] 66 ; meta ; text ESC \`. `src/client/state.rs` calls it in `try_present_frame_to` after `bold_color` and commits `host_blocks` only after a successful write; `present_surface_patch` returns `Ok(false)` (compose fallback) while blocks exist or a patch row holds a placeholder. `src/server/render_stream.rs` applies fallback-only presentation for direct-terminal ANSI clients. `src/protocol/render_ansi.rs` exposes its SGR writer as `pub(crate)`. | `REQ1`, `REQ4`, `REQ5` | Working on the final composed frame handles clipping, popups, scroll offset, and tab switches without per-surface logic; erasing before writing avoids kitty's displacement on the host. |
| `DES4` | **Capability negotiation**: `EndpointClientHello.text_sizing: bool` (optional, `A7`); `src/client/handshake.rs` sets it from `text_sizing_profile_values(term, kitty_window, blocked_transport, terminals)` and config, and stores the result with `text_sizing::set_host_supported`; `src/server/client_transport.rs` holds a `text_sizing::ClientRegistration` guard for the connection lifetime (endpoint clients with their flag, direct-terminal clients with `false`). `src/text_sizing.rs` keeps `{supporting, attached, active}` behind a mutex and mirrors `active` into an `AtomicBool` read by the PTY path (`A5`). | `REQ3` | A connection-scoped guard needs no new server events or wire messages and cannot leak on any disconnect path. |
| `DES5` | **Config**: `TerminalConfig.text_sizing: Option<bool>` in `src/config/model.rs`, `Config::text_sizing_enabled()` (default true) in `src/config.rs`, threaded to the handshake; one `terminal.text_sizing` entry in `docs/next/website/src/data/config-reference.json`. | `REQ3` | Kill switch for a render-path feature, matching `terminal.kitty_graphics`. |
| `DES6` | **Text extraction**: new `src/text_sizing/text.rs` `decode_text(String) -> String` (returns the input unchanged unless it contains `U+10EEED`): top-left placeholder → block text, other placeholders → removed. Applied in `src/pane.rs` wrappers `visible_text`, `visible_ansi`, `recent_text_snapshot`, `recent_ansi_snapshot`, `recent_unwrapped_text_snapshot`, `recent_unwrapped_ansi_snapshot`, `extract_selection`; not in `detection_text` or `recent_unwrapped_ansi`/`snapshot_history`. | `REQ6` | One choke point per public read path, while persistence keeps raw cells. |
| `DES7` | **Performance guard**: no change to `PaneTerminal::render` or dirty-patch collection; tracker skips work when inactive; presenter fast path returns immediately when the frame has no placeholder and no host blocks. | `REQ7` | Keeps multiplicative paths flat (`AGENTS.md`). |

Placeholder encoding example (block `s=2`, text `H`, nonce 0x1234 → hi 19, lo 100):

```text
top-left (0,0): U+10EEED, M0, M0, M19, M100, M(2·8+1=17), M0, M0, M0, M72('H'=72 <120 → final digit)
cell (1,1):     U+10EEED, M1, M1, M19, M100
```

### Verification

| ID | Proves | Checks | Layer | Command or method | Expected |
|---|---|---|---|---|---|
| `V1` | `REQ1`, `REQ6` | `DES1` | unit | `just test-one text_sizing::cell` | Encode→decode round-trips meta and text for ASCII, `∼`, CJK, emoji, and the 24-codepoint/56-mark limits (truncated at a codepoint); malformed graphemes (wrong base, mark out of range, missing header) decode to `None`; total marks ≤ 64. |
| `V2` | `REQ2`, `REQ3` | `DES2` | unit | `just test-one text_sizing::tracker` | Tracker finds OSC 66 with BEL and `ESC \` terminators, split across `observe` calls, with correct `end_offset`; clamps `s` to 1..7 and `w` to 0..7; ignores other OSCs and oversized bodies; drops C0/C1 from text. |
| `V3` | `REQ2`, `REQ5` | `DES2` | unit (real `PaneTerminal`, no PTY) | `just test-one text_sizing_` in `src/pane/terminal.rs` tests | With sizing active: `CR`+CPR after `w=2; ` reports col 3 and after `s=2; ` col 3; OpenTUI `w=1; ` col 2; block at the last row scrolls up so both rows exist; `w=7:s=7` wider than cols is discarded (cursor unchanged); DECAWM off backs up to `cols−s·w`; placeholder cells appear in the right cells; printing over a lower-row cell and `ECH` over the block leave no intact block. With sizing inactive: cursor stays at col 1 (today's behavior). |
| `V4` | `REQ1`, `REQ4`, `REQ5` | `DES3` | unit | `just test-one text_sizing::present` | On synthetic frames: intact block → its cells `skip` and `post` holds `CUP`+SGR+`ESC]66;s=2:w=1…;H ESC\`; block missing one cell, clipped at the frame edge, or covered → no `post`, top row shows `H` then spaces, others spaces; block present last frame and moved or removed → `pre` holds `CUP`+`ECH` for every old row; unchanged block → empty `pre`/`post`; full redraw → all old blocks erased and current ones redrawn; unsupported host → never `skip`, never OSC 66. |
| `V5` | `REQ4` | `DES3` | unit | `just test-one text_sizing_client` in `src/client/state.rs` tests | `try_present_frame_to` writes `pre` after `CSI ?2026h` and `post` before the final `CSI ?2026l`, commits host blocks only on success; `present_surface_patch` returns `false` while host blocks exist or when the patch contains a placeholder. |
| `V6` | `REQ3` | `DES4`, `DES5` | unit | `just test-one text_sizing` (registry, handshake profile, endpoint, config tests) | Registry: off at start, on with one supporting client, off when only unsupported clients remain, unchanged when all detach. Profile: kitty TERM or window id → true; ghostty/wezterm/xterm → false; tmux/screen → false; config `false` → false. Legacy hello JSON without `text_sizing` decodes to `false`, and serializing `false` omits the field so `endpoint-hello-v1.json` still matches. |
| `V7` | `REQ6` | `DES6` | unit | `just test-one text_sizing::text` | Text with a 2×2 `H` block becomes `H` with no marker characters; text without placeholders is returned unchanged (same allocation). |
| `V8` | `REQ1`–`REQ5` | `DES1`–`DES4` | Manual (live) | Separate session `herdr --session osc66-test` in a new kitty window, never the user's running server: run the user's probe script in a pane, then `bun run sandbox` in `/Users/hieunguyen/git/hieu/projects/so/.worktrees/kitty-text-sizing`, open `tour.md`; scroll, resize the pane, switch tabs/workspaces, open a popup over a heading; repeat in plain kitty and with `terminal.text_sizing = false` | Probe columns match plain kitty (2 / 3 / 3 with support; 2 / 1 / 1 with the switch off); H1 2×, H2 1.5×, `mc²` with a small raised 2; no leftover or shifted characters after each action. |
| `V9` | `REQ7` | `DES7` | Benchmark | `just bench-render-scale` on base and head | 1-pane and 15+-pane timings within run-to-run noise of the base; any change in scaling delta reported. |
| `V10` | `REQ1`–`REQ7` | all | Full suite | `just lint && just test` | Passes, including frozen endpoint fixtures, config-reference check, and UI hot-path architecture test. |

#### Test Strategy

- **Prioritize:** the cell contract (shared by server and client), cursor/CPR equality with kitty, the host erase-before-write ordering, and the gating that preserves today's behavior.
- **Skip:** Ghostty's own scroll/reflow/erase behavior (placeholders inherit it), serde boilerplate, trivial accessors.
- **Prove once:** placement and CPR at the `PaneTerminal` level (real core, no PTY); presenter on synthetic frames; client wiring once; live kitty rendering only manually, because kitty's pixels cannot be asserted in CI.
- **Use the project's layers:** Rust unit tests next to code, `AppState`-free; no new test infrastructure.

### Traceability

| REQ | Mockup | UDES | DES | V | Tasks |
|---|---|---|---|---|---|
| `REQ1` | — | — | `DES1`, `DES2`, `DES3` | `V1`, `V4`, `V8` | `G1-T1`, `G1-T2`, `G2-T1`, `G2-T2` |
| `REQ2` | — | — | `DES2` | `V2`, `V3`, `V8` | `G1-T2`, `G1-T3` |
| `REQ3` | — | — | `DES2`, `DES4`, `DES5` | `V3`, `V6`, `V8` | `G1-T3`, `G3-T1`, `G3-T2` |
| `REQ4` | — | — | `DES1`, `DES3` | `V4`, `V5`, `V8` | `G2-T1`, `G2-T2` |
| `REQ5` | — | — | `DES2`, `DES3` | `V3`, `V4`, `V8` | `G1-T3`, `G2-T1` |
| `REQ6` | — | — | `DES1`, `DES6` | `V1`, `V7` | `G1-T1`, `G1-T4` |
| `REQ7` | — | — | `DES7` | `V9`, `V10` | `G1-T3`, `G2-T2` |

## 2. Goal

### Status

- **State:** `In Progress`
- **Position:** `Overall verification`
- **Work branch:** `feat/kitty-text-sizing`
- **Base commit:** `41461ad7`

### Sources

- `.s1mple/S1MPLE.md` and its required references
- `.s1mple/docs/project.md`
- `.s1mple/docs/principles.md`
- Accepted mockups: `Not applicable — no UI`
- Prompts and template: `implementation-prompt.md`, `verification-prompt.md`, `remediation-prompt.md`, `verification-report-template.md` in the `s1mple-ship` skill's `templates/` directory

### Reports

```text
.s1mple/work/active/w261019eyld-kitty-text-sizing/reports/
├── g1-verification-report.md
├── g2-verification-report.md
├── g3-verification-report.md
├── overall-verification-report.md
└── harden-report.md
```

### Todos

```text
G1 (T1–T4) ← None: Server placeholder model
G2 (T1–T2) ← G1: Client presentation
G3 (T1–T2) ← G2: Capability negotiation and config
Overall verification
Harden
Deliver
```

### Execution Rules

- Start on the user's execution command. Record the work branch and base commit, create `<work-folder>/reports/`, then set the state to `In Progress`.
- Self-drive to the end. Ask no questions after the execution command.
- Investigate failures and try safe alternatives. If one group is blocked, continue the groups whose dependencies are met. When no safe, in-scope work remains, record the evidence and set `Blocked`.
- Do not change the decisions in Section 1, skip checks, or bypass permissions.
- Implementers commit their own changes. Do not push, amend, or rewrite history.
- Only the orchestrator writes this file. Commit only this file after each accepted group and after each later stage: `docs: update W261019EYLD ship evidence`.
- **Consultant:** For an exceptional blocker that normal investigation cannot solve, the orchestrator may invoke `s1mple-consultant` with the problem, evidence, and attempts. It advises only.
- **Subagent recovery:** If an agent that must continue is unavailable, spawn a fresh one of the same type with the original assignment, committed outputs, completed checks, and remaining work. Record the replacement under Evidence.

### Group Cycle

```text
fresh s1mple-implementer   implement → check → commit, per task
  → fresh s1mple-verifier  group scope, runs once → commits reports/g<n>-verification-report.md
  → Passed with no findings → accept
  → otherwise fresh s1mple-implementer remediates every finding in the report, once
  → orchestrator checks closure evidence → accept or block
```

1. Fill `implementation-prompt.md` and spawn a fresh `s1mple-implementer`.
2. Fill `verification-prompt.md` with scope `G<n>`, the implementer's change range and return, and report output `<work-folder>/reports/g<n>-verification-report.md`. Spawn a fresh `s1mple-verifier`. Confirm the report and its commit exist.
3. When the result is not `Passed` with no findings, fill `remediation-prompt.md` in `group` mode with the report path and spawn a fresh `s1mple-implementer`.
4. Accept the group only when every task has a commit SHA or a justified no-change reason, every finding in the report is `Fixed`, and all group checks pass. Record the evidence under [Groups](#groups).

Do not rerun verification or remediation. An unresolved finding blocks the group.

### Group Scheduling

Groups run sequentially in the main tree on `feat/kitty-text-sizing` (each depends on the previous one), which also shares one Cargo/Zig build cache.

## 3. Plan

```text
G1 Server placeholder model ──► G2 Client presentation ──► G3 Capability negotiation and config
```

### G1 — Server placeholder model

- **Depends on:** `None`
- **Context:** new `src/text_sizing.rs` and `src/text_sizing/{cell,tracker,inject,text}.rs`; `src/main.rs` (module line); `crates/ghostty-vt/src/lib.rs` (cursor accessors); `src/pane/terminal.rs` (core field, observe, ordered event); `src/pane.rs` (text decode in read wrappers).
- **Checks:** `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `just test-one text_sizing`
- **Accept when:** with `text_sizing::set_active_for_test(true)`, a `PaneTerminal` answers CPR like kitty for the probe and holds placeholder cells; with it off, behavior is unchanged; text reads show block text.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G1-T1` | Add the placeholder cell contract and module root | `REQ1`, `REQ6`, `DES1`, `DES4` | `src/text_sizing.rs`, `src/text_sizing/cell.rs`, `src/main.rs` | `TDD` | RED `V1` in `cell.rs` tests (round-trip, limits, malformed) → GREEN `Placeholder`, `BlockMeta`, `encode_cell`, `decode`, `is_placeholder`, constants; module root with `is_active`, `set_host_supported`/`host_supported`, `ClientRegistration` registry (`DES4` state only, wiring in G3) → REFACTOR names/docs | `just test-one text_sizing::cell` |
| `G1-T2` | Add the OSC 66 tracker | `REQ1`, `REQ2`, `DES2` | `src/text_sizing/tracker.rs` | `TDD` | RED `V2` → GREEN `TextSizingTracker::observe/drain_pending`, `TextSizingCommand`, meta parser with kitty clamping → REFACTOR share framing helpers inside the module | `just test-one text_sizing::tracker` |
| `G1-T3` | Inject placeholder blocks into the pane core | `REQ2`, `REQ3`, `REQ5`, `REQ7`, `DES2`, `DES7` | `src/text_sizing/inject.rs`, `crates/ghostty-vt/src/lib.rs`, `src/pane/terminal.rs` | `TDD` | RED `V3` in `src/pane/terminal.rs` tests (named `text_sizing_*`) → GREEN `cursor_x`, `cursor_pending_wrap`, `inject::apply` (placement per `A3`, shortcuts per `A10`), `GhosttyPaneCore` tracker + nonce fields, observe gated by `is_active`, `OrderedColorOrC1Event::TextSizing` arm → REFACTOR keep terminal.rs edits minimal | `just test-one text_sizing_` |
| `G1-T4` | Decode placeholders in text reads | `REQ6`, `DES6` | `src/text_sizing/text.rs`, `src/pane.rs` | `TDD` | RED `V7` → GREEN `decode_text`, apply in the `PaneRuntime` read wrappers listed in `DES6` → REFACTOR none | `just test-one text_sizing::text` |

### G2 — Client presentation

- **Depends on:** `G1`
- **Context:** new `src/text_sizing/present.rs` (plus its `mod` line in `src/text_sizing.rs`); `src/client/state.rs`; `src/protocol/render_ansi.rs` (expose SGR writer); `src/server/render_stream.rs` (fallback for ANSI clients).
- **Checks:** `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `just test-one text_sizing`, `just ui-hot-path-architecture-test`
- **Accept when:** composed frames with intact blocks produce `skip` cells, erase-before-write `pre`, and OSC 66 `post` bytes in the right places; unsupported hosts and broken blocks show fallback text; the patch path falls back while blocks exist.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G2-T1` | Add the frame presenter | `REQ1`, `REQ4`, `REQ5`, `DES3` | `src/text_sizing/present.rs`, `src/text_sizing.rs`, `src/protocol/render_ansi.rs` | `TDD` | RED `V4` → GREEN `Presenter`, `Pass`, block detection, fallback, `pre`/`post` encoding using the exposed SGR writer → REFACTOR fast path when no placeholder and no host blocks | `just test-one text_sizing::present` |
| `G2-T2` | Wire the presenter into client and server output | `REQ1`, `REQ4`, `REQ7`, `DES3`, `DES7` | `src/client/state.rs`, `src/server/render_stream.rs` | `TDD` | RED `V5` in `src/client/state.rs` tests (named `text_sizing_client_*`) → GREEN presenter field, `try_present_frame_to` insertion/commit, `present_surface_patch` fallback, fallback-only pass in `prepare_frame` → REFACTOR none | `just test-one text_sizing_client` |

### G3 — Capability negotiation and config

- **Depends on:** `G2`
- **Context:** `src/protocol/endpoint.rs`, `src/client/handshake.rs`, `src/client/mod.rs` and/or `src/client/loop_config.rs` (config flag), `src/server/client_transport.rs`, `src/config/model.rs`, `src/config.rs`, `docs/next/website/src/data/config-reference.json`.
- **Checks:** `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `just test-one text_sizing`, `just test-one endpoint`, `just maintenance-test`
- **Accept when:** a kitty-profile client advertises `text_sizing`, the server turns sizing on for its lifetime, legacy hellos still decode, and the config switch disables it.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G3-T1` | Advertise and register host support | `REQ3`, `DES4` | `src/protocol/endpoint.rs`, `src/client/handshake.rs`, `src/server/client_transport.rs`, `src/text_sizing.rs` (registry tests only) | `TDD` | RED `V6` (registry, profile, hello compatibility) → GREEN hello field, `text_sizing_profile_values`, `set_host_supported`, `ClientRegistration` guard in `handle_client_handshake` → REFACTOR none | `just test-one text_sizing`, `just test-one endpoint` |
| `G3-T2` | Add the `terminal.text_sizing` switch | `REQ3`, `DES5` | `src/config/model.rs`, `src/config.rs`, `src/client/mod.rs`, `src/client/loop_config.rs`, `docs/next/website/src/data/config-reference.json` | `TDD` | RED config default/opt-out test (`V6` config part) → GREEN field, `text_sizing_enabled()`, threading into the handshake, reference entry → REFACTOR none | `just test-one text_sizing`, `just maintenance-test` |

## 4. Overall Verification and Harden

Start after every group is accepted and merged.

```text
fresh verifier (overall, runs once) → Passed → harden
                                    → Failed → fresh implementer remediates all OVR findings, once → harden
                                    → Blocked → set Blocked
harden → Deliver
```

1. Fill `verification-prompt.md` with scope `overall`, change range `<base-commit>..HEAD`, all `V` cases, and report output `<work-folder>/reports/overall-verification-report.md`. Spawn a fresh `s1mple-verifier` in the main tree. Confirm the report and its commit exist. `V8` (live kitty) is run by the orchestrator in a separate `osc66-test` session and recorded under Evidence; `V9` runs on base and head.
2. When it fails, fill `remediation-prompt.md` in `overall` mode with the report path and spawn one fresh `s1mple-implementer` for every `OVR-VER<n>` finding.
3. Run `s1mple-harden` in workflow mode. Use this file as the goal and **Base commit** as the base.

## 5. Deliver

1. Read `.s1mple/docs/project.md`. Add the Fork Customizations row for OSC 66 text sizing (files touched, conflict risk, notes) and update its Last Modified date. Commit only that file:

   ```text
   docs(project): record kitty text sizing customization

   Work: W261019EYLD — kitty-text-sizing
   ```

2. Apply the Delivery behavior in `.s1mple/S1MPLE.md`. Do not push, open a PR, merge, or deploy without the user's authorization.
3. Fill [Completion Summary](#completion-summary), set the state to `Completed`, and commit this file.

## 6. Evidence

### Groups

| Group | Directory | Task commits | Verification report → status, SHA | Findings → fix SHAs | Merge SHA | Result |
|---|---|---|---|---|---|---|
| `G1` | main tree | T1 `f109a14b`, `6f3326a5`; T2 `b1dda237`; T3 `25d89a28`; T4 `7d0e37e5` | `reports/g1-verification-report.md` — `Failed` (1 Minor), `80958df5` | `G1-VER1` → `54d3804d` (tracker reset while inactive; `just test` 3939 passed) | Not applicable — main tree | `Accepted` |
| `G2` | main tree | T1 `f10f75e0`; T2 `de1ef1da` | `reports/g2-verification-report.md` — `Failed` (1 Critical, 2 Minor), `09662073` | `G2-VER1` → `75463502` (control characters rejected in decode, encode, and OSC 66 writer); `G2-VER2` → ship-file clarification `24a0cf48`; `G2-VER3` → `2292263f` (direct-terminal fallback test); `just test` 3949 passed | Not applicable — main tree | `Accepted` |
| `G3` | main tree | T1 `aed587b0`; T2 `f5232099` | `reports/g3-verification-report.md` — `Failed` (1 Minor), `2bff8787` | `G3-VER1` → `4fa2aaa7` (config reference wording; `just maintenance-test` passed) | Not applicable — main tree | `Accepted` |

### Verification Cases

| Case | Actual result |
|---|---|
| `V1` | `Pending` |
| `V2` | `Pending` |
| `V3` | `Pending` |
| `V4` | `Pending` |
| `V5` | `Pending` |
| `V6` | `Pending` |
| `V7` | `Pending` |
| `V8` | `Pending` |
| `V9` | `Pending` |
| `V10` | `Pending` |

### Overall Verification

- **Report:** `reports/overall-verification-report.md` — `Pending`
- **Result and report SHA:** `Pending`
- **Findings → fix SHAs:** `Pending`

### Harden

- **Report:** `reports/harden-report.md` — `Pending`
- **Result:** `Pending`

### Deliver Evidence

- **Project document:** `Pending`
- **Delivery actions:** `Pending`

### Blockers, Deviations, and Replacements

- **Planned deviation:** kitty lower-row displacement is not emulated (`A4`).
- **REQ5 clarification (G2-VER2):** `REQ5` said remaining cells "render blank" while `A8`/`REQ4`/`V4` specify top-row text fallback. The client cannot tell an overwrite from a popup covering the same cells, and kitty itself keeps a block visible after a lower-row write, so one fallback rule for every non-drawable block is kept (`A8`). `REQ5` wording aligned with `A8`; no behavior change. Resolved by the orchestrator in the ship file.

### Completion Summary

`Pending`

## Ship File Check

- [x] Every `REQ` has at least one `UDES` or `DES`, one `V`, and one task.
- [x] Every UI `REQ` has a `UDES` derived from an explicitly accepted mockup revision. (No UI `REQ`.)
- [x] No `DES` describes the UI, and no `UDES` has a `pending` `Uses`.
- [x] Every `UDES` and `DES` has a `V` and a task.
- [x] The Q&A Log has a row for every answered question and assumption.
- [x] When UI is needed, the mockup manifest is `Accepted`, and its accepted revisions match the UI Mockups section. (Not needed.)
- [x] Every task cites the `REQ`/`UDES`/`DES` it builds, and every `V` cites the `REQ` it proves and the `UDES` or `DES` it checks.
- [x] There are at most 4 groups, and no dependency cycles.
- [x] Each group's tasks share one context.
- [x] Parallel groups do not edit the same files, or a dependency orders them.
- [x] Every `V` has a runnable command or a concrete manual method.
- [x] Every `V` covers a unique, important risk at the lowest layer that catches it; no trivial or redundant case.
- [x] Every task has an approach, and every `V` that needs a new test is owned by a `TDD` task.
- [x] No placeholder remains.
