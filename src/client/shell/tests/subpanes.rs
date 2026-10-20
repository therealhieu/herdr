use super::*;
use crate::api::schema::{Method, PaneSubpaneInfo};
use crate::input::{KeybindAction, KeybindMatch};

fn member(pane_id: &str, visible: &str, index: u32, count: u32, label: &str) -> ClientShellPane {
    ClientShellPane {
        pane_id: pane_id.into(),
        focused: pane_id == visible,
        subpane: Some(PaneSubpaneInfo {
            visible_pane_id: visible.into(),
            index,
            count,
            label: label.into(),
        }),
        ..snapshot().panes[0].clone()
    }
}

/// `pane_1` is the visible, focused member at index 2 of a three-member slot.
fn stacked_snapshot() -> ClientShellSnapshot {
    let mut projected = snapshot();
    projected.panes = vec![
        member("pane_1", "pane_1", 2, 3, "claude"),
        member("pane_2", "pane_1", 0, 3, "codex"),
        member("pane_3", "pane_1", 1, 3, "zsh"),
    ];
    projected
}

fn shell_state(projected: ClientShellSnapshot) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 24).unwrap();
    state
}

fn endpoint_methods(outcome: &ClientShellInput) -> Vec<Method> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => Some(request.method.clone()),
            _ => None,
        })
        .collect()
}

fn press(state: &mut ClientShellState, action: KeybindAction) -> Vec<Method> {
    let mut outcome = ClientShellInput::default();
    state.record_binding(KeybindMatch::Action(action), &mut outcome);
    endpoint_methods(&outcome)
}

fn focused_pane(methods: &[Method]) -> Option<&str> {
    match methods {
        [Method::PaneFocus(target)] => Some(target.pane_id.as_str()),
        _ => None,
    }
}

#[test]
fn subpane_default_keys_resolve_and_new_creates_member_in_focused_slot() {
    let keybinds = Config::default().keybinds();
    for (key, action) in [
        ('t', KeybindAction::NewSubpane),
        ('.', KeybindAction::NextSubpane),
        (',', KeybindAction::PreviousSubpane),
    ] {
        let resolved = crate::input::resolve_prefix_binding(
            &keybinds,
            &crate::input::TerminalKey::new(KeyCode::Char(key), KeyModifiers::empty()),
        );
        assert!(
            matches!(resolved, Some(KeybindMatch::Action(found)) if found == action),
            "prefix+{key} should resolve to {action:?}"
        );
    }

    let mut state = shell_state(snapshot());
    let methods = press(&mut state, KeybindAction::NewSubpane);
    assert!(
        matches!(methods.as_slice(), [Method::PaneSubpaneCreate(params)]
            if params.pane_id == "pane_1" && params.focus && params.cwd.is_none()),
        "{methods:?}"
    );
}

#[test]
fn subpane_keys_step_with_wrap_and_jump_by_index() {
    let mut state = shell_state(stacked_snapshot());

    let next = press(&mut state, KeybindAction::NextSubpane);
    assert_eq!(focused_pane(&next), Some("pane_2"));
    let previous = press(&mut state, KeybindAction::PreviousSubpane);
    assert_eq!(focused_pane(&previous), Some("pane_3"));
    let jump = press(&mut state, KeybindAction::FocusSubpane(1));
    assert_eq!(focused_pane(&jump), Some("pane_3"));
    assert!(press(&mut state, KeybindAction::FocusSubpane(3)).is_empty());
}

#[test]
fn subpane_switch_keys_do_nothing_on_a_plain_pane() {
    let mut state = shell_state(snapshot());
    for action in [
        KeybindAction::NextSubpane,
        KeybindAction::PreviousSubpane,
        KeybindAction::FocusSubpane(0),
    ] {
        assert!(press(&mut state, action).is_empty(), "{action:?}");
    }
}
