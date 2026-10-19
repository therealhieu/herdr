# Rust

## Applies To

- **Paths:** `src/`, `crates/ghostty-vt/`, `tests/`, `build.rs`
- **Component Differences:** `vendor/` is vendored third-party code; change it only through indexed patches ([AGENTS.md](../../../AGENTS.md) Vendored libghostty-vt).

## Toolchain and Configuration

| Tool | Version or configuration source | Purpose |
|---|---|---|
| rustc / cargo | `rust-toolchain.toml` (1.96.1) | Build |
| rustfmt | `rust-toolchain.toml` | Formatting (`cargo fmt --check`) |
| clippy | `clippy.toml`, `justfile` `lint` (`-D warnings`) | Linting |
| cargo-nextest | `justfile` `test` | Test runner |
| Zig 0.16.0 | [AGENTS.md](../../../AGENTS.md) | Builds vendored libghostty-vt |

## Coding Conventions

| Convention | Basis | Applies to | Source |
|---|---|---|---|
| No `unwrap()` in production code | Documented | `src/`, `crates/` | [AGENTS.md](../../../AGENTS.md) Code Conventions |
| Log with `tracing` | Documented | `src/` | [AGENTS.md](../../../AGENTS.md) Code Conventions |
| `#[allow]` only with an explaining comment | Documented | All | [AGENTS.md](../../../AGENTS.md) Code Conventions |
| OS code in `src/platform/<os>.rs`; gate with `#[cfg(windows)]`/`#[cfg(unix)]`, not `cfg!` | Documented | All | [AGENTS.md](../../../AGENTS.md) Code Conventions |
| Never mutate state during render | Documented | `src/ui/` | [AGENTS.md](../../../AGENTS.md) Principles |
| Clippy warnings are errors | Tool-enforced | All | `justfile` `lint` |
| Fork: add code in new modules where practical to reduce upstream merge conflicts | Documented (fork policy) | All | [project.md](../project.md#fork-policy) |

## Types and Public Interfaces

- Generation-1 endpoint codecs and frozen fixtures are immutable; enums reachable from them are append-closed ([AGENTS.md](../../../AGENTS.md) Stable client endpoint contract).
- Use neutral server/API names, not UI names like sidebar or widget ([AGENTS.md](../../../AGENTS.md) Runtime/client boundary guardrail).

## Error Handling

- No `anyhow`/`thiserror` dependency. Observed: `io::Result` and `Result<_, String>` are both common (for example `src/remote/attach.rs`, `src/update.rs`). Match the surrounding module.

## Testing and Verification

- Unit tests live next to code in `#[cfg(test)] mod tests`. Build `AppState`/`Workspace` scenarios with `AppState::test_new()` (`src/app/state.rs:1036`) and `Workspace::test_new()` (`src/workspace.rs:1170`), without PTYs.
- Identity/state refactors use `assert_invariants_for_test()` with `test_with_adversarial_identity_state()` / `test_adversarial_identity_state()` ([AGENTS.md](../../../AGENTS.md) Testing).
- Detection tests cover the engine with synthetic manifests, not captured agent screens ([AGENTS.md](../../../AGENTS.md) Agent Detection Updates).

| Check | Working directory | Command | Evidence |
|---|---|---|---|
| Lint | repo root | `just lint` | Not run |
| One test filter | repo root | `just test-one <filter>` | Not run |
| Fork pre-commit check (macOS) | repo root | `just lint && just test` | Not run |

## Examples

Not needed.

## Sources and Open Questions

- **Sources:** [AGENTS.md](../../../AGENTS.md), `rust-toolchain.toml`, `clippy.toml`, `justfile`, `src/app/state.rs`, `src/workspace.rs`.
- **Open Questions:** None.
