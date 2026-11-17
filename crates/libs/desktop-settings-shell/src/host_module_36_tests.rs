//! Search localization and ordering use actual domain descriptors, without App resources.
use crate::{
    SystemTab,
    search::{
        SystemSettingsSearchEntry, SystemSettingsSearchTarget, localize_catalog, search_results,
    },
};
fn catalog(locale: config_ui::DesktopLocale) -> Vec<SystemSettingsSearchEntry> {
    let mut entries: Vec<_> = SystemTab::all()
        .map(|tab| {
            SystemSettingsSearchEntry::new(
                tab.label(),
                None,
                crate::system_tab_keywords(tab),
                SystemSettingsSearchTarget::TopLevel(tab),
            )
        })
        .collect();
    for (tab, descriptor) in [
        SystemTab::WebSearch,
        SystemTab::HttpRequest,
        SystemTab::Browser,
        SystemTab::Proxy,
        SystemTab::Tunnel,
    ]
    .into_iter()
    .zip(desktop_settings_network::system_settings_network::NETWORK_TABS)
    {
        entries.push(SystemSettingsSearchEntry::new(
            descriptor.label,
            Some("网络配置"),
            descriptor.keywords,
            SystemSettingsSearchTarget::Network(tab),
        ));
    }
    entries.extend(desktop_settings_providers::models_page::MODEL_TABS.iter().map(|descriptor| {
        SystemSettingsSearchEntry::new(
            descriptor.label,
            Some("模型"),
            descriptor.keywords,
            SystemSettingsSearchTarget::Models(descriptor.tab),
        )
    }));
    localize_catalog(entries, locale)
}
#[test]
fn search_results_show_secondary_pages_with_parent_paths() {
    use config_ui::DesktopLocale;

    let locale = DesktopLocale::English;

    let results = search_results(catalog(locale), "browser");
    let browser = results.first().expect("browser child result");
    assert_eq!(browser.target, SystemSettingsSearchTarget::Network(SystemTab::Browser));
    assert_eq!(browser.label, "Browser");
    assert_eq!(browser.parent_label, Some("Network settings"));

    let results = search_results(catalog(locale), "网络配置");
    assert_eq!(results[0].target, SystemSettingsSearchTarget::TopLevel(SystemTab::WebSearch));
    for tab in [
        SystemTab::WebSearch,
        SystemTab::HttpRequest,
        SystemTab::Browser,
        SystemTab::Proxy,
        SystemTab::Tunnel,
    ] {
        assert!(results.iter().any(|entry| {
            entry.target == SystemSettingsSearchTarget::Network(tab)
                && entry.parent_label == Some("Network settings")
        }));
    }
}

#[test]
fn search_matches_setting_labels_and_localizes_each_parent() {
    use crate::general_fields;
    use config_ui::DesktopLocale;

    let mut locale = DesktopLocale::SimplifiedChinese;
    for (selected_locale, query, label, path) in [
        (DesktopLocale::SimplifiedChinese, "语言", "语言", "界面 > 常规"),
        (DesktopLocale::TraditionalChinese, "語言", "語言", "介面 > 一般"),
        (DesktopLocale::English, "yuyan", "Language", "Interface > General"),
    ] {
        locale = selected_locale;
        let results = search_results(catalog(locale), query);
        let first = results.first().expect("language field");
        assert_eq!(
            first.target,
            SystemSettingsSearchTarget::GeneralField(&general_fields::LANGUAGE)
        );
        assert_eq!(first.label, label);
        assert_eq!(first.parent_path().as_deref(), Some(path));
    }

    let results = search_results(catalog(locale), "Font size");
    let paths: Vec<_> = results
        .iter()
        .filter(|entry| entry.label == "Font size")
        .filter_map(SystemSettingsSearchEntry::parent_path)
        .collect();
    assert_eq!(paths, ["Terminal > General", "Editor > General"]);
}

#[test]
fn archived_chats_browse_navigation_is_searchable() {
    let tab =
        SystemTab::all().find(|tab| tab.to_string() == "已归档的聊天").expect("archive navigation");
    assert!(tab_matches_query(tab, "archived"));
    assert!(tab_matches_query(tab, "归档"));
}

#[test]
fn themes_are_nested_under_general_navigation() {
    assert!(
        !SystemTab::all().any(|tab| tab == SystemTab::Themes),
        "Themes should be nested under General"
    );
    assert_eq!(SystemTab::Themes.to_string(), "主题");
}

fn tab_matches_query(tab: SystemTab, query: &str) -> bool {
    desktop_settings_network::system_settings_search::SystemSettingsSearchEntry::new(
        tab.label(),
        None,
        crate::system_tab_keywords(tab),
        tab,
    )
    .matches(query)
}

#[test]
fn multimodal_search_keywords_include_ocr_and_image_model() {

    use desktop_settings_providers::state::ModelConfigTab;

    for query in ["OCR", "图片模型"] {
        let results = search_results(catalog(config_ui::DesktopLocale::SimplifiedChinese), query);
        let entry = results.first().expect("multimodal search result");
        assert_eq!(entry.target, SystemSettingsSearchTarget::Models(ModelConfigTab::Multimodal));
        assert_eq!(entry.target.page(), SystemTab::Models);
        assert_eq!(entry.label, "多模态");
        assert_eq!(entry.parent_label, Some("模型"));
    }
}

use crate::cron_render_test_support;
#[path = "host_module_36_companion_tests.rs"]
mod companion;
#[path = "host_module_36_security_tests.rs"]
mod security;
#[path = "host_module_36_delivery_tests.rs"]
mod delivery;
