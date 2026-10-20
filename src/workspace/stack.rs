//! Subpane stacks: several terminals sharing one layout slot.
//!
//! The layout tree only ever holds the active member of a stack; the other
//! members are hidden but stay in `Tab::panes` with live runtimes.

use crate::layout::PaneId;

/// Terminals sharing one layout slot. `members[active]` is the layout leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubpaneStack {
    members: Vec<PaneId>,
    active: usize,
}

impl SubpaneStack {
    pub fn members(&self) -> &[PaneId] {
        &self.members
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_member(&self) -> PaneId {
        self.members[self.active]
    }

    pub fn hidden_members(&self) -> impl Iterator<Item = PaneId> + '_ {
        let active = self.active;
        self.members
            .iter()
            .enumerate()
            .filter(move |(idx, _)| *idx != active)
            .map(|(_, id)| *id)
    }
}

/// Leaf change caused by removing a stack member: `(old_leaf, new_leaf)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveOutcome {
    pub replaced_leaf: Option<(PaneId, PaneId)>,
}

/// All subpane stacks of one tab. Every stack has at least two unique
/// members, and no pane belongs to more than one stack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubpaneStacks {
    stacks: Vec<SubpaneStack>,
}

impl SubpaneStacks {
    pub fn is_empty(&self) -> bool {
        self.stacks.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SubpaneStack> {
        self.stacks.iter()
    }

    pub fn stack_of(&self, pane_id: PaneId) -> Option<&SubpaneStack> {
        self.stacks
            .iter()
            .find(|stack| stack.members.contains(&pane_id))
    }

    pub fn contains(&self, pane_id: PaneId) -> bool {
        self.stack_of(pane_id).is_some()
    }

    /// True for a stack member that is not the slot's layout leaf.
    pub fn is_hidden(&self, pane_id: PaneId) -> bool {
        self.stack_of(pane_id)
            .is_some_and(|stack| stack.active_member() != pane_id)
    }

    /// The layout leaf of the slot holding `pane_id`, or `pane_id` itself when
    /// it is not in a stack.
    pub fn visible_leaf(&self, pane_id: PaneId) -> PaneId {
        self.stack_of(pane_id)
            .map_or(pane_id, SubpaneStack::active_member)
    }

    /// Append `new` to the slot whose leaf is `visible`, creating a stack when
    /// the slot is plain. The active member does not change.
    pub fn add_member(&mut self, visible: PaneId, new: PaneId) -> bool {
        if visible == new || self.contains(new) || self.is_hidden(visible) {
            return false;
        }
        match self
            .stacks
            .iter_mut()
            .find(|stack| stack.members.contains(&visible))
        {
            Some(stack) => stack.members.push(new),
            None => self.stacks.push(SubpaneStack {
                members: vec![visible, new],
                active: 0,
            }),
        }
        true
    }

    /// Make a hidden member active. Returns `(old_leaf, new_leaf)`.
    pub fn activate(&mut self, pane_id: PaneId) -> Option<(PaneId, PaneId)> {
        let stack = self
            .stacks
            .iter_mut()
            .find(|stack| stack.members.contains(&pane_id))?;
        let idx = stack.members.iter().position(|id| *id == pane_id)?;
        if idx == stack.active {
            return None;
        }
        let old = stack.active_member();
        stack.active = idx;
        Some((old, pane_id))
    }

    /// Remove a member. Removing the active member activates the next member,
    /// else the previous one; a stack left with one member dissolves. Returns
    /// `None` when `pane_id` is not in a stack.
    pub fn remove_member(&mut self, pane_id: PaneId) -> Option<RemoveOutcome> {
        let stack_idx = self
            .stacks
            .iter()
            .position(|stack| stack.members.contains(&pane_id))?;
        let stack = &mut self.stacks[stack_idx];
        let idx = stack.members.iter().position(|id| *id == pane_id)?;
        let was_active = idx == stack.active;
        stack.members.remove(idx);
        if idx < stack.active || (was_active && idx == stack.members.len()) {
            stack.active -= 1;
        }
        let replaced_leaf = was_active.then(|| (pane_id, stack.active_member()));
        if stack.members.len() < 2 {
            self.stacks.remove(stack_idx);
        }
        Some(RemoveOutcome { replaced_leaf })
    }

    /// Layout order with each stack leaf followed by its hidden members.
    pub fn ordered_ids(&self, layout_ids: &[PaneId]) -> Vec<PaneId> {
        let mut ids = Vec::with_capacity(layout_ids.len());
        for &leaf in layout_ids {
            ids.push(leaf);
            if let Some(stack) = self
                .stacks
                .iter()
                .find(|stack| stack.active_member() == leaf)
            {
                ids.extend(stack.hidden_members());
            }
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: u32) -> PaneId {
        PaneId::from_raw(id)
    }

    fn members(stacks: &SubpaneStacks, member: PaneId) -> (Vec<PaneId>, PaneId) {
        let stack = stacks.stack_of(member).expect("pane should be in a stack");
        (stack.members().to_vec(), stack.active_member())
    }

    #[test]
    fn stack_add_member_creates_stack_from_plain_slot_and_appends() {
        let mut stacks = SubpaneStacks::default();
        assert!(stacks.add_member(pane(1), pane(2)));
        assert_eq!(members(&stacks, pane(1)), (vec![pane(1), pane(2)], pane(1)));
        assert!(stacks.is_hidden(pane(2)));
        assert!(!stacks.is_hidden(pane(1)));

        assert!(stacks.add_member(pane(1), pane(3)));
        assert_eq!(
            members(&stacks, pane(3)),
            (vec![pane(1), pane(2), pane(3)], pane(1))
        );

        assert!(!stacks.add_member(pane(1), pane(2)), "already a member");
        assert!(!stacks.add_member(pane(2), pane(4)), "hidden target");
        assert!(!stacks.add_member(pane(5), pane(5)), "self target");
        assert_eq!(stacks.iter().count(), 1);
    }

    #[test]
    fn stack_activate_returns_old_and_new_leaf() {
        let mut stacks = SubpaneStacks::default();
        stacks.add_member(pane(1), pane(2));
        stacks.add_member(pane(1), pane(3));

        assert_eq!(stacks.activate(pane(3)), Some((pane(1), pane(3))));
        assert_eq!(members(&stacks, pane(1)).1, pane(3));
        assert_eq!(stacks.activate(pane(3)), None, "already active");
        assert_eq!(stacks.activate(pane(9)), None, "not a member");
    }

    #[test]
    fn stack_remove_active_member_shows_next_else_previous() {
        let mut stacks = SubpaneStacks::default();
        stacks.add_member(pane(1), pane(2));
        stacks.add_member(pane(1), pane(3));
        stacks.activate(pane(2));

        let outcome = stacks.remove_member(pane(2)).expect("member");
        assert_eq!(outcome.replaced_leaf, Some((pane(2), pane(3))));
        assert_eq!(members(&stacks, pane(1)), (vec![pane(1), pane(3)], pane(3)));

        stacks.add_member(pane(3), pane(4));
        let outcome = stacks.remove_member(pane(3)).expect("member");
        assert_eq!(outcome.replaced_leaf, Some((pane(3), pane(4))));
        stacks.activate(pane(4));
        assert_eq!(members(&stacks, pane(1)), (vec![pane(1), pane(4)], pane(4)));

        stacks.add_member(pane(4), pane(5));
        stacks.activate(pane(5));
        let outcome = stacks.remove_member(pane(5)).expect("member");
        assert_eq!(
            outcome.replaced_leaf,
            Some((pane(5), pane(4))),
            "last member falls back to the previous one"
        );
    }

    #[test]
    fn stack_dissolves_when_one_member_remains() {
        let mut stacks = SubpaneStacks::default();
        stacks.add_member(pane(1), pane(2));
        stacks.activate(pane(2));

        let outcome = stacks.remove_member(pane(2)).expect("member");
        assert_eq!(outcome.replaced_leaf, Some((pane(2), pane(1))));
        assert!(stacks.is_empty());
        assert!(!stacks.contains(pane(1)));
    }

    #[test]
    fn stack_remove_hidden_member_keeps_leaf() {
        let mut stacks = SubpaneStacks::default();
        stacks.add_member(pane(1), pane(2));
        stacks.add_member(pane(1), pane(3));
        stacks.activate(pane(3));

        let outcome = stacks.remove_member(pane(1)).expect("member");
        assert_eq!(outcome.replaced_leaf, None);
        assert_eq!(members(&stacks, pane(3)), (vec![pane(2), pane(3)], pane(3)));
        assert!(stacks.remove_member(pane(9)).is_none());
    }

    #[test]
    fn stack_ordered_ids_put_hidden_members_after_their_leaf() {
        let mut stacks = SubpaneStacks::default();
        stacks.add_member(pane(2), pane(5));
        stacks.add_member(pane(2), pane(6));
        stacks.activate(pane(5));

        assert_eq!(
            stacks.ordered_ids(&[pane(1), pane(5), pane(3)]),
            vec![pane(1), pane(5), pane(2), pane(6), pane(3)]
        );
    }
}
