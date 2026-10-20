# W261020LXIN G3 Verification Report

## Scope

- **Scope:** `G3`
- **Tasks:** `G3-T1`, `G3-T2`
- **Change range:** `2d7f9f53..59484467`
- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/g3-verification-report.md`

## Status

`Failed`

Every assigned row is met and every group check passes. One Minor finding remains: pending agent resume still decides borders by leaf count, so a single-slot stack resumes its agent at the borderless size (`G3-VER1`).

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES5` (hidden members sized to the slot) | `V6` | `src/ui/panes.rs:348` `resize_hidden_subpanes`, called from `resize_tab_panes` (zoomed and tiled) and `compute_pane_infos_for_tab` (zoomed and tiled, only when `resize_panes`). Each call uses the same `terminal_inner_rect` the visible member uses. Test `subpane_hidden_member_is_sized_with_its_slot` (`:1902`) covers tiled and zoomed. | `Met` |
| `REQ1` | `UDES1` | `V6` | `src/subpane_strip.rs:70` `strip_layout` starts at `x = 1`, uses ` {g} {label} ` segments with a 1-cell separator, and is drawn in place of the title at `src/ui/panes.rs:846`–`:851`. Rendered row `┌ ● claude ─   shell × ─ + ─` (`:1853`). | `Met` |
| `REQ1` | `UDES2` | `V5`, `V6` | `Close(active)` follows the active member, and `Add` follows the last member (`src/subpane_strip.rs:91`–`:118`). Plain panes keep `pane_border_title` (`src/ui/panes.rs:852`). | `Met` |
| `REQ1` | `UDES5` | `V6` | `render_strip` label styles: focused active `accent`+bold, unfocused active `text`+bold, others `overlay0` (`src/subpane_strip.rs:246`–`:255`). The test asserts accent at column 15. | `Met` |
| `REQ1` | `UDES8` | `V6` | `tab_multi_pane` (`src/ui/panes.rs:55`) feeds all four border decisions. The strip draws only when `info.borders.contains(TOP)` (`:843`). Test `subpane_strip_falls_back_to_badge_and_needs_a_top_border` (`:1887`) covers `Borders::ALL` for a single slot and no strip with `pane_borders = off`. | `Met` |
| `REQ1` | `UDES13` | `V6` | When zoomed, `zoomed_pane_borders(app, tab_multi_pane(tab))` gives the zoomed pane a top border, and `render_pane_border_titles` draws the strip on that `PaneInfo`. Hidden members follow the zoomed size (`:1902`). Zoomed strip drawing has no render test (see Test Coverage). | `Met` |
| `REQ3` | `UDES3` | `V5` | `subpane_label` (`src/subpane_strip.rs:191`) checks manual label, then agent display name, then stripped title, then `shell`, and `fit_label` truncates to 20 columns with `…`. Test `subpane_strip_label_follows_manual_agent_title_shell_order`. | `Met` |
| `REQ3` | `UDES4` | `V5`, `V6` | The glyph comes from `crate::client::status_icon(status, app.status_indicators)` and the color from `status_color`. Status is `None` when `effective_agent_label()` is `None`, which leaves the slot blank (`src/ui/panes.rs:894`). Geometry does not take status as input. The render test shows identical rows for Working and Blocked. | `Met` |
| `REQ3` | `UDES6` | `V6` | Status comes from `pane_agent_status(terminal.state, pane.seen)`, the same function the API and sidebar use. Hidden blocked renders a red `×` before the label (test `:1853`). | `Met` |
| `REQ3` | `UDES14` | `V6` | The close `×` appears only after the active label (`Close` segment), and the status glyph sits at `x + 1`, before the label. The test checks the symbols-style blocked `×` at column 2. | `Met` |
| `REQ4` | `UDES7` | `V5`, `V6` | Badge mode (`src/subpane_strip.rs:121`–`:148`): active member, then `× `, then a right-aligned `[i/n g]`. `g` comes from `most_urgent_hidden_status` (`:306`), which skips Idle and Unknown. Badge mode has no `+`. Rendered row: `┌   shell × ─[2/2 ●]─┐` at width 22. | `Met` |
| `REQ3`, `REQ4` | `DES5` (pure `strip_layout`, signature, status-independent geometry) | `V5` | `pub(crate) fn strip_layout(labels: &[&str], active: usize, width: u16) -> StripLayout` matches DES5. The `x` value is relative to the slot. Five unit tests cover full fit, one column short, badge truncation, minimum width, and fewer than 2 members or an out-of-range active index. | `Met` |
| `REQ7` | `DES5` (hidden members are not render sources) | `V6` | `src/server/headless/render.rs:442` adds `!tab.subpanes.is_hidden(pane_id)`. `src/app/state.rs:988` and `src/server/headless/client_views.rs:515` already use `layout.pane_ids()`. Test `subpane_hidden_member_is_not_a_visible_render_source` (`src/server/headless/tests/mod.rs:1613`) checks that a hidden member is not visible, and that it becomes visible after activation while the old leaf is no longer visible. | `Met` |
| `REQ7` | `DES5` (strip work only for stacked slots) | `V6` | Plain panes cost only `tab.subpanes.is_empty()` checks in `resize_hidden_subpanes` and `render_pane_border_titles`. `tab_pane_chrome` makes the same single `Vec` as before, and plain panes get no new allocation or lock. `just ui-hot-path-architecture-test` passed. Render scaling (`V11`) is overall scope. | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V5` | `just test-one subpane_strip` | 7 tests run, 7 passed (5 `strip_layout`/label unit tests and 2 render tests) | `Passed` |
| `V6` | `just test-one subpane` | 20 tests run, 20 passed (includes the 2 strip render tests, `subpane_hidden_member_is_sized_with_its_slot`, `subpane_hidden_member_is_not_a_visible_render_source`) | `Passed` |
| Group lint | `just lint` | `cargo fmt --check` and `cargo clippy --all-targets --locked -D warnings` clean, exit 0 | `Passed` |
| Group full suite | `just test` | nextest 3984 passed, 16 skipped. The UI hot-path architecture test (6 tests) and the script, bun, and docs-contract tests passed. Exit 0. | `Passed` |
| UDES vs `M1@R1` | Compared the rendered test buffers with `mockups/m1-subpane-strip.html` (`leafHTML` style A and D, CSS `.st`, `.x`, `.add`, `.badge`) | Style A strip on the top border, a separated member per segment, close on the active member, `+` after the last member, and a right-aligned `[i/n g]` badge with the worst hidden status all match. The differences are the status glyph before the label (mockup: after), no selected-tab background, `×` visible in badge mode, and no `+` on plain panes. `UDES1`, `UDES2`, `UDES5`, `UDES7`, and `UDES14` decide each of these, and the ship file's UI Mockups Limits say that the TUI adaptation lives in the `UDES` rows. | `Passed` |

## Integration

Not applicable — group scope

## Findings

### G3-VER1 — Minor — Correctness — Pending agent resume sizes a single-slot stack as borderless

- **Builds:** `REQ1`, `DES5`, `UDES8` (`A15`: a slot holding a stack counts as multi-pane for border decisions), `G3-T2`
- **Evidence:** `src/app/agent_resume.rs:323`–`:335` still derives resume geometry through `apply_pane_chrome`, which sets `multi_pane = panes.len() > 1`:

  ```rust
  fn derived_pending_agent_resume_pane_infos(tab: &crate::workspace::Tab, ...) -> Vec<crate::layout::PaneInfo> {
      crate::ui::apply_pane_chrome(
          tab.layout.panes(terminal_area),
          pane_borders, pane_gaps, pane_outer_borders,
      )
  ```

  For a tab whose only slot holds a stack, this returns `Borders::NONE` under `pane_borders = auto`. The renderer's `tab_pane_chrome` (`src/ui/panes.rs:59`) returns `Borders::ALL`. `pending_agent_resume_candidates` takes `rows`/`cols` from this inner rect (`src/app/agent_resume.rs:133`–`:134`) for every tab except the active one, which is overridden from `view.pane_infos` (`:157`–`:173`). The other external caller, `pane_layout_snapshot` (`src/app/api/panes.rs:2148`), publishes only outer `rect`. For a single slot, `rect` does not depend on `multi_pane`, because gaps need neighbors. That caller is therefore unaffected.
- **Impact:** After a restart, a resumed agent in a single-slot stack on a background tab starts its PTY 2 columns and 2 rows too large. The next `resize_tab_panes` pass then resizes it, so the agent reflows once. This contradicts `A15`/`DES5`, which say border decisions treat a stacked slot as multi-pane. It also leaves two sources of truth for the chrome decision.
- **Resolution:** Derive pending-resume geometry from the stack-aware decision, for example by exposing `tab_pane_chrome`/`tab_multi_pane` from `src/ui` and calling it from `derived_pending_agent_resume_pane_infos`. A resumed single-slot stack member then gets the same inner size the renderer gives it.
- **Proof to close:** A unit test builds a single-slot tab holding a stack under `pane_borders = auto`. It shows that the pending-resume pane info for the visible member has `Borders::ALL` and the same inner size as `compute_pane_infos_for_tab`. `just lint` and `just test` pass.

## Test Coverage

- **Covered:** strip geometry in full mode, the one-column-short switch to badge, badge label truncation, below-minimum and invalid input, and the label fallback chain with 20-column truncation (unit). Strip on a single-slot border with the hidden member's status glyph and color, focused accent, symbols-style blocked `×`, status-independent geometry, badge row with the urgent hidden status, and no strip with `pane_borders = off` (render). Hidden member sizing in tiled and zoomed compute. Hidden member excluded from visible render sources and included after activation (headless server).
- **Missing:** No render test for the strip on a zoomed slot (`UDES13`). No test for the background-tab path `resize_tab_panes`, which calls the same `resize_hidden_subpanes` helper as the tested path. Both are low risk because they share the tested code. No test for pending-resume geometry (`G3-VER1`).
- **Low-value:** None

## Scope Changes

- Added `AppState.status_indicators`. It is set from `config.ui.status_indicators` at construction and on config reload (`src/app/mod.rs:543`, `:910`; `src/app/state.rs:838`). The server renderer needs this field to follow `UDES4`, so it is a justified, minimal addition.
- `src/app/mod.rs` and `src/client/mod.rs` gained one-line `pub(crate)` re-exports for `pane_agent_status` and `status_icon`/`status_color`/`status_priority`. This is the visibility widening that `DES5` names.
- `new_pane_terminal_size` (`src/ui/panes.rs:119`) now uses `tab_pane_chrome`, so a new or respawned shell in a stacked single slot starts at the bordered size. Plain panes are unchanged because `layout.panes(area).len() == pane_count()`.
- The visibility test lives in `src/server/headless/tests/mod.rs`, a file outside the planned file list. That is where `pty_sources_visible_to_any_render_target` is testable.
- Layout choices the ship file does not specify: the badge ends one border cell before the top-right corner, `[i/n]` is right-aligned when no hidden member is urgent, and the unfocused active member uses `palette.text` bold for "default fg". All are consistent with `UDES5` and `UDES7`.

## Blockers and Unverified Items

- `V11` (render-scale benchmark at 1 and 15+ panes) is overall scope and was not run. G3 adds per-pane `tab.subpanes.is_empty()` checks to the resize and title loops.
- Not verified: whether a hidden member's title or status change alone triggers a frame that refreshes the strip. Hidden output is no longer a render source by design, and terminal revision and status changes are expected to drive the redraw. Overall `V12` or a manual visual check (user) should confirm this.
- Not verified (outside G3): `pending_agent_resume_candidates` iterates layout pane infos only. A hidden stack member with a pending resume plan therefore resumes only after it is activated. This relates to G1 `DES4` and G2 restore and is flagged for overall verification.
- No real-terminal visual check was done (`V12`, user).
