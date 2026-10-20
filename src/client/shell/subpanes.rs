//! Client lookups for pane slots that hold several terminals. Membership comes
//! from the snapshot's `subpane` fields; the server owns every change.

use crate::input::KeybindAction;
use crate::protocol::{ClientShellPane, ClientShellSnapshot};

/// Members of the slot currently showing `visible_pane_id`, in member order.
/// Empty for a plain pane.
pub(super) fn slot_members<'a>(
    snapshot: &'a ClientShellSnapshot,
    visible_pane_id: &str,
) -> Vec<&'a ClientShellPane> {
    let mut members: Vec<_> = snapshot
        .panes
        .iter()
        .filter(|pane| {
            pane.subpane
                .as_ref()
                .is_some_and(|subpane| subpane.visible_pane_id == visible_pane_id)
        })
        .collect();
    members.sort_by_key(|pane| pane.subpane.as_ref().map(|subpane| subpane.index));
    members
}

/// The member a subpane switch key shows in the slot holding the focused pane,
/// or `None` on a plain pane or for an index past the last member.
pub(super) fn switch_target(
    snapshot: &ClientShellSnapshot,
    action: KeybindAction,
) -> Option<String> {
    let focused = snapshot.focused_pane_id.as_deref()?;
    let members = slot_members(snapshot, focused);
    let count = members.len();
    if count < 2 {
        return None;
    }
    let active = members.iter().position(|pane| pane.pane_id == focused)?;
    let index = match action {
        KeybindAction::NextSubpane => (active + 1) % count,
        KeybindAction::PreviousSubpane => (active + count - 1) % count,
        KeybindAction::FocusSubpane(index) => index,
        _ => return None,
    };
    members.get(index).map(|pane| pane.pane_id.clone())
}
