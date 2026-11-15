use super::*;
use crate::app::state::AutonomySettingsTab;
fn test_app() -> App { App::new().0 }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_autonomy() {
    use super::super::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::message::settings::SettingsMessage;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_autonomy"),
        test_app,
        |scene, app| {
            app.autonomy_settings.active_tab = AutonomySettingsTab::Permissions;
            let i18n = I18n::new(app.desktop_locale);
            let labels: Vec<_> = AUTONOMY_TABS.iter().map(|tab| i18n.__(tab.label)).collect();
            assert!(matches!(
                scene.check(app, view(app), &labels, 3),
                SettingsMessage::AutonomyTabSelected(AutonomySettingsTab::NonCli)
            ));
        },
    )
    .await;
}
