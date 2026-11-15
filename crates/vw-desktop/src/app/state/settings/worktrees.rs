pub use desktop_settings_workspace::state::*;
pub(crate) fn worktree_identity(app: &crate::app::App) -> WorktreeSettingsIdentity {
    WorktreeSettingsIdentity::from_endpoint(&crate::app::gateway_client_endpoint(), app.project_path.clone())
}
