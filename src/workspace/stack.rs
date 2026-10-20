//! Subpane stacks: several terminals sharing one layout slot.
//!
//! The layout tree only ever holds the active member of a stack; the other
//! members are hidden but stay in `Tab::panes` with live runtimes.

use std::path::PathBuf;

use tracing::warn;

use super::{NewPane, Tab, Workspace};
use crate::layout::PaneId;
use crate::pane::{PaneLaunchEnv, PaneState};
use crate::terminal::{TerminalId, TerminalRuntime, TerminalState};

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
        self.iter().find(|stack| stack.members().contains(&pane_id))
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

    /// Check stack invariants against the layout and return the layout ids
    /// plus every hidden member.
    #[cfg(test)]
    pub(crate) fn assert_invariants_for_test(
        &self,
        layout_ids: &[PaneId],
    ) -> std::collections::HashSet<PaneId> {
        let mut slot_ids: std::collections::HashSet<_> = layout_ids.iter().copied().collect();
        let mut members = std::collections::HashSet::new();
        for stack in &self.stacks {
            assert!(
                stack.members.len() >= 2,
                "stack {stack:?} has fewer than 2 members"
            );
            assert!(
                stack.active < stack.members.len(),
                "stack {stack:?} active out of range"
            );
            assert!(
                layout_ids.contains(&stack.active_member()),
                "stack {stack:?} active member is not a layout leaf"
            );
            for member in &stack.members {
                assert!(members.insert(*member), "pane {member:?} is in two stacks");
            }
            for hidden in stack.hidden_members() {
                assert!(
                    slot_ids.insert(hidden),
                    "hidden member {hidden:?} is also a layout leaf"
                );
            }
        }
        slot_ids
    }
}

impl Tab {
    /// Show hidden member `pane_id` in its slot without changing geometry,
    /// focus history, or zoom. Returns false when it is not a hidden member.
    pub(crate) fn activate_subpane(&mut self, pane_id: PaneId) -> bool {
        let Some((old, new)) = self.subpanes.activate(pane_id) else {
            return false;
        };
        if self.layout.replace_leaf(old, new) {
            return true;
        }
        warn!(
            pane = pane_id.raw(),
            "subpane stack leaf missing from layout"
        );
        self.subpanes.activate(old);
        false
    }

    /// Whether removing `pane_id` would leave this tab without panes.
    pub(crate) fn removing_pane_empties_tab(&self, pane_id: PaneId) -> bool {
        self.layout.pane_count() <= 1 && !self.subpanes.contains(pane_id)
    }

    /// Panes the user sees when viewing this tab: every pane except hidden
    /// stack members.
    pub(crate) fn visible_panes_mut(&mut self) -> impl Iterator<Item = &mut PaneState> {
        let subpanes = &self.subpanes;
        self.panes
            .iter_mut()
            .filter(|(pane_id, _)| !subpanes.is_hidden(**pane_id))
            .map(|(_, pane)| pane)
    }

    /// Remove a stack member, showing a neighbor when it was the active one.
    /// Returns `None` when `pane_id` is not in a stack.
    pub(crate) fn take_subpane_member(&mut self, pane_id: PaneId) -> Option<PaneState> {
        let slot_leaf = self.subpanes.visible_leaf(pane_id);
        let outcome = self.subpanes.remove_member(pane_id)?;
        let leaf = match outcome.replaced_leaf {
            Some((old, new)) => {
                self.layout.replace_leaf(old, new);
                new
            }
            None => slot_leaf,
        };
        if self.root_pane == pane_id {
            self.root_pane = leaf;
        }
        self.panes.remove(&pane_id)
    }

    /// Add an existing pane state to the slot holding `target` and show it.
    fn attach_subpane(&mut self, target: PaneId, pane_id: PaneId, pane: PaneState) -> bool {
        let leaf = self.subpanes.visible_leaf(target);
        if !self.layout.pane_ids().contains(&leaf) || self.panes.contains_key(&pane_id) {
            return false;
        }
        if !self.subpanes.add_member(leaf, pane_id) {
            return false;
        }
        self.panes.insert(pane_id, pane);
        self.activate_subpane(pane_id);
        true
    }

