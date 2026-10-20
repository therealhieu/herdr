# W261020LXIN G4 Verification Report

## Scope

- **Scope:** `G4`
- **Tasks:** `G4-T1`, `G4-T2`, `G4-T3`
- **Change range:** `fdbd9723..0b7dde1b`
- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/g4-verification-report.md`

## Status

`Failed`

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ6` | `DES7` | `V8` | `src/api/schema/panes.rs:PaneInfo.subpane` (`serde(default, skip_serializing_if = "Option::is_none")`), `PaneSubpaneInfo`, `PaneSubpaneCreateParams`; `src/api/schema.rs` `PaneSubpaneCreate => "pane.subpane.create", true, true` appended last; `src/app/creation.rs:pane_subpane_info`; `src/protocol/wire.rs:ClientShellPane.subpane` (`serde(default)`), filled in `src/server/client_shell.rs:148`; geometry allowlists `src/server/headless/client_views.rs:235,268,302`; digest pinned `src/server/client_commands.rs:257`; schema artifact regenerated `docs/next/api/herdr-api.schema.json`. `git diff fdbd9723..0b7dde1b -- tests/fixtures` is empty; `PROTOCOL_VERSION = 22` unchanged. The JSON snapshot path (`src/protocol/endpoint.rs:116` `serde_json::to_string`) carries the new field; the bincode `ServerMessage::ClientShellSnapshot` is rejected as unnegotiated by the client (`src/client/mod.rs:1531`), so the field addition does not change a live binary codec. `ClientShellPane` has no `deny_unknown_fields`, so older clients ignore `"subpane"`. | `Met` |
| `REQ2` | `DES7` | `V8` | `src/app/api/subpanes.rs:handle_pane_subpane_create` (resolves slot leaf, sizes from the slot, cwd follows `pane.split`, spawns via `Workspace::spawn_subpane`, emits `PaneCreated` and `layout.updated`, saves session); dispatched at `src/app/api.rs:1145`. `pane.focus` on a hidden member activates it (`subpane_focusing_hidden_member_emits_layout_updated`, now asserting the returned `subpane`). | `Met` |
| `REQ2` | `DES8`, `UDES12` | `V9` | Config: `src/config/model.rs` (defaults `prefix+t`, `prefix+period`, `prefix+comma`, `subpane` empty), `src/config/keybinds.rs`, `src/input/keybindings.rs` (`NewSubpane`, `NextSubpane`, `PreviousSubpane`, `FocusSubpane(usize)`), `src/input/keybind_help.rs`, `src/main.rs` template, 4 entries in `docs/next/website/src/data/config-reference.json`. Routing: `src/client/shell/actions.rs:1103-1112` → `pane.subpane.create` / `pane.focus` on `src/client/shell/subpanes.rs:switch_target` (no-op on a plain pane or out-of-range index). The new keys behave as specified, but the existing default `cycle_pane_next` (`prefix+tab`) now cycles into hidden members and cannot leave a stack (`G4-VER1`). | `Not met` |
| `REQ2`, `REQ4` | `DES8`, `UDES9`, `UDES10` | `V9` | `src/client/shell/subpanes.rs:strip_hits` (same `strip_layout` on `pane.rect.width` as `src/subpane_strip.rs:render_strip`, only when `inner_rect.y > rect.y`), `ShellHitMap.subpane_strips` (`src/client/shell/state.rs:96`) built in `src/client/shell/composition.rs:277`, `Down(Left)` branch before split handles (`src/client/shell/mouse.rs:2200-2210`); Member → `pane.focus`, Close → `request_pane_close`, Add → `pane.subpane.create`, Badge → `pane.focus` on `(active + 1) % n`. Bare border cells produce no hit, so they fall through. | `Met` |

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| `V8`, `V9`, group check | `just test-one subpane` | 33 tests run: 33 passed, 3981 skipped | `Passed` |
| `V8`, group check | `just test-one generated_protocol_schema_artifact_is_current` | 1 test run: 1 passed | `Passed` |
| `V8`, group check | `just test-one advertised_client_shell_method_shapes_stay_at_the_v1_contract` | 1 test run: 1 passed | `Passed` |
| `V8` | `git diff fdbd9723..0b7dde1b -- tests/fixtures`; `rg "pub const PROTOCOL_VERSION" src/protocol/wire.rs` | Empty diff; `PROTOCOL_VERSION: u32 = 22`, not in the diff | `Passed` |
| Group check | `just lint` | `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` clean, exit 0 | `Passed` |
| Group check | `just test` | nextest 3998 passed, 16 skipped; Python 150 OK and 6 OK; Bun 5 + 18 + 9 + 28 + 7 pass; exit 0 | `Passed` |
| `UDES12` regression review | Code reading of `src/client/shell/actions.rs:1064-1086` against `src/app/api/session.rs:56` | Default `prefix+tab` resolves through `resolve_prefix_binding` → `record_binding` → `endpoint_method_for_action`, which cycles `snapshot.panes` including hidden members | `Failed` (`G4-VER1`) |

## Integration

Not applicable — group scope

## Findings

### G4-VER1 — Major — Correctness / Regression — Default `prefix+tab` cycles into hidden subpane members and cannot leave a stack

