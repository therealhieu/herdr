# W261019EYLD: kitty-text-sizing

## Sources

| Source | Use |
|---|---|
| `.s1mple/S1MPLE.md` | Discuss behavior and required context |
| `.s1mple/docs/project.md` | Reusable project baseline |
| `vendor/libghostty-vt/src/terminal/osc.zig`, `osc/parsers/kitty_text_sizing.zig`, `stream.zig`, `page.zig` | Whether the vendored terminal core parses and stores OSC 66 |
| `src/pane/terminal.rs`, `src/pane.rs`, `src/pane/osc.rs` | PTY byte path, query replies, pane env, existing OSC side trackers |
| `src/protocol/wire.rs`, `src/protocol/endpoint.rs`, `src/protocol/render_ansi.rs`, `src/protocol/surface_scroll.rs` | Cell and frame wire types, frozen codecs, host blitting |
| `src/client/state.rs`, `src/client/frame_output.rs`, `src/client/handshake.rs`, `src/client/terminal_setup.rs` | Client composition, host capability detection |
| `src/kitty_graphics.rs`, `src/kitty_graphics/output.rs`, `src/config.rs` | Existing "model in emulator, re-emit to host" precedent |
| [kitty text sizing protocol](https://sw.kovidgoyal.net/kitty/text-sizing-protocol/) | External spec named by the user (not fetched during exploration) |

## Roadmap Binding

None — standalone work

### Previous Accepted Bindings

None

### Parent Synchronization

- **Registration or discussion update:** None

## 1. User Input

### Original Input

Request from the `so` project (terminal Markdown editor at `/Users/hieunguyen/git/hieu/projects/so`): make herdr support kitty's text sizing protocol (OSC 66), so `so` running inside herdr inside kitty can draw large headings and half-size scripts. Follow this repo's own workflow and instructions.

**Problem.** `so` uses OSC 66 (therealhieu/so PR #43). It works in plain kitty 0.49.2. Inside herdr 0.9.5 (same kitty), herdr drops the whole escape sequence, including its text, so `so` sees no support and falls back to normal text.

**Evidence (measured 2026-10-19)**, probe writes each case on a fresh line then sends CPR (`ESC[6n`); probe script in user request (`python3 -I probe66.py <out-file>`):

| Case | Plain kitty 0.49.2 | Inside herdr 0.9.5 |
|---|---|---|
| plain `a` | col 2 | col 2 |
| `ESC]66;w=2;a ESC\` | col 3 | col 1 (nothing drawn) |
| `ESC]66;s=2;a ESC\` | col 3 | col 1 (nothing drawn) |
| `TERM` | `xterm-kitty` | `xterm-256color` |

**What `so` sends (OpenTUI 0.5.14):**

- Startup probe: `ESC]66;w=1; ESC\` and `ESC]66;s=2; ESC\`, each followed by CPR; sizing is enabled only if the cursor moved as kitty moves it. Herdr must answer CPR the way kitty would, and only when it can really draw sized text.
- Width-pinned glyphs: `ESC]66;w=1;∼ ESC\` (and `▌ • │`) for non-ASCII glyphs.
- Headings, after each frame in one write wrapped in `CSI ?2026h ESC7 … ESC8 CSI ?2026l`, each placed with CUP and SGR: H1 `ESC]66;s=2;text ESC\` (2×, 2 rows); H2 chunks of ≤4 chars `ESC]66;s=2:w=<cells/2>:n=3:d=4:v=2;text ESC\` (1.5× inside 2 rows).
- Scripts: `ESC]66;n=1:d=2:v=0;xy ESC\` (superscript) and `v=1` (subscript), with `w=1`: two half-size characters in one cell.
- Erasing: before a frame changes a sized area, `so` writes CUP + ECH (`CSI n X`) on every row of the old block, then redraws.

**kitty rules apps depend on:** writing the top-left cell of a sized block erases the whole block; writing another top-row cell blanks that cell; writing a cell in a lower row pushes text right; after a block's top row is overwritten, an OSC 66 glyph written into its old lower row is still displaced unless every row was erased (ECH) or plainly written first.

**Expected result:** inside herdr in kitty the probe matches plain kitty and sized text shows at the right place and size; scrolling, pane resize, tab/pane/workspace switches, and redraws keep sized blocks aligned with no leftover or shifted characters; when the outer terminal cannot size text, herdr keeps today's behavior so apps detect no support and fall back.

**Live test:** use a separate session (`herdr --session osc66-test` in a new kitty window), never the user's running session or server. Check with `so`: `cd /Users/hieunguyen/git/hieu/projects/so/.worktrees/kitty-text-sizing && bun run sandbox`, open `tour.md`, compare with plain kitty (H1 2×, H2 1.5×, `$E = mc^2$` with a small raised 2).

### Consolidated Requirement or Problem

Herdr should support kitty's text sizing protocol (OSC 66) when the outer terminal is kitty, so apps inside a pane can draw scaled text (`s=`, `w=`, `n`/`d`, `v`/`h`) at the right place, with CPR answers matching kitty, and keep current behavior (no support detected) when the outer terminal cannot size text.

### Constraints and Assumptions

- Fork policy: no fork-only changes to the wire protocol, `PROTOCOL_VERSION` (22, `src/protocol/wire.rs:20`), or frozen endpoint codecs/fixtures; add advertised capabilities or optional data instead (`.s1mple/docs/project.md` Data and Contracts).
- Minimize upstream diff; prefer new files; record the customization in `project.md` Fork Customizations.
- Vendored libghostty-vt changes must be indexed patches in `vendor/libghostty-vt.patches.md` that reverse-apply cleanly (checked by `just test`).
- PTY parsing, pane render, and client blit are multiplicative performance paths; a change there needs a 1-pane vs 15-pane profile (`just bench-render-scale`).
- macOS only; verify with `just lint && just test`.
- Assumption: the target host is kitty ≥ 0.40 (text sizing shipped in kitty 0.40); the user measured 0.49.2.
- Assumption: the local TUI client (not SSH/remote) is the primary path; remote attach behavior is unknown and not yet scoped.

## 2. Project Structure

- `vendor/libghostty-vt/` — vendored terminal core (source commit `44f2a44d`, `vendor/libghostty-vt.vendor.json`); parses OSC 66, owns the cell grid, cursor, scrollback, reflow, and CPR.
- `vendor/patches/libghostty-vt/`, `vendor/libghostty-vt.patches.md` — local patch set (precedent for core changes).
- `crates/ghostty-vt/src/lib.rs` — Rust FFI over the C API; cell iteration exposes `CellWide` only (`lib.rs:464`).
- `src/pane/terminal.rs` — per-pane PTY byte processing, query replies, ratatui render of the grid.
- `src/pane/osc.rs` — herdr-side byte-stream OSC trackers (default colors, agent OSC, debug) that run before bytes reach the core.
- `src/pane.rs` — pane spawn env (`TERM`), PTY reader loop, clipboard/event fan-out.
- `src/protocol/wire.rs` — `CellData`, `FrameData`, `PaneSurfaceFrame`, `PaneSurfacePatch`, `ServerMessage`.
- `src/protocol/endpoint.rs` — stable endpoint codecs and capabilities.
- `src/protocol/render_ansi.rs` — `BlitEncoder`: diff-based ANSI output to the host terminal.
- `src/client/state.rs`, `src/client/frame_output.rs` — client composition and host write.
- `src/client/handshake.rs`, `src/client/terminal_setup.rs` — host terminal profile and startup queries.
- `src/kitty_graphics.rs`, `src/kitty_graphics/` — kitty graphics re-emission to the host.

## 3. Standards and Patterns

### Applicable Standards

- `AGENTS.md` "Multiplicative performance paths" — PTY parsing, render, and client fanout work must be profiled at 1 and ≥15 panes; keep terminal-core lock duration minimal.
- `AGENTS.md` "Runtime/client boundary guardrail" — sized-cell facts in pane terminal state are runtime facts; host-terminal capability and escape emission are client presentation.
- `AGENTS.md` "Stable client endpoint contract" — `shell.surface.v1` (`src/protocol/endpoint.rs:20`) and reachable types (`CellData`, `FrameData`) are append-closed; new data needs a new codec or advertised capability.
- `AGENTS.md` "Vendored libghostty-vt" — every local patch indexed with removal condition.
- `AGENTS.md` "Principles" — detection reads screen snapshots; render is pure.
- `src/protocol/wire.rs:697-710` — `CellData { symbol, fg, bg, modifier: u16, skip, hyperlink }`; `modifier` holds ratatui bits plus underline style in bits 12–15 (`wire.rs:1576-1577`).

### Existing Patterns

- `src/protocol/surface_scroll.rs:15-16` — optional delivery behind an advertised capability (`surface_scroll`) with its own frozen message kind; the receiver expands into ordinary patches.
- `src/protocol/endpoint.rs:23-24` — capability constants (`surface_interest`, `presentation_effects_fence`) advertised in the handshake.
- `src/kitty_graphics.rs` + `src/protocol/wire.rs:774-775` — kitty graphics: the core stores images/placements; herdr re-encodes them as host escape bytes carried in `FrameData::graphics` / `SurfaceGraphicsScene` and written after the text frame (`src/client/frame_output.rs:11-19`). Enabled by `terminal.kitty_graphics` (`src/config.rs:100-105`, default true).
- `src/client/handshake.rs:57-69` — host profile by `TERM_PROGRAM`, `TERM` (`xterm-kitty` etc.), `KITTY_WINDOW_ID`; blocked under SSH/tmux/screen.
- `src/pane/terminal.rs:1491-1560` — herdr inserts its own replies in byte order around core writes (default-color and C1 XTGETTCAP replies), splitting the write at event offsets.
- `src/pane/terminal.rs:1423`, `src/pane.rs:2614-2621` — core effect drained after write (clipboard) → `AppEvent::ClipboardWrite` → `ServerMessage::Clipboard` (`wire.rs:1422`).
- `src/client/bold_color.rs` — prior fork customization: client-only transform of `FrameData` cells before `BlitEncoder`.

## 4. Architecture

```text
app in pane ─PTY bytes─► Pane reader (src/pane.rs)
                           │ process_pty_bytes (src/pane/terminal.rs:1353)
                           ├─► herdr OSC trackers (src/pane/osc.rs)   [observe only]
                           └─► libghostty-vt Terminal.write ──► grid / scrollback / cursor
                                     │ OSC 66 → parsed, then dropped (stream.zig:2695)
                                     │ CPR → write_pty callback → pending_pty_responses → PTY
                                     ▼
              PaneTerminal::render (terminal.rs:2357) → ratatui Buffer (1 cell = 1 symbol)
                                     ▼
              server: PaneSurfaceFrame / PaneSurfacePatch (shell.surface.v1, bincode)
                                     ▼
              client: ComposedFrame → bold_color → BlitEncoder (render_ansi.rs:60) → kitty
