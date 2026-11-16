use super::*;
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_network() {
    use super::super::system_settings_common::tab_appearance_tests;
    use crate::app::message::settings::{self, SettingsMessage};

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_network"),
        || App::new().0,
        |scene, app| {
            app.navigation_panels.system_settings_tab = SystemTab::WebSearch;
            app.web_search_settings.save_error = Some("Fixture web search error".into());
            app.http_request_settings.save_error = Some("Fixture HTTP error".into());
            let labels: Vec<_> = NETWORK_TABS
                .iter()
                .map(|tab| connection_text(app.desktop_locale, tab.label))
                .collect();
            let before = scene.labels(view(app));
            assert!(before.contains("Fixture web search error"));
            assert!(!before.contains("Fixture HTTP error"));
            let message = scene.check(app, view(app), &labels, 1);
            assert!(matches!(message, SettingsMessage::SystemTabSelected(SystemTab::HttpRequest)));
            let _ = settings::update(app, message);
            assert_eq!(app.navigation_panels.system_settings_tab, SystemTab::HttpRequest);
            assert!(app.http_request_settings.save_error.is_none());
            app.http_request_settings.save_error = Some("Fixture HTTP error".into());
            let after = scene.labels(view(app));
            assert!(!after.contains("Fixture web search error"));
            assert!(after.contains("Fixture HTTP error"));
        },
    )
    .await;
}
