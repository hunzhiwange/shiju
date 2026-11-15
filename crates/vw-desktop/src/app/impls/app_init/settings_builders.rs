//! 启动设置装配的窄入口。
pub(super) use desktop_settings_channels::app_init::settings_builders::build_channels_settings;

/// Policy is loaded from the selected gateway, never from client-local config.
pub(super) fn build_worktree_settings() -> super::state::WorktreeSettingsState {
    Default::default()
}
