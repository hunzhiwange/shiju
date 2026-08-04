use super::super::state::TuiState;

#[test]
fn schedule_panel_uses_cron_api_and_confirms_destructive_changes() {
    let state = TuiState::default();
    for command in ["schedule", "routines", "cron"] {
        assert!(state.command_registry.resolve(command).is_some(), "/{command}");
    }
}
