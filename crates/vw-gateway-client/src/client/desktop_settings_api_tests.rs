//! 验证桌面设置、技能、插件、钩子及外部应用接口的传输契约。
//!
//! 覆盖租户请求头、项目路径规整、兼容包装器，以及无效或缺失字段的容错边界。

use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;

use crate::client::test_support;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    theme: String,
}

struct RejectSerialization;

impl Serialize for RejectSerialization {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("settings serialization rejected"))
    }
}

fn assert_tenant_headers(request: &test_support::RecordedRequest, tenant_scope: &TenantScope) {
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some(tenant_scope.platform_id().as_ref()));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some(tenant_scope.company_id().as_ref()));
}

#[tokio::test]
async fn desktop_settings_api_routes_skills_external_apps_and_preferences() {
    let tenant_scope =
        TenantScope::new("desktop-platform", "desktop-company").expect("desktop tenant scope");
    let skill = json!({
        "id": "rust",
        "title": "Rust",
        "description": "systems",
        "kind": "built_in",
        "resource_count": 1,
        "installed": false,
        "enabled": true,
        "source": "built_in",
        "source_path": null
    });
    let detail = json!({
        "id": "rust",
        "title": "Rust",
        "description": "systems",
        "kind": "built_in",
        "installed": true,
        "enabled": true,
        "source": "built_in",
        "source_path": null,
        "document_name": "SKILL.md",
        "document_content": "content",
        "can_install": true,
        "can_toggle": true,
        "can_delete": false
    });
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, json!([skill])),
            (200, detail),
            (200, json!({"path": "/repo/.codex/skills/new"})),
            (200, json!({"path": "/repo/.codex/skills/rust"})),
            (200, json!({"path": "/repo/.codex/skills/rust"})),
            (200, json!({"path": "/repo/.codex/skills/rust"})),
            (200, json!({"platform": "macos", "apps": [{"id": "finder", "available": true}]})),
            (200, json!({})),
            (200, json!({})),
            (200, json!({"command": "status", "output": "Service: running"})),
            (200, json!({"app_ui": {"system_settings": {"theme": "dark"}}})),
            (200, json!({})),
            (200, json!({"compact": true})),
            (200, json!({"compact": false})),
            (200, json!({"content": "select 1"})),
            (200, json!({})),
            (200, json!({"model": "gpt-5", "auto_model": true, "acp_agent": "codex"})),
            (200, json!({})),
        ],
        tenant_scope.clone(),
    );

    assert_eq!(server.client().skills_get(Some("/repo")).await.expect("skills").len(), 1);
    assert_eq!(
        server.client().skill_detail_get(Some("/repo"), "rust").await.expect("detail").id,
        "rust"
    );
    assert!(server.client().skill_create("/repo").await.expect("create").ends_with("/new"));
    assert!(
        server
            .client()
            .skill_install_builtin("/repo", "rust")
            .await
            .expect("install")
            .ends_with("rust")
    );
    assert!(
        server
            .client()
            .skill_set_enabled(Some("/repo"), "rust", false)
            .await
            .expect("enable")
            .ends_with("rust")
    );
    assert!(
        server
            .client()
            .skill_delete(Some("/repo"), "rust")
            .await
            .expect("delete")
            .ends_with("rust")
    );
    let apps = server.client().desktop_external_apps_get().await.expect("apps");
    assert_eq!(apps.apps, vec![("finder".to_string(), true)]);
    server.client().desktop_external_app_open("/repo", "finder").await.expect("open");
    server.client().desktop_external_path_reveal("/repo").await.expect("reveal");
    assert_eq!(
        server.client().desktop_service_command("status").await.expect("service").output,
        "Service: running"
    );
    let settings: Option<Settings> =
        server.client().desktop_system_settings_get().await.expect("settings");
    assert_eq!(settings.expect("some").theme, "dark");
    server
        .client()
        .desktop_system_settings_patch(&Settings { theme: "light".to_string() })
        .await
        .expect("settings patch");
    assert!(
        server.client().desktop_preferences_get().await.expect("prefs")["compact"]
            .as_bool()
            .unwrap()
    );
    assert!(
        !server
            .client()
            .desktop_preferences_patch(&json!({"compact": false}))
            .await
            .expect("prefs patch")["compact"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(server.client().desktop_tool_content_get("sql").await.expect("tool"), "select 1");
    server.client().desktop_tool_content_put("sql", "select 2").await.expect("tool put");
    assert_eq!(
        server.client().desktop_project_preferences_get("/repo").await.expect("project prefs"),
        Some(("gpt-5".to_string(), true, Some("codex".to_string())))
    );
    server
        .client()
        .desktop_project_preferences_put("/repo", "gpt-5", true, Some("codex"))
        .await
        .expect("project prefs put");

    assert!(server.take_request().path.contains("/v1/skills?project_path=%2Frepo"));
    assert!(
        server.take_request().path.contains("/v1/skills/detail?skill_id=rust&project_path=%2Frepo")
    );
    assert_eq!(server.take_request().path, "/v1/skills/create");
    assert_eq!(server.take_request().path, "/v1/skills/install-built-in");
    assert_eq!(server.take_request().path, "/v1/skills/set-enabled");
    assert_eq!(server.take_request().path, "/v1/skills/delete");
    assert_eq!(server.take_request().path, "/v1/desktop/external-apps");
    assert_eq!(server.take_request().path, "/v1/desktop/external-apps/open");
    assert_eq!(server.take_request().path, "/v1/desktop/external-path/reveal");
    assert_eq!(server.take_request().path, "/v1/desktop/service/status");
    let system_settings_get = server.take_request();
    assert_eq!(system_settings_get.path, "/v1/global/config");
    assert_tenant_headers(&system_settings_get, &tenant_scope);
    let request = server.take_request();
    assert_eq!(request.path, "/v1/global/config");
    assert_eq!(request.body, json!({"app_ui": {"system_settings": {"theme": "light"}}}));
    assert_tenant_headers(&request, &tenant_scope);
    let preferences_get = server.take_request();
    assert_eq!(preferences_get.path, "/v1/desktop/preferences");
    assert_tenant_headers(&preferences_get, &tenant_scope);
    let preferences_patch = server.take_request();
    assert_eq!(preferences_patch.path, "/v1/desktop/preferences");
    assert_tenant_headers(&preferences_patch, &tenant_scope);
    assert_eq!(server.take_request().path, "/v1/desktop/tool-content/sql");
    assert_eq!(server.take_request().path, "/v1/desktop/tool-content/sql");
    let project_preferences_get = server.take_request();
    assert!(
        project_preferences_get
            .path
            .contains("/v1/desktop/project-preferences?project_path=%2Frepo")
    );
    assert_tenant_headers(&project_preferences_get, &tenant_scope);
    let project_preferences_put = server.take_request();
    assert!(
        project_preferences_put
            .path
            .contains("/v1/desktop/project-preferences?project_path=%2Frepo")
    );
    assert_tenant_headers(&project_preferences_put, &tenant_scope);
    server.join();
}

#[tokio::test]
async fn skills_get_trims_project_path_and_omits_blank_values() {
    let server = test_support::server(vec![(200, json!([])), (200, json!([]))]);

    assert!(server.client().skills_get(Some("  /repo with spaces  ")).await.unwrap().is_empty());
    assert!(server.client().skills_get(Some("   ")).await.unwrap().is_empty());

    assert_eq!(server.take_request().path, "/v1/skills?project_path=%2Frepo+with+spaces");
    assert_eq!(server.take_request().path, "/v1/skills");
    server.join();
}

#[tokio::test]
async fn extension_client_uses_skill_and_plugin_routes() {
    let server = test_support::server(vec![(200, json!([])), (200, json!([]))]);

    assert!(server.client().skills_get(Some("/repo")).await.expect("skills").is_empty());
    assert!(server.client().plugins_get().await.expect("plugins").is_empty());
    assert_eq!(server.take_request().path, "/v1/skills?project_path=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/plugins");
    server.join();
}

#[tokio::test]
async fn desktop_client_uses_all_desktop_routes() {
    let server = test_support::server(vec![(200, json!({"command":"status","output":"ok"}))]);
    let response = server
        .client()
        .desktop_service_command_typed(super::DesktopServiceCommand::Status)
        .await
        .expect("typed service command");
    assert_eq!(response.output, "ok");
    assert_eq!(server.take_request().path, "/v1/desktop/service/status");
    assert!(server.client().desktop_service_command("shell").await.is_err());
    server.join();
}

#[test]
fn installed_skills_preserve_market_source_metadata() {
    let old: super::DesktopSkillCatalogEntryDto = serde_json::from_value(json!({
        "id": "local-skill",
        "title": "Local",
        "description": "Local skill",
        "kind": "personal",
        "resource_count": 0,
        "installed": true,
        "enabled": true,
        "source": "workspace",
        "source_path": "/repo/.vibewindow/skills/local-skill"
    }))
    .expect("old catalog payload remains compatible");
    assert_eq!(old.origin, None);
    assert_eq!(old.namespace, None);
    assert_eq!(old.package_type, None);
    assert_eq!(old.source_pack, None);

    let market: super::DesktopSkillCatalogEntryDto = serde_json::from_value(json!({
        "id": "skillhub--public--safe-skill",
        "title": "Safe Skill",
        "description": "Installed from market",
        "kind": "personal",
        "resource_count": 1,
        "installed": true,
        "enabled": true,
        "source": "skillhub",
        "source_path": "/repo/.vibewindow/skills/skillhub--public--safe-skill",
        "origin": "workspace",
        "namespace": "public",
        "package_type": "skill",
        "source_pack": "safe-suite"
    }))
    .expect("market catalog payload");
    assert_eq!(market.source, "skillhub");
    assert_eq!(market.origin.as_deref(), Some("workspace"));
    assert_eq!(market.namespace.as_deref(), Some("public"));
    assert_eq!(market.package_type.as_deref(), Some("skill"));
    assert_eq!(market.source_pack.as_deref(), Some("safe-suite"));
}

#[tokio::test]
async fn external_apps_parser_filters_invalid_entries_and_defaults_fields() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "platform": 7,
                "apps": [
                    {"id": "finder"},
                    {"id": "code", "available": "yes"},
                    {"id": "terminal", "available": true},
                    {"available": true},
                    null
                ]
            }),
        ),
        (200, json!({"platform": "linux", "apps": {}})),
    ]);

    let malformed = server.client().desktop_external_apps_get().await.unwrap();
    assert_eq!(malformed.platform, None);
    assert_eq!(
        malformed.apps,
        vec![
            ("finder".to_string(), false),
            ("code".to_string(), false),
            ("terminal".to_string(), true),
        ]
    );

    let non_array = server.client().desktop_external_apps_get().await.unwrap();
    assert_eq!(non_array.platform.as_deref(), Some("linux"));
    assert!(non_array.apps.is_empty());

    assert_eq!(server.take_request().path, "/v1/desktop/external-apps");
    assert_eq!(server.take_request().path, "/v1/desktop/external-apps");
    server.join();
}

