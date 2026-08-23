//! 验证会话模型、标题分叉、事件类型及项目上下文边界。
//! 无项目上下文的操作应在副作用前失败，分叉标题只处理末尾的合法序号。

use super::{
    CreateInput, Error, Role, Session, children, create_default_title, create_next, event, extra,
    fork, forked_title, get, instance_directory_opt, list, project_id, remove, touch, update,
};
use crate::app::agent::{id, storage};
use serde_json::Value;

fn assert_no_project<T>(result: Result<T, Error>) {
    assert!(matches!(result, Err(Error::NoProjectContext)));
}

#[test]
fn new_session_starts_empty_and_push_preserves_message_order() {
    let mut session = Session::new("session-1".to_string());

    assert_eq!(session.id, "session-1");
    assert!(session.messages.is_empty());

    session.push(Role::User, "hello".to_string());
    session.push(Role::Assistant, "hi".to_string());

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].role, Role::User);
    assert_eq!(session.messages[0].content, "hello");
    assert_eq!(session.messages[1].role, Role::Assistant);
}

#[test]
fn session_supports_all_roles_and_message_cloning() {
    let mut session = Session::new("session-roles".to_string());

    session.push(Role::System, "system".to_string());
    session.push(Role::Tool, "tool".to_string());

    let cloned = session.clone();

    assert_eq!(cloned.messages[0].role, Role::System);
    assert_eq!(cloned.messages[1].role, Role::Tool);
    assert_eq!(format!("{:?}", cloned.messages[1]), "Message { role: Tool, content: \"tool\" }");
}

#[test]
fn default_titles_use_expected_prefixes() {
    let parent = create_default_title(false);
    let child = create_default_title(true);

    assert!(parent.starts_with("New session - "));
    assert!(child.starts_with("Child session - "));
    assert!(parent.ends_with('Z'));
    assert!(child.ends_with('Z'));
}

#[test]
fn forked_title_adds_or_increments_suffix() {
    assert_eq!(forked_title("Design"), "Design (fork #1)");
    assert_eq!(forked_title("Design (fork #1)"), "Design (fork #2)");
    assert_eq!(forked_title("Design (fork #001)"), "Design (fork #2)");
    assert_eq!(forked_title("Design (fork #x)"), "Design (fork #x) (fork #1)");
}

#[test]
fn forked_title_handles_empty_nonterminal_and_out_of_range_suffixes() {
    assert_eq!(forked_title(""), " (fork #1)");
    assert_eq!(forked_title("Design (fork #0)"), "Design (fork #1)");
    assert_eq!(forked_title("Design (fork #2) notes"), "Design (fork #2) notes (fork #1)");
    assert_eq!(forked_title("Design (fork #18446744073709551616)"), "Design (fork #1)");
}

#[test]
fn extra_builds_json_map_from_static_pairs() {
    let map = extra([("id", Value::String("s".to_string())), ("n", serde_json::json!(1))]);

    assert_eq!(map.get("id").and_then(Value::as_str), Some("s"));
    assert_eq!(map.get("n").and_then(Value::as_i64), Some(1));
}

#[test]
fn extra_supports_empty_input_and_last_duplicate_wins() {
    assert!(extra::<0>([]).is_empty());

    let map = extra([("id", serde_json::json!(1)), ("id", serde_json::json!(2))]);

    assert_eq!(map.len(), 1);
    assert_eq!(map.get("id").and_then(Value::as_i64), Some(2));
}

#[test]
fn create_input_default_leaves_optional_fields_unset() {
    let input = CreateInput::default();

    assert!(input.parent_id.is_none());
    assert!(input.title.is_none());
    assert!(input.directory.is_empty());
    assert!(input.permission.is_none());
}

#[test]
fn event_definitions_keep_stable_public_types() {
    assert_eq!(event::CREATED.r#type, "session.created");
    assert_eq!(event::UPDATED.r#type, "session.updated");
    assert_eq!(event::DELETED.r#type, "session.deleted");
    assert_eq!(event::DIFF.r#type, "session.diff");
    assert_eq!(event::ERROR.r#type, "session.error");
}

#[test]
fn error_conversions_preserve_variant_and_display_message() {
    let id_source = id::Error::PrefixMismatch { id: "msg_1".to_string(), expected_prefix: "ses" };
    let id_message = id_source.to_string();
    let id_error = Error::from(id_source);
    assert!(matches!(&id_error, Error::Id(_)));
    assert_eq!(id_error.to_string(), id_message);

    let storage_source =
        storage::Error::NotFound(storage::NotFoundError { message: "missing session".to_string() });
    let storage_message = storage_source.to_string();
    let storage_error = Error::from(storage_source);
    assert!(matches!(&storage_error, Error::Storage(_)));
    assert_eq!(storage_error.to_string(), storage_message);

    let json_source = serde_json::from_str::<Value>("{").expect_err("invalid JSON should fail");
    let json_message = json_source.to_string();
    let json_error = Error::from(json_source);
    assert!(matches!(&json_error, Error::Json(_)));
    assert_eq!(json_error.to_string(), json_message);
}

#[test]
fn runtime_error_conversion_preserves_source_message() {
    let source = anyhow::anyhow!("isolated database task failed");
    let error = Error::from(source);

    assert!(matches!(&error, Error::Runtime(_)));
    assert_eq!(error.to_string(), "isolated database task failed");
}

#[test]
fn project_id_reports_missing_context_when_unset() {
    let err = project_id().expect_err("project context should be absent outside a task scope");

    assert!(matches!(err, Error::NoProjectContext));
    assert_eq!(err.to_string(), "no active project context");
    assert_eq!(instance_directory_opt(), None);
}

#[tokio::test]
// 上下文绑定接口统一在任何存储或事件副作用前检查项目作用域。
async fn context_bound_operations_reject_missing_project_before_side_effects() {
    assert_no_project(create_next(CreateInput::default()).await);
    assert_no_project(get("missing-session").await);
    assert_no_project(touch("missing-session").await);
    assert_no_project(
        update("missing-session", |_| {
            panic!("update callback must not run without project context");
        })
        .await,
    );
    assert_no_project(list().await);
    assert_no_project(children("missing-parent").await);
    assert_no_project(remove("missing-session").await);
    assert_no_project(fork("missing-session", None).await);
}
