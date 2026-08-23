//! 验证待办项解析、状态归一化与会话待办更新行为。
//! 网关字段名与存储字段分开验证，未知状态和优先级采用安全默认值。

use super::*;

#[test]
fn update_input_serializes_session_id_wire_name_and_empty_todos() {
    let value = serde_json::to_value(UpdateInput {
        session_id: "session-1".to_string(),
        todos: Vec::new(),
    })
    .expect("serialize update input");

    assert_eq!(value["sessionID"], "session-1");
    assert_eq!(value["todos"], serde_json::json!([]));
    assert!(value.get("session_id").is_none());

    let decoded: UpdateInput = serde_json::from_value(value).expect("deserialize update input");
    assert_eq!(decoded.session_id, "session-1");
    assert!(decoded.todos.is_empty());
}

#[test]
fn todo_status_and_priority_parse_with_safe_defaults() {
    assert_eq!(todo_status_to_str(&TodoStatus::InProgress), "in_progress");
    assert_eq!(todo_priority_to_str(&TodoPriority::High), "high");
    assert!(matches!(todo_status_from_str("completed"), TodoStatus::Completed));
    assert!(matches!(todo_status_from_str("unknown"), TodoStatus::Pending));
    assert!(matches!(todo_priority_from_str("low"), TodoPriority::Low));
    assert!(matches!(todo_priority_from_str("unknown"), TodoPriority::Medium));
}

#[test]
fn todo_status_and_priority_cover_all_variants() {
    assert_eq!(todo_status_to_str(&TodoStatus::Pending), "pending");
    assert_eq!(todo_status_to_str(&TodoStatus::InProgress), "in_progress");
    assert_eq!(todo_status_to_str(&TodoStatus::Completed), "completed");
    assert_eq!(todo_status_to_str(&TodoStatus::Cancelled), "cancelled");

    assert_eq!(todo_priority_to_str(&TodoPriority::Low), "low");
    assert_eq!(todo_priority_to_str(&TodoPriority::Medium), "medium");
    assert_eq!(todo_priority_to_str(&TodoPriority::High), "high");

    assert!(matches!(todo_status_from_str("in_progress"), TodoStatus::InProgress));
    assert!(matches!(todo_status_from_str("cancelled"), TodoStatus::Cancelled));
    assert!(matches!(todo_priority_from_str("high"), TodoPriority::High));
}

#[test]
fn canonical_default_status_and_priority_round_trip_without_reclassification() {
    let status = todo_status_from_str("pending");
    let priority = todo_priority_from_str("medium");

    assert!(matches!(status, TodoStatus::Pending));
    assert!(matches!(priority, TodoPriority::Medium));
    assert_eq!(todo_status_to_str(&status), "pending");
    assert_eq!(todo_priority_to_str(&priority), "medium");
}

#[test]
fn storage_error_conversion_preserves_variant_and_message() {
    let error = Error::from(storage::Error::NotFound(storage::NotFoundError {
        message: "missing todo state".to_string(),
    }));

    assert_eq!(error.to_string(), "missing todo state");
    assert!(matches!(
        error,
        Error::Storage(storage::Error::NotFound(storage::NotFoundError { message }))
            if message == "missing todo state"
    ));
}

#[test]
fn json_error_conversion_preserves_variant_and_display() {
    let json_error = serde_json::from_str::<UpdateInput>("{").expect_err("invalid update JSON");
    let expected_message = json_error.to_string();

    let error = Error::from(json_error);

    assert_eq!(error.to_string(), expected_message);
    assert!(matches!(error, Error::Json(_)));
}

#[test]
fn instance_directory_is_absent_outside_project_context() {
    assert_eq!(instance_directory_opt(), None);
}

#[tokio::test]
// 写入与读取跨越工具形状和界面存储形状，往返后仍需保持网关字段契约。
async fn update_persists_ui_todos_and_get_round_trips_gateway_shape() {
    let project = tempfile::tempdir().expect("project dir");
    let session_id = format!("todo-roundtrip-{}", std::process::id());
    let todos = vec![
        Info {
            id: "one".to_string(),
            content: "write tests".to_string(),
            status: TodoStatus::InProgress,
            priority: TodoPriority::High,
        },
        Info {
            id: "two".to_string(),
            content: "ship".to_string(),
            status: TodoStatus::Completed,
            priority: TodoPriority::Low,
        },
        Info {
            id: "three".to_string(),
            content: "review".to_string(),
            status: TodoStatus::Pending,
            priority: TodoPriority::Medium,
        },
        Info {
            id: "four".to_string(),
            content: "skip".to_string(),
            status: TodoStatus::Cancelled,
            priority: TodoPriority::Medium,
        },
    ];
    let mut expected = todos.clone();
    for (index, todo) in expected.iter_mut().enumerate() {
        todo.id = (index + 1).to_string();
    }

    let loaded = crate::app::agent::project::instance::provide(project.path(), None, move || {
        Box::pin(async move {
            update(UpdateInput { session_id: session_id.clone(), todos }).await.unwrap();
            get(&session_id).await
        })
    })
    .await
    .expect("project instance");

    assert_eq!(loaded, expected);
}

#[tokio::test]
async fn gateway_get_reads_todos_written_by_tool_with_same_session_key() {
    use crate::app::agent::tools::todo::{Todo, write_session_todos_for_test};

    let project = tempfile::tempdir().expect("project dir");
    let session_id = format!("todo-tool-roundtrip-{}", std::process::id());
    let tool_todos = vec![Todo {
        id: "1".to_string(),
        content: "tool wrote this".to_string(),
        status: "in_progress".to_string(),
        priority: "high".to_string(),
    }];

    let loaded = crate::app::agent::project::instance::provide(project.path(), None, {
        let session_id = session_id.clone();
        let tool_todos = tool_todos.clone();
        move || {
            Box::pin(async move {
                write_session_todos_for_test(&session_id, tool_todos).expect("tool write");
                get(&session_id).await
            })
        }
    })
    .await
    .expect("project instance");

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].content, "tool wrote this");
    assert!(matches!(loaded[0].status, TodoStatus::InProgress));
    assert!(matches!(loaded[0].priority, TodoPriority::High));
}