#[tokio::test]
async fn desktop_values_normalize_commands_and_default_invalid_fields() {
    let server = test_support::server(vec![
        (200, json!({"command": "restart", "output": "restarted"})),
        (200, json!({})),
        (200, json!({"content": 7})),
        (200, json!({"model": "   ", "auto_model": true, "acp_agent": "codex"})),
        (200, json!({"model": "gpt-5", "auto_model": "yes", "acp_agent": 4})),
    ]);

    let service = server.client().desktop_service_command("  ReStart  ").await.unwrap();
    assert_eq!(service.command, "restart");
    assert_eq!(service.output, "restarted");
    assert_eq!(server.client().desktop_tool_content_get("sql").await.unwrap(), "");
    assert_eq!(server.client().desktop_tool_content_get("notes").await.unwrap(), "");
    assert_eq!(server.client().desktop_project_preferences_get("/blank").await.unwrap(), None);
    assert_eq!(
        server.client().desktop_project_preferences_get("/defaults").await.unwrap(),
        Some(("gpt-5".to_string(), false, None))
    );

    assert_eq!(server.take_request().path, "/v1/desktop/service/restart");
    assert_eq!(server.take_request().path, "/v1/desktop/tool-content/sql");
    assert_eq!(server.take_request().path, "/v1/desktop/tool-content/notes");
    assert_eq!(server.take_request().path, "/v1/desktop/project-preferences?project_path=%2Fblank");
    assert_eq!(
        server.take_request().path,
        "/v1/desktop/project-preferences?project_path=%2Fdefaults"
    );
    server.join();
}

