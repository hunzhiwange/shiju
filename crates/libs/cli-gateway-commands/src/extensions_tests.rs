//! `extensions` skills/plugins 管理路由测试。

use std::sync::mpsc::TryRecvError;

use serde_json::{Value, json};

use super::session_tests::{assert_request, run_command, spawn_server};

#[tokio::test(flavor = "current_thread")]
async fn api_extension_commands_cover_skill_and_plugin_management_routes() {
    let (port, requests) = spawn_server(vec![Value::Null; 10]);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path"];
    for command in [
        &["extensions", "skill-create"][..],
        &["extensions", "skill-install", "rust"][..],
        &["extensions", "skill-enable", "rust", "--enabled", "true"][..],
        &["extensions", "skill-delete", "rust"][..],
        &["extensions", "plugin-enable", "demo", "--enabled", "false"][..],
        &["extensions", "plugin-delete", "demo"][..],
    ] {
        let error = run_command(&common, command, b"").await.expect_err("mutation requires --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let _ = run_command(&common, &["extensions", "skills"], b"").await;
    assert_request(&requests, "GET", "/v1/skills?project_path=%2Frepo+path", Value::Null);
    let _ = run_command(&common, &["extensions", "skill-detail", "rust"], b"").await;
    assert_request(
        &requests,
        "GET",
        "/v1/skills/detail?skill_id=rust&project_path=%2Frepo+path",
        Value::Null,
    );
    let yes =
        ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path", "--yes"];
    let mutations = [
        (
            &["extensions", "skill-create"][..],
            "/v1/skills/create",
            json!({"project_path":"/repo path"}),
        ),
        (
            &["extensions", "skill-install", "rust"][..],
            "/v1/skills/install-built-in",
            json!({"project_path":"/repo path","skill_id":"rust"}),
        ),
        (
            &["extensions", "skill-enable", "rust", "--enabled", "true"][..],
            "/v1/skills/set-enabled",
            json!({"project_path":"/repo path","skill_id":"rust","enabled":true}),
        ),
        (
            &["extensions", "skill-delete", "rust"][..],
            "/v1/skills/delete",
            json!({"project_path":"/repo path","skill_id":"rust"}),
        ),
    ];
    for (command, path, body) in mutations {
        let _ = run_command(&yes, command, b"").await;
        assert_request(&requests, "POST", path, body);
    }
    let _ = run_command(&common, &["extensions", "plugins"], b"").await;
    assert_request(&requests, "GET", "/v1/plugins", Value::Null);
    let _ = run_command(&common, &["extensions", "plugin-detail", "demo"], b"").await;
    assert_request(&requests, "GET", "/v1/plugins/detail?plugin_id=demo", Value::Null);
    let _ = run_command(&yes, &["extensions", "plugin-enable", "demo", "--enabled", "false"], b"")
        .await;
    assert_request(
        &requests,
        "POST",
        "/v1/plugins/set-enabled",
        json!({"plugin_id":"demo","enabled":false}),
    );
    let _ = run_command(&yes, &["extensions", "plugin-delete", "demo"], b"").await;
    assert_request(&requests, "POST", "/v1/plugins/delete", json!({"plugin_id":"demo"}));
}
