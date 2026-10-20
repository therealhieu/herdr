//! JSON API handlers for pane slots that hold several terminals.

use crate::api::schema::{
    EventData, EventEnvelope, EventKind, PaneSubpaneCreateParams, ResponseResult,
};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_pane_subpane_create(
        &mut self,
        id: String,
        params: PaneSubpaneCreateParams,
    ) -> String {
        let not_found = |id| {
            encode_error(
                id,
                "pane_not_found",
                format!("pane {} not found", params.pane_id),
            )
        };
        let Some((ws_idx, target)) = self.parse_pane_id(&params.pane_id) else {
            return not_found(id);
        };
        let Some(slot_leaf) = self.state.workspaces.get(ws_idx).and_then(|ws| {
            let tab = ws.tabs.get(ws.find_tab_index_for_pane(target)?)?;
            Some(tab.subpanes.visible_leaf(target))
        }) else {
            return not_found(id);
        };
        // A new member shares its slot, so it starts at the slot's current size.
        let (rows, cols) = self
            .state
            .new_pane_size(crate::ui::NewPanePlacement::Existing {
                ws_idx,
                pane: slot_leaf,
            });
        let cwd = params
            .cwd
            .clone()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                let follow_cwd = self.launch_cwd_for_pane_in_workspace(ws_idx, slot_leaf);
                Some(self.resolve_new_terminal_cwd(follow_cwd))
            });
        let default_shell = self.state.default_shell.clone();
        let scrollback_limit_bytes = self.state.pane_scrollback_limit_bytes;
        let host_terminal_theme = self.state.host_terminal_theme;
        let host_terminal_appearance = self.state.host_terminal_appearance;
        let previous_focus = self.state.current_pane_focus_target();
        let shell_config = crate::pane::PaneShellConfig::new(&default_shell, self.state.shell_mode);
        let Some(ws) = self.state.workspaces.get_mut(ws_idx) else {
            return not_found(id);
        };
        let (tab_idx, new_pane) = match ws.spawn_subpane(
            target,
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            host_terminal_appearance,
            shell_config,
            Vec::new(),
        ) {
            Some(Ok(result)) => result,
            Some(Err(err)) => {
                return encode_error(id, "pane_subpane_create_failed", err.to_string())
            }
            None => return not_found(id),
        };
        if params.focus {
            self.state.focus_pane_in_workspace(ws_idx, new_pane.pane_id);
            self.state
                .record_pane_focus_change(previous_focus, ws_idx, new_pane.pane_id);
            self.state.mode = crate::app::Mode::Terminal;
        }
        self.terminal_runtimes
            .insert(new_pane.terminal.id.clone(), new_pane.runtime);
        self.state
            .remove_alias_shadowed_by_new_pane(new_pane.pane_id);
        self.state
            .terminals
            .insert(new_pane.terminal.id.clone(), new_pane.terminal);
        self.schedule_session_save();
        let Some(pane) = self.pane_info(ws_idx, new_pane.pane_id) else {
            return encode_error(id, "pane_not_found", "created pane not found");
        };
        self.emit_event(EventEnvelope {
            event: EventKind::PaneCreated,
            data: EventData::PaneCreated { pane: pane.clone() },
        });
        self.emit_layout_updated_event(ws_idx, tab_idx);

        encode_success(id, ResponseResult::PaneInfo { pane })
    }
}
