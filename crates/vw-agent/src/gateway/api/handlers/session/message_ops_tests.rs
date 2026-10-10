// 覆盖会话消息及内容片段的查询、更新和删除边界。

use super::*;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::project;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use std::path::Path as FsPath;
use std::sync::Once;
use tempfile::TempDir;
use uuid::Uuid;
use vw_api_types::session::GatewaySessionMessageListQuery;

static TEST_HOME: Once = Once::new();
static MESSAGE_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir = std::env::temp_dir().join(format!("vw-message-ops-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

fn query_for(directory: &FsPath) -> InstanceQuery {
    InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) }
}

fn headers_for(directory: &FsPath) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        HeaderValue::from_str(&directory.to_string_lossy()).expect("valid directory header"),
    );
    headers
}

fn unique_session_id(label: &str) -> String {
    format!("message-ops-{label}-{}", Uuid::new_v4())
}

fn model_ref() -> agent_session::message::ModelRef {
    agent_session::message::ModelRef {
        provider_id: "test-provider".to_string(),
        model_id: "test-model".to_string(),
    }
}

fn user_info(session_id: &str, id: &str) -> agent_session::message::Info {
    agent_session::message::Info::User(Box::new(agent_session::message::UserInfo {
        id: id.to_string(),
        session_id: session_id.to_string(),
        time: agent_session::message::UserTime { created: 10 },
        summary: None,
        agent: "agent".to_string(),
        model: model_ref(),
        system: None,
        tools: None,
        variant: None,
    }))
}

fn assistant_info(session_id: &str, id: &str, parent_id: &str) -> agent_session::message::Info {
    agent_session::message::Info::Assistant(Box::new(agent_session::message::AssistantInfo {
        id: id.to_string(),
        session_id: session_id.to_string(),
        time: agent_session::message::AssistantTime {
            created: 11,
            completed: Some(12),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        },
        error: None,
        parent_id: parent_id.to_string(),
        model_id: "test-model".to_string(),
        provider_id: "test-provider".to_string(),
        mode: "chat".to_string(),
        agent: "agent".to_string(),
        path: agent_session::message::PathInfo {
            cwd: "/tmp".to_string(),
            root: "/tmp".to_string(),
        },
        summary: None,
        cost: 0.0,
        tokens: agent_session::message::TokenInfo {
            total: Some(3),
            input: 1,
            output: 2,
            reasoning: 0,
            cache: agent_session::message::TokenCacheInfo { read: 0, write: 0 },
        },
        variant: None,
        finish: Some("stop".to_string()),
    }))
}

fn text_part(
    session_id: &str,
    message_id: &str,
    part_id: &str,
    text: &str,
) -> agent_session::message::Part {
    agent_session::message::Part::Text(agent_session::message::TextPart {
        base: agent_session::message::PartBase {
            id: part_id.to_string(),
            session_id: session_id.to_string(),
            message_id: message_id.to_string(),
        },
        text: text.to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    })
}

fn text_content(part: &agent_session::message::Part) -> &str {
    match part {
        agent_session::message::Part::Text(part) => &part.text,
        _ => panic!("expected text part"),
    }
}

// 消息、片段和会话标识交叉组合，确保路径参数不一致时在访问存储前失败。
#[test]
fn message_handlers_are_available() {
    let _ = session_message_list;
    let _ = session_message_get;
    let _ = session_message_part_delete;
    let _ = session_message_part_patch;
}

// 错误用例固定失败发生在持久化之前，避免跨会话或跨作用域修改已有数据。
#[tokio::test]
async fn session_message_part_patch_rejects_body_path_mismatch_before_storage_access() {
    let part = agent_session::message::Part::Text(agent_session::message::TextPart {
        base: agent_session::message::PartBase {
            id: "prt-body".to_string(),
            session_id: "ses-body".to_string(),
            message_id: "msg-body".to_string(),
        },
        text: "updated".to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    });

    let error = session_message_part_patch(
        Path(("ses-path".to_string(), "msg-path".to_string(), "prt-path".to_string())),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(part),
    )
    .await
    .expect_err("mismatch should be rejected");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("part mismatch"));
    assert!(error.to_string().contains("ses-path"));
    assert!(error.to_string().contains("msg-path"));
    assert!(error.to_string().contains("prt-path"));
}

#[tokio::test]
async fn session_message_part_patch_rejects_message_mismatch_when_other_ids_match() {
    let part = text_part("ses-path", "msg-body", "prt-path", "updated");

    let error = session_message_part_patch(
        Path(("ses-path".to_string(), "msg-path".to_string(), "prt-path".to_string())),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(part),
    )
    .await
    .expect_err("message mismatch should be rejected");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("part mismatch"));
    assert!(error.to_string().contains("msg-path"));
}

#[tokio::test]
async fn session_message_part_patch_rejects_session_mismatch_when_other_ids_match() {
    let part = text_part("ses-body", "msg-path", "prt-path", "updated");

    let error = session_message_part_patch(
        Path(("ses-path".to_string(), "msg-path".to_string(), "prt-path".to_string())),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(part),
    )
    .await
    .expect_err("session mismatch should be rejected");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("part mismatch"));
    assert!(error.to_string().contains("ses-path"));
}

