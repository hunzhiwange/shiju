use serde_json::json;
use vw_api_types::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageListQuery, GatewaySessionPatchBody, GatewaySessionPatchTime,
    GatewaySessionResetBody, GatewaySessionSummarizeBody, GatewaySessionTitleGenerateBody,
    GatewaySessionTodoItem, GatewaySessionTodoPutBody,
};
use vw_api_types::todo::{TodoPriority, TodoStatus};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};

use super::{session_diff_query, session_message_list_query};
use crate::client::test_support;

fn chat_session() -> ChatSession {
    ChatSession {
        id: "s1".to_string(),
        title: "Session".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "hello".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("m1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

#[test]
fn session_message_list_query_omits_blank_directory_and_keeps_zero_limit() {
    let pairs = session_message_list_query(&GatewaySessionMessageListQuery {
        directory: Some("  \t".to_string()),
        limit: Some(0),
    });

    assert_eq!(pairs, vec![("limit".to_string(), "0".to_string())]);
    assert!(
        session_message_list_query(&GatewaySessionMessageListQuery {
            directory: None,
            limit: None,
        })
        .is_empty()
    );
}

#[test]
fn session_diff_query_omits_blank_ids_and_keeps_non_blank_id() {
    let blank = session_diff_query(&GatewaySessionDiffQuery {
        directory: Some("  ".to_string()),
        message_id: Some("\n\t".to_string()),
    });
    let populated = session_diff_query(&GatewaySessionDiffQuery {
        directory: None,
        message_id: Some("  m1  ".to_string()),
    });

    assert!(blank.is_empty());
    assert_eq!(populated, vec![("messageID".to_string(), "  m1  ".to_string())]);
}

#[tokio::test]
async fn session_create_and_fork_send_null_for_absent_optional_bodies() {
    let server =
        test_support::server(vec![(200, json!({"id": "created"})), (200, json!({"id": "forked"}))]);

    let created: serde_json::Value =
        server.client().session_create("/repo", &None).await.expect("create");
    let forked: serde_json::Value =
        server.client().session_fork("s1", None, &None).await.expect("fork");

    assert_eq!(created["id"], "created");
    assert_eq!(forked["id"], "forked");
    let create_request = server.take_request();
    assert_eq!(create_request.method, "POST");
    assert_eq!(create_request.path, "/v1/session?directory=%2Frepo");
    assert_eq!(create_request.body, serde_json::Value::Null);
    let fork_request = server.take_request();
    assert_eq!(fork_request.method, "POST");
    assert_eq!(fork_request.path, "/v1/session/s1/fork");
    assert_eq!(fork_request.body, serde_json::Value::Null);
    server.join();
}

#[tokio::test]
async fn session_optional_reads_distinguish_not_found_from_server_errors() {
    let server = test_support::server(vec![
        (404, json!({"message": "missing ui"})),
        (404, json!({"message": "missing raw payload"})),
        (500, json!({"message": "gateway unavailable"})),
    ]);

    assert!(
        server
            .client()
            .session_ui_get("missing", Some("/repo"))
            .await
            .expect("missing UI is not an error")
            .is_none()
    );
    assert!(
        server
            .client()
            .session_step_llm_raw_get("missing", "turn-missing", 0, None)
            .await
            .expect("missing raw payload is not an error")
            .is_none()
    );
    let error = server
        .client()
        .session_ui_get("failed", None)
        .await
        .expect_err("server error must be surfaced");

    assert!(error.contains("gateway request failed: 500"));
    assert_eq!(server.take_request().path, "/v1/session/missing/ui?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/missing/turn/turn-missing/step/0/llm-raw");
    assert_eq!(server.take_request().path, "/v1/session/failed/ui");
    server.join();
}

#[tokio::test]
async fn session_step_llm_raw_get_reads_only_requested_step() {
    let server = test_support::server(vec![(
        200,
        json!({"messages": [{"role": "user", "content": "requested step"}]}),
    )]);

    let response = server
        .client()
        .session_step_llm_raw_get("requested-session", "turn-17", 17, Some("/repo path"))
        .await;
    let request = server.take_request();

    assert_eq!(request.method, "GET");
    assert_eq!(
        request.path,
        "/v1/session/requested-session/turn/turn-17/step/17/llm-raw?directory=%2Frepo+path"
    );
    let payload = response.expect("raw payload request").expect("raw payload");
    assert_eq!(payload["messages"][0]["content"], "requested step");
    server.join();
}

#[tokio::test]
async fn session_step_llm_raw_get_distinguishes_null_not_found_and_server_errors() {
    let server = test_support::server(vec![
        (200, json!(null)),
        (404, json!({"message": "missing raw payload"})),
        (500, json!({"message": "storage failed"})),
    ]);

    assert!(
        server
            .client()
            .session_step_llm_raw_get("s1", "turn-a", 1, None)
            .await
            .expect("JSON null should be an absent payload")
            .is_none()
    );
    assert!(
        server
            .client()
            .session_step_llm_raw_get("s1", "turn-b", 2, None)
            .await
            .expect("404 should be an absent payload")
            .is_none()
    );
    let error = server
        .client()
        .session_step_llm_raw_get("s1", "turn-c", 3, None)
        .await
        .expect_err("storage failure should be surfaced");

    assert!(error.contains("gateway request failed: 500"));
    assert_eq!(server.take_request().path, "/v1/session/s1/turn/turn-a/step/1/llm-raw");
    assert_eq!(server.take_request().path, "/v1/session/s1/turn/turn-b/step/2/llm-raw");
    assert_eq!(server.take_request().path, "/v1/session/s1/turn/turn-c/step/3/llm-raw");
    server.join();
}

#[tokio::test]
async fn session_step_llm_raw_get_normalizes_legacy_turn_ids() {
    let server = test_support::server(vec![(200, json!(null)), (200, json!(null))]);

    server
        .client()
        .session_step_llm_raw_get("s1", "  ", 4, None)
        .await
        .expect("blank turn id should use legacy lookup");
    server
        .client()
        .session_step_llm_raw_get("s1", "legacy-step-5", 5, None)
        .await
        .expect("legacy cache key should use legacy lookup");

    assert_eq!(server.take_request().path, "/v1/session/s1/turn/legacy/step/4/llm-raw");
    assert_eq!(server.take_request().path, "/v1/session/s1/turn/legacy/step/5/llm-raw");
    server.join();
}

#[tokio::test]
async fn session_ui_get_with_raw_preserves_step_payloads() {
    let server = test_support::server(vec![(
        200,
        json!({
            "id": "s1",
            "title": "Session",
            "llm_raw_step_indices": [1, 7, 9],
            "steps": [
                {
                    "index": 1,
                    "turn_id": "turn-a",
                    "started_ms": 10,
                    "llm_raw": {"messages": [{"role": "user", "content": "hello"}]}
                },
                {"index": 2, "turn_id": "turn-b", "started_ms": 20},
                {
                    "index": 7,
                    "turn_id": "  ",
                    "started_ms": 30,
                    "llm_raw": {"messages": [{"role": "user", "content": "legacy"}]}
                }
            ]
        }),
    )]);

    let loaded = server
        .client()
        .session_ui_get_with_raw("s1", Some("/repo"))
        .await
        .expect("load session with raw")
        .expect("session exists");

    assert_eq!(loaded.session.id, "s1");
    assert_eq!(loaded.session.steps.len(), 3);
    assert_eq!(
        loaded.step_llm_raw[&("s1".to_string(), "turn-a".to_string(), 1)]["messages"][0]["content"],
        "hello"
    );
    assert!(!loaded.step_llm_raw.contains_key(&("s1".to_string(), "turn-b".to_string(), 2)));
    assert_eq!(
        loaded.step_llm_raw[&("s1".to_string(), "legacy-step-7".to_string(), 7)]["messages"][0]["content"],
        "legacy"
    );
    assert_eq!(loaded.session.steps[2].turn_id, None);
    assert_eq!(
        loaded.llm_raw_step_indices,
        vec![
            ("s1".to_string(), "turn-a".to_string(), 1),
            ("s1".to_string(), "legacy-step-7".to_string(), 7),
        ]
    );
    assert_eq!(server.take_request().path, "/v1/session/s1/ui?directory=%2Frepo");
    server.join();
}

#[tokio::test]
async fn session_archived_and_scope_updates_serialize_empty_values() {
    let server = test_support::server(vec![(200, json!(true)), (200, json!(true))]);

    assert!(server.client().session_archived_put(None, &[]).await.expect("clear archived"));
    assert!(server.client().session_scope_put(None).await.expect("clear scope"));

    let archived_request = server.take_request();
    assert_eq!(archived_request.method, "PUT");
    assert_eq!(archived_request.path, "/v1/session/archived");
    assert_eq!(archived_request.body, json!([]));
    let scope_request = server.take_request();
    assert_eq!(scope_request.method, "PUT");
    assert_eq!(scope_request.path, "/v1/session/scope");
    assert_eq!(scope_request.body, json!({}));
    server.join();
}

#[tokio::test]
async fn session_list_surfaces_non_success_status() {
    let server = test_support::server(vec![(500, json!({"message": "gateway unavailable"}))]);

    let error =
        server.client().session_list::<serde_json::Value>(None).await.expect_err("list error");

    assert!(error.contains("gateway request failed: 500"));
    assert_eq!(server.take_request().path, "/v1/session");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn session_todo_get_blocking_surfaces_non_success_status() {
    let server = test_support::server(vec![(500, json!({"message": "gateway unavailable"}))]);

    let error =
        server.client().session_todo_get_blocking("s1", None).expect_err("blocking todo error");

    assert!(error.contains("gateway request failed: 500"));
    assert_eq!(server.take_request().path, "/v1/session/s1/todo");
    server.join();
}

#[tokio::test]
async fn session_api_routes_lifecycle_messages_ui_scope_and_title_calls() {
    let server = test_support::server(vec![
        (200, json!({"items": [{"id": "s1"}]})),
        (200, json!({"id": "s1", "title": "Session"})),
        (200, json!([{"id": "m1"}])),
        (200, json!([{"id": "m2"}])),
        (200, json!({"id": "s2"})),
        (200, json!({"id": "s1", "title": "Renamed"})),
        (204, json!(null)),
        (200, json!({"id": "forked"})),
        (200, json!({"id": "s1", "reset": true})),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!([{"content": "Do it", "status": "pending", "priority": "high", "id": 7}])),
        (200, json!({"files": []})),
        (200, json!({"files": []})),
        (200, serde_json::to_value(chat_session()).expect("chat session json")),
        (404, json!({"missing": true})),
        (200, json!(true)),
        (
            200,
            json!([{
                "id": "s1",
                "title": "Session",
                "updated_ms": 2,
                "message_count": 1,
                "call_count": 0,
                "last_content": "hello"
            }]),
        ),
        (404, json!({"missing": true})),
        (404, json!({"missing": true})),
        (200, json!({"messages": [{"role": "User", "content": "hello"}]})),
        (200, json!(["s-old"])),
        (200, json!(true)),
        (200, json!("global-scope")),
        (200, json!(true)),
        (200, json!({"title": "Generated"})),
    ]);

    let list: serde_json::Value = server.client().session_list(Some("/repo")).await.expect("list");
    assert_eq!(list["items"][0]["id"], "s1");
    let get: serde_json::Value =
        server.client().session_get("s1", Some("/repo")).await.expect("get");
    assert_eq!(get["title"], "Session");
    let messages: serde_json::Value =
        server.client().session_messages("s1", Some("/repo")).await.expect("messages");
    assert_eq!(messages[0]["id"], "m1");
    let queried: serde_json::Value = server
        .client()
        .session_messages_query(
            "s1",
            &GatewaySessionMessageListQuery {
                directory: Some("/repo".to_string()),
                limit: Some(25),
            },
        )
        .await
        .expect("messages query");
    assert_eq!(queried[0]["id"], "m2");
    let created: serde_json::Value = server
        .client()
        .session_create(
            "/repo",
            &Some(GatewaySessionCreateBody {
                parent_id: Some("parent".to_string()),
                title: Some("New".to_string()),
            }),
        )
        .await
        .expect("create");
    assert_eq!(created["id"], "s2");
    let updated: serde_json::Value = server
        .client()
        .session_update(
            "s1",
            Some("/repo"),
            &GatewaySessionPatchBody {
                title: Some("Renamed".to_string()),
                time: Some(GatewaySessionPatchTime { archived: Some(123) }),
            },
        )
        .await
        .expect("update");
    assert_eq!(updated["title"], "Renamed");
    server.client().session_delete("s1", Some("/repo")).await.expect("delete");
    let forked: serde_json::Value = server
        .client()
        .session_fork(
            "s1",
            Some("/repo"),
            &Some(GatewaySessionForkBody { message_id: Some("m1".to_string()) }),
        )
        .await
        .expect("fork");
    assert_eq!(forked["id"], "forked");
    let reset: serde_json::Value = server
        .client()
        .session_reset(
            "s1",
            Some("/repo"),
            &GatewaySessionResetBody { message_id: "m1".to_string(), revert_code: true },
        )
        .await
        .expect("reset");
    assert!(reset["reset"].as_bool().unwrap());
    assert!(
        server
            .client()
            .session_summarize(
                "s1",
                Some("/repo"),
                &GatewaySessionSummarizeBody { message_id: "m1".to_string() },
            )
            .await
            .expect("summarize")
    );
    assert!(
        server
            .client()
            .session_todo_update(
                "s1",
                Some("/repo"),
                &GatewaySessionTodoPutBody {
                    todos: vec![GatewaySessionTodoItem {
                        id: "todo-1".to_string(),
                        content: "Do it".to_string(),
                        status: TodoStatus::Pending,
                        priority: TodoPriority::High,
                    }],
                },
            )
            .await
            .expect("todo update")
    );
    let todos = server.client().session_todo_get("s1", Some("/repo")).await.expect("todos");
    assert_eq!(todos[0].id, "7");
    let diff: serde_json::Value = server
        .client()
        .session_diff(
            "s1",
            &GatewaySessionDiffQuery {
                directory: Some("/repo".to_string()),
                message_id: Some("m1".to_string()),
            },
        )
        .await
        .expect("diff");
    assert_eq!(diff["files"], json!([]));
    let diff_without_message: serde_json::Value = server
        .client()
        .session_diff(
            "s1",
            &GatewaySessionDiffQuery { directory: None, message_id: Some("   ".to_string()) },
        )
        .await
        .expect("diff without message");
    assert_eq!(diff_without_message["files"], json!([]));
    assert_eq!(
        server.client().session_ui_get("s1", Some("/repo")).await.expect("ui get").expect("ui").id,
        "s1"
    );
    assert!(server.client().session_ui_get_any("missing").await.expect("ui any").is_none());
    assert!(
        server
            .client()
            .session_ui_save("s1", Some("/repo"), &chat_session())
            .await
            .expect("ui save")
    );
    assert_eq!(
        server.client().session_ui_previews(Some("/repo")).await.expect("previews")[0].last_content,
        Some("hello".to_string())
    );
    assert!(
        server
            .client()
            .session_preview_meta_get("missing", Some("/repo"))
            .await
            .expect("preview")
            .is_none()
    );
    assert!(
        server.client().session_path_get("missing", Some("/repo")).await.expect("path").is_none()
    );
    let raw_payload = server
        .client()
        .session_step_llm_raw_get("s1", "turn-2", 2, Some("/repo"))
        .await
        .expect("raw payload")
        .expect("raw payload");
    assert_eq!(raw_payload["messages"][0]["content"], "hello");
    assert_eq!(
        server.client().session_archived_get(Some("/repo")).await.expect("archived"),
        vec!["s-old".to_string()]
    );
    assert!(
        server
            .client()
            .session_archived_put(Some("/repo"), &["s-old".to_string()])
            .await
            .expect("archived put")
    );
    assert_eq!(
        server.client().session_scope_get(Some("/ignored"), Some("project")).await.expect("scope"),
        Some("global-scope".to_string())
    );
    assert!(server.client().session_scope_put(Some("global-scope")).await.expect("scope put"));
    assert_eq!(
        server
            .client()
            .session_title_generate(
                "s1",
                &GatewaySessionTitleGenerateBody {
                    content: "hello world".to_string(),
                    preferred_model: Some("gpt".to_string()),
                    acp_agent: None,
                },
            )
            .await
            .expect("title")
            .title,
        "Generated"
    );

    assert_eq!(server.take_request().path, "/v1/session?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/s1?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/s1/message?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/s1/message?directory=%2Frepo&limit=25");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session?directory=%2Frepo");
    assert_eq!(request.body["parentID"], "parent");
    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/session/s1?directory=%2Frepo");
    assert_eq!(request.body["time"]["archived"], 123);
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/session/s1?directory=%2Frepo");
    let request = server.take_request();
    assert_eq!(request.path, "/v1/session/s1/fork?directory=%2Frepo");
    assert_eq!(request.body["messageID"], "m1");
    let request = server.take_request();
    assert_eq!(request.path, "/v1/session/s1/reset?directory=%2Frepo");
    assert_eq!(request.body["revertCode"], true);
    assert_eq!(server.take_request().path, "/v1/session/s1/summarize?directory=%2Frepo");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/s1/todo?directory=%2Frepo");
    assert_eq!(request.body["todos"][0]["priority"], "high");
    assert_eq!(server.take_request().path, "/v1/session/s1/todo?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/s1/diff?directory=%2Frepo&messageID=m1");
    assert_eq!(server.take_request().path, "/v1/session/s1/diff");
    assert_eq!(server.take_request().path, "/v1/session/s1/ui?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/missing/any");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/s1/ui?directory=%2Frepo");
    assert_eq!(request.body["messages"][0]["content"], "hello");
    assert_eq!(server.take_request().path, "/v1/session/ui-previews?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/missing/preview?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/missing/path?directory=%2Frepo");
    assert_eq!(
        server.take_request().path,
        "/v1/session/s1/turn/turn-2/step/2/llm-raw?directory=%2Frepo"
    );
    assert_eq!(server.take_request().path, "/v1/session/archived?directory=%2Frepo");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/archived?directory=%2Frepo");
    assert_eq!(request.body, json!(["s-old"]));
    assert_eq!(server.take_request().path, "/v1/session/scope");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/scope");
    assert_eq!(request.body["scope"], "global-scope");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session/s1/title");
    assert_eq!(request.body["preferred_model"], "gpt");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn session_todo_get_blocking_uses_same_route_and_directory_query() {
    let tenant = vw_api_types::id::TenantScope::new("platform-session", "company-session")
        .expect("tenant scope");
    let server = test_support::server_with_tenant_scope(
        vec![(
            200,
            json!([{"content": "Blocking", "status": "done", "priority": "low", "id": "todo"}]),
        )],
        tenant,
    );

    let todos =
        server.client().session_todo_get_blocking("s1", Some("/repo")).expect("blocking todos");

    assert_eq!(todos[0].content, "Blocking");
    let request = server.take_request();
    assert_eq!(request.path, "/v1/session/s1/todo?directory=%2Frepo");
    assert_eq!(request.header(vw_api_types::id::PLATFORM_ID_HEADER), Some("platform-session"));
    assert_eq!(request.header(vw_api_types::id::COMPANY_ID_HEADER), Some("company-session"));
    server.join();
}
