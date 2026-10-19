# W261019EYLD G3 Verification Report

## Scope

- **Scope:** `G3`
- **Tasks:** `G3-T1`, `G3-T2`
- **Change range:** `4d4e186f..f5232099`
- **Report:** `.s1mple/work/active/w261019eyld-kitty-text-sizing/reports/g3-verification-report.md`

## Status

`Failed`

Both assigned rows are met and every check passes. One Minor documentation finding remains: the `terminal.text_sizing` reference text says `false` keeps OSC 66 dropped, which is only true when no other supporting client is attached.

## Coverage

| REQ | UDES or DES | V | Implementation evidence | Result |
|---|---|---|---|---|
| `REQ3` | `DES4` | `V6` | `src/protocol/endpoint.rs:EndpointClientHello::text_sizing` (`#[serde(default, skip_serializing_if = "is_false")]`); `src/client/handshake.rs:text_sizing_profile_values`, `text_sizing_profile_allowed` (unix) and the `false` stub (`not(unix)`), hello field from `text_sizing::host_supported()`; `src/server/client_transport.rs:777` (`text_sizing_supported = hello.text_sizing` only after every rejection branch), `:823` (`ClientRegistration` guard held until `client_read_loop_with_endpoint_controls` returns; `TerminalHello` clients register `false`); `src/text_sizing.rs:RegistryState`, `ClientRegistration`, `update_registry` (mutex + `AtomicBool` mirror) | `Met` |
| `REQ3` | `DES5` | `V6` (config part) | `src/config/model.rs:TerminalConfig::text_sizing` (`Option<bool>`, struct is `#[serde(default)]`); `src/config.rs:Config::text_sizing_enabled` (`unwrap_or(true)`); `src/client/mod.rs:213` (`text_sizing_enabled() && client_rendered_shell && text_sizing_profile_allowed()` → `set_host_supported`, before any `do_handshake`); `docs/next/website/src/data/config-reference.json:451` | `Met` |

Behavior checked by reading the code:

- **Frozen contract.** `tests/fixtures/` and `src/protocol/wire.rs` are unchanged since base `41461ad7`; `PROTOCOL_VERSION` unchanged. The new hello field is optional, so `endpoint-hello-v1.json` still decodes (`frozen_generation_one_handshake_decodes`) and a `false` flag serializes without the key. An older server ignores the field, and the client then receives no placeholders, so nothing reports false success.
- **Registration lifetime.** The guard is created after the `invalid_hello`, `unsupported_generation`, `no_common_core`, `invalid_surface`, incompatible `TerminalHello`, `ClientShellHello`, and unknown-first-message returns. It is created before the welcome write. Every later exit drops it when the function returns: `should_quit`, a failed welcome write or timeout reset (`?`), a failed writer spawn, a failed `ClientConnected` send, the read-loop `break`/`return Err`, and panic unwind. Handoff does not carry client sockets, so clients re-register through the handshake.
- **Registry semantics (`A5`).** `recompute` changes `active` only while `attached > 0`, so the value is kept when the last client detaches. The registry starts off. Unsupported clients count as attached. `update_registry` mutates and mirrors to `ACTIVE` under one mutex, and poisoning is recovered.
- **Active-flag transitions vs PTY parsing.** `src/pane/terminal.rs:1413` reads `is_active()` once per byte chunk. When the flag is off, the tracker resets (G1 fix), so a flip mid-sequence cannot inject a half-tracked command. A flip to on mid-body misses the `ESC ]` prefix and ignores the tail.
- **Profile (`A6`).** It returns true for `TERM=xterm-kitty` or `KITTY_WINDOW_ID`. `TMUX`/`STY` block it, and both stdin and stdout must be terminals. SSH is not blocked, unlike direct graphics, which follows `A6`. `KITTY_WINDOW_ID` is already stripped from pane environments (`src/pane.rs:125`), so a nested client does not inherit it.
- **Compile gating.** `#[cfg(any(unix, test))]`, `#[cfg(unix)]`, and `#[cfg(not(unix))]` mirror `direct_graphics_profile_*`. No `#[cfg(target_os)]` was added. The G1 `#[allow(dead_code)]` items in `src/text_sizing.rs` were removed, and clippy stays clean.

## Checks

