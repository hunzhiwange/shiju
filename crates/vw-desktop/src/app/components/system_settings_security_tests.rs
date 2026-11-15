use super::*;
use crate::app::state::SecuritySettingsTab;
use crate::app::App;
fn test_app() -> App { App::new().0 }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_security() {
    use super::super::system_settings_common::tab_appearance_tests;
    use crate::app::message::settings::SettingsMessage;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_security"),
        test_app,
        |scene, app| {
            app.security_settings.active_tab = SecuritySettingsTab::Bash;
            let labels: Vec<_> = SECURITY_TABS
                .iter()
                .map(|tab| security_tab_label(app.desktop_locale, tab.tab))
                .collect();
            assert!(matches!(
                scene.check(app, view(app), &labels, 5),
                SettingsMessage::SecurityTabSelected(SecuritySettingsTab::Guard)
            ));
        },
    )
    .await;
}
