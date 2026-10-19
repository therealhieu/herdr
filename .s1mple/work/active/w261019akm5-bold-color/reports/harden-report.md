# Harden Report

- **Outcome:** `Findings identified`
- **Mode:** `Workflow`
- **Target:** `.s1mple/work/active/w261019akm5-bold-color/` (ship file `w261019akm5-ship.md`)
- **Range:** `7c59eab9..HEAD`
- **Reviewed:** `src/config/bold_color.rs`, `src/config/model.rs`, `src/config.rs`, `src/client/bold_color.rs`, `src/client/state.rs`, `src/client/mod.rs`, `src/client/loop_config.rs`, `src/client/config_reload.rs`, `src/client/tests/mod.rs`, `docs/next/website/src/data/config-reference.json`; callers of `try_present_frame_to`, `present_surface_patch`, and `reload_local_client_config`
- **Not reviewed:** `.s1mple/` documents (excluded by the skill)

## Summary

| Category | Major | Minor |
|---|---|---|
| Correctness | 0 | 0 |
| Test Gap | 0 | 1 |
| Redundancy | 0 | 0 |
| Dev Speed | 0 | 0 |
| Workflow Leak | 0 | 0 |
| Principles | 0 | 0 |

The color rule, both presentation paths, and the unset no-op are correct and tested. The only gap is that config reload of `bold_color` has no automated check, although the existing reload test already exercises the exact function and needs one more key and assertion.

## Findings

### HRD-1 — Minor — Test Gap — Reload of `bold_color` is not asserted

**Evidence**

`src/client/tests/mod.rs:775`

```rust
        "[ui]\nredraw_on_focus_gained = false\nhost_cursor = \"drawn\"\nmouse_capture = false\n",
```

`src/client/tests/mod.rs:796`

```rust
    assert!(!redraw_on_focus_gained);
    assert!(draw_host_cursor);
    assert!(!mouse_capture);
```

The test passes `&mut bold_color` to `reload_local_client_config` but its config has no `bold_color` key and nothing asserts the reloaded value. Removing `*bold_color = loaded.config.ui.bold_color;` from `src/client/config_reload.rs:121` would leave every test green.

**Consequence:** A regression in reload plumbing (the only path for live changes) goes unnoticed; the requirement relies on a manual kitty check alone.

**Recommendation:** Add `bold_color = "#f8dea6"` to the test's config string and assert `bold_color == Some(crate::config::BoldColorConfig::Rgb(248, 222, 166))` after the reload.

## Checks

| Command | Result | Duration |
|---|---|---|
| `just lint` (`cargo fmt --check` + `cargo clippy --all-targets --locked -- -D warnings`) | `Passed` — exit 0 | 3 s |
| `just test` | `Passed` — exit 0; nextest 3917 passed / 14 skipped; Python 150 + 6 OK; Bun suites 0 failures | 43 s |
| Workflow-leak `rg -nP` scan over the 10 non-`.s1mple/` changed files | `Passed` — no matches (exit 1); bare `V<n>`/`G<n>-T<n>` read manually, none | <1 s |

## Limits

- **Unverified areas:** Live behavior in kitty (`V4`) is manual and pending. Fidelity limit by design: herdr resolves `inverse` cells server-side by swapping fg/bg into explicit colors (`src/pane/terminal.rs:3313`), so a bold inverse default-color cell keeps its swapped colors instead of taking the bold color as background like Ghostty; changing that needs server work, a stated non-goal.
- **Blockers:** None

## Resolution

Workflow mode only. The implementer fills this section; the sections above stay unchanged.

- **Status:** `Pending`

| Finding | Change | Verification | Commit | Result |
|---|---|---|---|---|
| `HRD-1` | `Pending` | `Pending` | `Pending` | `Pending` |

- **Final checks:** `Pending`
- **Remaining issues:** `Pending`