- **Builds:** `REQ2`, `DES8`, `UDES12`, `V9`, `G4-T2`
- **Evidence:** `src/client/shell/actions.rs:1064-1086` builds the cycle list from every snapshot pane in the focused tab:

  ```rust
  KeybindAction::CyclePaneNext | KeybindAction::CyclePanePrevious => {
      let panes = snapshot.panes.iter()
          .filter(|pane| pane.tab_id == focused_tab)
          .collect::<Vec<_>>();
      ...
      Some(Method::PaneFocus(PaneTarget { pane_id: panes[next].pane_id.clone() }))
  ```

  `snapshot.panes` comes from `session_metadata_snapshot` (`src/server/client_shell.rs:27`), which lists each visible leaf followed by its hidden members (`src/app/api/session.rs:56` `pane_ids_with_subpanes`; confirmed by `subpane_create_request_adds_member_and_lists_it_after_its_slot`). The default `cycle_pane_next = "prefix+tab"` (`src/config/model.rs:1198`) reaches this code through the prefix path (`src/client/shell/input.rs:706-709` `record_binding`), not through the surface-based `cycle_pane` (`src/client/shell/input.rs:1015`), which only navigate-mode `Tab` uses. The implementer's claim that "the keyboard path is safe via `cycle_pane`" holds only for navigate mode.
  Trace for a tab `A | [B visible, C hidden]`: order `A, B, C`; from `B`, `prefix+tab` → `pane.focus(C)` → C is activated, order becomes `A, C, B`; from `C` → `pane.focus(B)` (hidden) → B is activated. `prefix+tab` alternates B/C forever and never reaches `A`.
- **Impact:** Once a stack exists, the default pane-cycle key stops moving between pane slots and instead flips stack members. `REQ1` ("tabs and panes without stacks behave exactly as before") and the `A11` rule ("geometry, navigation, and visibility code keeps using layout ids only") are broken for the existing action. Older clients connected to this server have the same behavior because they also see hidden members in `snapshot.panes`, which overall verification should note.
- **Resolution:** Exclude hidden members from the `CyclePaneNext`/`CyclePanePrevious` list in `endpoint_method_for_action` (keep a pane only when `subpane` is `None` or `subpane.visible_pane_id == pane_id`), or cycle the surface panes like `cycle_pane`. Review the other `snapshot.panes` navigation consumers the same way and record the choice.
- **Proof to close:** A client-shell test with a plain pane plus a two-member stack shows `KeybindAction::CyclePaneNext` through `record_binding` going plain → visible member → plain (never a hidden member), and the same for `CyclePanePrevious`; `just test-one subpane`, `just lint`, and `just test` pass.

### G4-VER2 — Minor — Test Coverage — Previous-subpane wrap from the first member is not proven

- **Builds:** `V9`, `UDES12`, `G4-T2`
- **Evidence:** `src/client/shell/tests/subpanes.rs:subpane_keys_step_with_wrap_and_jump_by_index` focuses `pane_1` at index 2 of 3. `NextSubpane` → index 0 proves the forward wrap, but `PreviousSubpane` → index 1 is a plain step; no case presses previous from index 0. `V9` requires "next wraps, previous wraps".
- **Impact:** The backward wrap in `src/client/shell/subpanes.rs:44` (`(active + count - 1) % count`) has no protection against an off-by-one or underflow change.
- **Resolution:** Add a case where the focused member is at index 0 and `PreviousSubpane` targets the last member.
- **Proof to close:** The extended test passes under `just test-one subpane`.

## Test Coverage

- **Covered:** `pane.subpane.create` adds a member, returns `PaneInfo.subpane` with `index`/`count`/`label`, makes the new member the visible leaf, and keeps invariants; `pane.list` lists hidden members right after their slot's visible leaf; `pane.focus` on a hidden member returns its `subpane`; method digest pinned and v1 method shapes unchanged; schema artifact current; default keys resolve; `NewSubpane` maps to `pane.subpane.create` on the focused pane; next wraps forward; jump by index and out-of-range no-op; next/previous/jump no-op on a plain pane; strip member, `×`, `+`, and badge clicks route correctly on a row that is also a split handle without starting a drag; no strip hits without a top border; unsupported servers are handled by the generic `supports_endpoint_method` notice path in `push_endpoint_method_with_kind` (`src/client/shell/actions.rs:380`).
- **Missing:** Pane cycling with a stack present (`G4-VER1`); previous-subpane wrap (`G4-VER2`).
- **Low-value:** None. The click tests place clicks with `strip_layout` itself, which is acceptable under the Test Strategy ("prove layout math once").

## Scope Changes

- `ClientShellPane.subpane` uses `#[serde(default)]` without `skip_serializing_if`, so the JSON snapshot carries `"subpane": null` for every plain pane. DES7 asked for the same optional shape as `PaneInfo`. This is within the endpoint contract (an optional field with a default; old clients ignore unknown fields; the frozen snapshot fixture still parses). The stated reason, the bincode round-trip test of `ServerMessage::ClientShellSnapshot`, is a test-only constraint because live clients reject that binary message. Accepted, no finding.
- Handler in `src/app/api/subpanes.rs` and client logic in `src/client/shell/subpanes.rs` (new files) instead of the listed upstream files. This keeps the upstream diff smaller, which matches the fork rule.
- `focus: false` on `pane.subpane.create` still makes the new member visible in its slot. This follows `DES3` ("activates it") and is documented in the implementer return.
- `operation_ui_impact_preserves_existing_classification` list updated to include `pane.subpane.create`, which follows from the new method.

## Blockers and Unverified Items

- `V11` (render-scale benchmark) and `V12` (live smoke) belong to overall verification and were not run.
- Client/server strip geometry agreement was checked by code reading: both call `strip_layout(labels, active, rect.width)` with `subpane_label` labels and offset from `rect.x`. A terminal missing from `app.terminals` renders with `FALLBACK_LABEL` on the server but is left out of `pane.list` on the client. That case was not exercised and should not occur in a consistent state.