| Check | Command or method | Actual result | Result |
|---|---|---|---|
| fmt | `cargo fmt --check` | exit 0 | `Passed` |
| clippy | `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings | `Passed` |
| `V6` | `just test-one text_sizing` | 36 passed (includes `registry_follows_attached_clients_and_keeps_value_when_all_detach`, `text_sizing_profile_requires_kitty_host_outside_multiplexers`, `text_sizing_hello_flag_is_optional_and_omitted_when_false`, `text_sizing_default_on_with_opt_out`) | `Passed` |
| endpoint | `just test-one endpoint` | 283 passed | `Passed` |
| maintenance | `just maintenance-test` | exit 0 (unittest suites + bun 5/5) | `Passed` |
| config reference | `python3 scripts/config_reference_check.py` | exit 0 | `Passed` |
| frozen fixtures | `just test-one frozen`; `just test-one client_commands`; `git diff --quiet 41461ad7..f5232099 -- tests/fixtures src/protocol/wire.rs` | 10 passed; 5 passed (method-shape digests); no diff | `Passed` |
| transport/wire regression | `just test-one client_transport`; `just test-one wire` | 30 passed; 74 passed | `Passed` |
| full suite (regression) | `cargo nextest run --locked` | 3953 passed, 14 skipped | `Passed` |

## Integration

Not applicable — group scope.

## Findings

### G3-VER1 — Minor — documentation — `terminal.text_sizing` reference overstates what `false` does

- **Builds:** `REQ3`, `DES5`, `G3-T2`
- **Evidence:** `docs/next/website/src/data/config-reference.json:454`

  ```json
  "description": "Render kitty text sizing protocol (OSC 66) sized text when the outer terminal is kitty. Set false to keep dropping OSC 66 so applications fall back to unsized text. Takes effect on the next attach."
  ```

  The switch is read only by the client (`src/client/mod.rs:213`) and makes only that client unsupported. Under `A5`, the server keeps synthesizing blocks while any other attached client supports sizing, and it keeps the last value after all clients detach. In those cases OSC 66 is not dropped. The client shows unscaled fallback text, and apps still get kitty-sized CPR answers.
- **Impact:** A user who sets `false` while another kitty client is attached, or after a supporting client detached, still sees OSC 66 consumed with the cursor moved. The documentation says the opposite. It also does not say this is a local-client setting in remote sessions, which the sibling `terminal.kitty_graphics` entry does say.
- **Resolution:** Reword the description so it states the actual behavior. It is a client-side setting that stops this client from advertising text sizing, and it takes effect on the next attach. The server sizes OSC 66 while at least one attached client supports it. In remote sessions, the local client configuration controls it.
- **Proof to close:** The updated description matches `A5`/`A12`, and `python3 scripts/config_reference_check.py` and `just maintenance-test` pass.

## Test Coverage

- **Covered:** Registry transitions per `A5`: off at start, on with one supporter, still on with mixed clients, off with only unsupported clients, unchanged when all detach, off on an unsupported reattach. Profile truth table: kitty `TERM`, kitty window id, ghostty, wezterm, plain xterm, a multiplexer, and non-terminal stdio. Hello compatibility: `false` omitted, legacy JSON decodes to `false`, `true` round-trips, and the frozen gen-1 hello still decodes. Config default-on and opt-out.
- **Missing:** No test pins the `config && client_rendered_shell && profile` combination in `run_client_with_mode` (the implementer disclosed this). The registration wiring in `handle_client_handshake` is also untested: no test checks that a `text_sizing: true` hello activates sizing or that rejected hellos never register. Both are short and verified by reading the code. The live `V8` run with `terminal.text_sizing = false` (expected probe columns 2/1/1) covers the switch end to end at overall verification. Neither gap is required by `V6`, so neither is raised as a finding.
- **Low-value:** None.

## Scope Changes

- `DES4` placed the host-support computation in `src/client/handshake.rs`, and `G3-T2` listed `src/client/loop_config.rs`. The flag is instead computed in `src/client/mod.rs:213` and stored in the process-wide `set_host_supported`. `do_handshake` reads that flag for every endpoint hello. The behavior is equivalent, and the implementer disclosed the change.
- Test-only `text_sizing: false` fields were added to existing hello builders in `src/protocol/endpoint.rs` and `src/server/client_transport.rs` tests. This is required by the new struct field.

## Blockers and Unverified Items

- The Windows build of the `#[cfg(not(unix))]` stub was not compiled. The fork is macOS-only, and `S1MPLE.md` skips Windows lint.
- Observation outside the `A6` decision, not raised as a finding: only `TMUX`/`STY` count as blocking multiplexers, the same as the existing direct-graphics profile. Zellij inside kitty inherits `KITTY_WINDOW_ID` and would advertise support. Blocks would then be `skip` cells that rely on an OSC 66 the multiplexer may drop.
- Observation outside G3 scope: metadata-only endpoint connections register like any client, for example `probe_remote_endpoint` before a saved-SSH attach. A probe from a kitty client briefly counts as supporting, and one from a non-kitty process as unsupported. This follows `A5`'s "attached client" rule, and a real attach normally follows at once.
