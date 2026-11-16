use super::*;
fn test_app() -> App { App::new().0 }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_gateway() {
    use super::super::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_gateway"),
        test_app,
        |scene, app| {
            app.gateway_settings.active_tab = GatewaySettingsTab::Config;
            let labels: Vec<_> = GATEWAY_TABS
                .iter()
                .map(|tab| connection_text(app.general_settings.desktop_locale, tab.label))
                .collect();
            let content = connection_text(app.general_settings.desktop_locale, "生成 skey");
            assert!(!scene.labels(view(app)).contains(content));
            let message = scene.check(app, view(app), &labels, 1);
            assert!(matches!(
                message,
                SettingsMessage::Gateway(GatewayMessage::TabSelected(GatewaySettingsTab::Skeys))
            ));
            let _ = crate::app::message::settings::update(app, message);
            assert_eq!(app.gateway_settings.active_tab, GatewaySettingsTab::Skeys);
            assert!(scene.labels(view(app)).contains(content));
        },
    )
    .await;
}