```

- libghostty-vt — the only emulator: owns cell grid, cursor movement, scrollback, reflow, alternate screen, and CPR.
- Pane terminal (`src/pane/terminal.rs`) — wraps the core under a mutex; adds herdr-specific replies and trackers.
- Server surface pipeline — converts pane grids into `CellData` rows and patches.
- Client — composes pane surfaces with chrome and blits diffs; owns host capability knowledge.

## 5. Code Flow

```text
bytes → process_pty_bytes → trackers.observe → write_pty_bytes_with_ordered_responses
      → core.write → (OSC 66 parsed: log.debug "unimplemented", text discarded)
      → CPR reply from core (cursor never moved) → PTY
render → RowIterator/RowCells → CellWide {Narrow, Wide, SpacerTail, SpacerHead} → ratatui cell
      → CellData → PaneSurfacePatch → client → BlitEncoder diff → host
```

1. The PTY reader calls `process_pty_bytes` (`src/pane/terminal.rs:1353`), which runs byte-level trackers, then writes bytes to the core in event-ordered slices (`terminal.rs:1491`).
2. The core OSC parser recognizes OSC 66 (`vendor/libghostty-vt/src/terminal/osc.zig:155-156, 947`) into `OSC { scale: u3, width: u3, numerator: u4, denominator: u4, valign, halign, text }` (`osc/parsers/kitty_text_sizing.zig:29-37`, payload max 4096).
3. Stream dispatch lists `.kitty_text_sizing` among "unimplemented OSC callback" arms and only logs (`vendor/libghostty-vt/src/terminal/stream.zig:2686-2699`). The payload text is never printed, so the cursor does not move — matching the user's "col 1, nothing drawn" measurement.
4. CPR (`ESC[6n`) is answered by the core (`stream_terminal.zig:1341`) through the write-pty callback into `pending_pty_responses` (`src/pane/terminal.rs:1158`), drained and sent to the PTY. Herdr does not compute CPR itself.
5. The core cell model has no multicell/scale storage: `Cell.Wide` is a 2-bit enum `narrow | wide | spacer_tail | spacer_head` (`vendor/libghostty-vt/src/terminal/page.zig:2213-2225`); no `multicell`/`scale` symbols exist in `page.zig` or `PageList.zig`.
6. Render walks the core's render state row by row and writes one ratatui cell per grid cell (`src/pane/terminal.rs:2357-2470`); the dirty-patch path is `ghostty_collect_dirty_patch` (`terminal.rs:2617`).
7. The client receives surfaces/patches, composes `FrameData`, applies `bold_color`, and `BlitEncoder` writes only changed cells with CUP+SGR+symbol (`src/client/state.rs:570-597`, `src/protocol/render_ansi.rs:84-128`). Scroll-aware delivery shifts retained rows (`src/protocol/surface_scroll.rs`).
8. Pane env sets `TERM=xterm-256color` (`PANE_TERM`, `src/pane.rs:72,105`), `TERM_PROGRAM=herdr`, and strips `KITTY_WINDOW_ID` (`src/pane.rs:125`), so apps cannot identify kitty and must probe.

## 6. Boundaries

### In Scope

- Making OSC 66 sequences inside a pane produce correctly sized and positioned text on a kitty host, with kitty-equivalent cursor movement and CPR.
- Pane grid semantics for sized blocks (overwrite, erase, ECH, displacement rules listed by the user), across scroll, resize/reflow, alternate screen, and tab/pane/workspace switches.
- Host capability gating: no behavior change when the host cannot size text.
- Fork bookkeeping: patch index, Fork Customizations row, tests, perf profile.

### Out of Scope

- Changing `TERM` advertised to panes.
- Upstream contribution (fork policy).
- Windows/Linux verification (fork is macOS only; compile-gating still applies).
- Changes to the `so` project or OpenTUI.

### Interfaces and Dependencies

- libghostty-vt OSC parser and stream handler (vendored Zig; patch required for any core semantics).
- `crates/ghostty-vt` FFI (`bindings.rs` already has `GHOSTTY_OSC_COMMAND_KITTY_TEXT_SIZING = 22`, `bindings.rs:4108`, but no cell-level accessor for size data).
- `shell.surface.v1` codec (`CellData`, `FrameData`, `PaneSurfacePatch`) — frozen; frozen fixtures `tests/fixtures/endpoint-*-v1.json` and shape digests (`src/server/client_commands.rs:192`).
- Endpoint capability negotiation (`src/protocol/endpoint.rs`).
- Client host profile (`src/client/handshake.rs:57-95`) and startup queries (`src/client/terminal_setup.rs:175-176`, DA1 + kitty keyboard + 1016 only; no XTVERSION or OSC 66 probe).
- Agent detection reads core screen snapshots; sized text that is not in the grid is invisible to detection.

## 7. Unknowns

- Whether upstream Ghostty (newer than `44f2a44d`) has implemented OSC 66 grid storage that could be vendored instead of patched; not checked.
- How kitty stores sized blocks precisely (multicell metadata per cell, how `w=0` width is computed from text, `n/d` fractional cell semantics, CPR column after `s=2` with multi-char text, scrollback/reflow behavior of partially scrolled blocks); requires reading the spec and/or kitty source.
- Where per-cell size metadata could travel without changing `shell.surface.v1`: a new codec, an advertised optional side channel (like `surface_scroll` or graphics scene), or re-emitting raw OSC 66 bytes; unused `modifier` bits 9–11 exist but reinterpreting them changes a frozen codec's meaning.
- Whether the server knows the host can size text: today only the client inspects `TERM`/`TERM_PROGRAM`/`KITTY_WINDOW_ID`; CPR replies come from the server-side core, so a "host supports sizing" fact must reach the pane before apps probe. Behavior with multiple attached clients of differing capability is undefined.
- Remote/SSH attach and `herdr --session` direct-terminal mode (`ServerMessage::Terminal`) behavior.
- Interaction with copy mode, selection, search, and `herdr agent read` text extraction for sized text.
- Performance cost in render and blit loops at 15 panes; no measurement yet.
- Whether kitty requires a matching block to be fully redrawn (erase + re-emit) after partial overwrites, which affects `BlitEncoder` diff granularity.
