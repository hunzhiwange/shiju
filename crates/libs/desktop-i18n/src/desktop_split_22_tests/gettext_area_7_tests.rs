use crate::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_general_editor_sources_have_catalog_entries() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    for source in [
        include_str!("../../../../vw-desktop/src/app/components/system_settings_general.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_editor.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_acp.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_browser.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_cron.rs"),
        include_str!("../../../desktop-settings-cron/src/system_settings_cron/schedule.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_themes/editor.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_themes/gallery.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_themes/preview.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_multimodal.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_projects.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_query_classification.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_heartbeat.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_autonomy.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_quick_messages.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_dialogue_flow.rs"),
        include_str!("../../../../vw-desktop/src/app/components/system_settings_reliability.rs"),
    ] {
        for tail in source.split(".__(\"").skip(1) {
            let encoded = tail.split('"').next().unwrap();
            let key: String = serde_json::from_str(&format!("\"{encoded}\"")).unwrap();
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
        }
    }
}

#[test]
fn desktop_gettext_area_7_all_help_documents_have_complete_catalog_entries() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let mut count = 0;
    for source in [
        include_str!("../../../desktop-settings-appearance/src/help.rs"),
        include_str!("../../../desktop-settings-agents/src/system_settings_autonomy.rs"),
        include_str!("../../../desktop-settings-cron/src/system_settings_cron/mod.rs"),
        include_str!("../../../desktop-settings-cron/src/system_settings_heartbeat.rs"),
        include_str!("../../../desktop-settings-network/src/system_settings_proxy.rs"),
        include_str!("../../../desktop-settings-runtime/src/system_settings_reliability.rs"),
        include_str!("../../../desktop-settings-cron/src/system_settings_scheduler.rs"),
    ] {
        for block in source.split("r#\"").skip(1) {
            let key = block.split("\"#").next().unwrap();
            assert!(en.contains_key(key), "missing English help: {key}");
            assert!(tw.contains_key(key), "missing Traditional Chinese help: {key}");
            assert_eq!(I18n::new(DesktopLocale::SimplifiedChinese).__(key), key);
            assert_ne!(I18n::new(DesktopLocale::English).__(key), key);
            count += 1;
        }
    }
    assert_eq!(count, 20);
}
