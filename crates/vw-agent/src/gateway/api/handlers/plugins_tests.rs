// 覆盖插件列表、详情校验及变更请求的输入边界。

use std::fs;
use std::path::Path;

use serde_json::json;
use vw_config_types::plugins::PluginsConfig;

use axum::Json;
use axum::extract::Query;
use axum::http::StatusCode;

fn write_plugin(plugin_root: &Path, name: &str) {
    fs::create_dir_all(plugin_root.join(".vibewindow-plugin")).expect("manifest dir");
    fs::create_dir_all(plugin_root.join("skills")).expect("skills dir");
    fs::write(plugin_root.join(".mcp.json"), json!({ "mcpServers": [] }).to_string()).expect("mcp");
    fs::write(
        plugin_root.join(".vibewindow-plugin/plugin.json"),
        json!({
            "name": name,
            "version": "0.2.0",
            "description": format!("{name} plugin"),
            "skills": "./skills",
            "mcpServers": "./.mcp.json",
            "interface": {
                "displayName": "Demo Plugin",
                "shortDescription": "Shown in settings"
            }
        })
        .to_string(),
    )
    .expect("manifest");
}

// 变更处理器先拒绝空标识，确保非法请求不会触发插件扫描或配置加载。
#[test]
fn plugins_handler_lists_installed_plugins() {
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    let plugins_dir = config.join("plugins");
    write_plugin(&plugins_dir.join("demo-plugin"), "demo-plugin");
    fs::write(plugins_dir.join("demo-plugin.disabled"), "").expect("disabled marker");

    let items = super::collect_plugin_catalog(workspace, config.clone(), PluginsConfig::default())
        .expect("catalog");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "demo-plugin");
    assert_eq!(items[0].display_name, "Demo Plugin");
    assert_eq!(items[0].version, "0.2.0");
    assert!(!items[0].enabled);
    assert_eq!(items[0].component_count, 3);

    let detail = super::resolve_plugin_detail(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
        "demo-plugin",
    )
    .expect("detail");

    assert_eq!(detail.id, "demo-plugin");
    assert_eq!(detail.capabilities.skills, 1);
    assert_eq!(detail.capabilities.mcp_servers, 1);
    assert_eq!(detail.capabilities.hooks, 0);
    assert!(detail.can_toggle);
    assert!(detail.can_delete);
}

#[test]
fn plugin_detail_rejects_blank_and_missing_ids() {
    let blank =
        super::resolve_plugin_detail("/workspace", "/config", PluginsConfig::default(), " \t ")
            .expect_err("blank plugin id should be rejected before discovery");
    assert_eq!(blank.to_string(), "plugin_id is required");

    let temp = tempfile::tempdir().expect("temp dir");
    let missing = super::resolve_plugin_detail(
        temp.path().join("workspace"),
        temp.path().join("config"),
        PluginsConfig::default(),
        " missing-plugin ",
    )
    .expect_err("unknown plugin should be rejected");
    assert_eq!(missing.to_string(), "Plugin not found: missing-plugin");
}

#[tokio::test]
async fn plugin_mutation_handlers_reject_blank_ids_before_loading_config() {
    let detail_error =
        super::detail_get(Query(super::PluginDetailQuery { plugin_id: "  ".to_string() }))
            .await
            .expect_err("blank detail id should be rejected");
    assert_eq!(detail_error.status, StatusCode::BAD_REQUEST);
    assert_eq!(detail_error.message, "plugin_id is required");

    let toggle_error = super::set_enabled_post(Json(super::PluginSetEnabledRequest {
        plugin_id: "\n".to_string(),
        enabled: true,
    }))
    .await
    .expect_err("blank toggle id should be rejected");
    assert_eq!(toggle_error.status, StatusCode::BAD_REQUEST);
    assert_eq!(toggle_error.message, "plugin_id is required");

    let delete_error =
        super::delete_post(Json(super::PluginMutationRequest { plugin_id: "\t".to_string() }))
            .await
            .expect_err("blank delete id should be rejected");
    assert_eq!(delete_error.status, StatusCode::BAD_REQUEST);
    assert_eq!(delete_error.message, "plugin_id is required");
}

#[test]
fn plugin_text_helpers_trim_values_and_fall_back_for_empty_text() {
    assert_eq!(super::non_empty_string(Some("  Plugin Name  ")), Some("Plugin Name".to_string()));
    assert_eq!(super::non_empty_string(Some(" \t ")), None);
    assert_eq!(super::non_empty_string(None), None);
}

#[test]
fn codex_plugin_compat_t06_gateway_retains_exact_marketplace_ids() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    write_plugin(&package, "demo-plugin");
    let plugins = config.join("plugins");
    for market in ["alpha", "beta"] {
        crate::plugins::install_plugin_from_marketplace(
            package.to_str().unwrap(),
            &plugins,
            market,
        )
        .unwrap();
    }
    let items = super::collect_plugin_catalog(
        temp.path().join("workspace"),
        &config,
        PluginsConfig::default(),
    )
    .unwrap();
    assert_eq!(
        items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["demo-plugin@alpha", "demo-plugin@beta"]
    );
    let detail = super::resolve_plugin_detail(
        temp.path().join("workspace"),
        &config,
        PluginsConfig::default(),
        "demo-plugin@beta",
    )
    .unwrap();
    assert_eq!(detail.id, "demo-plugin@beta");
    assert!(detail.can_toggle && detail.can_delete);
    assert!(
        super::resolve_plugin_detail(
            temp.path().join("workspace"),
            config,
            PluginsConfig::default(),
            "demo-plugin"
        )
        .is_err()
    );
}
