//! 验证会话回退输入、错误转换、消息裁剪与快照恢复行为。

use super::super::{message, session as session_api};
use crate::app::agent::snapshot;
use serde_json::{Value, json};
use std::path::PathBuf;

fn part_base(message_id: &str, part_id: &str) -> message::PartBase {
    message::PartBase {
        id: part_id.to_string(),
        session_id: "session-1".to_string(),
        message_id: message_id.to_string(),
    }
}

fn text_part(message_id: &str, part_id: &str) -> message::Part {
    message::Part::Text(message::TextPart {
        base: part_base(message_id, part_id),
        text: part_id.to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    })
}

fn patch_part(message_id: &str, part_id: &str, hash: &str) -> message::Part {
    message::Part::Patch(message::PatchPart {
        base: part_base(message_id, part_id),
        hash: hash.to_string(),
        files: vec![format!("{hash}.rs")],
        additions: None,
        deletions: None,
    })
}

fn user_message(id: &str, parts: Vec<message::Part>) -> message::WithParts {
    message::WithParts {
        info: message::Info::User(Box::new(message::UserInfo {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            time: message::UserTime { created: 1 },
            summary: None,
            agent: "agent".to_string(),
            model: message::ModelRef {
                provider_id: "provider".to_string(),
                model_id: "model".to_string(),
            },
            system: None,
            tools: None,
            variant: None,
        })),
        parts,
    }
}

fn assistant_message(id: &str, parent_id: &str, parts: Vec<message::Part>) -> message::WithParts {
    message::WithParts {
        info: message::Info::Assistant(Box::new(message::AssistantInfo {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            time: message::AssistantTime {
                created: 2,
                completed: Some(3),
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            },
            error: None,
            parent_id: parent_id.to_string(),
            model_id: "model".to_string(),
            provider_id: "provider".to_string(),
            mode: "chat".to_string(),
            agent: "agent".to_string(),
            path: message::PathInfo { cwd: "/tmp".to_string(), root: "/tmp".to_string() },
            summary: None,
            cost: 0.0,
            tokens: message::TokenInfo {
                total: Some(0),
                input: 0,
                output: 0,
                reasoning: 0,
                cache: message::TokenCacheInfo { read: 0, write: 0 },
            },
            variant: None,
            finish: Some("stop".to_string()),
        })),
        parts,
    }
}

fn session_info(revert: Option<session_api::RevertInfo>) -> session_api::Info {
    session_api::Info {
        id: "session-1".to_string(),
        slug: "session-1".to_string(),
        project_id: "project-1".to_string(),
        directory: "/tmp".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "test".to_string(),
        time: session_api::TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        revert,
    }
}

fn message_ids(messages: &[message::WithParts]) -> Vec<&str> {
    messages.iter().map(|message| message.info.id()).collect()
}

#[test]
fn revert_input_serializes_with_expected_camel_case_keys() {
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
        part_id: Some("part-1".to_string()),
    };

    let value = serde_json::to_value(&input).unwrap();

    assert_eq!(value["sessionID"], "session-1");
    assert_eq!(value["messageID"], "message-1");
    assert_eq!(value["partID"], "part-1");
}

#[test]
fn revert_input_skips_absent_part_id() {
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
        part_id: None,
    };

    let value = serde_json::to_value(&input).unwrap();

    assert!(value.get("partID").is_none());
}

#[test]
fn revert_input_deserializes_gateway_keys_and_defaults_missing_part_id() {
    let input: super::RevertInput = serde_json::from_value(json!({
        "sessionID": "session-1",
        "messageID": "message-1"
    }))
    .unwrap();

    assert_eq!(input.session_id, "session-1");
    assert_eq!(input.message_id, "message-1");
    assert_eq!(input.part_id, None);
}

#[test]
fn revert_input_rejects_internal_snake_case_keys() {
    let result = serde_json::from_value::<super::RevertInput>(json!({
        "session_id": "session-1",
        "message_id": "message-1"
    }));

    assert!(result.is_err());
}

#[test]
fn session_error_conversion_preserves_variant_and_display() {
    let error: super::Error = session_api::Error::NoProjectContext.into();

    assert!(matches!(&error, super::Error::Session(session_api::Error::NoProjectContext)));
    assert_eq!(error.to_string(), "no active project context");
}

#[test]
fn snapshot_error_conversion_preserves_variant_and_display() {
    let error: super::Error = snapshot::Error::Git("invalid snapshot".to_string()).into();

    assert!(
        matches!(&error, super::Error::Snapshot(snapshot::Error::Git(message)) if message == "invalid snapshot")
    );
    assert_eq!(error.to_string(), "invalid snapshot");
}

#[test]
fn helpers_use_unscoped_defaults_outside_project_context() {
    assert_eq!(super::instance_directory_opt(), None);
    assert_eq!(super::worktree_path(), PathBuf::from("."));
}

