//! 验证插件目录的筛选、操作消息、详情面板和各类加载状态。
//!
//! 插件夹具组合加载、空、筛选、选中、忙碌和禁用状态，覆盖详情面板回退。

use crate::app::App;
use crate::app::state::PluginCatalogItem;

fn plugin(
    id: &str,
    display_name: &str,
    enabled: bool,
    source_path: Option<&str>,
) -> PluginCatalogItem {
    PluginCatalogItem {
        id: id.to_string(),
        display_name: display_name.to_string(),
        version: "1.0.0".to_string(),
        description: "Shown in settings".to_string(),
        enabled,
        component_count: 2,
        source_path: source_path.map(ToOwned::to_owned),
    }
}

#[test]
fn desktop_plugins_tab_renders_catalog_items() {
    let plugin = PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "0.2.0".to_string(),
        description: "Shown in settings".to_string(),
        enabled: true,
        component_count: 3,
        source_path: Some("/tmp/demo-plugin".to_string()),
    };

    assert!(super::plugin_matches_query(&plugin, "DEMO"));
    assert!(super::plugin_matches_query(&plugin, "0.2"));
    assert!(super::plugin_matches_query(&plugin, "settings"));
    assert!(super::plugin_matches_query(&plugin, "/tmp"));
    assert!(!super::plugin_matches_query(&plugin, "missing"));

    let _ = super::plugin_catalog_item(plugin.clone(), true);
    let _ = super::plugin_detail_panel(Some(&plugin), false);
}

#[test]
fn desktop_plugins_actions_dispatch_settings_messages() {
    let enable = super::plugin_set_enabled_message("demo-plugin", false);
    let delete = super::plugin_delete_message("demo-plugin");

    assert!(format!("{enable:?}").contains("PluginsSetEnabledRequested"));
    assert!(format!("{delete:?}").contains("PluginsDeleteRequested"));
}

#[test]
fn plugin_initials_use_name_words_then_fall_back_to_uppercase_id() {
    let mut plugin = PluginCatalogItem {
        id: "xy-plugin".to_string(),
        display_name: "remote_skill provider".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        enabled: false,
        component_count: 0,
        source_path: None,
    };

    assert_eq!(super::plugin_initials(&plugin), "RS");

    plugin.display_name = "---___".to_string();
    assert_eq!(super::plugin_initials(&plugin), "XY");
}

#[test]
fn plugin_query_trims_whitespace_and_matches_empty_query() {
    let plugin = PluginCatalogItem {
        id: "demo-plugin".to_string(),
        display_name: "Demo Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Local plugin".to_string(),
        enabled: true,
        component_count: 1,
        source_path: None,
    };

    assert!(super::plugin_matches_query(&plugin, "   "));
    assert!(super::plugin_matches_query(&plugin, "  DEMO  "));
    assert!(!super::plugin_matches_query(&plugin, "missing"));
}

#[test]
fn plugin_detail_panel_builds_empty_busy_and_disabled_variants() {
    let disabled = plugin("disabled-plugin", "Disabled Plugin", false, None);

    let _ = super::plugin_detail_panel(None, false);
    let _ = super::plugin_detail_panel(Some(&disabled), true);
    let _ = super::plugin_catalog_item(disabled, false);
}

#[test]
fn plugins_view_builds_loading_empty_filtered_and_selected_fallback_states() {
    let mut app = App::new().0;

    app.skills_settings.plugin_loading = true;
    let _ = super::view(&app);

    app.skills_settings.plugin_loading = false;
    app.skills_settings.plugin_status_message = Some("加载失败".to_string());
    app.skills_settings.plugin_status_is_error = true;
    let _ = super::view(&app);

    app.skills_settings.plugin_catalog = vec![
        plugin("alpha-plugin", "Alpha Plugin", true, Some("/tmp/alpha")),
        plugin("beta-plugin", "Beta Plugin", false, None),
    ];
    app.skills_settings.query = "alpha".to_string();
    app.skills_settings.selected_plugin_id = Some("missing-plugin".to_string());
    let _ = super::view(&app);

    app.skills_settings.query = "nothing".to_string();
    let _ = super::view(&app);
}
