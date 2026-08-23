//! `api session-content` 内容、UI 与状态路由测试。

use std::sync::mpsc::TryRecvError;

use serde_json::{Value, json};

use super::session_tests::{assert_request, run_command, spawn_server};

#[tokio::test(flavor = "current_thread")]
async fn api_session_content_commands_cover_message_ui_archive_and_scope_routes() {
    let meta = json!({"id":"s/one","title":"One","updated_ms":2,"message_count":0,"call_count":0});
    let ui = json!({"id":"s/one","title":"One","messages":[],"message_ids":[],"calls":[],"steps":[],"created_ms":1,"updated_ms":2});
    let responses = vec![
        json!([{"id":"m/one"}]), json!({"id":"m/one","parts":[]}),
        json!([meta.clone()]), json!(["s/old"]), json!("scope-a"),
        ui.clone(), meta, json!("/repo/session.json"), ui.clone(),
        json!({"secret":"raw-only"}), json!({"type":"text"}), json!(true),
        json!(true), json!(true), json!(true),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path"];
    let part = br#"{"type":"text"}"#;
    let archived = br#"["s/old"]"#;
    let scope = br#"{"scope":"scope-a"}"#;
    let ui_input = ui.to_string();
    let dangerous = [
        (&["session-content", "part-update", "s/one", "m/one", "p/one", "--input", "-"][..], part.as_slice()),
        (&["session-content", "part-delete", "s/one", "m/one", "p/one"][..], b"".as_slice()),
        (&["session-content", "archived-put", "--input", "-"][..], archived.as_slice()),
        (&["session-content", "scope-put", "--input", "-"][..], scope.as_slice()),
        (&["session-content", "ui-save", "s/one", "--input", "-"][..], ui_input.as_bytes()),
    ];
    for (command, input) in dangerous {
        let error = run_command(&common, command, input).await.expect_err("mutation requires --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let reads = [
        (&["session-content", "messages", "s/one", "--limit", "7"][..], "/v1/session/s%2Fone/message?directory=%2Frepo+path&limit=7"),
        (&["session-content", "message", "s/one", "m/one"][..], "/v1/session/s%2Fone/message/m%2Fone?directory=%2Frepo+path"),
        (&["session-content", "ui-previews"][..], "/v1/session/ui-previews?directory=%2Frepo+path"),
        (&["session-content", "archived-get"][..], "/v1/session/archived?directory=%2Frepo+path"),
        (&["session-content", "scope-get"][..], "/v1/session/scope"),
        (&["session-content", "ui-get", "s/one"][..], "/v1/session/s%2Fone/ui?directory=%2Frepo+path"),
        (&["session-content", "preview", "s/one"][..], "/v1/session/s%2Fone/preview?directory=%2Frepo+path"),
        (&["session-content", "path", "s/one"][..], "/v1/session/s%2Fone/path?directory=%2Frepo+path"),
        (&["session-content", "any", "s/one"][..], "/v1/session/s%2Fone/any"),
        (&["session-content", "raw", "s/one", "t/one", "3"][..], "/v1/session/s%2Fone/turn/t%2Fone/step/3/llm-raw?directory=%2Frepo+path"),
    ];
    for (index, (command, path)) in reads.into_iter().enumerate() {
        let output = run_command(&common, command, b"").await.expect("read");
        if index == 5 { assert!(!String::from_utf8_lossy(&output).contains("raw-only")); }
        if index == 9 { assert!(String::from_utf8_lossy(&output).contains("raw-only")); }
        assert_request(&requests, "GET", path, Value::Null);
    }

    let yes_common = ["--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path", "--yes"];
    run_command(&yes_common, &["session-content", "part-update", "s/one", "m/one", "p/one", "--input", "-"], part).await.unwrap();
    assert_request(&requests, "PATCH", "/v1/session/s%2Fone/message/m%2Fone/part/p%2Fone?directory=%2Frepo+path", json!({"type":"text"}));
    run_command(&yes_common, &["session-content", "part-delete", "s/one", "m/one", "p/one"], b"").await.unwrap();
    assert_request(&requests, "DELETE", "/v1/session/s%2Fone/message/m%2Fone/part/p%2Fone?directory=%2Frepo+path", json!({}));
    run_command(&yes_common, &["session-content", "archived-put", "--input", "-"], archived).await.unwrap();
    assert_request(&requests, "PUT", "/v1/session/archived?directory=%2Frepo+path", json!(["s/old"]));
    run_command(&yes_common, &["session-content", "scope-put", "--input", "-"], scope).await.unwrap();
    assert_request(&requests, "PUT", "/v1/session/scope", json!({"scope":"scope-a"}));
    run_command(&yes_common, &["session-content", "ui-save", "s/one", "--input", "-"], ui_input.as_bytes()).await.unwrap();
    assert_request(&requests, "PUT", "/v1/session/s%2Fone/ui?directory=%2Frepo+path", ui);
}