#[tokio::test]
async fn desktop_project_preferences_get_treats_404_as_missing_preferences() {
    let server =
        test_support::server(vec![(404, json!({"error": "project preferences not found"}))]);

    let preferences =
        server.client().desktop_project_preferences_get("/missing").await.expect("missing prefs");

    assert_eq!(preferences, None);
    assert_eq!(
        server.take_request().path,
        "/v1/desktop/project-preferences?project_path=%2Fmissing"
    );
    server.join();
}

#[tokio::test]
async fn system_settings_reports_decode_and_encode_errors() {
    let server =
        test_support::server(vec![(200, json!({"app_ui": {"system_settings": {"theme": 7}}}))]);

    let decode_error = server
        .client()
        .desktop_system_settings_get::<Settings>()
        .await
        .expect_err("invalid settings should fail decoding");
    assert!(decode_error.contains("invalid type"));

    let encode_error = server
        .client()
        .desktop_system_settings_patch(&RejectSerialization)
        .await
        .expect_err("invalid settings should fail encoding");
    assert!(encode_error.contains("settings serialization rejected"));

    assert_eq!(server.take_request().path, "/v1/global/config");
    server.join();
}

#[tokio::test]
async fn desktop_settings_routes_propagate_gateway_errors() {
    let server = test_support::server(vec![
        (500, json!({"error": "create failed"})),
        (500, json!({"error": "detail failed"})),
        (500, json!({"error": "preferences failed"})),
        (500, json!({"error": "content failed"})),
    ]);

    let create_error = server.client().skill_create("/repo").await.expect_err("create error");
    assert!(create_error.contains("500"));
    assert!(create_error.contains("create failed"));

    let detail_error = server.client().plugin_detail_get("demo").await.expect_err("detail error");
    assert!(detail_error.contains("500"));
    assert!(detail_error.contains("detail failed"));

    let preferences_error = server
        .client()
        .desktop_preferences_patch(&json!({"compact": true}))
        .await
        .expect_err("preferences error");
    assert!(preferences_error.contains("500"));
    assert!(preferences_error.contains("preferences failed"));

    let content_error = server
        .client()
        .desktop_tool_content_put("sql", "select 1")
        .await
        .expect_err("content error");
    assert!(content_error.contains("500"));
    assert!(content_error.contains("content failed"));

    assert_eq!(server.take_request().method, "POST");
    assert_eq!(server.take_request().method, "GET");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "PUT");
    server.join();
}

