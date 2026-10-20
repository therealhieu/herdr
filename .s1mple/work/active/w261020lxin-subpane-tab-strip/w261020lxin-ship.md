# W261020LXIN subpane-tab-strip — Ship

## Table of Contents

- [1. Decisions](#1-decisions)
  - [Q&A Log](#qa-log)
  - [Requirements](#requirements)
  - [UI Mockups](#ui-mockups)
  - [UI Design](#ui-design)
  - [System Design](#system-design)
  - [Verification](#verification)
    - [Test Strategy](#test-strategy)
  - [Traceability](#traceability)
- [2. Goal](#2-goal)
  - [Status](#status)
  - [Sources](#sources)
  - [Reports](#reports)
  - [Todos](#todos)
  - [Execution Rules](#execution-rules)
  - [Group Cycle](#group-cycle)
  - [Group Scheduling](#group-scheduling)
- [3. Plan](#3-plan)
  - [G1 — Stack model and lifecycle](#g1--stack-model-and-lifecycle)
  - [G2 — Persistence and handoff](#g2--persistence-and-handoff)
  - [G3 — Strip rendering](#g3--strip-rendering)
  - [G4 — API and client interaction](#g4--api-and-client-interaction)
- [4. Overall Verification and Harden](#4-overall-verification-and-harden)
- [5. Deliver](#5-deliver)
- [6. Evidence](#6-evidence)
- [Ship File Check](#ship-file-check)

## 1. Decisions

- **Prompt:** Add native subpanes to herdr: one pane slot holds several terminals, shown as a tab strip with agent status on the slot's top border (style A), with a `[n/m ◐]` badge when the strip does not fit.
- **Goal:** A user can add, switch, and close terminals inside one pane slot with the mouse or keys, sees every member's name and agent status on the slot's top border (or a badge when narrow), and gets the same stacks back after a server restart or live handoff.
- **Scope:** Server-side stack state on `Tab`, member lifecycle (create, activate, close, exit, move out), hidden-member sizing and render skipping, strip and badge rendering, mouse and keyboard interaction, JSON API exposure, session persistence and handoff. · **Non-goals:** drag to merge or tear out, merging two existing panes into a stack, styles B/C/D, a badge drop-down menu, `+` on plain (non-stack) pane borders, CLI subcommands, `layout.export`/`layout.apply` round-trip of stacks (they keep exporting visible leaves only), public docs under `docs/next/website/src/content/docs/`, Windows/Linux verification.
- **UI:** `needed — pane slot top border (strip, badge fallback)`. The look was settled by the accepted mockup `M1@R1`.

Upstream diff impact: no new wire codec, `PROTOCOL_VERSION`, frozen fixture, `SNAPSHOT_VERSION`, or `HANDOFF_VERSION` change. Logic lives in two new files (`src/workspace/stack.rs`, `src/subpane_strip.rs`). Small hooks land in upstream hot files (`src/workspace/tab.rs`, `src/workspace.rs`, `src/layout.rs`, `src/ui/panes.rs`, `src/server/headless/render.rs`, `src/persist/{snapshot,restore}.rs`, `src/api/schema*.rs`, `src/app/**`, `src/client/shell/{actions,input,mouse,composition,state}.rs`, `src/config/{model,keybinds}.rs`, `src/input/keybindings.rs`). Merge conflict risk on the next upstream sync is medium: the hooks are small but sit in frequently edited files.

Runtime/client classification (`AGENTS.md` Runtime/client boundary guardrail): stack membership, active member, member labels, and member order are shared runtime facts (server state, JSON API, persisted). Strip drawing, the fit/badge decision, colors, and hit geometry are presentation, computed by one pure function that both the server renderer and the TUI client call.

### Q&A Log

The user asked for a self-driven ship with execution, so no questions were asked; every decision is an assumption grounded in code, the accepted mockup, or project rules.

| Q | Question | Answer | Decides |
|---|---|---|---|
| `A1` | Which visual style? | Style A (tab strip on the top border) with badge fallback. The user chose it after reviewing the published mockup and asked to build "style A". | `REQ3`, `REQ4`, `UDES1`–`UDES14` |
| `A2` | Do workspaces and tabs change? | No. Subpanes add one level inside a pane slot; the user confirmed tabs stay. Code and API say "subpane"/"stack", never "tab". | `REQ1`, `DES1` |
| `A3` | New `Node::Stack` variant or a side table on `Tab`? | Side table: `Tab.subpanes` lists stacks; the BSP leaf always holds the active member's `PaneId`. `Node::Stack` needs about 24 exhaustive match edits in `src/layout.rs` (`:496`–`:681`), `src/persist/*`, and `src/app/api/layouts.rs`, and changes the public `layout.export` schema; the side table keeps `Node`, `LayoutSnapshot`, and `LayoutNode` unchanged (explorer reports). | `DES1`, `DES2` |
| `A4` | How is a hidden member sized? | Same size as the slot: the resize loops (`resize_tab_panes` `src/ui/panes.rs:314`, `compute_pane_infos_for_tab` `:368`) also resize the slot's hidden members. `PaneRuntime::resize` dedupes equal sizes (`src/pane.rs:3345`), so activation causes no PTY resize. | `DES5` |
| `A5` | Do hidden members count as visible for rendering? | No. `any_shell_surface_contains_pane` (`src/server/headless/render.rs:441`) tests `tab.panes`, so it must also exclude hidden members; otherwise their output defeats the hidden-source early exit (`AGENTS.md` Multiplicative performance paths). | `DES5`, `REQ7` |
| `A6` | Can the client hit-test strip segments without a codec change? | Yes. Pane rects already reach the client (`PaneSurfacePane`, frozen bincode `src/protocol/wire.rs:1090`). The client-shell snapshot is JSON (`ClientShellPane` `:1058`), so an optional `subpane` field (member order and label) is allowed. The client runs the same pure layout function as the renderer on `rect.width` to get segment ranges. Strip clicks are tested before split drag handles because the vertical-split handle covers the lower pane's top border row (`split_hit_rect` `src/server/client_shell.rs:573`). | `DES5`, `DES8` |
| `A7` | Which API changes? | One new advertised method `pane.subpane.create`. Activation reuses `pane.focus` (a hidden member is activated, then focused; today it silently no-ops, `src/layout.rs:273`). Closing reuses `pane.close`. `PaneInfo` gains an optional `subpane` object and `pane.list`/`pane.get` include hidden members. Existing method shapes (`PaneTarget` and others) stay byte-identical; the new method's digest is pinned like `pane.clear` (`src/server/client_commands.rs:241`). | `DES7` |
| `A8` | Segment widths and labels | Segment ` {g} {label} ` has a fixed 1-column status slot, so the strip does not move when status changes and client and server agree on geometry from labels alone. Label: manual pane label → detected agent display name → terminal title → `shell`, truncated to 20 columns. | `DES5`, `UDES1`–`UDES14` |
| `A9` | Which status glyphs and colors? | Reuse herdr's sidebar mapping: `status_icon` with the configured `ui.status_indicators` style, `status_color`, `status_priority` (`src/client/shell.rs:178`–`:236`), widened to `pub(crate)` instead of copied. Panes without a detected agent show a blank status slot. | `DES5` |
| `A10` | Keybinds | `new_subpane = "prefix+t"`, `next_subpane = "prefix+."`, `previous_subpane = "prefix+,"` (all free in the defaults, `src/config/model.rs:1100`–`:1190`; `prefix+[` is copy mode), indexed `subpane` 1–9 unbound by default (`prefix+1..9` is `switch_tab`). The existing `close_pane` (`prefix+x`) closes the active member. Keys follow the `zoom`/`focus_agent` config pipeline (`src/config/model.rs:445`, `src/config/keybinds.rs:399`, `src/input/keybindings.rs:57`). | `DES8` |
| `A11` | Where do hidden members appear in lists? | Data lists that drive the sidebar, agent lists, `pane.list`, metadata expiry, plugin record cleanup, and agent resume include hidden members, ordered right after their visible leaf. Geometry, navigation, and visibility code keeps using layout ids only. | `DES4` |
| `A12` | When is a hidden agent "seen"? | Viewing a tab marks only layout-visible panes seen (today it marks all `tab.panes`, `src/app/actions.rs:466`, `src/workspace.rs:455`); activating a member in the viewed tab marks it seen. So a hidden agent that finishes shows Done until the user looks at it. | `DES3`, `REQ3` |
| `A13` | Persistence format | Optional `subpanes` list on `TabSnapshot` with `serde(default, skip_serializing_if = "Vec::is_empty")`, no version bump (bumping makes old binaries discard the whole file, `src/persist/snapshot.rs:497`). Older binaries ignore the field and restore only visible leaves; hidden members are then lost. Recorded as a known limitation. | `DES6`, `REQ5` |
| `A14` | What closes a stack? | Closing or exiting the active member activates the next member, else the previous. A stack left with one member dissolves into a plain pane. Moving a member to another tab or workspace removes it from the stack first. | `DES1`, `DES3` |
| `A15` | Borders | A slot holding a stack counts as multi-pane for `pane_borders = auto` (four `multi_pane` checks, `src/ui/panes.rs:106`, `:204`, `:322`, `:385`), so a single-slot tab still shows the strip. With `pane_borders = off` or no top border, no strip is drawn and no strip hits are produced (the client checks `inner_rect.y > rect.y`); keys still work. | `DES5`, `DES8` |
| `A16` | Close glyph conflicts with symbols-style blocked `×`? | Keep `×` for close: it sits after the label on the active segment only, while a status glyph always sits before the label in its status color. | `UDES1`–`UDES14` |

### Requirements

| ID | Requirement | Source |
|---|---|---|
| `REQ1` | A pane slot can hold two or more terminals (a stack). Exactly one member is visible; hidden members keep running, keep parsing output, and stay sized to the slot so switching never reflows them. Tabs and panes without stacks behave exactly as before. | Prompt, `A2`–`A4` |
| `REQ2` | The user can add a member (new shell in the slot's cwd), switch to any member, and close members, with the mouse on the strip and with keys. Closing the active member shows a neighbor; a stack with one member left becomes a plain pane; a member whose process exits is removed the same way. | Prompt, `A10`, `A14` |
| `REQ3` | The slot's top border shows every member's name and agent status, including hidden members; a hidden agent that finishes shows Done until viewed. Hidden agents stay visible in the sidebar and agent lists. | Prompt, `A8`, `A9`, `A11`, `A12` |
| `REQ4` | When the strip does not fit, the border shows the active member plus a `[i/n g]` badge with the most urgent hidden status; clicking the badge shows the next member. | Prompt, `A1` |
| `REQ5` | Stacks, member order, and the active member survive a server restart (session restore) and a live handoff. Session files without stacks load unchanged. | Prompt, `A13` |
| `REQ6` | Stack membership is exposed through the JSON API (list, get, create, focus-activates) and the client-shell snapshot, without changing the wire protocol, `PROTOCOL_VERSION`, frozen codecs, or v1 fixtures. | Prompt, `A6`, `A7` |
| `REQ7` | Hidden members add no presentation work, and render scaling is profiled at 1 and 15+ panes. | `AGENTS.md` Multiplicative performance paths, `A5` |

### UI Mockups

- **Manifest:** `mockups/manifest.md` — status `Accepted`, round 1
- **Accepted revisions:** `M1@R1 → mockups/m1-subpane-strip.html` (style A and its badge fallback only)
- **Limits:** HTML, not a cell grid. Hover-only `×`, the badge drop-down menu, drag-and-drop, and the pulsing dot do not carry over; the TUI adaptation is in the `UDES` rows.

### UI Design

| ID | Surface | Aspect | Decision | Source | Builds | Uses |
|---|---|---|---|---|---|---|
| `UDES1` | Subpane strip | Layout | A slot holding a stack draws a strip on its top border, starting one column right of the corner, in place of the normal title. Each member is a segment ` {g} {label} ` with a fixed 1-column status slot `g` before the label; one border-line cell separates segments. Reuse the title placement in `render_pane_border_titles` (`src/ui/panes.rs:732`). | `M1@R1` — style A tab strip | `REQ1`, `REQ3` | `DES5` |
| `UDES2` | Subpane strip | Components | The active segment ends with ` ×` (close). A ` + ` segment follows the last member. Plain (non-stack) panes show no `+` and keep their normal title. | `M1@R1` — strip `×` and `+` | `REQ1`, `REQ2` | `DES5` |
| `UDES3` | Subpane strip | Components | Member label: manual pane label → detected agent display name → terminal title → `shell`, truncated to 20 columns with `…`. | `M1@R1` — tab labels (claude, codex, zsh) | `REQ3` | `DES5` |
| `UDES4` | Subpane strip | States | Status glyph and color follow the configured `ui.status_indicators` style and sidebar colors (`status_icon` `src/client/shell.rs:178`, `status_color` `:225`). A pane with no detected agent gets a blank status slot. The slot keeps its width, so the strip never shifts when status changes. | `M1@R1` — status dots and legend | `REQ3` | `DES5` |
| `UDES5` | Subpane strip | States | Focused slot: active member in the focused title style (`palette.accent`, bold, `src/ui/panes.rs:767`). Unfocused slot: active member bold in the default fg. Other members `overlay0` (`src/ui/panes.rs:769`). | `M1@R1` — focused vs unfocused stack | `REQ1` | `DES5` |
| `UDES6` | Subpane strip | States | A hidden member whose agent finishes shows Done (✓/teal) until activated; activation marks it seen. A hidden blocked member shows its blocked glyph on its segment, and the workspace tab shows it through the existing aggregate. | `M1@R1` — "Block a background agent" | `REQ3` | `DES3`, `DES5` |
| `UDES7` | Subpane strip | Responsive | When the full strip does not fit in `width − 2`, switch to badge mode: ` {g} {label} × ` for the active member on the left (label truncated to fit) and a right-aligned `[{i}/{n}{ g}]`, where `g` is the highest-priority hidden status (`status_priority` `src/client/shell.rs:203`), omitted when all hidden members are idle or have no agent. No `+` in badge mode. | `M1@R1` — narrow width, badge fallback | `REQ4` | `DES5` |
| `UDES8` | Subpane strip | Responsive | The strip appears only where the slot draws a top border. A slot holding a stack counts as multi-pane for `pane_borders = auto` (`src/config/model.rs:898`), so a single-slot tab with a stack still gets borders (`apply_pane_chrome` `src/ui/panes.rs:198`). With `pane_borders = off`, or a top-row slot under `pane_outer_borders = false`, no strip is drawn; keys still work. | `M1@R1` — strip on the border | `REQ1` | `DES5`, `DES8` |
| `UDES9` | Subpane strip | Interactions | Clicking a member segment activates it and focuses the slot. `×` closes the active member. `+` adds a new shell member in the same cwd and activates it. The badge activates the next member, wrapping. | `M1@R1` — click tab, `×`, `+`, badge | `REQ2`, `REQ4` | `DES7`, `DES8` |
| `UDES10` | Subpane strip | Interactions | Strip segment clicks win over the split drag handle on the same row. Clicks on the bare border line between or after segments fall through to existing behavior (`handle_mouse` `src/client/shell/mouse.rs:683`). No drag-to-merge or tear-out. | `M1@R1` — tab strip clicks | `REQ2` | `DES8` |
| `UDES11` | Subpane strip | States | Closing the active member activates the next member, else the previous. With one member left the stack becomes a plain pane: the strip disappears and the normal title returns. | `M1@R1` — close tab, stack collapses | `REQ2` | `DES1`, `DES3` |
| `UDES12` | Keyboard | Interactions | Defaults: `new_subpane` = `prefix+t`, `next_subpane` = `prefix+.`, `previous_subpane` = `prefix+,` (both wrap), indexed `subpane` 1–9 unbound. Existing `close_pane` (`prefix+x`) closes the active member. Keys act on the focused slot; on a plain pane next, previous, and jump do nothing, and new turns it into a stack. | `M1@R1` — keyboard hints | `REQ2` | `DES8` |
| `UDES13` | Subpane strip | States | In a zoomed slot the strip shows on the zoomed pane's top border when that border is drawn; activating a member keeps the slot zoomed. | `M1@R1` — stack slot (zoom derived) | `REQ1`, `REQ2` | `DES2`, `DES5` |
| `UDES14` | Subpane strip | Accessibility | Status is carried by glyph shape as well as color (● dots style; ◐/✓/× symbols style). Every mouse action has a key equivalent (`UDES12`). The close `×` appears only after the active label, so it is not confused with a symbols-style blocked `×`, which always sits before a label. | `M1@R1` — legend, keyboard hints | `REQ2`, `REQ3` | `DES5`, `DES8` |

`UDES` rows are the UI authority. Implementation builds them with the project's own components and uses the accepted mockup only as a visual reference, never as code.

### System Design

| ID | Decision | Builds | Why |
|---|---|---|---|
| `DES1` | New module `src/workspace/stack.rs`: `SubpaneStacks` (a `Vec<SubpaneStack { members: Vec<PaneId>, active: usize }>`) stored as one new field `Tab.subpanes`. Invariants: every stack has ≥ 2 unique members; `active < members.len()`; `members[active]` is in the layout; other members are not; no pane is in two stacks; `layout ids ∪ hidden members == tab.panes keys`. Pure operations: `stack_of`, `is_hidden`, `add_member(visible, new)` (creates a stack when the slot is plain), `activate(pane) -> Option<(old_leaf, new_leaf)>`, `remove_member(pane) -> RemoveOutcome { replaced_leaf: Option<(old, new)> }` (active → next else previous; one member left → stack dissolves), `ordered_ids(layout_ids)` (visible leaf followed by its hidden members). | `REQ1`, `REQ2` | Keeps `Node`, `LayoutSnapshot`, and `LayoutNode` unchanged (`A3`) and puts the logic in a fork-only file. |
| `DES2` | `TileLayout::replace_leaf(old, new) -> bool` in `src/layout.rs`: replaces a leaf id in place and moves `focus` and `prev_focus` from `old` to `new`; split ratios and shape stay unchanged; it never touches `Tab.zoomed`. | `REQ1`, `REQ2` | Activation must swap the visible member without changing geometry or resetting zoom (`src/workspace/tab.rs:412` resets zoom on layout edits). |
| `DES3` | Lifecycle hooks. `Tab::spawn_subpane(target, cwd, …) -> NewPane` allocates a `PaneId`, spawns the runtime at the target's current PTY size with the same `SplitCommand` plumbing as `split_pane_with_runtime` (`src/workspace/tab.rs:313`), inserts into `tab.panes` and the stack, and activates it. Every removal path (`Tab::detach_pane`, `Tab::take_pane_for_move`, `Workspace::close_pane`/`remove_pane`/`take_pane_for_move`) first runs a stack-aware removal when the pane is in a stack, before any `layout.pane_count() <= 1` check, so it never closes the tab while other members remain; `handle_pane_died` (`src/app/actions.rs:2061`) inherits this through `remove_pane`. `focus_pane_in_workspace` (`src/app/actions.rs:226`) activates a hidden member before focusing it. Seen: viewing a tab marks only layout-visible panes seen; activation inside the viewed tab marks the new member seen (`A12`). Activation emits `layout.updated`. | `REQ1`, `REQ2`, `REQ3` | Removal must stay correct for user close, process exit, and moves (`A14`); focus must activate (`A7`). |
| `DES4` | `Tab::pane_ids_with_subpanes()` returns layout order with each visible leaf followed by its hidden members. Data lists use it: `session_snapshot` (`src/app/api/session.rs:57`), `collect_agent_infos` (`src/app/agents.rs:31`), `collect_panes_for_workspace` (`src/app/creation.rs:180`), sidebar `pane_details` (`src/workspace/aggregate.rs:27`), metadata expiry (`src/app/actions.rs:291`), plugin record cleanup on tab close (`src/app/api/tabs.rs:232`), and agent resume capture (`src/agent_resume.rs:191`). Geometry, navigation, and visibility sites keep `layout.pane_ids()`. Each remaining `layout.pane_ids()` site from the explorer list is reviewed and the choice recorded in the group return. | `REQ3`, `REQ6` | Hidden agents must stay visible in status lists (`A11`). |
| `DES5` | New module `src/subpane_strip.rs` (declared in `src/main.rs`): `subpane_label(&TerminalState) -> String` (`A8`) and pure `strip_layout(labels: &[&str], active: usize, width: u16) -> StripLayout { mode: Full | Badge, segments: Vec<StripSegment { x: u16, width: u16, kind: Member(i) | Close(i) | Add | Badge } > }`. Widths depend only on labels and width. `render_pane_border_titles` (`src/ui/panes.rs:732`) draws the strip for slots holding a stack, using status from `pane_agent_status` (`src/app/api_helpers.rs:96`, widened to `pub(crate)`) and glyph/color/priority from `status_icon`/`status_color`/`status_priority` (`src/client/shell.rs:178`–`:236`, widened to `pub(crate)`). A slot holding a stack counts as multi-pane for border decisions (`A15`). The resize loops resize hidden members to the slot's inner size (`A4`). `any_shell_surface_contains_pane` and the zoom-aware visibility checks (`src/app/state.rs:988`, `src/server/headless/client_views.rs:515`) treat hidden members as not visible (`A5`). Strip work runs only for slots holding a stack. | `REQ1`, `REQ3`, `REQ4`, `REQ7` | One pure layout function gives the renderer and the client identical geometry (`A6`); the hidden-source exit stays effective. |
| `DES6` | `TabSnapshot` gains `subpanes: Vec<SubpaneStackSnapshot { members: Vec<u32>, active: usize }>` (`serde(default, skip_serializing_if = "Vec::is_empty")`). `capture_tab` writes it; `restore_tab` (`src/persist/restore.rs:491`) spawns hidden members from the saved `panes` entries, remaps ids through the same id map, and rebuilds stacks, dropping an invalid stack with a `tracing::warn!` (missing member, active out of range, fewer than 2 members) while keeping its panes as plain leaves or discarding unrestorable hidden ones. `handoff_pane_aliases` (`:122`), `migrated_public_pane_numbers_by_old_raw` (`:159`), and handoff import include hidden members so their PTY fds are adopted, not leaked. No `SNAPSHOT_VERSION` or `HANDOFF_VERSION` bump (`A13`). | `REQ5` | Additive optional field keeps old files loading and old binaries working (`A13`). |
| `DES7` | API: `PaneInfo` (`src/api/schema/panes.rs:499`) gains optional `subpane: Option<PaneSubpaneInfo { visible_pane_id: String, index: u32, count: u32, label: String }>` (index 0-based, `serde(default, skip_serializing_if = "Option::is_none")`). New method `pane.subpane.create { pane_id, cwd: Option<String>, focus: bool = true }` returning the new pane's `PaneInfo`, defined with `client_shell = true`, dispatched next to `pane.split` (`src/app/api/panes.rs:34`), included in the geometry allowlists (`src/server/headless/client_views.rs:244`–`:295`), emitting `PaneCreated` and `layout.updated`, with its shape digest pinned in `src/server/client_commands.rs:241` (the v1 fixture is not edited). `pane.focus` on a hidden member activates it. `docs/next/api/herdr-api.schema.json` is regenerated (`HERDR_UPDATE_API_SCHEMA=1`). `ClientShellPane` (`src/protocol/wire.rs:1058`) gains the same optional `subpane` field, filled in `src/server/client_shell.rs:139`. | `REQ2`, `REQ6` | Stack membership is a shared runtime fact (`A7`); additive optional JSON fields and a new advertised method are the sanctioned extension paths (`AGENTS.md` Stable client endpoint contract). |
| `DES8` | Client contract. Config keys `new_subpane`, `next_subpane`, `previous_subpane` and indexed `subpane` follow the `zoom`/`focus_agent` pipeline (`src/config/model.rs`, `src/config/keybinds.rs`, `src/input/keybindings.rs`, `src/input/keybind_help.rs`, `src/main.rs` template, one `docs/next/website/src/data/config-reference.json` entry each) with the defaults from `A10`. `endpoint_method_for_action` (`src/client/shell/actions.rs:862`) maps new-subpane to `Method::PaneSubpaneCreate` and next/previous/indexed to `Method::PaneFocus` on the target member computed from the snapshot's `subpane` fields (no-op on a plain pane). `ShellHitMap` (`src/client/shell/state.rs:85`) gains strip hits computed in `composition.rs` from the snapshot members and each pane rect via `strip_layout`, only when `inner_rect.y > rect.y`; `mouse.rs` tests them on `Down(Left)` before split handles and maps Member → `pane.focus`, Close → `pane.close`, Add → `pane.subpane.create`, Badge → `pane.focus` on the next member. | `REQ2`, `REQ4`, `REQ6` | Actions reach the server only as API methods (`src/client/shell/actions.rs:6`); geometry comes from the shared pure function (`A6`). |

Data flow:

```text
TerminalState ─► subpane_label ─┬─► PaneInfo.subpane ─► ClientShellPane.subpane ─► client strip_layout ─► hit map
Tab.subpanes ───────────────────┘                                                   (same fn, same width)
                                └─► render_pane_border_titles ─► strip_layout ─► cells in FrameData
```

### Verification

| ID | Proves | Checks | Layer | Command or method | Expected |
|---|---|---|---|---|---|
| `V1` | `REQ1`, `REQ2` | `DES1` | Unit | `just test-one stack` (tests in `src/workspace/stack.rs`) | Add creates a stack from a plain slot and appends; activate returns `(old, new)` leaves; removing the active member picks next, else previous; removing to one member dissolves the stack; removing a hidden member returns no leaf change; `ordered_ids` puts hidden members right after their leaf. |
| `V2` | `REQ1` | `DES2` | Unit | `just test-one replace_leaf` | `replace_leaf` swaps the leaf, moves `focus` and `prev_focus`, keeps split ratios and `splits()` equal, and returns false for an unknown or already-present id. |
| `V3` | `REQ2` | `DES3`, `UDES11` | Unit (`Workspace::test_new`/`AppState::test_new`, no PTYs) | `just test-one subpane` | In a single-slot tab with a 2-member stack: closing the active member, closing the hidden member, and a hidden member dying each keep the tab and leave a plain pane; moving a member to another tab removes it from the stack; `assert_invariants_for_test()` (extended for stacks) passes after each step, including on adversarial identity state with a stack added. |
| `V4` | `REQ2`, `REQ3` | `DES3`, `UDES6` | Unit | `just test-one subpane` | Focusing a hidden member activates and focuses it; viewing the tab does not mark a hidden member seen, so a hidden idle agent reports `Done`; activation marks it seen (`Idle`). |
| `V5` | `REQ3`, `REQ4` | `DES5`, `UDES2`, `UDES3`, `UDES7` | Unit | `just test-one subpane_strip` | `strip_layout`: three short labels fit in `Full` mode with exact `x` ranges for members, the active `×`, and `+`; a width one column too small switches to `Badge`; the badge label truncates with `…`; width below the minimum yields no segments; changing nothing but status does not change segments. `subpane_label` follows manual → agent → title → `shell` and truncates at 20 columns. |
| `V6` | `REQ1`, `REQ3`, `REQ7` | `DES5`, `UDES1`, `UDES4`–`UDES8`, `UDES13` | Unit (render to a test buffer) | `just test-one subpane` (tests next to `src/ui/panes.rs` hooks) | A tab with a stack renders the strip text on the slot's top border with the hidden member's status glyph; a single-slot stack tab draws borders under `auto`; a hidden member's runtime size equals the active member's after compute; `any_shell_surface_contains_pane` is false for a hidden member and true after activation. |
| `V7` | `REQ5` | `DES6` | Unit | `just test-one subpane` (tests in `src/persist/`) | Capture → JSON → parse → restore keeps member order, active member, and remapped ids; the existing session fixtures without `subpanes` still parse and restore unchanged; an invalid stack is dropped with its warning and the tab still restores; handoff aliases cover hidden members. |
| `V8` | `REQ2`, `REQ6` | `DES7` | Unit / contract | `just test-one subpane` and `just test-one generated_protocol_schema_artifact_is_current` and `just test-one advertised_client_shell_method_shapes_stay_at_the_v1_contract` | `pane.subpane.create` adds a member to the target slot and returns its `PaneInfo` with `subpane`; `pane.list` includes hidden members after their leaf; `pane.focus` on a hidden member activates it; the schema artifact is current; the new method digest is pinned; `git diff <base> -- tests/fixtures/` is empty and `PROTOCOL_VERSION` is unchanged. |
| `V9` | `REQ2`, `REQ4` | `DES8`, `UDES9`, `UDES10`, `UDES12` | Unit (client shell tests) | `just test-one subpane` | Default keys resolve to the new actions; next wraps, previous wraps, jump picks the index, and all three are no-ops on a plain pane; a click on a member segment sends `pane.focus` for that member, on `×` sends `pane.close`, on `+` sends `pane.subpane.create`, on the badge sends `pane.focus` for the next member; a strip click on a row that is also a split handle does not start a drag; no strip hit exists when `inner_rect.y == rect.y`. |
| `V10` | All | All | Lint + full suite | `just lint && just test` | Both pass, including `scripts/test_config_reference_check.py` and the UI hot-path architecture test. |
| `V11` | `REQ7` | `DES5` | Benchmark | `just bench-render-scale` on the base commit and on the final head | 1-pane and 15+-pane results reported for both; no material regression in the no-stack case; the delta is recorded. |
| `V12` | `REQ1`, `REQ2`, `REQ5` | `DES3`, `DES6`, `DES7` | Live smoke (debug server, API-driven) | `env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- …` against the debug `herdr-dev` server: create a member with `pane.subpane.create`, list, focus each member, exit a member's shell, restart the server, list again | Members appear with `subpane` info, focus switches the visible member, an exited member disappears, and the stack is back after restart. Visual strip check on a real terminal is left to the user and reported as not run by the agent. |

#### Test Strategy

Aim for the fewest cases that catch the failures that matter. Test count is not a goal.

- **Prioritize:** identity invariants (layout vs `tab.panes`), every removal path, the hidden-source render exit, persistence round-trip and old-file compatibility, the frozen API contract, and the strip/split-handle click conflict.
- **Skip:** trivial accessors, serde derive shapes, keybind config plumbing already covered by the generic keybind tests, and pixel-exact colors.
- **Prove once:** layout math in `strip_layout` unit tests; render tests only check that the strip lands on the border and borders appear; client tests only check routing.
- **Use the project's layers:** unit tests next to code with `test_new` helpers and `assert_invariants_for_test()` (`AGENTS.md` Testing). No new test infrastructure.
- **UI:** cover fit, badge, no-border, and focused vs hidden-status states; skip static content.

### Traceability

| REQ | Mockup | UDES | DES | V | Tasks |
|---|---|---|---|---|---|
| `REQ1` | `M1@R1` | `UDES1`, `UDES2`, `UDES5`, `UDES8`, `UDES13` | `DES1`, `DES2`, `DES3`, `DES5` | `V1`, `V2`, `V3`, `V6`, `V12` | `G1-T1`, `G1-T2`, `G3-T2` |
| `REQ2` | `M1@R1` | `UDES2`, `UDES9`–`UDES14` | `DES1`, `DES3`, `DES7`, `DES8` | `V1`, `V3`, `V4`, `V8`, `V9`, `V12` | `G1-T1`, `G1-T2`, `G4-T1`, `G4-T2`, `G4-T3` |
| `REQ3` | `M1@R1` | `UDES1`, `UDES3`, `UDES4`, `UDES6`, `UDES14` | `DES3`, `DES4`, `DES5` | `V4`, `V5`, `V6` | `G1-T2`, `G1-T3`, `G3-T1`, `G3-T2` |
| `REQ4` | `M1@R1` | `UDES7`, `UDES9` | `DES5`, `DES8` | `V5`, `V9` | `G3-T1`, `G4-T3` |
| `REQ5` | — | — | `DES6` | `V7`, `V12` | `G2-T1` |
| `REQ6` | — | — | `DES4`, `DES7` | `V8` | `G1-T3`, `G4-T1` |
| `REQ7` | — | — | `DES5` | `V6`, `V11` | `G3-T2` |

## 2. Goal

### Status

- **State:** `In Progress`
- **Position:** `G2` and `G3` (parallel)
- **Work branch:** `feat/subpane-tab-strip`
- **Base commit:** `11ab68e9` — the diff base for overall verification and harden

### Sources

- `.s1mple/S1MPLE.md` and its required references
- `.s1mple/docs/project.md`
- `.s1mple/docs/principles.md`
- Accepted mockups: `.s1mple/work/active/w261020lxin-subpane-tab-strip/mockups/`
- Prompts and template: `implementation-prompt.md`, `verification-prompt.md`, `remediation-prompt.md`, `verification-report-template.md` in the `s1mple-ship` skill's `templates/` directory

### Reports

Verifiers and harden write reports here; each report is committed by its writer and never edited afterwards.

```text
.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/
├── g1-verification-report.md … g4-verification-report.md
├── overall-verification-report.md
└── harden-report.md                  written by s1mple-harden
```

### Todos

At execution start, create one todo per line with the todo tool, in this order:

```text
G1 (T1–T3) ← None: Stack model and lifecycle
G2 (T1) ← G1: Persistence and handoff
G3 (T1–T2) ← G1: Strip rendering
G4 (T1–T3) ← G1, G3: API and client interaction
Overall verification
Harden
Deliver
```

Update the todos throughout execution: mark one in progress when it starts, and complete it only when its completion conditions hold with evidence. Mark a group done only after it is accepted and merged. Keep a blocked todo open with the blocker in its description.

### Execution Rules

- Start on the user's execution command. Record the work branch and base commit, create `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/`, then set the state to `In Progress`.
- Self-drive to the end. Ask no questions after the execution command.
- Investigate failures and try safe alternatives. If one group is blocked, continue the groups whose dependencies are met. When no safe, in-scope work remains, record the evidence and set `Blocked`.
- Do not change the decisions in Section 1, skip checks, or bypass permissions.
- Implementers commit their own changes. Do not push, amend, or rewrite history.
- Only the orchestrator writes this file. Commit only this file after each accepted group and after each later stage: `docs: update W261020LXIN ship evidence`.
- **Consultant:** For an exceptional blocker that normal investigation cannot solve, the orchestrator may invoke `s1mple-consultant` with the problem, evidence, and attempts. It advises only.
- **Subagent recovery:** If an agent that must continue is unavailable, spawn a fresh one of the same type with the original assignment, committed outputs, completed checks, and remaining work. Record the replacement under Evidence.

### Group Cycle

```text
fresh s1mple-implementer   implement → check → commit, per task
  → fresh s1mple-verifier  group scope, runs once → commits reports/g<n>-verification-report.md
  → Passed with no findings → accept
  → otherwise fresh s1mple-implementer remediates every finding in the report, once
  → orchestrator checks closure evidence → accept or block
```

1. Fill `implementation-prompt.md` and spawn a fresh `s1mple-implementer`.
2. Fill `verification-prompt.md` with scope `G<n>`, the implementer's change range and return, and report output `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/g<n>-verification-report.md`. Spawn a fresh `s1mple-verifier`. Confirm the report and its commit exist.
3. When the result is not `Passed` with no findings, fill `remediation-prompt.md` in `group` mode with the report path and spawn a fresh `s1mple-implementer`.
4. Accept the group only when every task has a commit SHA or a justified no-change reason, every finding in the report is `Fixed`, and all group checks pass. Record the evidence under [Groups](#groups).

Do not rerun verification or remediation. An unresolved finding blocks the group.

### Group Scheduling

A group is ready when every group in its **Depends on** is accepted and merged. Start every ready group at once.

```text
one ready group and none running → run it in the main tree
otherwise → one worktree per group, run in parallel
accepted → merge into the work branch → recompute ready groups
```

1. For a worktree group:
   - Add `.s1mple/.worktrees/` to `$(git rev-parse --git-common-dir)/info/exclude` if missing.
   - Run `git worktree add -b w261020lxin-g<n> .s1mple/.worktrees/w261020lxin-g<n> HEAD`.
2. Pass the group's directory as `Working directory` in every prompt for that group.
3. Merge accepted groups one at a time from the main tree: `git merge --no-ff --no-commit w261020lxin-g<n>`.
   - Clean: `git commit --no-edit`, `git worktree remove <path>`, `git branch -d w261020lxin-g<n>`.
   - Conflict: `git merge --abort`. Fill `remediation-prompt.md` in `merge` mode and spawn a fresh `s1mple-implementer` in the group worktree, then repeat this step.

Note: the user prefers branches over worktrees for their own work; the internal per-group worktrees above exist only so G2 and G3 can run in parallel, and they are removed after each merge.

## 3. Plan

Each group holds tasks that share one context. G2 and G3 edit disjoint files (`src/persist/**` vs `src/subpane_strip.rs`, `src/main.rs`, `src/ui/**`, `src/server/headless/render.rs`, `src/app/api_helpers.rs`, `src/client/shell.rs`).

```text
G1 Stack model and lifecycle ─┬─► G2 Persistence and handoff
                              └─► G3 Strip rendering ─► G4 API and client interaction
```

### G1 — Stack model and lifecycle

- **Depends on:** `None`
- **Context:** Server-side stack state: `src/workspace/stack.rs` (new), `src/workspace/tab.rs`, `src/workspace.rs`, `src/layout.rs`, `src/app/actions.rs`, and the data-list sites in `src/app/**`, `src/workspace/aggregate.rs`, `src/agent_resume.rs`.
- **Checks:** `just test-one stack`, `just test-one replace_leaf`, `just test-one subpane`, `just lint`
- **Accept when:** Stacks can be created, activated, and dismantled through `Tab`/`Workspace`/`App` APIs with invariants holding on every removal path, and hidden members appear in data lists.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G1-T1` | Add the subpane stack model and leaf replacement | `REQ1`, `REQ2`, `DES1`, `DES2`, `UDES11` | `src/workspace/stack.rs` (new), `src/workspace.rs` (module line), `src/workspace/tab.rs` (field + constructors), `src/layout.rs` | `TDD` | RED `V1` tests in `src/workspace/stack.rs` (add, activate, remove active → next/previous, dissolve at one member, remove hidden, `ordered_ids`) and `V2` tests in `src/layout.rs` (`replace_leaf` focus/prev_focus/ratios/unknown id) → GREEN `SubpaneStacks`, `Tab.subpanes`, `TileLayout::replace_leaf` → REFACTOR keep `layout.rs` edits to the one method | `just test-one stack`, `just test-one replace_leaf` |
| `G1-T2` | Wire stack lifecycle into tab, workspace, and app paths | `REQ1`, `REQ2`, `REQ3`, `DES3`, `UDES6`, `UDES11`, `UDES13` | `src/workspace/tab.rs` (`spawn_subpane`, `detach_pane`, `take_pane_for_move`, activation helper), `src/workspace.rs` (`close_pane`, `remove_pane`, `take_pane_for_move`, seen marking `:455`, `assert_invariants_for_test`, adversarial helper), `src/app/actions.rs` (`focus_pane_in_workspace`, seen marking `:466`), `src/app/state.rs` invariants if they compare layout and panes | `TDD` | RED `V3` and `V4` tests with `Workspace::test_new`/`AppState::test_new` (close active, close hidden, hidden dies, move member out, focus hidden activates, hidden Done until activated, invariants incl. adversarial state with a stack) → GREEN hooks before every `pane_count() <= 1` decision; activation via `replace_leaf`; spawn reuses `SplitCommand` plumbing without inserting a layout leaf → REFACTOR shared removal helper on `Tab` | `just test-one subpane` |
| `G1-T3` | Include hidden members in data lists | `REQ3`, `REQ6`, `DES4` | `src/workspace/tab.rs` (`pane_ids_with_subpanes`), `src/app/api/session.rs`, `src/app/agents.rs`, `src/app/creation.rs`, `src/workspace/aggregate.rs`, `src/app/actions.rs` (`:291`), `src/app/api/tabs.rs` (`:232`), `src/agent_resume.rs` | `Direct — call-site swaps proven by one V3 assertion and the existing suite` | Review every `layout.pane_ids()` site; swap data-list sites to `pane_ids_with_subpanes()`; keep geometry/navigation/visibility sites; list each site and the choice in the group return → extend one `V3` test to assert a hidden member appears in `session_snapshot().panes` and agent infos | `just test-one subpane`, `just test` |

### G2 — Persistence and handoff

- **Depends on:** `G1`
- **Context:** `src/persist/snapshot.rs`, `src/persist/restore.rs`, and handoff import paths they own.
- **Checks:** `just test-one subpane`, `just test-one snapshot`, `just test-one restore`, `just lint`
- **Accept when:** Stacks round-trip through capture/restore and handoff aliases; old fixtures load unchanged.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G2-T1` | Persist and restore subpane stacks | `REQ5`, `DES6` | `src/persist/snapshot.rs`, `src/persist/restore.rs` | `TDD` | RED `V7` tests (round-trip keeps order/active/remapped ids; fixtures without `subpanes` parse and restore; invalid stack dropped with warning; handoff aliases include hidden members) → GREEN optional `subpanes` on `TabSnapshot`, `capture_tab`, `restore_tab` member spawn + remap + rebuild, `handoff_pane_aliases`, `migrated_public_pane_numbers_by_old_raw`, handoff import → REFACTOR add the new field to test struct literals with `..` or `Vec::new()` only | `just test-one subpane`, `just test-one snapshot`, `just test-one restore` |

### G3 — Strip rendering

- **Depends on:** `G1`
- **Context:** Server render path: `src/subpane_strip.rs` (new), `src/main.rs` (module line), `src/ui/panes.rs`, `src/ui.rs`, `src/server/headless/render.rs`, `src/app/state.rs:988`, `src/server/headless/client_views.rs:515`, `src/app/api_helpers.rs` (visibility), `src/client/shell.rs` (visibility of status helpers).
- **Checks:** `just test-one subpane_strip`, `just test-one subpane`, `just lint`, `just test`
- **Accept when:** A slot holding a stack renders the strip or badge on its top border, hidden members are sized to the slot and skipped as render sources.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G3-T1` | Add the pure strip layout and label functions | `REQ3`, `REQ4`, `DES5`, `UDES1`–`UDES4`, `UDES7` | `src/subpane_strip.rs` (new), `src/main.rs` | `TDD` | RED `V5` tests (full fit ranges, badge one column short, badge truncation, below-minimum empty, status-independent geometry, label fallback chain and 20-column truncation) → GREEN `strip_layout`, `subpane_label`, `StripLayout`/`StripSegment` → REFACTOR none beyond the module | `just test-one subpane_strip` |
| `G3-T2` | Render the strip and keep hidden members sized and invisible | `REQ1`, `REQ3`, `REQ4`, `REQ7`, `DES5`, `UDES1`, `UDES4`–`UDES8`, `UDES13`, `UDES14` | `src/ui/panes.rs`, `src/ui.rs` (if the background resize path needs it), `src/server/headless/render.rs`, `src/app/state.rs`, `src/server/headless/client_views.rs`, `src/app/api_helpers.rs`, `src/client/shell.rs` | `TDD` | RED `V6` tests (strip text and hidden status glyph on the top border; single-slot stack tab bordered under `auto`; hidden member size equals active after compute; `any_shell_surface_contains_pane` false for hidden, true after activation) → GREEN draw path in `render_pane_border_titles` with styles from `UDES5`, stack-aware `multi_pane`, hidden-member resize in the resize loops, visibility exclusions, `pub(crate)` on `pane_agent_status`, `status_icon`, `status_color`, `status_priority` → REFACTOR keep strip drawing in `src/subpane_strip.rs` helpers where possible | `just test-one subpane`, `just test` |

### G4 — API and client interaction

- **Depends on:** `G1`, `G3`
- **Context:** API schema and handlers, the client-shell snapshot, and the TUI client input path: `src/api/schema.rs`, `src/api/schema/panes.rs`, `src/app/api.rs`, `src/app/api/panes.rs`, `src/server/headless/client_views.rs`, `src/server/client_commands.rs`, `src/protocol/wire.rs` (`ClientShellPane` only), `src/server/client_shell.rs`, `docs/next/api/herdr-api.schema.json`, `src/config/{model,keybinds}.rs`, `src/input/{keybindings,keybind_help}.rs`, `src/main.rs` (template), `docs/next/website/src/data/config-reference.json`, `src/client/shell/{actions,input,state,composition,mouse}.rs`.
- **Checks:** `just test-one subpane`, `just test-one generated_protocol_schema_artifact_is_current`, `just test-one advertised_client_shell_method_shapes_stay_at_the_v1_contract`, `just lint`, `just test`
- **Accept when:** The API exposes stacks and creates members; keys and strip clicks drive them; no frozen contract changed.

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G4-T1` | Expose stacks through the JSON API and client-shell snapshot | `REQ2`, `REQ6`, `DES7`, `UDES9` | API and snapshot files listed in Context | `TDD` | RED `V8` tests (create adds a member and returns `subpane`; list order includes hidden; focus activates; digest pinned) → GREEN `PaneSubpaneInfo`, `PaneInfo.subpane`, `pane.subpane.create` definition + handler + allowlists, `ClientShellPane.subpane`, schema regeneration with `HERDR_UPDATE_API_SCHEMA=1` → REFACTOR reuse `handle_pane_split` helpers | `just test-one subpane`, `just test-one generated_protocol_schema_artifact_is_current`, `just test-one advertised_client_shell_method_shapes_stay_at_the_v1_contract` |
| `G4-T2` | Add subpane keybinds | `REQ2`, `DES8`, `UDES12` | `src/config/{model,keybinds}.rs`, `src/input/{keybindings,keybind_help}.rs`, `src/main.rs`, `docs/next/website/src/data/config-reference.json`, `src/client/shell/{actions,input}.rs` | `TDD` | RED `V9` key cases (defaults resolve; next/previous wrap; jump by index; no-op on plain pane; new maps to `pane.subpane.create`) → GREEN config fields and defaults, `KeybindAction` variants, resolver rows, help entries, template comments, reference entries, `endpoint_method_for_action` mapping, `indexed_navigation_target_exists` → REFACTOR none | `just test-one subpane`, `python3 -m unittest scripts/test_config_reference_check.py` via `just test` |
| `G4-T3` | Route strip clicks from the client | `REQ2`, `REQ4`, `DES8`, `UDES9`, `UDES10` | `src/client/shell/{state,composition,mouse}.rs` | `TDD` | RED `V9` mouse cases (member → `pane.focus`; `×` → `pane.close`; `+` → `pane.subpane.create`; badge → `pane.focus` next; strip click on a split-handle row does not start a drag; no hits when `inner_rect.y == rect.y`) → GREEN `ShellHitMap` strip hits from snapshot + `strip_layout`, `Down(Left)` branch before split handles → REFACTOR none | `just test-one subpane`, `just test` |

## 4. Overall Verification and Harden

Start after every group is accepted and merged.

```text
fresh verifier (overall, runs once) → Passed → harden
                                    → Failed → fresh implementer remediates all OVR findings, once → harden
                                    → Blocked → set Blocked
harden → Deliver
```

1. Fill `verification-prompt.md` with scope `overall`, change range `<base-commit>..HEAD`, all `V` cases (including `V10`–`V12`), and report output `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/overall-verification-report.md`. Spawn a fresh `s1mple-verifier` in the main tree. Confirm the report and its commit exist.
2. When it fails, fill `remediation-prompt.md` in `overall` mode with the report path and spawn one fresh `s1mple-implementer` for every `OVR-VER<n>` finding.
3. Run `s1mple-harden` in workflow mode. Use this file as the goal and **Base commit** as the base.

## 5. Deliver

1. Read `.s1mple/docs/project.md`. Add the subpane row to Fork Customizations and update its Last Modified date. Commit only that file:

   ```text
   docs(project): record native subpane stacks customization

   Work: W261020LXIN — subpane-tab-strip
   ```

2. Apply the Delivery behavior in `.s1mple/S1MPLE.md`. Do not push, open a PR, merge, or deploy without the user's authorization.
3. Fill [Completion Summary](#completion-summary), set the state to `Completed`, and commit this file.

## 6. Evidence

### Groups

| Group | Directory | Task commits | Verification report → status, SHA | Findings → fix SHAs | Merge SHA | Result |
|---|---|---|---|---|---|---|
| `G1` | main tree | `b83908f1` (T1), `b68875f3` (T2), `c50bd7c7` (T3) | `reports/g1-verification-report.md` — `Failed`, `982bbb24` | `G1-VER1` → `0f6ecbc0`, `G1-VER2` → `a5769ee0` | Not applicable (main tree) | Accepted — all checks pass after remediation (`just test` 3975 passed, `just lint` clean) |
| `G2` | `Pending` | `Pending` | `reports/g2-verification-report.md` — `Pending` | `Pending` | `Pending` | `Pending` |
| `G3` | `Pending` | `Pending` | `reports/g3-verification-report.md` — `Pending` | `Pending` | `Pending` | `Pending` |
| `G4` | `Pending` | `Pending` | `reports/g4-verification-report.md` — `Pending` | `Pending` | `Pending` | `Pending` |

### Verification Cases

| Case | Actual result |
|---|---|
| `V1` | Passed — `just test-one stack` 21/21 |
| `V2` | Passed — `just test-one replace_leaf` 2/2 |
| `V3` | Passed — `just test-one subpane` 11/11 (incl. move/swap onto hidden member regressions) |
| `V4` | Passed — focus activates; hidden idle agent reports Done until activated |
| `V5` | `Pending` |
| `V6` | `Pending` |
| `V7` | `Pending` |
| `V8` | `Pending` |
| `V9` | `Pending` |
| `V10` | `Pending` |
| `V11` | `Pending` |
| `V12` | `Pending` |

### Overall Verification

- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/overall-verification-report.md` — `Pending`
- **Result and report SHA:** `Pending`
- **Findings → fix SHAs:** `Pending`

### Harden

- **Report:** `.s1mple/work/active/w261020lxin-subpane-tab-strip/reports/harden-report.md` — `Pending`
- **Result:** `Pending`

### Deliver Evidence

- **Project document:** `Pending`
- **Delivery actions:** `Pending`

### Blockers, Deviations, and Replacements

- G1: `Tab`/`Workspace` stack methods live in `src/workspace/stack.rs` instead of `tab.rs` (smaller upstream diff). `src/app/terminal_targets.rs:141` was also switched to member-inclusive ids so hidden agents can be found by name.
- G1: `pane_is_in_active_tab` treats hidden members as outside the active tab, so a hidden agent that finishes now notifies (toast/sound) even when its tab is active — consistent with `REQ3` because the user cannot see it.
- G1: `pane.move`/`pane.swap` aimed at a hidden member resolve to the slot's visible leaf (`G1-VER1` fix).
- G1: `layout.updated` is emitted by `pane.focus` activation and stack-member removal; activation through `AppState::focus_pane_in_workspace` alone (no API call) emits nothing — client actions go through `pane.focus`, checked again in G4.
- G1: commit `0f6ecbc0` body says `Task: G1-T3 — Include hidden subpanes in pane and agent lists`; the plan title is `Include hidden members in data lists` (history not rewritten).

### Completion Summary

`Pending`

## Ship File Check

- [x] Every `REQ` has at least one `UDES` or `DES`, one `V`, and one task.
- [x] Every UI `REQ` has a `UDES` derived from an explicitly accepted mockup revision.
- [x] No `DES` describes the UI, and no `UDES` has a `pending` `Uses`.
- [x] Every `UDES` and `DES` has a `V` and a task.
- [x] The Q&A Log has a row for every answered question and assumption.
- [x] When UI is needed, the mockup manifest is `Accepted`, and its accepted revisions match the UI Mockups section.
- [x] Every task cites the `REQ`/`UDES`/`DES` it builds, and every `V` cites the `REQ` it proves and the `UDES` or `DES` it checks.
- [x] There are at most 4 groups, and no dependency cycles.
- [x] Each group's tasks share one context.
- [x] Parallel groups do not edit the same files, or a dependency orders them.
- [x] Every `V` has a runnable command or a concrete manual method.
- [x] Every `V` covers a unique, important risk at the lowest layer that catches it; no trivial or redundant case.
- [x] Every task has an approach, and every `V` that needs a new test is owned by a `TDD` task.
- [x] No placeholder remains.
