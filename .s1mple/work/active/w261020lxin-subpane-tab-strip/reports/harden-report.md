# Harden Report

- **Outcome:** `Findings identified`
- **Mode:** `Workflow`
- **Target:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/` (ship file `w261020lxin-ship.md`)
- **Range:** `11ab68e94807c05a81ba544f34e2112b8029e9d3..HEAD` (`HEAD` = `a44a4180`)
- **Reviewed:** 57 changed files outside `.s1mple/`. Read in full: `src/workspace/stack.rs` (production part), `src/subpane_strip.rs` (production part), `src/app/api/subpanes.rs`, `src/client/shell/subpanes.rs`. Read as diffs: `src/ui/panes.rs`, `src/layout.rs`, `src/workspace.rs`, `src/workspace/tab.rs`, `src/app/actions.rs`, `src/app/mod.rs`, `src/client/shell/{actions,mouse}.rs`. Followed callers for the pane API handlers that take a pane id (`split`, `move`, `swap`, `zoom`, `resize`, `focus`).
- **Not reviewed:** `docs/next/api/herdr-api.schema.json` (generated). The test bodies in `src/persist/restore.rs` and `src/app/agent_resume.rs` were read only where the group reports cite them; the overall verification and the group verifiers already checked them.

## Summary

| Category | Major | Minor |
|---|---|---|
| Correctness | 0 | 1 |
| Test Gap | 0 | 0 |
| Redundancy | 0 | 0 |
| Dev Speed | 0 | 0 |
| Workflow Leak | 0 | 0 |
| Principles | 0 | 0 |

The change is sound. The only confirmed issue: `pane.resize` silently does nothing for a hidden member, while `pane.split`, `pane.move`, `pane.swap`, `pane.zoom`, and `pane.focus` all act on the member's slot.

## Findings

### HRD-1 — Minor — Correctness — `pane.resize` on a hidden subpane member is a silent no-op

**Evidence**

`src/app/api/panes.rs:766`

```rust
            .is_some_and(|tab| tab.layout.resize_pane(pane_id, direction, amount, area));
```

`src/layout.rs:353`

```rust
        if !self.pane_ids().contains(&pane_id) {
            return false;
```

`handle_pane_resize` passes the resolved pane id to `TileLayout::resize_pane`. A hidden member is in `tab.panes` (so it resolves) but is not a layout leaf, so `resize_pane` returns `false`. The API then reports `changed: false` without an error. The sibling handlers resolve a hidden id to its slot first: split, move, and swap through `tab.subpanes.visible_leaf(pane_id)` (`G1-VER1`, `OVR-VER3` fixes), and zoom and focus by activating the member through `focus_pane_in_workspace` (`src/app/actions.rs:834`).

**Consequence:** A script that lists panes (which now includes hidden members) and resizes one of them gets a success response but no layout change. This is inconsistent with the rest of the pane API.

**Recommendation:** Resolve the target through `tab.subpanes.visible_leaf(pane_id)` before calling `resize_pane`, the same way the split handler does. Add one regression test next to the existing `subpane_split_on_hidden_member_splits_next_to_visible_leaf` test.

## Checks

| Command | Result | Duration |
|---|---|---|
| `just lint` | `Passed` — fmt check and clippy `-D warnings` clean | 39 s |
| `just test` | `Passed` — nextest 4001 passed, 16 skipped; Python 150 + 6 OK; Bun suites 5, 18, 9, 28, 7 pass | 93 s |
| Workflow-leak scan (`rg -nP …` over `git diff --name-only 11ab68e9..HEAD -- . ':!.s1mple'`, generated schema excluded) | `Passed` — no matches | < 1 s |

## Limits

- **Unverified areas:** the visual look of the strip in a real terminal (left to the user, as recorded under `V12`).
- **Blockers:** None.

## Resolution

Workflow mode only. The implementer fills this section; the sections above stay unchanged.

- **Status:** `Pending`

| Finding | Change | Verification | Commit | Result |
|---|---|---|---|---|
| `HRD-1` | `Pending` | `Pending` | `Pending` | `Pending` |

- **Final checks:** `Pending`
- **Remaining issues:** `Pending`
