# Harden Report

- **Outcome:** `Findings identified`
- **Mode:** `Workflow`
- **Target:** `.s1mple/work/active/w261019eyld-kitty-text-sizing/` (W261019EYLD kitty-text-sizing, ship file `w261019eyld-ship.md`)
- **Range:** `41461ad7..HEAD`
- **Reviewed:** `src/text_sizing.rs`, `src/text_sizing/{cell,inject,present,text,tracker}.rs`, and the edits in `src/pane/terminal.rs`, `src/pane.rs`, `src/client/{state,handshake,mod}.rs`, `src/server/{client_transport,render_stream}.rs`, `src/protocol/{endpoint,render_ansi}.rs`, `src/config.rs`, `src/config/model.rs`, `crates/ghostty-vt/src/lib.rs`, `docs/next/website/src/data/config-reference.json`, plus the `BlitEncoder` diff writer they rely on (`src/protocol/render_ansi.rs:1006`)
- **Not reviewed:** `.s1mple/` documents and `vendor/` (excluded by the skill and `project.md`)

## Summary

| Category | Major | Minor |
|---|---|---|
| Correctness | 0 | 0 |
| Test Gap | 0 | 0 |
| Redundancy | 0 | 0 |
| Dev Speed | 0 | 0 |
| Workflow Leak | 0 | 0 |
| Principles | 0 | 1 |

No correctness defects were confirmed. Placement and cursor logic match kitty's `handle_fixed_width_multicell_command`. The tracker resets while inactive. Decoding rejects control characters before any host write. The encoder's `to_skip` handling keeps a wide fallback grapheme intact. The only finding is fork-policy upstream diff size: about 480 lines of new tests sit inside upstream hot files, so they should move into child test modules before the next upstream sync.

## Findings

### HRD-1 — Minor — Principles — Text sizing tests enlarge upstream hot files

**Evidence**

`src/pane/terminal.rs:7650`

```rust
    fn text_sizing_pane(cols: u16, rows: u16) -> (GhosttyPaneTerminal, mpsc::Sender<Bytes>) {
```

`src/client/state.rs:891`

```rust
mod text_sizing_tests {
```

`src/server/render_stream.rs:869`

```rust
    fn text_sizing_direct_terminal_frames_carry_fallback_text_only() {
```

`git diff --numstat 41461ad7..HEAD` counts +359 lines in `src/pane/terminal.rs` (about 333 of them are text sizing tests appended to the end of upstream's `mod tests`), +128 in `src/client/state.rs` (about 110 in an inline test module), and +40 in `src/server/render_stream.rs` (37 test lines). The fork policy says to minimize the upstream diff and prefer new files (`.s1mple/docs/project.md` Fork Policy rule 4; `.s1mple/S1MPLE.md` Implementation: "prefer new files, avoid unrelated edits to upstream files").

**Consequence:** Upstream regularly appends tests to the end of these same test modules, especially `src/pane/terminal.rs` (7983 lines). Every `upstream/master` sync that touches those tails will conflict with several hundred fork-only lines. That makes the cheap merge the fork policy promises harder.

**Recommendation:** Move each block into a child test module file declared with one line in the upstream file. Child modules keep access to the parent's private items:
- `#[cfg(test)] mod text_sizing_tests;` in `src/pane/terminal.rs`, backed by `src/pane/terminal/text_sizing_tests.rs`.
- The same for `src/client/state.rs` → `src/client/state/text_sizing_tests.rs`.
- The same for `src/server/render_stream.rs` → `src/server/render_stream/text_sizing_tests.rs`.

Keep the test names, so `just test-one text_sizing` still selects them.

## Checks

| Command | Result | Duration |
|---|---|---|
| `just lint` | `Passed` — `cargo fmt --check` clean; clippy `-D warnings` clean | 3 s (warm cache) |
| Workflow-leak scan (`rg -nP …` over `git diff --name-only 41461ad7..HEAD`, excluding `.s1mple/`) | `Passed` — no matches | <1 s |
| `just test` | `Passed` — nextest 3956 passed, 16 skipped; maintenance, UI hot-path, integration-asset, and docs-contract suites passed | 48 s |
| Manual scan for `unwrap()`/`expect(`, sleeps, and spawned threads in the new code | `Passed` — every `unwrap`/`expect` is in `#[cfg(test)]` code; no sleeps; the two profiles are `#[ignore]` | <1 s |

## Limits

- **Unverified areas:** Live host behavior was not re-run in this review; ship evidence `V8` covers it. Windows `not(unix)` stubs were not compiled (macOS-only fork). Two acknowledged deviations were not treated as findings, because ship `A3`/`A4` decided them: kitty's lower-row displacement, and scroll-region-height discard.
- **Blockers:** None

## Resolution

Workflow mode only. The implementer fills this section; the sections above stay unchanged.

- **Status:** `Pending`

| Finding | Change | Verification | Commit | Result |
|---|---|---|---|---|
| `HRD-1` | `Pending` | `Pending` | `Pending` | `Pending` |

- **Final checks:** `Pending`
- **Remaining issues:** `Pending`
