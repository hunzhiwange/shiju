#![cfg(not(target_arch = "wasm32"))]
// Cross-domain comparison with the General settings strip remains in the host.
use super::*;
const DB_URL: &str = "postgres://fixture.invalid/資料庫/記憶/長目錄/long-fixture-database-name";
const QDRANT_KEY: &str = "fixture-secret";
const QDRANT_URL: &str = "https://fixture.invalid/qdrant";
fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.memory_settings.active_tab = MemorySettingsTab::General;
    app.memory_settings.backend = "qdrant".into();
    app.memory_settings.qdrant_url_input = QDRANT_URL.into();
    app.memory_settings.qdrant_api_key_input = QDRANT_KEY.into();
    app.storage_settings.provider = "postgres".into();
    app.storage_settings.schema = "fixture_schema".into();
    app.storage_settings.table = "fixture_memories".into();
    app.storage_settings.tls = false;
    app.storage_settings.db_url_input = DB_URL.into();
    app.storage_settings.connect_timeout_secs_input = "30".into();
    app
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_core_appearance_memory() {
    use super::super::system_settings_common::tab_appearance_tests;

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_core_appearance_memory"),
        fixture,
        |scene, app| {
            let locale = app.desktop_locale;
            let qdrant_label = memory_tab_label(locale, MemorySettingsTab::Qdrant);
            app.memory_settings.backend = "sqlite".into();
            assert!(!scene.labels(view(app)).contains(qdrant_label));
            app.memory_settings.backend = "qdrant".into();
            let labels: Vec<_> = MEMORY_TABS
                .iter()
                .chain(QDRANT_TAB)
                .map(|tab| memory_tab_label(locale, tab.tab))
                .collect();
            let content = runtime_text(locale, "Qdrant 地址");
            assert!(!scene.labels(view(app)).contains(content));
            let message = scene.check(app, view(app), &labels, labels.len() - 1);
            assert!(matches!(
                message,
                SettingsMessage::Memory(MemoryMessage::TabSelected(MemorySettingsTab::Qdrant))
            ));
            let _ = crate::app::message::settings::update(app, message);
            assert_eq!(app.memory_settings.active_tab, MemorySettingsTab::Qdrant);
            assert!(scene.labels(view(app)).contains(content));
        },
    )
    .await;
}
