# TypeScript and JavaScript

## Applies To

- **Paths:** `src/integration/assets/` (agent hook/plugin assets), `scripts/docs/`, `scripts/release-workflows.test.ts`
- **Component Differences:** Integration assets run inside third-party agent CLIs (for example opencode, pi), so they must work in that host's runtime without bundling. `.pi/extensions/` is local tooling with its own `package.json` and `tsconfig.json`.

## Toolchain and Configuration

| Tool | Version or configuration source | Purpose |
|---|---|---|
| Bun | `justfile` (`bun test`) | Test runner |
| Node | `justfile` `release-docs-check` | Runs `scripts/docs/*.mjs` |

No root `package.json`, `tsconfig.json`, formatter, or linter is configured.

## Coding Conventions

| Convention | Basis | Applies to | Source |
|---|---|---|---|
| No npm dependencies; use `node:` built-ins | Observed | `src/integration/assets/`, `scripts/docs/` | `src/integration/assets/herdr-agent-state.test.ts` |
| Bump the integration version once per release when an asset changes | Documented | `src/integration/assets/` | [AGENTS.md](../../../AGENTS.md) Code Conventions |

## Types and Public Interfaces

- Assets talk to the Herdr socket API; keep them on the public JSON API ([AGENTS.md](../../../AGENTS.md) Runtime/client boundary guardrail).

## Error Handling

Not established; match the surrounding asset.

## Testing and Verification

- Tests sit beside the asset as `*.test.ts` using `bun:test`. A new test file must be added to the `justfile` recipe to run.

| Check | Working directory | Command | Evidence |
|---|---|---|---|
| Integration assets | repo root | `just integration-assets-test` | Not run |
| Docs tooling | repo root | `just docs-contract-test` | Not run |

## Examples

Not needed.

## Sources and Open Questions

- **Sources:** `justfile`, `src/integration/assets/`, `scripts/docs/`.
- **Open Questions:** None.