#[tokio::test]
async fn desktop_skill_compatibility_wrappers_delegate_to_skill_routes() {
    let skill = json!({
        "id": "rust",
        "title": "Rust",
        "description": "systems",
        "kind": "built_in",
        "resource_count": 1,
        "installed": false,
        "enabled": true,
        "source": "built_in",
        "source_path": null
    });
    let detail = json!({
        "id": "rust",
        "title": "Rust",
        "description": "systems",
        "kind": "built_in",
        "installed": true,
        "enabled": true,
        "source": "built_in",
        "source_path": null,
        "document_name": "SKILL.md",
        "document_content": "content",
        "can_install": true,
        "can_toggle": true,
        "can_delete": false
    });
    let server = test_support::server(vec![
        (200, json!([skill])),
        (200, detail),
        (200, json!({"path": "/repo/.codex/skills/new"})),
        (200, json!({"path": "/repo/.codex/skills/rust"})),
        (200, json!({"path": "/repo/.codex/skills/rust"})),
        (200, json!({"path": "/repo/.codex/skills/rust"})),
        (200, json!({"app_ui": {}})),
    ]);

    assert_eq!(server.client().desktop_skills_get(None).await.expect("skills").len(), 1);
    assert_eq!(
        server.client().desktop_skill_detail_get(None, "rust").await.expect("detail").id,
        "rust"
    );
    assert!(server.client().desktop_skill_create("/repo").await.expect("create").ends_with("new"));
    assert!(
        server
            .client()
            .desktop_skill_install_builtin("/repo", "rust")
            .await
            .expect("install")
            .ends_with("rust")
    );
    assert!(
        server
            .client()
            .desktop_skill_set_enabled(None, "rust", true)
            .await
            .expect("enable")
            .ends_with("rust")
    );
    assert!(
        server.client().desktop_skill_delete(None, "rust").await.expect("delete").ends_with("rust")
    );
    let settings: Option<Settings> =
        server.client().desktop_system_settings_get().await.expect("no settings");
    assert!(settings.is_none());

    assert_eq!(server.take_request().path, "/v1/desktop/skills");
    assert_eq!(server.take_request().path, "/v1/desktop/skills/detail?skill_id=rust");
    assert_eq!(server.take_request().path, "/v1/desktop/skills/create");
    assert_eq!(server.take_request().path, "/v1/desktop/skills/install-built-in");
    assert_eq!(server.take_request().path, "/v1/desktop/skills/set-enabled");
    assert_eq!(server.take_request().path, "/v1/desktop/skills/delete");
    assert_eq!(server.take_request().path, "/v1/global/config");
    server.join();
}

