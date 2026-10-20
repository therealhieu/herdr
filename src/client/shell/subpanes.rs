//! Client lookups for pane slots that hold several terminals. Membership comes
//! from the snapshot's `subpane` fields; the server owns every change.

use ratatui::layout::Rect;

use crate::input::KeybindAction;
use crate::protocol::{ClientShellPane, ClientShellSnapshot};
use crate::subpane_strip::StripSegmentKind;

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

/// What a click on one strip segment asks the server to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SubpaneStripAction {
    Focus(String),
    Close(String),
    Create(String),
}

/// One clickable strip segment, in absolute client cells.
pub(super) struct SubpaneStripHit {
    pub(super) rect: Rect,
    pub(super) action: SubpaneStripAction,
}

/// Strip segments for every slot that holds several terminals and draws a top
/// border, laid out by the same function the server renders with.
pub(super) fn strip_hits(
    snapshot: &ClientShellSnapshot,
    panes: &[super::PaneHit],
) -> Vec<SubpaneStripHit> {
    if !snapshot.panes.iter().any(|pane| pane.subpane.is_some()) {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for pane in panes.iter().filter(|pane| pane.inner_rect.y > pane.rect.y) {
        let members = slot_members(snapshot, &pane.pane_id);
        let Some(active) = members
            .iter()
            .position(|member| member.pane_id == pane.pane_id)
        else {
            continue;
        };
        let labels: Vec<&str> = members
            .iter()
            .filter_map(|member| member.subpane.as_ref())
            .map(|subpane| subpane.label.as_str())
            .collect();
        let layout = crate::subpane_strip::strip_layout(&labels, active, pane.rect.width);
        let member_id = |index: usize| members.get(index).map(|member| member.pane_id.clone());
        for segment in layout.segments {
            let action = match segment.kind {
                StripSegmentKind::Member(index) => member_id(index).map(SubpaneStripAction::Focus),
                StripSegmentKind::Close(index) => member_id(index).map(SubpaneStripAction::Close),
                StripSegmentKind::Add => Some(SubpaneStripAction::Create(pane.pane_id.clone())),
                StripSegmentKind::Badge => {
                    member_id((active + 1) % members.len()).map(SubpaneStripAction::Focus)
                }
            };
            if let Some(action) = action {
                hits.push(SubpaneStripHit {
                    rect: Rect::new(
                        pane.rect.x.saturating_add(segment.x),
                        pane.rect.y,
                        segment.width,
                        1,
                    ),
                    action,
                });
            }
        }
    }
    hits
}

impl super::ClientShellState {
    pub(super) fn handle_subpane_strip_click(
        &mut self,
        action: SubpaneStripAction,
        outcome: &mut super::ClientShellInput,
    ) {
        use crate::api::schema::{Method, PaneSubpaneCreateParams, PaneTarget};

        self.mode = super::ClientShellMode::Terminal;
        match action {
            SubpaneStripAction::Focus(pane_id) => {
                self.push_endpoint_method(Method::PaneFocus(PaneTarget { pane_id }), outcome);
            }
            SubpaneStripAction::Close(pane_id) => self.request_pane_close(pane_id, outcome),
            SubpaneStripAction::Create(pane_id) => self.push_endpoint_method(
                Method::PaneSubpaneCreate(PaneSubpaneCreateParams {
                    pane_id,
                    cwd: None,
                    focus: true,
                }),
                outcome,
            ),
        }
    }
}
