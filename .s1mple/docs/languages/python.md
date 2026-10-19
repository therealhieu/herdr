# Python

## Applies To

- **Paths:** `scripts/*.py`, `scripts/windows_input/`
- **Component Differences:** None.

## Toolchain and Configuration

| Tool | Version or configuration source | Purpose |
|---|---|---|
| Python 3 | `justfile` (`python3`, `python` on Windows) | Run scripts and tests |
| unittest | `justfile` `maintenance-test` | Test runner |

No formatter or linter is configured.

## Coding Conventions

| Convention | Basis | Applies to | Source |
|---|---|---|---|
| Standard library only | Observed | `scripts/` | imports in `scripts/*.py` |
| Each script has a `test_<name>.py` sibling, imported as `from scripts import <name>` | Observed | `scripts/` | `scripts/test_release.py` |
| New tests must be added to the `maintenance-test` list to run | Tool-enforced | `scripts/` | `justfile` |

## Types and Public Interfaces

Not established.

## Error Handling

Not established; match the surrounding script.

## Testing and Verification

- Tests use `unittest.TestCase` with `tempfile` and `unittest.mock`.

| Check | Working directory | Command | Evidence |
|---|---|---|---|
| Maintenance tests | repo root | `just maintenance-test` | Not run |
| UI hot-path architecture | repo root | `just ui-hot-path-architecture-test` | Not run |

## Examples

Not needed.

## Sources and Open Questions

- **Sources:** `justfile`, `scripts/test_release.py`.
- **Open Questions:** None.