#[tokio::test]
async fn desktop_settings_api_routes_plugins() {
    let plugin = json!({
        "id": "demo-plugin",
        "display_name": "Demo Plugin",
        "version": "0.2.0",
        "description": "Shown in settings",
        "enabled": true,
        "component_count": 3,
        "source_path": "/tmp/demo-plugin"
    });
    let detail = json!({
        "id": "demo-plugin",
        "display_name": "Demo Plugin",
        "version": "0.2.0",
        "description": "Shown in settings",
        "enabled": true,
        "source_path": "/tmp/demo-plugin",
        "manifest_path": "/tmp/demo-plugin/.vibewindow-plugin/plugin.json",
        "capabilities": {
            "skills": 1,
            "mcp_servers": 1,
            "hooks": 0,
            "interface": true
        },
        "can_toggle": true,
        "can_delete": true
    });
    let server = test_support::server(vec![
        (200, json!([plugin])),
        (200, detail),
        (200, json!({"path": "/tmp/demo-plugin"})),
        (200, json!({"path": "/tmp/demo-plugin"})),
    ]);

    let plugins = server.client().plugins_get().await.expect("plugins");
    assert_eq!(plugins[0].id, "demo-plugin");
    assert_eq!(plugins[0].component_count, 3);
    let detail = server.client().plugin_detail_get("demo-plugin").await.expect("detail");
    assert_eq!(detail.capabilities.skills, 1);
    assert!(server.client().plugin_set_enabled("demo-plugin", false).await.is_ok());
    assert!(server.client().plugin_delete("demo-plugin").await.is_ok());

    assert_eq!(server.take_request().path, "/v1/plugins");
    assert!(server.take_request().path.contains("/v1/plugins/detail?plugin_id=demo-plugin"));
    assert_eq!(server.take_request().path, "/v1/plugins/set-enabled");
    assert_eq!(server.take_request().path, "/v1/plugins/delete");
    server.join();
}

#[tokio::test]
async fn hooks_client_uses_typed_contracts() {
    let hook = json!({
        "plugin_id": "demo-plugin",
        "event": "before_tool_call",
        "command": "/bin/sh",
        "args": ["-c", "exit 0"],
        "working_dir": "/tmp/demo-plugin",
        "mode": "blocking",
        "hook_hash": "abc123",
        "trusted": false
    });
    let audit = json!({
        "timestamp": "2026-08-17T00:00:00Z",
        "tool_name": "shell",
        "duration_ms": 12,
        "success": true
    });
    let server = test_support::server(vec![
        (200, json!([hook])),
        (200, json!({"plugin_id": "demo-plugin", "hook_hash": "abc123", "trusted": true})),
        (200, json!({"plugin_id": "demo-plugin", "hook_hash": "abc123", "trusted": false})),
        (200, json!([audit])),
    ]);

    let hooks = server.client().hooks_get().await.expect("hooks");
    assert_eq!(hooks[0].plugin_id, "demo-plugin");
    assert!(!hooks[0].trusted);
    assert!(server.client().hook_trust("demo-plugin", "abc123").await.expect("trust").trusted);
    assert!(!server.client().hook_revoke("demo-plugin", "abc123").await.expect("revoke").trusted);
    let audit = server.client().hook_audit_get().await.expect("audit");
    assert_eq!(audit[0].tool_name, "shell");
    assert_eq!(audit[0].duration_ms, 12);

    assert_eq!(server.take_request().path, "/v1/hooks");
    let trust = server.take_request();
    assert_eq!(trust.path, "/v1/hooks/trust");
    assert_eq!(trust.body, json!({"plugin_id": "demo-plugin", "hook_hash": "abc123"}));
    let revoke = server.take_request();
    assert_eq!(revoke.path, "/v1/hooks/revoke");
    assert_eq!(revoke.body, json!({"plugin_id": "demo-plugin", "hook_hash": "abc123"}));
    assert_eq!(server.take_request().path, "/v1/hooks/audit");
    server.join();
}
