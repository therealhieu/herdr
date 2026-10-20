# W261020LXIN Mockups

- **Status:** `Accepted`
- **Goal:** One pane slot holds several terminals, shown as a tab strip with agent status on the slot's top border (style A), with a compact badge when the strip does not fit.

## Mockups

| Mockup | Surface | States · viewports | Path | Status |
|---|---|---|---|---|
| `M1@R1` | Pane slot top border (subpane strip) | Style A strip, badge fallback (narrow), plain pane with `+`, focused vs unfocused slot, background agent blocked, member closed, stack collapses to plain pane; styles B, C, D shown as rejected alternatives · wide 1060 px and narrow 640 px terminal widths | `m1-subpane-strip.html` | `Accepted` |

## Rounds

| Round | Mode | Instructions | Before | After | Result |
|---|---|---|---|---|---|
| 1 | `build` | Initial interactive mockup written in the chat session before this work started (published as https://claude.ai/artifact/7VJsBQt9HNNbNmg7mbp6wi): four styles, width toggle, drag to merge or split, live layout tree. | `None` | `M1@R1` | `Accepted` |

## Limits and Assumptions

- The mockup is HTML, not a cell grid. Colors, the pulsing dot, hover-only `×`, the drop-down badge menu, and drag-and-drop do not map to the TUI one to one. The TUI adaptation is recorded as `UDES` rows in the ship file.
- Drag to merge or tear out, and styles B, C, and D are out of scope for this work.
- The mockup was not written by a `fork` subagent; it is the reviewed artifact the user chose from, copied here unchanged except the page title.

## Acceptance

- **Accepted revisions:** `M1@R1` — style A (tab strip) and its badge fallback only.
- **Evidence:** Round 1. After reviewing the published mockup, the user asked to "create a worktree for style A" and then to "self-drive /s1mple-ship with execution" for style A.
