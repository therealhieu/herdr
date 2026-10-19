# W261019AKM5 Client bold color — Ship

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
  - [G1 — Bold color config and client presentation](#g1--bold-color-config-and-client-presentation)
- [4. Overall Verification and Harden](#4-overall-verification-and-harden)
- [5. Deliver](#5-deliver)
- [6. Evidence](#6-evidence)
  - [Groups](#groups)
  - [Verification Cases](#verification-cases)
  - [Overall Verification](#overall-verification)
  - [Harden](#harden)
  - [Deliver Evidence](#deliver-evidence)
  - [Blockers, Deviations, and Replacements](#blockers-deviations-and-replacements)
  - [Completion Summary](#completion-summary)
- [Ship File Check](#ship-file-check)

## 1. Decisions

- **Prompt:** Add a client-side `[ui] bold_color` setting that mirrors Ghostty's `bold-color`, so bold text looks the same in kitty (which has no such option) as in Ghostty (`bold-color = #f8dea6`).
- **Goal:** With `[ui] bold_color = "#f8dea6"`, the herdr TUI client draws bold default-foreground text in `#f8dea6` and bold palette colors 0–7 as their bright variants; with the key unset, output is byte-for-byte unchanged.
- **Scope:** config key, client frame and surface-patch presentation, live config reload · **Non-goals:** server-side `TerminalAnsi` encoding, direct terminal attach passthrough, other `docs/` edits beyond the `Q1` config-reference entry, wire protocol changes.
- **UI:** `not needed — no new screen, component, or interaction; the change recolors existing cells by a fixed rule copied from Ghostty`

```text
today:  pane cell (bold, fg=default) → BlitEncoder → "\e[0;1;39;49m" → kitty draws white
after:  pane cell (bold, fg=default) → apply bold_color → BlitEncoder → "\e[0;1;38;2;248;222;166;49m" → yellow
```

### Q&A Log

| Q | Question | Answer | Decides |
|---|---|---|---|
| `A1` | Where does the transform run? | Client only, on `FrameData` cells before `BlitEncoder::encode`, next to the existing hyperlink and drawn-cursor transforms — `src/client/state.rs:569` | `DES2` |
| `A2` | Copy which Ghostty semantics? | Ghostty `Style.fg()`: bold + default fg → configured color; bold + palette 0–7 → index + 8 in both modes; `bright` leaves default fg unchanged — `vendor/libghostty-vt/src/terminal/style.zig:170` | `REQ1`, `REQ2`, `DES1` |
| `A3` | Ghostty also recolors bold RGB fg equal to the default fg. Copy it? | No. Herdr sends a matching default fg as reset (SGR 39), so reset covers it; panes that set an explicit default fg via OSC 10 keep their RGB — `src/pane/terminal.rs:3519` | `DES1` |
| `A4` | Pane cells only, or the whole frame? | Whole composed frame, like Ghostty, which recolors everything it draws. The client cannot tell pane cells from herdr chrome after composition | `REQ1`, `DES2` |
| `A5` | Value format? | Unset (default), `"bright"`, or `"#rrggbb"`, matching Ghostty's accepted forms | `REQ3`, `DES1` |
| `A6` | Invalid value handling? | Same as other typed `[ui]` keys: serde rejects the `[ui]` section and the loader reports it — `src/config/io.rs:553` | `DES1` |
| `A7` | Document in `docs/next` config reference? | ~~No~~ Superseded by `Q1`: the assumption that `config_reference_check.py` runs only in `release-docs-check` was wrong; `scripts/test_config_reference_check.py:253` runs it in every `just test` | `DES4` |
| `A8` | Performance? | One integer pass over cells per presented frame or patch, client-side, independent of pane count; skipped entirely when unset. No 1-vs-15-pane benchmark: `bench-render-scale` measures server render work, which does not change | `DES2`, `V3` |
| `Q1` | During G1, `just test` failed because `ui.bold_color` is missing from the config reference. How should the fork satisfy the check? | Add a `ui.bold_color` entry to `docs/next/website/src/data/config-reference.json` (user chose the recommended option over `pub(crate)` or a checker exception) | `DES4` |

### Requirements

| ID | Requirement | Source |
|---|---|---|
| `REQ1` | With `bold_color = "#rrggbb"`, bold cells whose fg is the default color render in that RGB color, and bold cells with palette colors 0–7 (named or indexed) render as palette 8–15 | prompt, `A2`, `A4` |
| `REQ2` | With `bold_color = "bright"`, bold palette 0–7 render as 8–15 and bold default fg stays default | `A2`, `A5` |
| `REQ3` | With `bold_color` unset, client output is unchanged | prompt, `A5` |
| `REQ4` | Changing `bold_color` and reloading config updates the visible frame without restarting the client | prompt |

### UI Mockups

Not applicable — no new UI surface; the rule is fixed by Ghostty's behavior.

### UI Design

Not applicable — no UI.

### System Design

| ID | Decision | Builds | Why |
|---|---|---|---|
| `DES1` | New `src/config/bold_color.rs`: `BoldColorConfig { Bright, Rgb(u8, u8, u8) }` with a serde string deserializer (`"bright"` or `#rrggbb`), plus `bold_color: Option<BoldColorConfig>` on `UiConfig` (default `None`) in `src/config/model.rs`; re-export from `src/config.rs` | `REQ1`, `REQ2`, `REQ3` | New file keeps the upstream diff to a field, a default, and a re-export |
| `DES2` | New `src/client/bold_color.rs`: `apply(cells: &mut [CellData], rule: BoldColorConfig)` rewrites `CellData.fg` for cells with the bold modifier bit: reset → packed RGB (`Rgb` only); named 0x01–0x08 → +8; indexed 0–7 → +8. Called in `ClientState::try_present_frame_to` on `frame_data.cells` and in `present_surface_patch` on each row's cells, before encoding, only when `Some` | `REQ1`, `REQ2`, `REQ3` | Both client presentation paths feed `BlitEncoder`; transforming before encode keeps the encoder's committed frame consistent for diffing |
| `DES3` | `ClientState.bold_color: Option<BoldColorConfig>`, set from `ClientLoopConfig` at startup and from `reload_local_client_config` when `[ui]` is valid; the existing reload recompose then presents a recolored frame through the encoder diff | `REQ4` | Follows the `draw_host_cursor` pattern — `src/client/config_reload.rs:118` |
| `DES4` | Record the customization in `.s1mple/docs/project.md` Fork Customizations, and add a `ui.bold_color` entry to `docs/next/website/src/data/config-reference.json` beside the other `ui.*` keys (`Q1`) | `REQ1` | Fork policy 5 allows a `docs/` edit the change requires; the reference test requires it |

### Verification

| ID | Proves | Checks | Layer | Command or method | Expected |
|---|---|---|---|---|---|
| `V1` | `REQ1`, `REQ2`, `REQ3` | `DES2` | unit | `just test-one bold_color` | Rule table: `Rgb` maps bold reset → `0x02rrggbb`; bold named red `0x02` → `0x0A`; bold indexed 1 → indexed 9; bold indexed 8, 200, RGB, and non-bold cells unchanged; `Bright` leaves bold reset unchanged |
| `V2` | `REQ1`, `REQ2`, `REQ3` | `DES1` | unit | `just test-one bold_color` | `[ui] bold_color = "#f8dea6"` parses to `Rgb(248,222,166)`; `"bright"` to `Bright`; missing key to `None`; `"yellow"` and `"#f8dea"` fail the `[ui]` section |
| `V3` | `REQ1`, `REQ3` | `DES2` | unit | `just test-one bold_color` | Through `ClientState::try_present_frame_to` with a test writer: a bold default cell emits `38;2;248;222;166` when configured, and the emitted bytes equal the unconfigured baseline when unset; a surface patch recolors the same way |
| `V4` | `REQ4` | `DES3` | Manual | Run the dev client in kitty, set `bold_color`, reload config (`herdr server reload-config` or the reload key) | Bold default text turns `#f8dea6` without restarting the client |
| `V5` | all | `DES1`–`DES4` | lint + tests | `just lint && just test` | Both pass |

#### Test Strategy

- Prove the color rule once at the unit layer (`V1`); prove the config contract once (`V2`).
- `V3` covers the integration risk that matters: the transform sits on both client presentation paths and keeps unset output identical.
- Reload is plumbing through an existing path; prove it manually (`V4`) instead of adding client-loop test infrastructure.

### Traceability

| REQ | Mockup | UDES | DES | V | Tasks |
|---|---|---|---|---|---|
| `REQ1` | — | — | `DES1`, `DES2`, `DES4` | `V1`, `V2`, `V3` | `G1-T1`, `G1-T2`, `G1-T4` |
| `REQ2` | — | — | `DES1`, `DES2` | `V1`, `V2` | `G1-T1`, `G1-T2` |
| `REQ3` | — | — | `DES1`, `DES2` | `V1`, `V2`, `V3` | `G1-T1`, `G1-T2` |
| `REQ4` | — | — | `DES3` | `V4` | `G1-T3` |

## 2. Goal

### Status

- **State:** `In Progress`
- **Position:** `Deliver`
- **Work branch:** `feat/bold-color`
- **Base commit:** `7c59eab9`

### Sources

- `.s1mple/S1MPLE.md` and its required references
- `.s1mple/docs/project.md`
- `.s1mple/docs/principles.md`
- Accepted mockups: `Not applicable — no UI`
- Prompts and template: `implementation-prompt.md`, `verification-prompt.md`, `remediation-prompt.md`, `verification-report-template.md` in the `s1mple-ship` skill's `templates/` directory

### Reports

Verifiers and harden write reports here; each report is committed by its writer and never edited afterwards.

```text
.s1mple/work/active/w261019akm5-bold-color/reports/
├── g1-verification-report.md
├── overall-verification-report.md
└── harden-report.md
```

### Todos

At execution start, create one todo per line with the coding agent's todo tool, in this order:

```text
G1 (T1–T4) ← None: Bold color config and client presentation
Overall verification
Harden
Deliver
```

Update the todos throughout execution: mark one in progress when it starts, and complete it only when its completion conditions hold with evidence. Mark a group done only after it is accepted and merged. Keep a blocked todo open with the blocker in its description.

### Execution Rules

- Start on the user's execution command. Record the work branch and base commit, create the reports folder, then set the state to `In Progress`.
- Self-drive to the end. Ask no questions after the execution command.
- Investigate failures and try safe alternatives. When no safe, in-scope work remains, record the evidence and set `Blocked`.
- Do not change the decisions in Section 1, skip checks, or bypass permissions.
- Implementers commit their own changes. Do not push, amend, or rewrite history.
- Only the orchestrator writes this file. Commit only this file after each accepted group and after each later stage: `docs: update W261019AKM5 ship evidence`.
- **Consultant:** For an exceptional blocker that normal investigation cannot solve, the orchestrator may invoke `s1mple-consultant` with the problem, evidence, and attempts. It advises only.
- **Subagent recovery:** If an agent that must continue is unavailable, spawn a fresh one of the same type with the original assignment, committed outputs, completed checks, and remaining work. Record the replacement under Evidence.

### Group Cycle

```text
fresh s1mple-implementer   implement → check → commit, per task
  → fresh s1mple-verifier  group scope, runs once → commits reports/g1-verification-report.md
  → Passed with no findings → accept
  → otherwise fresh s1mple-implementer remediates every finding in the report, once
  → orchestrator checks closure evidence → accept or block
```

1. Fill `implementation-prompt.md` and spawn a fresh `s1mple-implementer`.
2. Fill `verification-prompt.md` with scope `G1`, the implementer's change range and return, and report output `reports/g1-verification-report.md`. Spawn a fresh `s1mple-verifier`. Confirm the report and its commit exist.
3. When the result is not `Passed` with no findings, fill `remediation-prompt.md` in `group` mode with the report path and spawn a fresh `s1mple-implementer`.
4. Accept the group only when every task has a commit SHA or a justified no-change reason, every finding in the report is `Fixed`, and all group checks pass.

Do not rerun verification or remediation. An unresolved finding blocks the group.

### Group Scheduling

One group, so it runs in the main tree on the work branch. No worktrees or merges.

## 3. Plan

```text
G1 Bold color config and client presentation
```

### G1 — Bold color config and client presentation

- **Depends on:** `None`
- **Context:** `src/config/` (`bold_color.rs` new, `model.rs`, `config.rs`) and `src/client/` (`bold_color.rs` new, `state.rs`, `mod.rs`, `loop_config.rs`, `config_reload.rs`)
- **Checks:** `just test-one bold_color`, `just lint && just test`
- **Accept when:** `V1`–`V3` and `V5` pass, and `V4` is observed in kitty

| Task | Title | Builds | Files | Approach | Steps | Checks |
|---|---|---|---|---|---|---|
| `G1-T1` | Add `ui.bold_color` config type | `REQ1`, `REQ2`, `REQ3`, `DES1` | `src/config/bold_color.rs`, `src/config/model.rs`, `src/config.rs` | `TDD` | RED `V2` tests in `src/config/bold_color.rs` (parse `#f8dea6`, `bright`, missing, invalid via `UiConfig`) → GREEN `BoldColorConfig`, its `Deserialize`, the `UiConfig.bold_color` field and default → REFACTOR none | `just test-one bold_color` |
| `G1-T2` | Apply the bold color rule to presented cells | `REQ1`, `REQ2`, `REQ3`, `DES2` | `src/client/bold_color.rs`, `src/client/mod.rs`, `src/client/state.rs` | `TDD` | RED `V1` rule tests in `src/client/bold_color.rs` and `V3` presentation tests next to `ClientState` tests → GREEN `bold_color::apply`, `ClientState.bold_color`, calls in `try_present_frame_to` and `present_surface_patch` → REFACTOR none | `just test-one bold_color` |
| `G1-T3` | Load and reload `bold_color` in the client | `REQ4`, `DES3` | `src/client/mod.rs`, `src/client/loop_config.rs`, `src/client/config_reload.rs`, `src/client/tests/mod.rs` if signatures change | `Direct — plumbing through the existing reload path; V4 proves it manually and V5 covers compile and existing reload tests` | Add `ClientLoopConfig.bold_color` from `loaded_config.config.ui`; set `ClientState.bold_color` at startup; assign it in `reload_local_client_config` beside `draw_host_cursor` → run `V4` and `V5` | `just lint && just test` |
| `G1-T4` | Record the fork customization | `REQ1`, `DES4` | `.s1mple/docs/project.md` | `Direct — documentation` | Add a Fork Customizations row naming the touched upstream files and conflict risk | Review |

## 4. Overall Verification and Harden

Start after G1 is accepted.

```text
fresh verifier (overall, runs once) → Passed → harden
                                    → Failed → fresh implementer remediates all OVR findings, once → harden
                                    → Blocked → set Blocked
harden → Deliver
```

1. Fill `verification-prompt.md` with scope `overall`, change range `<base-commit>..HEAD`, all `V` cases, and report output `reports/overall-verification-report.md`. Spawn a fresh `s1mple-verifier` in the main tree. Confirm the report and its commit exist.
2. When it fails, fill `remediation-prompt.md` in `overall` mode with the report path and spawn one fresh `s1mple-implementer` for every `OVR-VER<n>` finding.
3. Run `s1mple-harden` in workflow mode. Use this file as the goal and **Base commit** as the base.

## 5. Deliver

1. Read `.s1mple/docs/project.md`. Update facts the change affected and its Last Modified date. Commit only that file:

   ```text
   docs(project): record client bold color customization

   Work: W261019AKM5 — Client bold color
   ```

   If nothing is affected, record why under [Deliver Evidence](#deliver-evidence) without an empty commit.
2. Apply the Delivery behavior in `.s1mple/S1MPLE.md`: propose the commit messages, push only to `origin`, open the PR on the fork with `gh pr create --base master`, and wait for `Fork CI`. Do not push, open a PR, merge, or deploy without the user's authorization.
3. Fill [Completion Summary](#completion-summary), set the state to `Completed`, and commit this file.

## 6. Evidence

### Groups

| Group | Directory | Task commits | Verification report → status, SHA | Findings → fix SHAs | Merge SHA | Result |
|---|---|---|---|---|---|---|
| `G1` | main tree | `6842bd84` T1, `e8fffcf7` T2, `adb6237c` T3, `4268444c` `1bc58df1` `3ee5f750` T4 | `reports/g1-verification-report.md` — `Passed`, `4c5da113` | None | Not applicable — main tree | `Accepted` |

### Verification Cases

| Case | Actual result |
|---|---|
| `V1` | Passed — `just test-one bold_color`, rule table tests green (8/8 in filter) |
| `V2` | Passed — config parse tests green (`#f8dea6`, `bright`, missing, invalid) |
| `V3` | Passed — frame recolor, unset byte-identical baseline, surface patch |
| `V4` | `Pending` |
| `V5` | Passed — `just lint && just test`: nextest 3917 passed / 14 skipped; Python 150 + 6 OK; Bun 0 failures |

### Overall Verification

- **Report:** `reports/overall-verification-report.md` — `Passed`
- **Result and report SHA:** `Passed`, `f680c40d`; 0 findings; `REQ4` Unverified pending manual `V4`
- **Findings → fix SHAs:** None

### Harden

- **Report:** `reports/harden-report.md` — `d72ffee4`, resolution `b4ee5192`
- **Result:** 1 Minor finding (`HRD-1`, reload of `bold_color` not asserted) — `Fixed` in `c320048d`; `just lint && just test` passed after the fix

### Deliver Evidence

- **Project document:** `.s1mple/docs/project.md` Fork Customizations row (G1-T4) and Last Modified — `a1a2a05f`
- **Delivery actions:** Pending user authorization to push `feat/bold-color` to `origin` and open a fork PR; `V4` manual kitty check pending with the user

### Blockers, Deviations, and Replacements

- **Decision change (`Q1`):** `A7` was wrong — `scripts/test_config_reference_check.py:253` runs the reference check in every `just test`. The user chose to add a `ui.bold_color` entry to `docs/next/website/src/data/config-reference.json` (`1bc58df1`). Its `values` read `bright`/`rgb` because the checker derives them from enum variants; upstream does the same for `terminal.new_cwd`.
- **Test placement:** presentation tests live in `src/client/bold_color.rs` to keep upstream edits small.
- **Fidelity limit:** server-side inverse resolution means bold inverse default-color cells do not take the bold color as background, unlike Ghostty (harden Limits).
- **Replacements:** None

### Completion Summary

`Pending`

## Ship File Check

Confirm before setting the state to `Ready` and committing:

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
