# W261019AKM5 G1 Verification Report

## Scope

- **Scope:** `G1`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`, `G1-T4`
- **Change range:** `7c59eab9..3ee5f750`
- **Report:** `.s1mple/work/active/w261019akm5-bold-color/reports/g1-verification-report.md`

## Status

`Passed`

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES1` | `V2` | `src/config/bold_color.rs:BoldColorConfig` (`Rgb(u8,u8,u8)`, strict `#rrggbb` parse), `src/config/model.rs:UiConfig.bold_color` (default `None`), `src/config.rs` re-export | `Met` |
| `REQ1` | `DES2` | `V1`, `V3` | `src/client/bold_color.rs:apply`/`bold_fg` (bold + named `0x00` → packed `0x02rrggbb`; named `0x01`–`0x08` and indexed 0–7 → `+8`); called in `src/client/state.rs:try_present_frame_to` before `frame_with_drawn_cursor`/encode, and in `src/client/state.rs:present_surface_patch` before `patch_rows_with_drawn_cursor`/`encode_patch`/`commit_patch` | `Met` |
| `REQ1` | `DES4` | `V5` | `.s1mple/docs/project.md` Fork Customizations row; `docs/next/website/src/data/config-reference.json` `ui.bold_color` entry after `ui.redraw_on_focus_gained` | `Met` |
| `REQ2` | `DES1` | `V2` | `src/config/bold_color.rs:BoldColorConfig::parse` (`"bright"` → `Bright`) | `Met` |
| `REQ2` | `DES2` | `V1` | `src/client/bold_color.rs:apply` (`Bright` → `default_fg = None`, so bold reset stays reset; palette 0–7 still `+8`) | `Met` |
| `REQ3` | `DES1` | `V2` | `src/config/model.rs:UiConfig::default` `bold_color: None`; `bold_color_defaults_to_none` | `Met` |
| `REQ3` | `DES2` | `V1`, `V3` | Both call sites guarded by `if let Some(rule) = self.bold_color`; `bold_color_unset_keeps_presented_frame_bytes` compares bytes to a plain `BlitEncoder` baseline | `Met` |
| `REQ4` | `DES3` | `V4` | `src/client/loop_config.rs:ClientLoopConfig.bold_color`, `src/client/mod.rs` (startup from `loaded_config.config.ui.bold_color` into `ClientState`), `src/client/config_reload.rs:reload_local_client_config` (assigned beside `draw_host_cursor` only when `[ui]` is valid) | `Unverified` — manual kitty check `V4` is assigned to the orchestrator; code path inspected and compiles, existing reload tests pass |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `just test-one bold_color` | `bold_color_rule_matches_ghostty` passes: reset → RGB(248,222,166); Red/Black/Gray → LightRed/DarkGray/White; Indexed 1 → 9; Indexed 8, 200, RGB, LightRed, non-bold unchanged; `Bright` leaves bold reset unchanged and brightens Red and Indexed 1. 8 tests run, 8 passed | `Passed` |
| `V2` | `just test-one bold_color` | `bold_color_parses_hex_rgb`, `bold_color_parses_bright`, `bold_color_defaults_to_none`, `bold_color_rejects_invalid_values` (`"yellow"`, `"#f8dea"`) pass | `Passed` |
| `V3` | `just test-one bold_color` | `bold_color_recolors_presented_frame` finds `38;2;248;222;166`; `bold_color_unset_keeps_presented_frame_bytes` equals baseline; `bold_color_recolors_surface_patch` confirms the encoder's committed frame holds the recolored cell | `Passed` |
| `V4` | Manual reload in kitty | Not run by the verifier; assigned to the orchestrator | `Not run` — manual, orchestrator-owned (see Blockers and Unverified Items) |
| `V5` | `just lint && just test` | Exit 0. Lint clean; nextest 3917 passed / 14 skipped; Python 150 OK and 6 OK; Bun suites 5 + 18 + 9 + 28 + 7 pass, 0 fail (includes `test_config_reference_check.py`) | `Passed` |
| Ghostty fidelity | Compare `src/client/bold_color.rs:bold_fg` with `vendor/libghostty-vt/src/terminal/style.zig:170` | `.none` + bold + `.color` → configured color, `.bright` → default; `.palette` idx < 8 + bold + any `bold` option → idx + 8. Herdr named `0x01`–`0x08` map to palette 0–7, so `+8` lands on `0x09`–`0x10` (palette 8–15). RGB-equal-to-default rule intentionally skipped (`A3`) | `Passed` |
| Encoder consistency | Inspect `src/client/state.rs` | Frame path: transformed `frame_data` is both encoded and passed to `blit_encoder.commit`. Patch path: transformed `patch.rows` feed `patch_rows_with_drawn_cursor`, `encode_patch`, and `commit_patch`. No other `blit_encoder.encode`/`commit` call in `src/client` | `Passed` |
| Fork rules | `rg` over changed files | No `unwrap()` in production code, no s1mple IDs in code, no `PROTOCOL_VERSION`/wire/fixture change; upstream edits are field/default/re-export/call-site lines only; customization recorded in project.md | `Passed` |

## Integration

Not applicable — group scope

## Findings

None

## Test Coverage

- **Covered:** color rule for both modes including the 0–7 boundary on named and indexed colors, RGB and non-bold passthrough (`V1`); config parse, default, and rejection (`V2`); full-frame recolor, unset byte identity, and surface-patch recolor reaching the encoder's committed state (`V3`); reload signature through the two updated existing reload tests (`src/client/tests/mod.rs`).
- **Missing:** None against the Test Strategy. Reload behavior is intentionally proven manually by `V4`.
- **Low-value:** None. `bold_color_unset_keeps_presented_frame_bytes` exercises no new branch, but it is the planned `V3` guard for `REQ3`.

## Scope Changes

- `ui.bold_color` reference entry lists `values: ["bright", "rgb"]` because `scripts/config_reference_check.py` derives values from enum variant names; `"rgb"` is not a literal setting. This follows the existing upstream precedent `terminal.new_cwd` (`"path"` for `Path(String)`), and the description explains the `"#rrggbb"` form. Disclosed by the implementer and in project.md. Accepted.
- `V3` presentation tests live in `src/client/bold_color.rs` instead of next to existing `ClientState` tests, to keep upstream edits small. Accepted.
- Reload repaints only through the client shell recompose; direct attach passthrough remains a non-goal per Decisions.

## Blockers and Unverified Items

- `V4` (`REQ4`): manual kitty reload check is assigned to the orchestrator and was not run in this verification. Recorded as `Unverified`, not a finding.
