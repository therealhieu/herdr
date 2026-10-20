# herdr (therealhieu fork)

- **Last Modified:** 2026-10-20 +07

Use this as the reusable baseline for discussion and plan inspection. Keep change-specific details in work documents. Cite important claims, distinguish rules from observations, and link authoritative sources instead of duplicating them. Repository-root paths are written as `path`; document links are relative to this file.

## Purpose and Scope

Herdr is a terminal-based runtime for coding agents: a server owns sessions, workspaces, tabs, panes (PTYs), and agent detection, and a TUI attaches as one client ([README](../../README.md), [AGENTS.md](../../AGENTS.md)).

This repository is a **long-lived custom fork**. It is maintained alongside upstream and is not intended to be merged back.

| Remote | Repository | Role |
|---|---|---|
| `origin` | `therealhieu/herdr` | This fork; all pushes and PRs go here |
| `upstream` | `herdrdev/herdr` | Source of upstream releases; fetch only |

At initialization (2026-10-19, `143bf5fd`), fork `master` equals `upstream/master`; there are no custom changes yet.

## Fork Policy

The goal is a custom version that can still absorb upstream releases cheaply.

1. **Upstream universal rules still apply.** Follow the "Universal Project Rules", "Testing", "Commit Style", and "Code Conventions" sections of [AGENTS.md](../../AGENTS.md). They keep fork code compatible with upstream clients and servers and keep merges small.
2. **Upstream maintainer-only rules do not apply.** `therealhieu` is not in `.github/MAINTAINERS`, so skip "Maintainer Workflow", "Local Can Machine Workflow", and "Release Channels".
3. **Never contribute to upstream.** Do not push, open PRs, or open issues on `herdrdev/herdr`. The `gh` default repo is `therealhieu/herdr` (`gh repo set-default`); confirm with `gh repo set-default --view` in a fresh clone.
4. **Minimize the diff against upstream.** Prefer new files and modules over editing upstream hot files. When an upstream file must change, keep the edit small and local.
5. **Leave upstream-owned files alone** unless the change requires it: `AGENTS.md`/`CLAUDE.md`, `README*.md`, `CHANGELOG.md`, `docs/`, `distribution/`, `skills/herdr/SKILL.md`, existing files in `.github/`. Put fork guidance in `.s1mple/`; add fork-only files (such as `fork-ci.yml`) instead of editing upstream ones.
6. **Record every customization** in [Fork Customizations](#fork-customizations) so upstream syncs know what to protect.
7. **Sync by merge.** Bring upstream in with `git fetch upstream && git merge upstream/master` on a sync branch, run `just lint && just test`, then merge into fork `master`. Do not rebase fork `master`; it is published.
8. **macOS only.** The fork serves one user on macOS. Verify on macOS only; skip Windows checks (`just windows-lint`, `just check`'s Windows stage, xwin). Still follow upstream compile-gating rules so upstream code keeps merging cleanly.

### Fork Customizations

| Change | Files touched | Upstream conflict risk | Notes |
|---|---|---|---|
| s1mple workspace | `.s1mple/` | None (fork-only directory) | Agent guidance for this fork |
| macOS-only CI | `.github/workflows/fork-ci.yml` (new file); upstream workflows disabled in repo settings | None for the file; on sync, re-align its action pins, Rust, Zig, and Bun versions with upstream `ci.yml` | Mirrors upstream `ci.yml` macOS job |
| Client `[ui] bold_color` (Ghostty `bold-color`: `"bright"` or `"#rrggbb"`) | New: `src/config/bold_color.rs`, `src/client/bold_color.rs`. Edited: `src/config.rs` (module + re-export), `src/config/model.rs` (`UiConfig` field + default), `src/client/state.rs` (field + two `bold_color::apply` calls), `src/client/mod.rs`, `src/client/loop_config.rs`, `src/client/config_reload.rs`, `src/client/tests/mod.rs` (reload argument), `docs/next/website/src/data/config-reference.json` (one `ui.bold_color` entry, required by `scripts/test_config_reference_check.py` in `just test`) | Low: one-line additions in upstream structs and the reload signature, plus one JSON object after `ui.redraw_on_focus_gained`; conflicts only if upstream edits those lines | Client-only recolor before `BlitEncoder`; no wire changes. The reference `values` are `bright`/`rgb` because the checker derives them from the enum variants |
| Kitty text sizing passthrough (OSC 66) with `[terminal] text_sizing` (default `true`) | New: `src/text_sizing.rs`, `src/text_sizing/{cell,tracker,inject,present,text}.rs`, `src/pane/terminal/text_sizing_tests.rs`, `src/client/state/text_sizing_tests.rs`, `src/server/render_stream/text_sizing_tests.rs`. Edited: `src/main.rs` (module line), `crates/ghostty-vt/src/lib.rs` (`MODE_WRAPAROUND`, `cursor_x`, `cursor_pending_wrap`), `src/pane/terminal.rs` (tracker fields, observe gate, `TextSizing` event arm, test module line), `src/pane.rs` (`decode_text` in text reads and selection), `src/client/state.rs` (`Presenter` field and three calls in `try_present_frame_to`, patch-path fallback, test module line), `src/client/mod.rs` (`set_host_supported`), `src/client/handshake.rs` (kitty profile check, hello field), `src/protocol/endpoint.rs` (optional `text_sizing` hello field), `src/protocol/render_ansi.rs` (`build_sgr` made `pub(crate)`), `src/server/client_transport.rs` (`ClientRegistration` guard), `src/server/render_stream.rs` (`apply_fallback` for direct-terminal clients, test module line), `src/config.rs`, `src/config/model.rs` (`TerminalConfig.text_sizing`), `docs/next/website/src/data/config-reference.json` (one `terminal.text_sizing` entry) | Low to medium: small hooks in hot files `src/pane/terminal.rs`, `src/client/state.rs` and `src/server/render_stream.rs` conflict only if upstream rewrites those exact lines; on a libghostty-vt update, re-check the three `crates/ghostty-vt` accessors | Server writes self-describing placeholder cells (`U+10EEED` plus variation-selector marks) into the core grid; the TUI client re-emits OSC 66 to kitty hosts and shows unscaled fallback text elsewhere. Active only while a kitty-capable client is attached. No `PROTOCOL_VERSION`, frozen codec or fixture change; the hello field is optional and skipped when false |
| Native subpanes: a pane slot holds a stack of terminals with a tab strip and agent status on its top border, badge `[i/n g]` fallback, keys `new_subpane` (`prefix+t`), `next_subpane` (`prefix+period`), `previous_subpane` (`prefix+comma`), indexed `subpane`, and API method `pane.subpane.create` | New: `src/workspace/stack.rs` (model and lifecycle), `src/subpane_strip.rs` (pure strip layout, label, drawing), `src/app/api/subpanes.rs`, `src/client/shell/subpanes.rs`, `src/client/shell/tests/subpanes.rs`. Edited (small hooks): `src/main.rs` (module line), `src/layout.rs` (`replace_leaf`), `src/workspace/tab.rs` (`subpanes` field, removal hooks), `src/workspace.rs` (removal hooks, invariant, adversarial helper), `src/workspace/aggregate.rs`, `src/app/{actions,agents,agent_resume,creation,mod,state,terminal_targets,terminal_titles,api_helpers}.rs`, `src/app/api.rs`, `src/app/api/{panes,session,tabs}.rs` (member-inclusive lists, focus activation, hidden-target resolution, `layout.updated`), `src/ui.rs`, `src/ui/panes.rs` (stack-aware chrome, hidden-member resize, strip drawing), `src/server/headless.rs`, `src/server/headless/{render,client_views}.rs`, `src/server/client_shell.rs`, `src/server/client_commands.rs` (pinned method digest), `src/persist/{snapshot,restore}.rs` (optional `subpanes` on `TabSnapshot`, restore and handoff), `src/api/schema.rs`, `src/api/schema/{panes,operations,tests}.rs`, `src/protocol/wire.rs` (optional `ClientShellPane.subpane`), `src/config/{model,keybinds}.rs`, `src/input/{keybindings,keybind_help}.rs`, `src/client/mod.rs`, `src/client/shell.rs` (status helpers `pub(crate)`), `src/client/shell/{actions,input,mouse,composition,state}.rs`, `docs/next/api/herdr-api.schema.json` (regenerated), `docs/next/website/src/data/config-reference.json` (4 key entries), plus test helpers | Medium: many small hooks in hot upstream files (`src/ui/panes.rs`, `src/persist/restore.rs`, `src/app/api/panes.rs`, `src/client/shell/actions.rs`, config keybind tables); conflicts appear when upstream edits the same lines. A regenerated API schema conflicts whenever upstream changes the schema: regenerate after merging | Side table `Tab.subpanes`: the BSP leaf is always the active member, hidden members live only in `tab.panes`, so `Node`, `LayoutSnapshot`, and `LayoutNode` are unchanged. No `PROTOCOL_VERSION`, frozen codec, fixture, `SNAPSHOT_VERSION`, or `HANDOFF_VERSION` change. Known limits: older binaries drop hidden members on restore; older clients can cycle into hidden members; `pane_borders = off` shows no strip; `layout.export`/`layout.apply` ignore stacks |

## Technology Stack

| Technology | Role | Version or configuration source |
|---|---|---|
| Rust (edition 2021) | Server, TUI client, CLI | `rust-toolchain.toml` (1.96.1), `Cargo.toml` |
| tokio, ratatui, crossterm | Async runtime, TUI rendering, terminal I/O | `Cargo.toml` |
| libghostty-vt (vendored, Zig 0.16.0 build) | Terminal emulation core | `vendor/libghostty-vt.vendor.json`, `crates/ghostty-vt` |
| portable-pty (vendored patch) | PTY abstraction | `Cargo.toml` `[patch.crates-io]` |
| Python 3 (stdlib) | Release, docs, and maintenance scripts and their tests | `scripts/`, `justfile` |
| Bun / Node | Agent integration assets tests, docs tooling | `justfile`, `scripts/docs/` |
| just | Task runner for all checks | `justfile` |
| Nix | Packaging | `flake.nix`, `nix/package.nix` |

## Repository and Component Map

| Path | Component and responsibility |
|---|---|
| `src/server/` | Server runtime: sessions, panes, client fanout |
| `src/client/` | Client-side TUI connection and endpoint catalog |
| `src/app/` | `AppState` (pure data), actions, input, API handling |
| `src/workspace.rs`, `src/layout.rs` | Workspace/tab/pane organization and layout |
| `src/pane.rs`, `src/pane/`, `src/pty/`, `src/terminal/` | Pane state, PTY runtime, terminal state |
| `src/ui/` | Rendering (pure, reads `&AppState`) |
| `src/detect/` | Agent status detection; manifests in `src/detect/manifests/` |
| `src/protocol/` | Wire protocol (`wire.rs`, `PROTOCOL_VERSION`) and endpoint codecs |
| `src/api/` | JSON API schema and handlers |
| `src/platform/` | OS-specific code (`linux/`, `macos/`, `windows/`) |
| `src/integration/` | Agent integrations; JS/TS/shell hook assets in `src/integration/assets/` |
| `src/remote/` | SSH/remote attach |
| `crates/ghostty-vt/` | Rust bindings to vendored libghostty-vt |
| `tests/` | Integration tests (spawn real server/client) |
| `scripts/` | Release, docs, Windows, and maintenance tooling with unit tests |
| `vendor/` | Vendored libghostty-vt and portable-pty plus local patches |

## Architecture and Key Flows

```text
agent CLI ─PTY─► pane runtime ─► terminal state (libghostty-vt) ─► detector (screen snapshot)
                                          │
                              server state (AppState, workspaces)
                                          │
                 ┌────────────────────────┼──────────────────────┐
                 ▼                        ▼                      ▼
        TUI client (endpoint)     JSON API / events       herdr CLI commands
```

Design rules (documented in [AGENTS.md](../../AGENTS.md) "Principles"): state is separated from runtime, render is pure, platform code is isolated, detection is decoupled and evidence-based, and new shared behavior belongs in server state exposed through the JSON API, not the private TUI socket.

## Boundaries and Dependencies

| Boundary | Responsibilities and dependencies | Source |
|---|---|---|
| State vs runtime | `AppState`/`PaneState` are pure data; PTYs live in `PaneRuntime` | [AGENTS.md](../../AGENTS.md) Principles |
| Server vs TUI client | Runtime facts in server/API; presentation state in client only | [AGENTS.md](../../AGENTS.md) Runtime/client boundary guardrail |
| Platform | OS code only in `src/platform/<os>.rs`; no `#[cfg(target_os)]` in core | [AGENTS.md](../../AGENTS.md) Principles, Code Conventions |
| Detection | Reads screen snapshots only; never parser or viewport state | [AGENTS.md](../../AGENTS.md) Principles, Agent Detection Updates |

## Data and Contracts

- **Data Ownership and Storage:** The server owns session state and persistence (`src/persist/`, `src/persist.rs`); clients hold only presentation state.

| Contract | Required behavior | Source |
|---|---|---|
| Client endpoint generation 1 | Named codecs immutable; new JSON fields optional; frozen fixtures never re-blessed | [AGENTS.md](../../AGENTS.md) Stable client endpoint contract, `tests/fixtures/endpoint-*-v1.json` |
| Private wire protocol | `PROTOCOL_VERSION` (currently 22) guards same-install compatibility | `src/protocol/wire.rs:20` |
| JSON API schema | Published schema for external tools | `docs/next/api/herdr-api.schema.json` |
| Agent detection manifests | TOML manifests, hot-reloadable | `src/detect/manifests/`, [AGENTS.md](../../AGENTS.md) Agent Detection Updates |

**Fork constraint:** avoid fork-only changes to the wire protocol or frozen endpoint codecs. A fork-only `PROTOCOL_VERSION` bump can collide with a later upstream bump of the same number. Add new advertised API methods or optional fields instead.

## Engineering Standards and Patterns

| Standard or pattern | Basis | Applies to | Source |
|---|---|---|---|
| Multiplicative performance paths (render, PTY parse, detection, fanout) | Documented | `src/ui/`, `src/pane*`, `src/detect/`, `src/server/` | [AGENTS.md](../../AGENTS.md) |
| UI hot-path architecture boundaries | Tool-enforced | `src/ui/`, `src/app/` | `scripts/test_ui_hot_path_architecture.py` |
| Reuse existing TUI modal/screen patterns (mouse-first) | Documented | `src/ui/` | [AGENTS.md](../../AGENTS.md) Principles |
| Vendored patch index | Tool-enforced | `vendor/` | `vendor/libghostty-vt.patches.md`, `scripts/test_vendor_libghostty_vt.py` |
| Integration asset versions bump once per release | Documented | `src/integration/assets/` | [AGENTS.md](../../AGENTS.md) Code Conventions |
| Lowercase conventional commits | Tool-enforced | All commits | `.githooks/commit-msg`, `scripts/conventional_commits.py`, `.github/workflows/ci.yml` |

- **Language Rules:** [Rust](languages/rust.md), [Python](languages/python.md), [TypeScript/JavaScript](languages/typescript.md). Shell, PowerShell, Nix, Perl, and C# are excluded: they are small, upstream-owned build and packaging scripts; follow the surrounding file's style.

## Constraints

- Platforms: upstream supports Linux, macOS, and Windows; this fork targets and verifies macOS (aarch64) only. Platform code must still compile-gate correctly ([AGENTS.md](../../AGENTS.md) Code Conventions).
- No `unwrap()` in production Rust; no new dependencies without a reason ([AGENTS.md](../../AGENTS.md) Code Conventions).
- Local patches to vendored code must be indexed and reverse-apply cleanly ([AGENTS.md](../../AGENTS.md) Vendored libghostty-vt).

## Development and Verification

- **Local Setup:** Rust toolchain from `rust-toolchain.toml`, `cargo-nextest`, `just`, Python 3, Bun, Node, and exactly Zig 0.16.0 (Homebrew ships a newer version; installed from ziglang.org at `~/.local/bin/zig`). Use rustup's `cargo`, not Homebrew's `rust` (uninstalled 2026-10-19): it ignores `rust-toolchain.toml`, and newer clippy lints fail `just lint`. Install hooks with `just install-hooks`. Windows cross-lint setup (`xwin`) is not used in this fork.
- **Test Layers:** Rust unit tests next to code; integration tests in `tests/`; Python `unittest` and Bun tests for scripts, docs, and integration assets; ignored `bench-*` profiles for performance.
- **CI Checks:** Only the fork-only `Fork CI` (`.github/workflows/fork-ci.yml`) runs: one macOS job with PR-title validation and `just ci 'not binary(live_handoff)'` (lint + tests), on PRs and `master` pushes. Document-only changes (`*.md`, `*.mdx`, `.s1mple/**`) skip it, except `skills/herdr/SKILL.md` (compiled into the binary) and `vendor/**` (checked by maintenance tests). All upstream workflows, including `CI` and `Nix`, are disabled in the repo's Actions settings (`gh workflow disable`), not by editing their files, so upstream syncs stay conflict-free. New forks need a one-time "enable workflows" click on the Actions tab before anything runs.

| Check | Working directory | Command or method | Source | Evidence |
|---|---|---|---|---|
| Lint | repo root | `just lint` | `justfile` | Passed 2026-10-19 on `143bf5fd` with rustc 1.96.1 and Zig 0.16.0; failed with Homebrew rustc 1.99.0 (`clippy::chunks_exact_to_as_chunks` in `crates/ghostty-vt/src/lib.rs`) |
| Tests | repo root | `just test` | `justfile`, [AGENTS.md](../../AGENTS.md) Testing | Passed 2026-10-19 on `143bf5fd`: nextest 3909 passed / 14 skipped; Python unittest 150 + 6 OK; Bun asset and docs tests 0 failures |
| Fork pre-commit check (macOS) | repo root | `just lint && just test` | `justfile`; replaces `just check`, which adds Windows lint | Passed 2026-10-19 (see rows above) |
| Render scaling (perf-sensitive changes) | repo root | `just bench-render-scale` | [AGENTS.md](../../AGENTS.md) Multiplicative performance paths | Not run |

## Sources and Open Questions

- **Sources:** [AGENTS.md](../../AGENTS.md), [CONTRIBUTING.md](../../CONTRIBUTING.md), `Cargo.toml`, `rust-toolchain.toml`, `justfile`, `.githooks/`, `.github/workflows/ci.yml`, `.github/MAINTAINERS`, `git remote -v`, `gh repo set-default --view`.
- **Open Questions:**
  - Fork release and distribution: not established. Upstream `herdr update` and channels point at upstream releases, so a custom build must be installed manually (for example `cargo install --path .`).
