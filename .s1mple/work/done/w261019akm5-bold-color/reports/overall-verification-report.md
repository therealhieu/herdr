# W261019AKM5 Overall Verification Report

## Scope

- **Scope:** `overall`
- **Tasks:** `G1-T1`, `G1-T2`, `G1-T3`, `G1-T4`
- **Change range:** `7c59eab9..741ff7fa`
- **Report:** `.s1mple/work/active/w261019akm5-bold-color/reports/overall-verification-report.md`

## Status

`Passed`

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ1` | `DES1` | `V2` | `src/config/bold_color.rs:BoldColorConfig::parse` (strict `#rrggbb` → `Rgb(u8,u8,u8)`), `src/config/model.rs:UiConfig.bold_color`, `src/config.rs` re-export | `Met` |
| `REQ1` | `DES2` | `V1`, `V3` | `src/client/bold_color.rs:apply`/`bold_fg`: bold + named `0x00` → `0x02rrggbb`; named `0x01`–`0x08` and indexed 0–7 → `+8` (matches `src/protocol/wire.rs:color_to_u32` packing, `Black=0x01`…`Gray=0x08`, `DarkGray=0x09`…`White=0x10`); called in `src/client/state.rs:try_present_frame_to` and `src/client/state.rs:present_surface_patch` before encode and commit | `Met` |
| `REQ1` | `DES4` | `V5` | `.s1mple/docs/project.md` Fork Customizations row; `docs/next/website/src/data/config-reference.json` `ui.bold_color` entry (required by `scripts/test_config_reference_check.py`, passes in `just test`) | `Met` |
| `REQ2` | `DES1` | `V2` | `src/config/bold_color.rs:BoldColorConfig::parse` (`"bright"` → `Bright`) | `Met` |
| `REQ2` | `DES2` | `V1` | `src/client/bold_color.rs:apply` sets `default_fg = None` for `Bright`, so bold reset stays reset while palette 0–7 still map to 8–15 | `Met` |
| `REQ3` | `DES1` | `V2` | `src/config/model.rs:UiConfig::default` `bold_color: None`; `bold_color_defaults_to_none` | `Met` |
| `REQ3` | `DES2` | `V1`, `V3` | Both call sites guarded by `if let Some(rule) = self.bold_color`; `bold_color_unset_keeps_presented_frame_bytes` matches a plain `BlitEncoder` baseline byte for byte | `Met` |
| `REQ4` | `DES3` | `V4` | Startup: `src/client/mod.rs` `ClientLoopConfig { bold_color: loaded_config.config.ui.bold_color }` → `ClientState.bold_color`. Reload: `src/client/config_reload.rs:apply_reload` → `reload_local_client_config` assigns `*bold_color` only when `[ui]` is valid → `shell.compose` → `state.present_frame` → `try_present_frame_to` applies the new rule and diffs against the committed (recolored) frame | `Unverified` — manual kitty check `V4` is run by the user after delivery; code path inspected end to end |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V1` | `just test-one bold_color` | 8 tests run, 8 passed; `bold_color_rule_matches_ghostty` covers reset → RGB, Red/Black/Gray → LightRed/DarkGray/White, indexed 1 → 9, indexed 8/200, RGB, LightRed, non-bold unchanged, `Bright` leaves reset unchanged | `Passed` |
| `V2` | `just test-one bold_color` | `bold_color_parses_hex_rgb`, `bold_color_parses_bright`, `bold_color_defaults_to_none`, `bold_color_rejects_invalid_values` (`"yellow"`, `"#f8dea"`) pass | `Passed` |
| `V3` | `just test-one bold_color` | `bold_color_recolors_presented_frame` emits `38;2;248;222;166`; `bold_color_unset_keeps_presented_frame_bytes` equals baseline; `bold_color_recolors_surface_patch` leaves the recolored cell in the encoder's committed frame | `Passed` |
| `V4` | Manual reload in kitty | Not run; the user runs it after delivery | `Not run` — manual, user-owned (see Blockers and Unverified Items) |
| `V5` | `just lint && just test` | Exit 0. Lint clean; nextest 3917 passed / 14 skipped; Python 150 OK and 6 OK; Bun 5 + 18 + 9 + 28 + 7 pass, 0 fail | `Passed` |
| Encoder consistency | Inspect `src/client/state.rs` and `rg "blit_encoder\.(encode\|commit)" src/client` | Only two encode/commit sites (`try_present_frame_to`, `present_surface_patch`); both transform cells before encode and commit the transformed cells. A patch fallback (`Ok(false)`) leads to a full-frame repaint, which also applies the rule | `Passed` |
| Fork rules | Inspect diff stat and changed files | No `unwrap()` in production code; no `src/protocol/` or `PROTOCOL_VERSION` change; no fixture change; upstream edits are field/default/re-export/argument/call-site lines; customization recorded in `.s1mple/docs/project.md` | `Passed` |

## Integration

Single group `G1`; no cross-group handoffs. The end-to-end chain holds:

```text
[ui] bold_color (serde, src/config/bold_color.rs)
  → UiConfig.bold_color (src/config/model.rs)
  → ClientLoopConfig.bold_color (src/client/mod.rs, loop_config.rs)
  → ClientState.bold_color (startup)
  → reload_local_client_config (only when [ui] is valid) → apply_reload recompose → present_frame
  → bold_color::apply on frame cells / patch rows → BlitEncoder encode + commit
```

- Invalid `[ui]` on reload keeps the previous `bold_color`, consistent with the other `[ui]` keys (`reload_local_client_config_keeps_ui_preferences_when_ui_is_invalid` updated and passing).
- Toggling the setting is visible through the encoder diff: the committed frame holds the previously transformed cells, so a changed rule produces differing cells and emitted bytes.
- Final scope matches Decisions: no server-side `TerminalAnsi` change, no direct attach passthrough change, no wire protocol change. The `docs/next` config reference edit is the recorded `Q1`/`DES4` supersession of the original non-goal.

## Findings

None

## Test Coverage

- **Covered:** color rule in both modes, including the 0–7 boundary on named and indexed colors and RGB/non-bold passthrough (`V1`); config parse, default, and rejection (`V2`); full-frame recolor, unset byte identity, and surface-patch recolor reaching the committed encoder state (`V3`); reload signature through the two updated reload tests in `src/client/tests/mod.rs`.
- **Missing:** None against the Test Strategy. Reload behavior is proven manually by `V4` by design.
- **Low-value:** None.

## Scope Changes

- `docs/next/website/src/data/config-reference.json` gained a `ui.bold_color` entry, superseding the original `docs/` non-goal per `Q1`/`DES4`. Its `values` are `["bright", "rgb"]` because `scripts/config_reference_check.py` derives values from enum variant names; the description explains the `"#rrggbb"` form. Recorded in `.s1mple/docs/project.md`.
- `V3` presentation tests live in `src/client/bold_color.rs` rather than next to existing `ClientState` tests, to keep upstream edits small.

## Blockers and Unverified Items

- `V4` (`REQ4`): manual kitty reload check is scheduled for the user after delivery. Recorded as `Unverified`, not a finding; the reload code path was inspected end to end.
