//! 桌面设置兼容门面的线协议覆盖。
use super::super::test_support;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Settings {
    #[allow(dead_code)]
    theme: String,
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