#[tokio::test]
async fn session_message_list_honors_zero_limit_and_unbounded_newest_first() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let session_id = unique_session_id("list-limits");
    let older_id = "001";
    let newer_id = "002";
    let older = user_info(&session_id, older_id);
    let newer = assistant_info(&session_id, newer_id, older_id);

    agent_session::message::update_message(&older).await.expect("older message should persist");
    agent_session::message::update_message(&newer).await.expect("newer message should persist");

    let Json(empty) = session_message_list(
        Path(session_id.clone()),
        Query(GatewaySessionMessageListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            limit: Some(0),
        }),
        HeaderMap::new(),
    )
    .await
    .expect("zero-limit message list should succeed");
    assert!(empty.is_empty());

    let Json(all) = session_message_list(
        Path(session_id.clone()),
        Query(GatewaySessionMessageListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("unbounded message list should succeed");
    assert_eq!(
        all.iter().map(|message| message.info.id()).collect::<Vec<_>>(),
        vec![newer_id, older_id]
    );

    agent_session::message::remove_message(&session_id, older_id).await.expect("older cleanup");
    agent_session::message::remove_message(&session_id, newer_id).await.expect("newer cleanup");
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_message_list_rejects_incomplete_history() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let session_id = unique_session_id("incomplete-history");
    let valid = user_info(&session_id, "001");
    agent_session::message::update_message(&valid).await.expect("valid message should persist");
    crate::app::agent::storage::write(
        &["message", &session_id, "002"],
        &serde_json::json!({"role": "bad", "secret_marker": "must-not-leak"}),
    )
    .await
    .expect("bad message fixture should persist");

    let error = session_message_list(
        Path(session_id.clone()),
        Query(GatewaySessionMessageListQuery {
            directory: Some(project.path().to_string_lossy().to_string()),
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect_err("unbounded message list should reject incomplete history");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(!error.to_string().contains("must-not-leak"));

    agent_session::message::remove_message(&session_id, "001").await.expect("valid cleanup");
    crate::app::agent::storage::remove(&["message", &session_id, "002"])
        .await
        .expect("bad cleanup");
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_message_list_returns_empty_for_unknown_session() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let Json(messages) = session_message_list(
        Path(unique_session_id("empty")),
        Query(GatewaySessionMessageListQuery {
            directory: Some(project.path().to_string_lossy().to_string()),
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("unknown-session message list should succeed");

    assert!(messages.is_empty());
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_message_handlers_round_trip_messages_and_parts() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let session_id = unique_session_id("round-trip");
    let older_id = "001";
    let newer_id = "002";
    let older = user_info(&session_id, older_id);
    let newer = assistant_info(&session_id, newer_id, older_id);
    let older_part = text_part(&session_id, older_id, "part-old", "first");
    let newer_part = text_part(&session_id, newer_id, "part-new", "second");

    agent_session::message::update_message(&older).await.expect("older message should persist");
    agent_session::message::update_part(&older_part).await.expect("older part should persist");
    agent_session::message::update_message(&newer).await.expect("newer message should persist");
    agent_session::message::update_part(&newer_part).await.expect("newer part should persist");

    let Json(listed) = session_message_list(
        Path(session_id.clone()),
        Query(GatewaySessionMessageListQuery { directory: None, limit: Some(1) }),
        headers_for(project_path),
    )
    .await
    .expect("message list should succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].info.id(), newer_id);
    assert_eq!(
        listed[0].parts.iter().map(agent_session::message::Part::id).collect::<Vec<_>>(),
        vec!["part-new"]
    );

    let Json(loaded) = session_message_get(
        Path((session_id.clone(), older_id.to_string())),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("message get should succeed");
    assert_eq!(loaded.info.id(), older_id);
    assert_eq!(text_content(&loaded.parts[0]), "first");

    let replacement = text_part(&session_id, older_id, "part-old", "edited");
    let Json(saved) = session_message_part_patch(
        Path((session_id.clone(), older_id.to_string(), "part-old".to_string())),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(replacement),
    )
    .await
    .expect("part patch should succeed");
    assert_eq!(saved.id(), "part-old");
    assert_eq!(text_content(&saved), "edited");

    let Json(updated) = session_message_get(
        Path((session_id.clone(), older_id.to_string())),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("updated message should load");
    assert_eq!(updated.parts.len(), 1);
    assert_eq!(text_content(&updated.parts[0]), "edited");

    let Json(deleted) = session_message_part_delete(
        Path((session_id.clone(), older_id.to_string(), "part-old".to_string())),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("part delete should succeed");
    assert!(deleted);

    let Json(after_delete) = session_message_get(
        Path((session_id.clone(), older_id.to_string())),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("message should load after part delete");
    assert!(after_delete.parts.is_empty());

    agent_session::message::remove_message(&session_id, older_id).await.expect("older cleanup");
    agent_session::message::remove_part(&session_id, newer_id, "part-new")
        .await
        .expect("newer part cleanup");
    agent_session::message::remove_message(&session_id, newer_id).await.expect("newer cleanup");
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_message_get_maps_missing_message_to_bad_request() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let error = session_message_get(
        Path((unique_session_id("missing"), "missing-message".to_string())),
        Query(query_for(project.path())),
        HeaderMap::new(),
    )
    .await
    .expect_err("missing message should fail");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.to_string().contains("Resource not found"));
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn session_message_part_delete_is_idempotent_for_missing_part() {
    let _guard = MESSAGE_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project = TempDir::new().expect("project dir");
    let Json(deleted) = session_message_part_delete(
        Path((
            unique_session_id("missing-part"),
            "missing-message".to_string(),
            "missing-part".to_string(),
        )),
        Query(query_for(project.path())),
        HeaderMap::new(),
    )
    .await
    .expect("missing part delete should be idempotent");

    assert!(deleted);
    project::instance::dispose_all().await;
}