#[test]
fn collect_revert_plan_uses_last_user_for_message_target_and_collects_later_patches() {
    let messages = vec![
        user_message("001", vec![text_part("001", "user-text")]),
        assistant_message(
            "002",
            "001",
            vec![text_part("002", "assistant-text"), patch_part("002", "patch-1", "tree-1")],
        ),
        assistant_message("003", "001", vec![patch_part("003", "patch-2", "tree-2")]),
    ];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "002".to_string(),
        part_id: None,
    };

    let (info, patches) = super::collect_revert_plan(&messages, &input).unwrap();

    assert_eq!(info.message_id, "001");
    assert_eq!(info.part_id, None);
    assert!(info.snapshot.is_none());
    assert!(info.diff.is_none());
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].hash, "tree-1");
    assert_eq!(patches[0].files, vec!["tree-1.rs"]);
    assert_eq!(patches[1].hash, "tree-2");
}

#[test]
fn collect_revert_plan_keeps_part_target_after_useful_content() {
    let messages = vec![assistant_message(
        "002",
        "001",
        vec![
            text_part("002", "keep"),
            text_part("002", "target"),
            patch_part("002", "patch-after", "tree-after"),
        ],
    )];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "002".to_string(),
        part_id: Some("target".to_string()),
    };

    let (info, patches) = super::collect_revert_plan(&messages, &input).unwrap();

    assert_eq!(info.message_id, "002");
    assert_eq!(info.part_id.as_deref(), Some("target"));
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].hash, "tree-after");
}

#[test]
fn collect_revert_plan_drops_part_target_without_prior_useful_content() {
    let messages = vec![
        user_message("001", Vec::new()),
        assistant_message(
            "002",
            "001",
            vec![
                patch_part("002", "patch-before", "tree-before"),
                text_part("002", "target"),
                patch_part("002", "patch-after", "tree-after"),
            ],
        ),
    ];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "002".to_string(),
        part_id: Some("target".to_string()),
    };

    let (info, patches) = super::collect_revert_plan(&messages, &input).unwrap();

    assert_eq!(info.message_id, "001");
    assert_eq!(info.part_id, None);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].hash, "tree-after");
}

#[test]
fn collect_revert_plan_returns_none_when_target_is_missing() {
    let messages = vec![user_message("001", vec![text_part("001", "text")])];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "missing".to_string(),
        part_id: None,
    };

    assert!(super::collect_revert_plan(&messages, &input).is_none());
}

#[test]
fn collect_revert_plan_does_not_match_message_target_without_parts() {
    let messages = vec![user_message("001", Vec::new())];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "001".to_string(),
        part_id: None,
    };

    assert!(super::collect_revert_plan(&messages, &input).is_none());
}

#[test]
fn collect_revert_plan_uses_target_id_when_no_prior_user_exists() {
    let messages = vec![
        assistant_message("001", "missing-parent", vec![text_part("001", "target")]),
        assistant_message("002", "missing-parent", vec![patch_part("002", "patch-1", "tree-1")]),
    ];
    let input = super::RevertInput {
        session_id: "session-1".to_string(),
        message_id: "001".to_string(),
        part_id: None,
    };

    let (info, patches) = super::collect_revert_plan(&messages, &input).unwrap();

    assert_eq!(info.message_id, "001");
    assert_eq!(info.part_id, None);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].hash, "tree-1");
}

#[test]
fn collect_cleanup_messages_excludes_target_and_keeps_following_messages() {
    let messages = vec![
        user_message("001", Vec::new()),
        user_message("002", Vec::new()),
        user_message("003", Vec::new()),
    ];

    let remove = super::collect_cleanup_messages(messages, "002", false);

    assert_eq!(message_ids(&remove), vec!["003"]);
}

#[test]
fn collect_cleanup_messages_includes_target_and_following_messages() {
    let messages = vec![
        user_message("001", Vec::new()),
        user_message("002", Vec::new()),
        user_message("003", Vec::new()),
    ];

    let remove = super::collect_cleanup_messages(messages, "002", true);

    assert_eq!(message_ids(&remove), vec!["002", "003"]);
}

#[test]
fn collect_cleanup_messages_returns_empty_when_target_is_missing() {
    let messages = vec![user_message("001", Vec::new()), user_message("003", Vec::new())];

    let remove = super::collect_cleanup_messages(messages, "002", true);

    assert!(remove.is_empty());
}

#[tokio::test]
async fn cleanup_without_revert_is_a_noop() {
    let session = session_info(None);

    super::cleanup(&session).await.unwrap();

    assert!(session.revert.is_none());
}

#[tokio::test]
async fn unrevert_rejects_missing_project_context_before_snapshot_access() {
    let error = super::unrevert("session-1").await.unwrap_err();

    assert!(matches!(error, super::Error::Session(session_api::Error::NoProjectContext)));
}

#[test]
fn extra_builds_json_map_from_pairs_and_overwrites_duplicate_keys() {
    let map = super::extra([
        ("sessionID", Value::String("session-1".to_string())),
        ("count", json!(1)),
        ("count", json!(2)),
    ]);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("sessionID").and_then(Value::as_str), Some("session-1"));
    assert_eq!(map.get("count").and_then(Value::as_i64), Some(2));
}
