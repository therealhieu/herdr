# W261020LXIN G2 Verification Report

## Scope

- **Scope:** `G2`
- **Tasks:** `G2-T1`
- **Change range:** `2d7f9f53..827bea16`
- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/g2-verification-report.md`

## Status

`Passed`

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ5` | `DES6` | `V7` | `src/persist/snapshot.rs:SubpaneStackSnapshot`, `TabSnapshot.subpanes` (`serde(default, skip_serializing_if = "Vec::is_empty")`), `capture_tab` writes members and active index; `src/persist/restore.rs:plan_subpane_stacks` remaps hidden members through the tab's `id_map`, `restore_tab` spawns or adopts them in the same loop as layout panes, `rebuild_subpane_stacks` rebuilds via `SubpaneStacks::add_member` + `activate`, `validate_subpane_stack` drops invalid stacks with `warn!("dropping invalid saved subpane stack")`; `snapshot_pane_ids_with_subpanes` feeds `handoff_pane_aliases` and `migrated_public_pane_numbers_by_old_raw`; `restore_workspace` numbers via `Tab::pane_ids_with_subpanes`. `SNAPSHOT_VERSION` stays 3, `HANDOFF_VERSION` stays 1. | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V7` | `just test-one subpane` | 15 passed, 3980 skipped; includes `persist::restore::tests::{subpane_stacks_round_trip_through_capture_and_restore, invalid_subpane_stack_is_dropped_and_tab_still_restores, subpane_handoff_adopts_hidden_runtimes_and_aliases_them}` and `persist::snapshot::tests::session_fixtures_without_subpanes_parse_and_serialize_unchanged` | `Passed` |
| Group check | `just test-one snapshot` | 88 passed, 3907 skipped | `Passed` |
| Group check | `just test-one restore` | 96 passed, 3899 skipped | `Passed` |
| Group check | `just lint` | `cargo fmt --check` and `cargo clippy --all-targets --locked -D warnings` clean | `Passed` |
| Regression (adversarial helper change) | `cargo nextest run --locked` | 3979 passed, 16 skipped | `Passed` |
| Contract | `git diff 2d7f9f53..827bea16 -- tests/fixtures`; `rg VERSION` on diff | Fixtures untouched; no `SNAPSHOT_VERSION`, `HANDOFF_VERSION`, or `PROTOCOL_VERSION` change | `Passed` |

## Integration

Not applicable — group scope

## Findings

None

## Test Coverage

- **Covered:** capture → JSON → parse → restore keeps member order (compared through public pane numbers in member order), active index, fresh pane ids, the active member as layout leaf, all four panes, and `AppState` invariants; fixtures without `subpanes` parse with empty stacks and serialize without a `subpanes` key; existing fixture restore tests (`failed_cold_restore_preserves_panes_and_saved_directories`, `legacy_restore_precomputes_missing_public_pane_numbers`) still pass on the new path; five invalid stack shapes (missing member, active out of range, one member, non-active member that is a layout leaf, active member not a leaf) and a member claimed by an earlier stack are dropped while the tab restores; strict handoff (real fds, unix) consumes all four runtimes, including hidden members, and aliases every old member id to its new id with the active member preserved; `Workspace::test_adversarial_identity_state()` now carries a stack, so every identity test exercises it.
- **Missing:** the `warn!` on a dropped stack is confirmed by inspection only (no log-capture infrastructure, and the Test Strategy forbids adding new test infrastructure); the duplicate-member rejection has no dedicated case; the promote-surviving-member path in `rebuild_subpane_stacks` is reachable only after a failed handoff import, which strict handoff already rejects.
- **Low-value:** None

## Scope Changes

- `src/workspace.rs` (+3 lines, test-only) adds a stack to `Workspace::test_adversarial_identity_state()`. It is outside G2's listed files but matches the `AGENTS.md` Testing rule for identity changes and was allowed by the orchestrator. Full suite passes.
- Hidden members of an invalid stack are discarded rather than kept. This matches `DES6` ("discarding unrestorable hidden ones"): a hidden member has no layout slot to occupy once its stack is dropped.

## Blockers and Unverified Items

- `V12` (live smoke across a server restart) is not assigned to G2 and was not run.
- A hand-corrupted snapshot whose invalid stack holds hidden members with exported handoff fds would make strict handoff fail with "did not consume N pane runtime(s)". A snapshot written by this binary always passes validation, so this was judged not reachable in practice and is not a finding.
- Known planned limitation (`A13`): older binaries ignore `subpanes` and lose hidden members.