    /// Spawn a shell as a new member of the slot holding `target` and show it.
    /// Nothing changes when the spawn fails.
    #[allow(clippy::too_many_arguments)] // Mirrors the split spawn arguments.
    pub(crate) fn spawn_subpane(
        &mut self,
        target: PaneId,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        host_terminal_appearance: Option<crate::terminal_theme::HostAppearance>,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
    ) -> std::io::Result<NewPane> {
        let leaf = self.subpanes.visible_leaf(target);
        if !self.layout.pane_ids().contains(&leaf) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "subpane target pane is not in the layout",
            ));
        }
        let pane_id = PaneId::alloc();
        let cwd = cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()));
        let runtime = TerminalRuntime::spawn(
            pane_id,
            rows,
            cols,
            cwd.clone(),
            scrollback_limit_bytes,
            host_terminal_theme,
            host_terminal_appearance,
            shell_config,
            launch_env,
            self.events.clone(),
            self.render_notify.clone(),
            self.render_dirty.clone(),
        )?;
        let terminal_id = TerminalId::alloc();
        let terminal = TerminalState::new(terminal_id.clone(), cwd);
        if !self.attach_subpane(leaf, pane_id, PaneState::new(terminal_id)) {
            return Err(std::io::Error::other("subpane could not join its slot"));
        }
        Ok(NewPane {
            pane_id,
            terminal,
            runtime,
        })
    }
}

impl Workspace {
    /// Spawn a shell as a new member of the slot holding `pane_id`. Returns
    /// `None` when the pane is not in this workspace.
    // Mirrors the split spawn arguments. Only the JSON API creates members, and
    // that handler lands separately from the stack model.
    #[allow(clippy::too_many_arguments, dead_code)]
    pub fn spawn_subpane(
        &mut self,
        pane_id: PaneId,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        host_terminal_appearance: Option<crate::terminal_theme::HostAppearance>,
        shell_config: crate::pane::PaneShellConfig<'_>,
        extra_env: Vec<(String, String)>,
    ) -> Option<std::io::Result<(usize, NewPane)>> {
        let tab_idx = self.find_tab_index_for_pane(pane_id)?;
        let pane_number = self.next_public_pane_number;
        let tab_number = self.tabs[tab_idx].number;
        let launch_env = self.launch_env_for_new_pane(tab_number, pane_number, extra_env);
        let new_pane = match self.tabs[tab_idx].spawn_subpane(
            pane_id,
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            host_terminal_appearance,
            shell_config,
            &launch_env,
        ) {
            Ok(new_pane) => new_pane,
            Err(err) => return Some(Err(err)),
        };
        self.register_new_pane_with_number(new_pane.pane_id, pane_number);
        Some(Ok((tab_idx, new_pane)))
    }

    /// Add a runtime-less member to the slot holding `target` and show it.
    #[cfg(test)]
    pub(crate) fn test_add_subpane(&mut self, target: PaneId) -> PaneId {
        let tab_idx = self
            .find_tab_index_for_pane(target)
            .expect("target pane must be in the workspace");
        let pane_id = PaneId::alloc();
        let pane = PaneState::new(TerminalId::alloc());
        assert!(self.tabs[tab_idx].attach_subpane(target, pane_id, pane));
        self.register_new_pane(pane_id);
        pane_id
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

#[cfg(test)]
mod lifecycle_tests {
    use ratatui::layout::Direction;

    use crate::app::AppState;
    use crate::detect::{Agent, AgentState};
    use crate::events::AppEvent;
    use crate::layout::PaneId;
    use crate::workspace::Workspace;

    /// Single-slot tab whose stack is `[root, member]` with `member` visible.
    fn single_slot_stack() -> (Workspace, PaneId, PaneId) {
        let mut ws = Workspace::test_new("stack");
        let root = ws.tabs[0].root_pane;
        let member = ws.test_add_subpane(root);
        assert_eq!(ws.tabs[0].layout.pane_ids(), vec![member]);
        ws.assert_invariants_for_test();
        (ws, root, member)
    }

    fn app_with(ws: Workspace) -> AppState {
        let mut state = AppState::test_new();
        state.workspaces.push(ws);
        state.active = Some(0);
        state.ensure_test_terminals();
        state
    }

    #[test]
    fn subpane_closing_active_member_shows_neighbor_and_keeps_tab() {
        let (mut ws, root, member) = single_slot_stack();

        assert!(!ws.close_pane(member));

        let tab = &ws.tabs[0];
        assert_eq!(tab.layout.pane_ids(), vec![root]);
        assert_eq!(tab.layout.focused(), root);
        assert!(tab.subpanes.is_empty());
        ws.assert_invariants_for_test();
    }

    #[test]
    fn subpane_closing_hidden_member_keeps_visible_leaf() {
        let (mut ws, root, member) = single_slot_stack();

        assert!(!ws.close_pane(root));

        let tab = &ws.tabs[0];
        assert_eq!(tab.layout.pane_ids(), vec![member]);
        assert_eq!(tab.root_pane, member);
        assert!(tab.subpanes.is_empty());
        ws.assert_invariants_for_test();
    }

    #[test]
    fn subpane_hidden_member_exit_keeps_tab_and_workspace() {
        let (ws, root, member) = single_slot_stack();
        let mut state = app_with(ws);
        let root_terminal = state.terminal_id_for_pane(0, root).unwrap();
        assert!(!state.close_pane_would_close_workspace(0, root));

        state.handle_app_event(AppEvent::PaneDied {
            pane_id: root,
            exit_reason: crate::platform::ChildExitReason::Exited,
        });

        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].tabs[0].layout.pane_ids(), vec![member]);
        assert!(!state.terminals.contains_key(&root_terminal));
        state.assert_invariants_for_test();
    }

