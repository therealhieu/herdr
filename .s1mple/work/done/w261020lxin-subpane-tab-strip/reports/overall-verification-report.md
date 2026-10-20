# W261020LXIN Overall Verification Report

## Scope

- **Scope:** `overall`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`, `G2-T1`, `G3-T1`, `G3-T2`, `G4-T1`, `G4-T2`, `G4-T3` (all tasks)
- **Change range:** `11ab68e94807c05a81ba544f34e2112b8029e9d3..0cb59f2a`
- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/overall-verification-report.md`

## Status

`Failed`

Every check passes (`V1`–`V12`), the four groups integrate correctly, and no frozen contract changed. One Major gap remains. After a server restart with the default `session.resume_agents_on_restore = true`, a hidden stack member that holds an agent is not resumed until the user activates it (`OVR-VER1`). This breaks the `REQ1` promise that hidden members keep running. Two Minor findings cover compatibility for older clients (`OVR-VER2`) and `pane.split` on hidden ids (`OVR-VER3`).

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES1`, `DES2` | `V1`, `V2` | `src/workspace/stack.rs:SubpaneStacks` (`add_member`, `activate`, `remove_member`, `ordered_ids`), `src/layout.rs:TileLayout::replace_leaf` | `Met` |
| `REQ1` | `DES3`, `DES5`, `UDES8`, `UDES13` | `V3`, `V6`, `V12` | `src/ui/panes.rs:resize_hidden_subpanes` (tiled and zoomed branches of `resize_tab_panes` and `compute_pane_infos_for_tab`), `tab_multi_pane`; `src/server/headless/render.rs:443` excludes hidden members; tests `subpane_hidden_member_is_sized_with_its_slot`, `subpane_hidden_member_is_not_a_visible_render_source`, `subpane_focusing_hidden_member_activates_it_and_keeps_zoom` | `Met` |
| `REQ1` | `DES3`, `DES6` (restart with agent resume) | `V7`, `V12` | `src/app/agent_resume.rs:pending_agent_resume_candidates` only iterates layout pane infos, so hidden members with a resume plan stay without a runtime | `Not met` (`OVR-VER1`) |
| `REQ2` | `DES1`, `DES3`, `UDES11` | `V1`, `V3`, `V12` | `Tab::take_subpane_member`, stack-aware checks in `Workspace::close_pane`/`remove_pane`/`take_pane_for_move`, `Tab::detach_pane`/`take_pane_for_move`, `AppState::close_pane_would_close_workspace`; live exit of a hidden and then the active member | `Met` |
| `REQ2` | `DES7`, `DES8`, `UDES2`, `UDES9`, `UDES10`, `UDES12`, `UDES14` | `V8`, `V9`, `V12` | `src/app/api/subpanes.rs:handle_pane_subpane_create`, `handle_pane_focus` activation (`src/app/api/panes.rs:492`), `src/client/shell/subpanes.rs` (`switch_target`, `strip_hits`, `handle_subpane_strip_click`), `src/client/shell/mouse.rs:2200` strip hits before split handles, `src/client/shell/actions.rs` `NewSubpane`/`NextSubpane`/`PreviousSubpane`/`FocusSubpane` | `Met` |
| `REQ3` | `DES3`, `DES4`, `UDES6` | `V4` | `Tab::visible_panes_mut` for seen marking, `focus_pane_in_workspace` marks the activated member seen, `pane_ids_with_subpanes` in agent lists, `pane.list`, sidebar `pane_details`, terminal targets | `Met` |
| `REQ3` | `DES5`, `UDES1`, `UDES3`, `UDES4`, `UDES5`, `UDES14` | `V5`, `V6` | `src/subpane_strip.rs:subpane_label`, `render_strip`; `src/ui/panes.rs:render_subpane_strip`; test `subpane_strip_renders_on_single_slot_border_with_hidden_status` | `Met` |
| `REQ4` | `DES5`, `DES8`, `UDES7`, `UDES9` | `V5`, `V9` | `strip_layout` badge mode, `most_urgent_hidden_status`; client badge hit → `pane.focus` on the next member | `Met` |
| `REQ5` | `DES6` | `V7`, `V12` | `src/persist/snapshot.rs:SubpaneStackSnapshot`, `src/persist/restore.rs` (`snapshot_pane_ids_with_subpanes`, `validated_subpane_stacks`, `plan_subpane_stacks`, `rebuild_subpane_stacks`, `handoff_pane_aliases`); live restart restored `[1,4,5]` with member 4 active | `Met` (shell members); agent members see `OVR-VER1` |
| `REQ6` | `DES4`, `DES7` | `V8` | `PaneInfo.subpane` via `App::pane_subpane_info` (`src/app/creation.rs`), `ClientShellPane.subpane` (`src/protocol/wire.rs:1067`), `pane.subpane.create` digest pinned in `src/server/client_commands.rs:257`; no `tests/fixtures/` diff; `PROTOCOL_VERSION` 22 at base and head | `Met` |
| `REQ7` | `DES5` | `V6`, `V11` | Hidden members excluded from `any_shell_surface_contains_pane`; strip work gated on `!tab.subpanes.is_empty()`; benchmark shows no material no-stack regression | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `just test-one stack` | 24 passed | `Passed` |
| `V2` | `just test-one replace_leaf` | 2 passed | `Passed` |
| `V3`, `V4`, `V6`, `V7`, `V8`, `V9` | `just test-one subpane` | 34 passed | `Passed` |
| `V5` | `just test-one subpane_strip` | 10 passed | `Passed` |
| `V7` (support) | `just test-one snapshot`, `just test-one restore` | 88 passed, 96 passed | `Passed` |
| `V8` | `just test-one generated_protocol_schema_artifact_is_current`, `just test-one advertised_client_shell_method_shapes_stay_at_the_v1_contract`, `git diff 11ab68e9..HEAD -- tests/fixtures/` | 1 passed, 1 passed, empty diff; `PROTOCOL_VERSION = 22` at base and head | `Passed` |
| `V10` | `just lint && just test` | Exit 0. fmt and clippy clean; nextest 3999 passed, 16 skipped; maintenance 150 OK; UI hot-path architecture 6 OK; integration and docs contract suites pass | `Passed` |
| `V11` | `just bench-render-scale` on base (detached worktree at `11ab68e9`) then on HEAD, back to back; then the core `server::render_scale_benchmark::render_scale_profile` twice more on each, alternating | Both full runs exit 101 because the same unrelated test, `native_file_render_scale_profile`, panics on base and head ("source profile requires working CoW retention, not decoded fallback", an environment limit). All 8 other profiles complete. Combined-pipeline medians (µs), base runs 1/2 → head runs 1/2: background 1 pane 594/791 → 599/707; background 15 panes 612/618 → 614/606; active 1 pane 594/596 → 652/585; active 15 panes 675/661 → 731/682; active 50 panes 810/774 → 945/818. Snapshot + JSON framing for 15 panes and 1 client: 63/63 → 59/62. Deltas fall within the run-to-run spread on the same commit (base background 1 pane alone varied 594→791), so there is no material no-stack regression. No stack cardinality profile exists. | `Passed` |
| `V12` | Debug server only: `env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- --session v12smoke server`, JSON socket API at `~/.config/herdr-dev/sessions/v12smoke/herdr.sock` | `workspace.create` → `w1:p1`. Two `pane.subpane.create` calls → `w1:p2`, `w1:p3` with `subpane {visible_pane_id:"w1:p3", index:2, count:3}`. `pane.list` orders the visible leaf first, then hidden members. `pane.focus` on p1, p2, p3 each made that member the `visible_pane_id`, and `layout.export` shows only the visible leaf. `exit` in hidden p2 → count 2. `exit` in active p3 → the stack dissolved into plain `w1:p1` with `subpane` absent. After rebuilding `[p1, p4, p5]` with p4 active, `server stop` saved `subpanes:[{members:[1,4,5],active:1}]`. Restart restored the same order and active member, and hidden p1 shows a live shell prompt (`pane.read`). Server stopped and session directory removed afterwards. The visual strip check on a real terminal was not run (left to the user). | `Passed` |
| Mockup / `UDES` | Rendered-buffer tests vs `M1@R1` and the `UDES` rows | Strip row `┌ ● claude ─   shell × ─ + ─` on a single-slot stack under `pane_borders = auto`. Badge row and no-border cases are covered by `subpane_strip_falls_back_to_badge_and_needs_a_top_border`. Differences from the HTML mockup (hover-only `×`, drop-down badge, pulsing dot, drag) are all listed under UI Mockups Limits. | `Passed` |

## Integration

- **G1 model ↔ G2 restore order.** `snapshot_pane_ids_with_subpanes` (`src/persist/restore.rs:170`) produces "layout leaf, then hidden members in member order skipping the active one". That is the same order as `SubpaneStacks::ordered_ids` (`src/workspace/stack.rs`), which `Tab::pane_ids_with_subpanes` uses. `handoff_pane_aliases` and `migrated_public_pane_numbers_by_old_raw` zip these two lists. The round-trip and handoff tests (`subpane_stacks_round_trip_through_capture_and_restore`, `subpane_handoff_adopts_hidden_runtimes_and_aliases_them`) and the live restart (`[1,4,5]`, active 1) confirm they agree. When a member fails to restore, the zip can shift, the same as the existing layout pruning. This is not new risk.
- **G3 strip ↔ G4 client hits.** Both sides call `strip_layout(labels, active, rect.width)`. The server takes labels from `subpane_label(terminal)` in `render_subpane_strip`. The client takes them from `PaneInfo.subpane.label`, which `pane_subpane_info` (`src/app/creation.rs`) also fills from `subpane_label(terminal)`. Both use the outer pane `rect.width` and offset from `rect.x`. The client snapshot is recomputed and diffed on every full render (`src/server/headless/render.rs:637`), so it refreshes in the same frame that redraws the strip, including hidden title changes (`terminal_title_needs_full_render`) and hidden status changes (`subpane_hidden_member_status_change_requests_a_full_render`).
- **G1 activation ↔ G4 `pane.focus` and `layout.updated`.** Client member clicks and switch keys send `pane.focus`. `handle_pane_focus` activates the member and emits `layout.updated` when the visible leaf changed. Removing the active member emits it through `layout_update_target_after_pane_removal`. Both are tested in `src/app/api/panes.rs` (`subpane_focusing_hidden_member_emits_layout_updated`, `subpane_closing_active_member_of_single_slot_emits_layout_updated`).
- **Hidden-member visibility ↔ hidden-source exits.** `any_shell_surface_contains_pane` excludes hidden members. `pane_visible_on_active_surface` (`src/app/state.rs:974`), `client_views.rs:520`, and the render target set (`render.rs:317`) use layout ids, so hidden output triggers no presentation work. Strip drawing and hidden resizing return early when `tab.subpanes.is_empty()`.
- **Open item (a), hidden agent resume.** This is a real `REQ1` gap; see `OVR-VER1`.
- **Open item (b), older clients cycling into hidden members.** This is a compatibility gap; see `OVR-VER2`.
- **Open item (c), `ClientShellPane.subpane` serialized as `null`.** Not a defect. The endpoint snapshot is JSON (`src/protocol/endpoint.rs:116`), `ClientShellPane` has no `deny_unknown_fields`, and older clients ignore the extra key. The server never sends the bincode `ServerMessage::ClientShellSnapshot` variant; it is used only by client handling and round-trip tests. The frozen snapshot fixture tests pass.

## Findings

### OVR-VER1 — Major — correctness — Hidden agent members are not resumed after a restart until they are activated

- **Builds:** `REQ1`, `REQ5`, `DES3`, `DES4`, `DES6`, `G1-T3`, `G2-T1`
- **Evidence:** `src/app/agent_resume.rs:99`–`:139`. `pending_agent_resume_candidates` loops over `pending_agent_resume_pane_infos`, which is built from `tab.layout.panes(area)` (through `apply_tab_pane_chrome`) plus `view.pane_infos`. Both contain layout leaves only:

  ```rust
  for info in self.pending_agent_resume_pane_infos(ws_idx, tab_idx, tab, terminal_area) {
      let Some(pane) = tab.panes.get(&info.id) else { continue; };
  ```

  `resume_agents_on_restore` defaults to `true` (`src/config/model.rs:280`). Restore attaches `pending_agent_resume_plan` to every saved agent pane (`src/persist/restore.rs:768`), hidden members included. A hidden member's plan therefore runs only after `pane.focus` makes it a leaf. `start_pending_agent_resume_for_terminal` (G1 switched it to `pane_ids_with_subpanes`) is called only for a direct terminal attach (`src/server/headless.rs:1835`).
- **Impact:** After a restart, a stack such as `[claude, codex, zsh]` brings back only the visible agent. Hidden agents have no process. They show no live status on the strip or in the sidebar, and they do no background work until the user clicks them. That contradicts `REQ1` ("hidden members keep running") on the default config path that `REQ5` is meant to preserve.
- **Resolution:** Include hidden stack members in `pending_agent_resume_candidates`. Give each the size of its slot's visible leaf, the same `pane_inner` that `resize_hidden_subpanes` uses (for example, after collecting the leaf infos, append one candidate per hidden member of `tab.subpanes.stack_of(info.id)` with the leaf's `inner_rect`). Keep the existing spacing and theme gates.
- **Proof to close:** A unit test in `src/app/agent_resume.rs` (`subpane` in its name) builds a tab whose stack has a hidden member with a pending resume plan and no runtime. It asserts that `start_pending_agent_resumes(now, true)` starts that member, sized the same as the visible leaf, without activating it. `just lint` and `just test` pass.

### OVR-VER2 — Minor — compatibility — Older clients see hidden members in `snapshot.panes` and can cycle into them

- **Builds:** `REQ1`, `REQ6`, `DES4`, `DES7`
- **Evidence:** `ClientShellSnapshot.panes` comes from `session_metadata_snapshot` (`src/app/api/session.rs:56`), which now uses `pane_ids_with_subpanes`. The `G4-VER1` fix (`8b006609`) filters hidden members only in this build's client (`src/client/shell/actions.rs:1071`). A client built before this change does not know `subpane`. Its `prefix+tab` (`CyclePaneNext`) iterates every pane in the tab, so on a stack it alternates between members (the trace in `G4-VER1`) and never reaches the other slots. The ship file records this as known (Evidence, G4).
- **Impact:** When an older TUI client is connected to this server (a client still running after a live update, or a remote or SSH endpoint on an older version), pane cycling breaks in tabs that hold a stack. Tabs without stacks are unaffected. The fork is single-platform, and stacks exist only once a user creates them on the new server, so exposure is limited.
- **Resolution:** Choose one and record it: (1) accept and document the limitation in the ship file and Fork Customizations, or (2) keep hidden members out of `ClientShellSnapshot.panes` and carry their ids and labels in the visible pane's `subpane` data (for example, an optional `members` list), which the new client's strip hits already need.
- **Proof to close:** For (1), the limitation is recorded in `.s1mple/docs/project.md` Fork Customizations. For (2), a `subpane` test asserts that `snapshot.panes` holds only layout leaves while strip hits and switch keys still reach every member, and `just test` passes.

### OVR-VER3 — Minor — API consistency — `pane.split` on a hidden member id fails instead of using its slot

- **Builds:** `REQ2`, `REQ6`, `DES4`, `DES7`
- **Evidence:** In the `V12` live smoke, `pane.split {"target_pane_id":"w1:p1"}` with `w1:p1` hidden returned `{"error":{"code":"pane_split_failed","message":"split target pane is not in the layout"}}`. `handle_pane_split` (`src/app/api/panes.rs:34`) passes the hidden id straight to `TileLayout::split_pane` (`src/workspace/tab.rs:333`). `pane.move` and `pane.swap` map hidden targets to `subpanes.visible_leaf` (`G1-VER1` fix), but `pane.split` does not. Nothing changes in state, so this is not corruption.
- **Impact:** An agent or script that takes a pane id from `pane.list` (which now includes hidden members) and splits it gets an error, while `pane.move` and `pane.swap` with the same id succeed. The same id behaves differently depending on the method.
- **Resolution:** In `handle_pane_split`, resolve an explicit `target_pane_id` through `tab.subpanes.visible_leaf(pane_id)`, as the move and swap handlers do. Review `pane.zoom` and `pane.resize` targets the same way and record the choice.
- **Proof to close:** A `subpane` test splits a hidden member and asserts that the new pane sits next to the slot's visible leaf, the stack is unchanged, and `AppState::assert_invariants_for_test()` holds.

## Test Coverage

- **Covered:** Stack invariants on every removal path (close active, close hidden, hidden exit, move out, adversarial identity state). `replace_leaf` geometry and focus history. Seen and Done semantics for hidden agents. Strip layout math (full, badge, truncation, minimum width, label chain). Rendered strip and badge rows, borders for a single-slot stack, and the no-top-border case. Hidden sizing, tiled and zoomed. Hidden members as non-render sources. Full-render triggers for hidden title and status changes. Persistence round trip, fixtures without `subpanes`, invalid-stack drop, handoff fd adoption. API create, list, focus, move, swap, and `layout.updated`. Method digest pin and frozen fixtures. Client keys (wrap both ways, jump, plain-pane no-ops), strip click routing over split handles, badge click, and pane cycling that skips hidden members.
- **Missing:** Resuming hidden agent members after restore (`OVR-VER1`). `pane.split` on a hidden member (`OVR-VER3`). No render-scale profile with stacks present (the benchmark covers only the no-stack case that `V11` requires). The invalid-stack warning is checked by code reading only.
- **Low-value:** None.

## Scope Changes

- Recorded and justified deviations, checked against the diff: stack methods live in `src/workspace/stack.rs` instead of `tab.rs`. `src/app/terminal_targets.rs` uses member-inclusive ids. `pane_is_in_active_tab` treats hidden members as outside the active tab, so a hidden agent that finishes still notifies. Move and swap targets resolve to the visible leaf. `AppState.status_indicators` is new. The API handler and client logic are in the new files `src/app/api/subpanes.rs` and `src/client/shell/subpanes.rs`. Keys are spelled `prefix+period` and `prefix+comma`. `focus: false` on `pane.subpane.create` still shows the new member.
- `docs/next/api/herdr-api.schema.json` and `docs/next/website/src/data/config-reference.json` were updated as planned. No `tests/fixtures/`, `PROTOCOL_VERSION`, `SNAPSHOT_VERSION`, or `HANDOFF_VERSION` change. `layout.export` still exports only visible leaves, as the non-goal states.

## Blockers and Unverified Items

- `just bench-render-scale` exits 101 on both base and head. The cause is `native_file_render_scale_profile`, which needs CoW retention that this machine lacks. It is unrelated to this change and fails identically at base. The relevant `render_scale_profile` completed on both.
- The visual strip check in a real terminal (colors, glyph alignment, click feel) was not run. It is left to the user.
- A hidden agent's live resume behavior (`OVR-VER1`) was confirmed by code reading and the restore path, not by a live agent session, because the smoke used plain shells.
