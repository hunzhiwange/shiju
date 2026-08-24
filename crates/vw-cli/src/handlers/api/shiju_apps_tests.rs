//! `shiju-apps` 账号、应用与敏感输出测试。

use std::sync::mpsc::TryRecvError;

use serde_json::{Value, json};

use super::session_tests::{assert_request, run_command, spawn_server};

const SECRET: &str = "SENTINEL_SHIJU_SECRET";

#[tokio::test(flavor = "current_thread")]
async fn api_shiju_account_and_app_commands_use_existing_gateway_client() {
    let responses = vec![
        json!({"state":"state-1","authorization_url":"https://auth.example/login","poll_interval_seconds":3}),
        json!({"status":"approved","credential":SECRET}),
        json!({"company_name":"Company","company_status":"enabled","name":"User","account_name":"user","user_status":"enabled"}),
        json!({"apps":[]}),
        json!({"apps":[{"installationId":"install-1","appId":"app-1","name":"App","description":null,"version":"1","source":"market","reconcileStatus":"ready","runtimeStatus":"running","runtimeUrl":SECRET}]}),
        json!({"apiKeys":[]}),
        json!({"personal":{"configured":false,"canEdit":true,"apiKeyId":null,"keyName":null,"apiKeyMask":null,"status":null,"expiresAt":null,"updatedAt":null}}),
        Value::Null,
        json!({"installationId":"install-1","name":"App","runtimeUrl":SECRET,"runtimeSetCookie":SECRET}),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    let credential = format!(r#"{{"credential":"{SECRET}"}}"#);
    let binding = format!(r#"{{"credential":"{SECRET}","apiKeyId":42}}"#);
    for (command, input) in [
        (&["shiju-apps", "binding-put", "install-1", "--input", "-"][..], binding.as_bytes()),
        (&["shiju-apps", "launch", "install-1", "--input", "-"][..], credential.as_bytes()),
    ] {
        let error = run_command(&common, command, input).await.expect_err("write requires --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        (&["shiju-apps", "login"][..], b"".as_slice(), "POST", "/v1/shiju/account/login", json!({})),
        (&["shiju-apps", "poll", "--input", "-"][..], br#"{"state":"state-1"}"#.as_slice(), "POST", "/v1/shiju/account/login/poll", json!({"state":"state-1"})),
        (&["shiju-apps", "profile", "--input", "-"][..], credential.as_bytes(), "GET", "/v1/shiju/account/profile", Value::Null),
        (&["shiju-apps", "catalog", "--input", "-"][..], credential.as_bytes(), "GET", "/v1/shiju/apps/catalog", Value::Null),
        (&["shiju-apps", "list", "--input", "-"][..], credential.as_bytes(), "GET", "/v1/shiju/apps", Value::Null),
        (&["shiju-apps", "api-keys", "--input", "-"][..], credential.as_bytes(), "GET", "/v1/shiju/apps/api-keys", Value::Null),
        (&["shiju-apps", "binding-get", "install-1", "--input", "-"][..], credential.as_bytes(), "GET", "/v1/shiju/apps/install-1/api-key-binding", Value::Null),
    ];
    for (command, input, method, path, body) in cases {
        let output = run_command(&common, command, input).await.unwrap();
        assert!(!String::from_utf8_lossy(&output).contains(SECRET));
        assert_request(&requests, method, path, body);
    }
    let yes = ["--host", "127.0.0.1", "--port", port.as_str(), "--yes"];
    let output = run_command(&yes, &["shiju-apps", "binding-put", "install-1", "--input", "-"], binding.as_bytes()).await.unwrap();
    assert!(!String::from_utf8_lossy(&output).contains(SECRET));
    assert_request(&requests, "PUT", "/v1/shiju/apps/install-1/api-key-binding", json!({"apiKeyId":42}));
    let output = run_command(&yes, &["shiju-apps", "launch", "install-1", "--input", "-"], credential.as_bytes()).await.unwrap();
    assert!(!String::from_utf8_lossy(&output).contains(SECRET));
    assert_request(&requests, "POST", "/v1/shiju/apps/install-1/launch", Value::Null);
}
