use serde_json::json;

use super::{PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto};
use crate::client::test_support;

#[test]
fn pending_permission_request_deserializes_complete_payload() {
    let request: PendingPermissionRequestDto = serde_json::from_value(json!({
        "id": "request-1",
        "session_id": "session-1",
        "permission": "shell",
        "patterns": ["cargo metadata", "cargo tree"],
        "metadata": {
            "reason": "inspect dependencies",
            "arguments": {"depth": 2}
        },
        "always": ["cargo metadata"],
        "tool": {
            "message_id": "message-1",
            "call_id": "call-1"
        }
    }))
    .expect("complete permission request");

    assert_eq!(request.id, "request-1");
    assert_eq!(request.session_id, "session-1");
    assert_eq!(request.permission, "shell");
    assert_eq!(request.patterns, vec!["cargo metadata", "cargo tree"]);
    assert_eq!(request.metadata["reason"], json!("inspect dependencies"));
    assert_eq!(request.metadata["arguments"], json!({"depth": 2}));
    assert_eq!(request.always, vec!["cargo metadata"]);
    assert_eq!(
        request.tool,
        Some(PendingPermissionToolDto {
            message_id: "message-1".to_string(),
            call_id: "call-1".to_string(),
        })
    );
}

#[test]
fn pending_permission_request_defaults_optional_collections_and_omits_absent_tool() {
    let request: PendingPermissionRequestDto = serde_json::from_value(json!({
        "id": "request-2",
        "session_id": "session-2",
        "permission": "read"
    }))
    .expect("minimal permission request");

    assert!(request.patterns.is_empty());
    assert!(request.metadata.is_empty());
    assert!(request.always.is_empty());
    assert_eq!(request.tool, None);
    assert_eq!(
        serde_json::to_value(request).expect("serialized permission request"),
        json!({
            "id": "request-2",
            "session_id": "session-2",
            "permission": "read",
            "patterns": [],
            "metadata": {},
            "always": []
        })
    );
}

#[test]
fn pending_permission_reply_uses_lowercase_wire_values() {
    for (reply, wire_value) in [
        (PendingPermissionReplyDto::Once, "once"),
        (PendingPermissionReplyDto::Always, "always"),
        (PendingPermissionReplyDto::Reject, "reject"),
    ] {
        assert_eq!(serde_json::to_value(reply).expect("serialized reply"), json!(wire_value));
        assert_eq!(
            serde_json::from_value::<PendingPermissionReplyDto>(json!(wire_value))
                .expect("deserialized reply"),
            reply
        );
    }
}

#[tokio::test]
async fn permission_list_deserializes_requests_and_scopes_directory() {
    let server = test_support::server(vec![(
        200,
        json!([{
            "id": "request-3",
            "session_id": "session-3",
            "permission": "edit",
            "patterns": ["src/**/*.rs"],
            "metadata": {"reason": "update implementation"},
            "always": ["src/**/*.rs"],
            "tool": {"message_id": "message-3", "call_id": "call-3"}
        }]),
    )]);

    let requests =
        server.client().permission_list(Some("/tmp/project")).await.expect("permission list");

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].id, "request-3");
    assert_eq!(requests[0].patterns, vec!["src/**/*.rs"]);
    assert_eq!(requests[0].metadata["reason"], json!("update implementation"));
    assert_eq!(
        requests[0].tool,
        Some(PendingPermissionToolDto {
            message_id: "message-3".to_string(),
            call_id: "call-3".to_string(),
        })
    );
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/permission?directory=%2Ftmp%2Fproject");
    assert_eq!(request.body, serde_json::Value::Null);
    server.join();
}

#[tokio::test]
async fn permission_list_omits_directory_query_when_directory_is_none() {
    let server = test_support::server(vec![(200, json!([]))]);

    let requests = server.client().permission_list(None).await.expect("permission list");

    assert!(requests.is_empty());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/permission");
    server.join();
}

#[tokio::test]
async fn permission_list_propagates_non_success_status() {
    let server = test_support::server(vec![(404, json!({"error": "missing"}))]);

    let error = server.client().permission_list(None).await.expect_err("permission list error");

    assert!(!error.is_empty(), "error message should not be empty: {error}");
    assert_eq!(server.take_request().path, "/v1/permission");
    server.join();
}

#[tokio::test]
async fn permission_reply_serializes_variants_and_returns_server_boolean() {
    let server =
        test_support::server(vec![(200, json!(true)), (200, json!(false)), (200, json!(true))]);

    assert!(
        server
            .client()
            .permission_reply(
                "request-once",
                PendingPermissionReplyDto::Once,
                Some("/tmp/project"),
                Some("allow this operation".to_string()),
            )
            .await
            .expect("once reply")
    );
    assert!(
        !server
            .client()
            .permission_reply("request-always", PendingPermissionReplyDto::Always, None, None)
            .await
            .expect("always reply")
    );
    assert!(
        server
            .client()
            .permission_reply("request-reject", PendingPermissionReplyDto::Reject, None, None)
            .await
            .expect("reject reply")
    );

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/permission/request-once/reply?directory=%2Ftmp%2Fproject");
    assert_eq!(request.body, json!({"reply": "once", "message": "allow this operation"}));

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/permission/request-always/reply");
    assert_eq!(request.body, json!({"reply": "always", "message": null}));

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/permission/request-reject/reply");
    assert_eq!(request.body, json!({"reply": "reject", "message": null}));
    server.join();
}

#[tokio::test]
async fn permission_reply_propagates_non_success_status() {
    let server = test_support::server(vec![(404, json!({"error": "request not found"}))]);

    let error = server
        .client()
        .permission_reply("missing", PendingPermissionReplyDto::Reject, None, None)
        .await
        .expect_err("permission reply error");

    assert!(!error.is_empty(), "error message should not be empty: {error}");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/permission/missing/reply");
    assert_eq!(request.body, json!({"reply": "reject", "message": null}));
    server.join();
}
