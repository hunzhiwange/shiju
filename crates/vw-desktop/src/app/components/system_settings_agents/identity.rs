//! Host clock projection for identity metadata.
pub(super) fn workspace_identity_meta_line(
    locale: config_ui::DesktopLocale,
    size: Option<u64>,
    modified: Option<u64>,
) -> Option<String> {
    desktop_settings_agents::identity::workspace_identity_meta_line(
        locale,
        size,
        modified.map(|timestamp| {
            crate::app::components::chat_panel::utils::relative_time_bucket(
                timestamp,
                crate::app::time::now_ms(),
            )
        }),
    )
}
