# s1mple

## Table of Contents

- [1. Persona](#1-persona)
- [2. Required Context](#2-required-context)
- [3. Workflow](#3-workflow)
- [4. Workspace Layout](#4-workspace-layout)

## 1. Persona

- Be professional and pragmatic.
- Think critically instead of following instructions blindly. Optimize for the user's underlying intent, not only the literal request.
- If a request appears unsafe, incorrect, destructive, low-quality, or misaligned with the project's goals, pause and explain the concern. Recommend a better alternative and ask for confirmation when the decision belongs to the user.
- Apply the [project principles](docs/principles.md) to every decision.
- Make assumptions explicit. Verify facts when possible and state uncertainty when verification is not possible.
- Report outcomes faithfully. Do not hide failures, skipped steps, unresolved risks, or incomplete work.

## 2. Required Context

1. Read and apply [communication rules](docs/communication.md) before responding or creating or updating documents.
2. Read and apply [principles](docs/principles.md) before discussion, planning, implementation, review, or verification.
3. Read [project context](docs/project.md) before discussion, planning, or implementation.
4. Read the applicable language rules below before editing code.

Follow existing repository instructions. Surface conflicts before proceeding.

This is a custom fork of `herdrdev/herdr`. Follow the [fork policy](docs/project.md#fork-policy): upstream universal rules in `AGENTS.md` apply; upstream maintainer, local-machine, and release sections do not.

**Language Rules:**

- [Rust](docs/languages/rust.md) — `src/`, `crates/ghostty-vt/`, `tests/`, `build.rs`
- [Python](docs/languages/python.md) — `scripts/*.py`, `scripts/windows_input/`
- [TypeScript/JavaScript](docs/languages/typescript.md) — `src/integration/assets/`, `scripts/docs/`, `scripts/*.test.ts`

## 3. Workflow

```text
s1mple-discuss → s1mple-plan → execute the finalized plan
```

**Exploration**

- Do not use the coding agent's native `Explore` subagent.
- Inspect directly when practical. When delegated exploration is genuinely necessary, prefer `s1mple-explore`.
- If governing or default instructions require another subagent type, such as `fork`, use that subagent instead.

**Discuss**

- Discuss features and behavior changes before planning. Small local fixes may skip discussion.
- State how the change affects the upstream diff: which upstream files it touches and how likely they are to conflict on the next sync.
- Classify shared runtime facts vs TUI presentation state ([AGENTS.md](../AGENTS.md) Runtime/client boundary guardrail).

**Discuss UI**

- Herdr is a mouse-first TUI. Mock up screens as ASCII terminal layouts and reuse existing modal, screen, and close-action patterns ([AGENTS.md](../AGENTS.md) Principles).

**Plan**

- Classify refactor risk as described in [AGENTS.md](../AGENTS.md) Testing; refactor-risk work needs characterization tests before code moves.
- For work in render, PTY parsing, detection, or client fanout paths, plan the 1-pane vs 15-pane profile ([AGENTS.md](../AGENTS.md) Multiplicative performance paths).

**Implementation**

- Work on a branch off fork `master`, one branch per change.
- Keep the upstream diff small: prefer new files, avoid unrelated edits to upstream files, and do not reformat upstream code.
- Add each customization to [Fork Customizations](docs/project.md#fork-customizations).
- Do not change the wire protocol, `PROTOCOL_VERSION`, or frozen endpoint fixtures for fork-only features.

**Verification**

- Run `just lint && just test` before committing. This fork is macOS-only, so it replaces upstream's `just check` and skips Windows lint. Report any skipped check.
- Agent detection changes need a live smoke test ([AGENTS.md](../AGENTS.md) Agent Detection Updates).

After overall verification passes and before project documentation finalization, run the `s1mple-harden` skill in workflow mode on the work folder.

**Delivery**

- Use lowercase conventional commits with no AI co-author lines. Propose the commit message and get approval before committing.
- Push only to `origin` (`therealhieu/herdr`). Open PRs only on the fork with `gh pr create --base master` (`gh` defaults to `therealhieu/herdr`; check with `gh repo set-default --view`).
- Wait for `Fork CI` on the PR (`gh pr checks --watch`); its single macOS job must pass.
- Never push, open PRs, or open issues on `herdrdev/herdr`.
- Do not use upstream release commands (`just preview`, `just release*`).
- Upstream sync: merge `upstream/master` into a sync branch, resolve conflicts against [Fork Customizations](docs/project.md#fork-customizations), re-align `fork-ci.yml` with upstream `ci.yml` toolchain versions, run `just lint && just test`, then merge into fork `master`.

## 4. Workspace Layout

Keep s1mple files under `.s1mple/` in the project root:

```text
.s1mple/
├── S1MPLE.md
├── docs/
│   ├── communication.md
│   ├── principles.md
│   ├── project.md
│   └── languages/
│       ├── python.md
│       ├── rust.md
│       └── typescript.md
└── work/
    ├── active/
    └── done/
```

- `S1MPLE.md` is the entrypoint for shared instructions and required context.
- `docs/communication.md` defines rules for chat and authored documents.
- `docs/principles.md` defines the principles that drive decisions, delivery, testing, and documents.
- `docs/project.md` describes architecture, contracts, constraints, development checks, and fork policy.
- `docs/languages/` contains language rules scoped to applicable project paths.
- `work/active/` contains work in progress.
- `work/done/` contains completed work.
