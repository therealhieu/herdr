use super::*;
use crate::api::schema::{Method, PaneSubpaneInfo};
use crate::input::{KeybindAction, KeybindMatch};
use crate::subpane_strip::{strip_layout, StripSegmentKind};

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

fn pane_surface_with(width: u16, top_border: bool) -> PaneSurfaceFrame {
    let mut pane_surface = surface();
    let top = u16::from(top_border);
    let pane = &mut pane_surface.panes[0];
    pane.rect = SurfaceRect {
        x: 0,
        y: 0,
        width,
        height: 6,
    };
    pane.inner_rect = SurfaceRect {
        x: 1,
        y: top,
        width: width - 2,
        height: 6 - top - 1,
    };
    // A vertical split handle covering the slot's top border row.
    pane_surface.splits.push(PaneSurfaceSplit {
        direction: PaneSurfaceSplitDirection::Vertical,
        pos: 0,
        area: SurfaceRect {
            x: 0,
            y: 0,
            width,
            height: 12,
        },
        hit_rect: SurfaceRect {
            x: 0,
            y: 0,
            width,
            height: 1,
        },
        path: vec![true],
    });
    pane_surface
}

fn strip_state(width: u16, top_border: bool) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(stacked_snapshot()));
    state.set_pane_surface(pane_surface_with(width, top_border));
    state.compose(106, 24).unwrap();
    state
}

/// Click the strip segment of `kind`, placed by the same layout the server
/// renders with.
fn click_segment(state: &mut ClientShellState, kind: StripSegmentKind) -> Vec<Method> {
    let pane = state.hits.panes[0].rect;
    let layout = strip_layout(&["codex", "zsh", "claude"], 2, pane.width);
    let segment = layout
        .segments
        .iter()
        .find(|segment| segment.kind == kind)
        .unwrap_or_else(|| panic!("{kind:?} segment in {layout:?}"));
    let outcome =
        state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: pane.x + segment.x + segment.width - 1,
            row: pane.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(
        state.chrome_drag.is_none(),
        "a strip click must not start a split drag"
    );
    endpoint_methods(&outcome)
}

#[test]
fn subpane_strip_clicks_route_member_close_and_add_before_split_handles() {
    let mut state = strip_state(40, true);

    let member = click_segment(&mut state, StripSegmentKind::Member(0));
    assert_eq!(focused_pane(&member), Some("pane_2"));
    let close = click_segment(&mut state, StripSegmentKind::Close(2));
    assert!(
        matches!(close.as_slice(), [Method::PaneClose(target)] if target.pane_id == "pane_1"),
        "{close:?}"
    );
    let add = click_segment(&mut state, StripSegmentKind::Add);
    assert!(
        matches!(add.as_slice(), [Method::PaneSubpaneCreate(params)]
            if params.pane_id == "pane_1" && params.focus),
        "{add:?}"
    );
}

#[test]
fn subpane_strip_badge_click_shows_the_next_member() {
    let mut state = strip_state(24, true);

    let badge = click_segment(&mut state, StripSegmentKind::Badge);
    assert_eq!(focused_pane(&badge), Some("pane_2"));
}

#[test]
fn subpane_strip_has_no_hits_without_a_top_border() {
    let state = strip_state(40, false);

    assert!(state.hits.subpane_strips.is_empty());
}
