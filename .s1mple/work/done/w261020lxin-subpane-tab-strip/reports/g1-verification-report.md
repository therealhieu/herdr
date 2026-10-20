# W261020LXIN G1 Verification Report

## Scope

- **Scope:** `G1`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`
- **Change range:** `11ab68e94807c05a81ba544f34e2112b8029e9d3..c50bd7c7`
- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/g1-verification-report.md`

## Status

`Failed`

The stack model, leaf replacement, removal hooks, seen marking, and list inclusion all work, and every planned check passes. Remediation is still needed: a probe shows that `pane.move` aimed at a hidden member tears the source pane out of its tab and then reports an error (`G1-VER1`). Also, the `layout.updated` event that `DES3` requires on activation is never emitted (`G1-VER2`).

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1`, `REQ2` | `DES1` | `V1` | `src/workspace/stack.rs:SubpaneStacks::{add_member, activate, remove_member, ordered_ids}`, `SubpaneStack`, `RemoveOutcome`; `src/workspace/tab.rs:Tab.subpanes` | `Met` |
| `REQ1` | `DES2` | `V2` | `src/layout.rs:TileLayout::replace_leaf` (moves `focus`/`prev_focus`, leaves ratios and `zoomed` alone) | `Met` |
| `REQ1`, `REQ2` | `DES3`, `UDES11` | `V3` | `src/workspace/stack.rs:Tab::{take_subpane_member, removing_pane_empties_tab}`; hooks in `Tab::{detach_pane, take_pane_for_move}`, `Workspace::{close_pane, remove_pane, take_pane_for_move, close_focused}`, `AppState::close_pane_would_close_workspace`; `Workspace::assert_invariants_for_test` uses `SubpaneStacks::assert_invariants_for_test` | `Not met`: removal paths are correct, but hidden members are now listed and accepted as `pane.move` targets, and that path corrupts the source tab (`G1-VER1`) |
| `REQ2`, `REQ3` | `DES3`, `UDES6` | `V4` | `src/app/actions.rs:AppState::focus_pane_in_workspace` (activate + mark seen), `pane_is_in_active_tab`, `mark_active_tab_seen`; `src/workspace.rs:Workspace::switch_tab` via `Tab::visible_panes_mut` | `Not met`: seen/Done behavior is met, but `DES3` "Activation emits `layout.updated`" is not built (`G1-VER2`) |
| `REQ1`, `REQ2` | `UDES13` | `V4` | `Tab::activate_subpane` → `replace_leaf`, which never touches `Tab.zoomed`; test `subpane_focusing_hidden_member_activates_it_and_keeps_zoom` | `Met` |
| `REQ3`, `REQ6` | `DES4` | `V3`, `V4` | `src/workspace/stack.rs:Tab::pane_ids_with_subpanes`; call sites `src/app/api/session.rs:56`, `src/app/agents.rs:30`, `src/app/creation.rs:183,195`, `src/workspace/aggregate.rs:26`, `src/app/actions.rs:295,347,599,621`, `src/app/api/tabs.rs:232`, `src/app/agent_resume.rs:191`, `src/app/terminal_targets.rs:140`; test `subpane_hidden_agent_reports_done_in_lists_until_activated` reads `session_snapshot().panes` and `.agents` | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `cargo nextest run --locked stack` (same as `just test-one stack`) | 21 tests run, 21 passed (6 `workspace::stack::tests`, 7 `lifecycle_tests`, plus unrelated name matches) | `Passed` |
| `V2` | `cargo nextest run --locked replace_leaf` | 2 tests run, 2 passed | `Passed` |
| `V3`, `V4` | `cargo nextest run --locked subpane` | 7 tests run, 7 passed | `Passed` |
| Group lint | `just lint` | `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` pass, exit 0 | `Passed` |
| Group suite | `just test` | nextest 3971 passed, 16 skipped; Python 150 + 6 OK; Bun suites 0 failures; exit 0 | `Passed` |
| Probe: move onto hidden target | Temporary test in a throwaway detached worktree (removed afterwards; no repo change): stack `[1 hidden, 2 visible]` in tab 0; tab "src" holds panes 3 and 4; `handle_pane_move(3 → tab 0, target_pane_id = hidden 1)` | Response `pane_move_failed` "target pane could not be split"; tabs went from 2 to 3: `tab1 "src" = [4]`, new `tab2 "src" = [3]` | `Failed` |
| Probe: close active member in single-slot stack | Same throwaway worktree: stack `[1 hidden, 2 visible]`, `handle_pane_close(2)` | Response ok, layout becomes `[1]`, events `[PaneClosed]` only; no `LayoutUpdated` | `Failed` |
| `UDES6` state | Code review + `V4` test | A hidden member that finishes stays `Done` after the tab is viewed and becomes `Idle` on activation | `Passed` |
| `UDES11` state | Code review + `V3`/`V1` tests | Closing the active member shows the next member, else the previous one; one member left dissolves the stack | `Passed` |
| `UDES13` state | Code review + `V4` zoom test | Activation keeps `tab.zoomed` | `Passed` |

## Integration

Not applicable — group scope

## Findings

### G1-VER1 — Major — correctness — `pane.move` onto a hidden member tears the source pane out of its tab

- **Builds:** `REQ2`, `DES3`, `DES4`, `G1-T2`, `G1-T3`
- **Evidence:** G1 lists hidden members in `pane.list` and `session_snapshot`, and they resolve as valid targets. The target check in `src/app/api/panes.rs:1082-1091` only asks `find_tab_index_for_pane` (backed by `tab.panes`), so a hidden member passes. The source is then taken at `src/app/api/panes.rs:1136-1143`. The insert fails because `Tab::insert_existing_pane` → `TileLayout::insert_pane_near` only accepts layout leaves (`src/workspace/tab.rs:497-509`). `recover_failed_pane_move` (`src/app/api/panes.rs:1369`) then always creates a **new tab** for the source:

  ```text
  PROBE move resp={"id":"r","error":{"code":"pane_move_failed","message":"target pane could not be split"}}
  PROBE tabs before=2 after=3
  PROBE tab1 name=Some("src") layout=[PaneId(4)]
  PROBE tab2 name=Some("src") layout=[PaneId(3)]
  ```

  Before G1, a validated target was always a layout leaf, so this recovery path could not be reached this way.
- **Impact:** An API or agent call that uses a pane id taken from `pane.list` gets an error. The source pane has still been moved out of its tab into a duplicate tab, so the user's layout changes even though the call failed.
- **Resolution:** Deal with a hidden-member target before the source is taken. Either map the target to its slot's visible leaf (`tab.subpanes.visible_leaf(target)`) or reject it with an error, with no state change. Do the same for any other pre-validation that accepts hidden ids and then needs a layout leaf (for example, the explicit `pane.swap` target).
- **Proof to close:** A unit test in `src/app/api/panes.rs` (`subpane` in its name) moves a pane in a multi-pane tab onto a hidden member. It asserts that the source tab is unchanged and that the call either succeeds next to the visible leaf or returns an error. `AppState::assert_invariants_for_test()` holds afterwards.

### G1-VER2 — Minor — spec — Visible-leaf changes emit no `layout.updated`

- **Builds:** `DES3`, `G1-T2`
- **Evidence:** `DES3` says "Activation emits `layout.updated`". `handle_pane_focus` (`src/app/api/panes.rs:491-507`) calls `focus_pane_in_workspace` but emits no event, and the implementer deferred this to G4. The G4 plan (`DES7`/`G4-T1`) does not mention it, so it would be lost. Removal has a related gap: `layout_update_target_after_pane_removal` (`src/app/api/panes.rs:2190-2208`) emits only when `layout.pane_count() > 1`. When the active member of a single-slot stack is closed or exits, the leaf changes but no event is sent (probe: events `[PaneClosed]`, layout `[2] → [1]`).
- **Impact:** API subscribers that track tab layouts through `layout.updated` keep showing a pane that is no longer visible after `pane.focus` activates a member, and after the active member of a single-slot stack closes or exits.
- **Resolution:** Emit `layout.updated` when `pane.focus` activates a hidden member, for example by having `handle_pane_focus` compare the layout ids before and after. Make `layout_update_target_after_pane_removal` also return the tab when the removed pane is the active member of a stack, so `pane.close` and pane exit emit it.
- **Proof to close:** A `subpane` test captures `EventHub` events. It asserts `LayoutUpdated` for `pane.focus` on a hidden member and for `pane.close` on the active member of a single-slot stack.

## Test Coverage

- **Covered:** pure stack operations (add, activate, remove active → next/previous, dissolve, remove hidden, `ordered_ids`); `replace_leaf` geometry and focus history; close-active, close-hidden, hidden-member exit through `PaneDied`, move-out to another tab, and `close_pane_would_close_workspace`; invariants on adversarial identity state with a 3-member stack; focus activates a hidden member and keeps zoom; a hidden agent shows `Done` in pane and agent lists until activated.
- **Missing:** API paths that receive hidden ids from the lists G1 now fills (`pane.move` target, `pane.swap` explicit target); event emission on activation and on removal of the active member in a single-slot stack. Both are covered by the findings' proofs.
- **Low-value:** None. The `stack` filter also matches unrelated tests by name, but that adds no redundant cases.

## Scope Changes

- `src/app/terminal_targets.rs:140` was switched to `pane_ids_with_subpanes` without being planned. This is correct: terminal-target lookups by name need to find hidden agents, and the runtimes stay live.
- `pane_is_in_active_tab` now treats hidden members as outside the active tab. This also changes notifications: a hidden agent that finishes in the active tab now gets a toast and sound. This is correct under `REQ3`/`A12`, because the user cannot see that pane, and the toast target activates the member through `focus_pane_in_workspace`. The ship file should record it as a behavior change.
- `Workspace::spawn_subpane` carries `#[allow(clippy::too_many_arguments, dead_code)]` with a comment until G4 calls it. This is acceptable as a temporary measure, and G4 must remove `dead_code`.
- The shared adversarial helper was left without a stack; the dedicated test `subpane_invariants_hold_on_adversarial_identity_state` adds one instead. This meets `V3`.
- rustfmt re-wrapped the `src/app/agent_resume.rs:191` block after the call-site change. This is a mechanical result of the swap, not an unrelated reformat.
- `layout.updated` on activation was deferred to G4 without a G4 plan entry (see `G1-VER2`).

## Blockers and Unverified Items

- The `spawn_subpane` PTY spawn path (`Tab::spawn_subpane`, `Workspace::spawn_subpane`) has no caller yet and was not exercised with a real PTY. It is reviewed by code only. G4 (`V8`) and the live smoke test (`V12`) cover it.
- `UDES6`, `UDES11`, and `UDES13` were checked only as state behavior, because G3 does the rendering.