    #[test]
    fn subpane_member_moved_to_another_tab_leaves_stack() {
        let (mut ws, root, first) = single_slot_stack();
        let second = ws.test_add_subpane(first);
        let other_tab = ws.test_add_tab(Some("other"));
        let other_root = ws.tabs[other_tab].root_pane;

        let taken = ws.take_pane_for_move(second).expect("member is movable");
        assert_eq!(taken.removed_tab_idx, None);
        assert_eq!(ws.tabs[0].layout.pane_ids(), vec![first]);
        assert_eq!(
            ws.tabs[0]
                .subpanes
                .stack_of(root)
                .map(|s| s.members().to_vec()),
            Some(vec![root, first])
        );
        ws.insert_moved_pane_into_tab(
            other_tab,
            other_root,
            taken.moved,
            Direction::Horizontal,
            0.5,
            false,
        )
        .map_err(|_| ())
        .expect("insert moved member");
        ws.assert_invariants_for_test();
    }

    #[test]
    fn subpane_invariants_hold_on_adversarial_identity_state() {
        let mut state = AppState::test_with_adversarial_identity_state();
        let ws = &mut state.workspaces[0];
        let tab_idx = ws.active_tab;
        let leaf = ws.tabs[tab_idx].layout.pane_ids()[0];
        let first = ws.test_add_subpane(leaf);
        let second = ws.test_add_subpane(leaf);
        state.ensure_test_terminals();
        state.assert_invariants_for_test();

        assert!(state.focus_pane_in_workspace(0, leaf));
        state.assert_invariants_for_test();
        state.handle_app_event(AppEvent::PaneDied {
            pane_id: leaf,
            exit_reason: crate::platform::ChildExitReason::Exited,
        });
        state.assert_invariants_for_test();
        assert!(!state.workspaces[0].close_pane(first));
        state.assert_invariants_for_test();
        assert!(state.workspaces[0].tabs[tab_idx].subpanes.is_empty());
        assert!(state.workspaces[0].tabs[tab_idx]
            .layout
            .pane_ids()
            .contains(&second));
    }

    #[test]
    fn subpane_focusing_hidden_member_activates_it_and_keeps_zoom() {
        let mut ws = Workspace::test_new("stack");
        let root = ws.tabs[0].root_pane;
        ws.test_split(Direction::Horizontal);
        let member = ws.test_add_subpane(root);
        ws.tabs[0].zoomed = true;
        let mut state = app_with(ws);

        assert!(state.focus_pane_in_workspace(0, root));

        let tab = &state.workspaces[0].tabs[0];
        assert_eq!(tab.layout.focused(), root);
        assert!(!tab.layout.pane_ids().contains(&member));
        assert!(tab.subpanes.is_hidden(member));
        assert!(tab.zoomed);
        state.assert_invariants_for_test();
    }

    #[test]
    fn subpane_hidden_agent_stays_unseen_until_activated() {
        let (ws, root, _member) = single_slot_stack();
        let mut state = app_with(ws);
        state.outer_terminal_focus = Some(true);
        let terminal_id = state.terminal_id_for_pane(0, root).unwrap();
        state.terminals.get_mut(&terminal_id).unwrap().state = AgentState::Working;

        state.handle_app_event(AppEvent::StateChanged {
            pane_id: root,
            agent: Some(Agent::Pi),
            state: AgentState::Idle,
            visible_blocker: false,
            visible_working: false,
            process_exited: false,
            observed_at: std::time::Instant::now(),
        });
        state.switch_workspace_tab(0, 0);
        state.mark_active_tab_seen();
        assert!(!state.workspaces[0].pane_state(root).unwrap().seen);

        assert!(state.focus_pane_in_workspace(0, root));
        assert!(state.workspaces[0].pane_state(root).unwrap().seen);
    }
}
