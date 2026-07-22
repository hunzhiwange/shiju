use super::*;
use crate::app::agent::storage;
use serde_json::json;
use uuid::Uuid;

fn unique_session_id(label: &str) -> String {
    format!("message-test-{label}-{}", Uuid::new_v4())
}

fn model_ref() -> ModelRef {
    ModelRef { provider_id: "test-provider".to_string(), model_id: "test-model".to_string() }
}

fn user_info(session_id: &str, id: &str) -> Info {
    Info::User(Box::new(UserInfo {
        id: id.to_string(),
        session_id: session_id.to_string(),
        time: UserTime { created: 10 },
        summary: None,
        agent: "agent".to_string(),
        model: model_ref(),
        system: None,
        tools: None,
        variant: None,
    }))
}

fn assistant_info(session_id: &str, id: &str, parent_id: &str) -> Info {
    Info::Assistant(Box::new(AssistantInfo {
        id: id.to_string(),
        session_id: session_id.to_string(),
        time: AssistantTime { created: 11, completed: Some(12) },
        error: None,
        parent_id: parent_id.to_string(),
        model_id: "test-model".to_string(),
        provider_id: "test-provider".to_string(),
        mode: "chat".to_string(),
        agent: "agent".to_string(),
        path: PathInfo { cwd: "/tmp".to_string(), root: "/tmp".to_string() },
        summary: None,
        cost: 0.0,
        tokens: TokenInfo {
            total: Some(3),
            input: 1,
            output: 2,
            reasoning: 0,
            cache: TokenCacheInfo { read: 0, write: 0 },
        },
        variant: None,
        finish: Some("stop".to_string()),
    }))
}

fn part_base(session_id: &str, message_id: &str, part_id: &str) -> PartBase {
    PartBase {
        id: part_id.to_string(),
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
    }
}

fn text_part(session_id: &str, message_id: &str, part_id: &str, text: &str) -> Part {
    Part::Text(TextPart {
        base: part_base(session_id, message_id, part_id),
        text: text.to_string(),
        synthetic: None,
        ignored: None,
        time: None,
        metadata: None,
    })
}

#[test]
fn event_definitions_preserve_public_message_event_names() {
    assert_eq!(event::UPDATED.r#type, "message.updated");
    assert_eq!(event::REMOVED.r#type, "message.removed");
    assert_eq!(event::PART_UPDATED.r#type, "message.part.updated");
    assert_eq!(event::PART_REMOVED.r#type, "message.part.removed");
}

#[tokio::test]
async fn update_get_and_remove_messages_round_trip() {
    let session_id = unique_session_id("round-trip");
    let user = user_info(&session_id, "user-1");
    let assistant = assistant_info(&session_id, "assistant-1", "user-1");

    update_message(&user).await.expect("user message should persist");
    update_message(&assistant).await.expect("assistant message should persist");

    let loaded_user = get(&session_id, "user-1").await.expect("user message should load");
    assert_eq!(loaded_user.info.id(), "user-1");
    assert!(loaded_user.parts.is_empty());

    let loaded_assistant =
        get(&session_id, "assistant-1").await.expect("assistant message should load");
    assert_eq!(loaded_assistant.info.id(), "assistant-1");

    remove_message(&session_id, "user-1").await.expect("user message should remove");
    assert!(get(&session_id, "user-1").await.is_err());

    remove_message(&session_id, "assistant-1").await.expect("assistant cleanup should succeed");
}

#[tokio::test]
async fn parts_use_current_layout_sort_by_part_id_and_skip_bad_records() {
    let session_id = unique_session_id("parts-current");
    let message_id = format!("{session_id}-message-1");
    let part_b = text_part(&session_id, &message_id, "part-b", "second");
    let part_a = text_part(&session_id, &message_id, "part-a", "first");

    update_part(&part_b).await.expect("part b should persist");
    update_part(&part_a).await.expect("part a should persist");
    storage::write(&["part", &message_id, "bad"], &json!({"type": "unknown"}))
        .await
        .expect("bad part fixture should persist");

    let loaded = parts(&session_id, &message_id).await.expect("parts should load");
    assert_eq!(loaded.iter().map(Part::id).collect::<Vec<_>>(), vec!["part-a", "part-b"]);

    remove_part(&session_id, &message_id, "part-a").await.expect("part a cleanup");
    remove_part(&session_id, &message_id, "part-b").await.expect("part b cleanup");
    remove_part(&session_id, &message_id, "bad").await.expect("bad part cleanup");
}

#[tokio::test]
async fn parts_fall_back_to_legacy_session_message_layout() {
    let session_id = unique_session_id("parts-legacy");
    let message_id = format!("{session_id}-message-legacy");
    let legacy = text_part(&session_id, &message_id, "legacy-part", "legacy text");

    storage::write(&["part", &session_id, &message_id, "legacy-part"], &legacy)
        .await
        .expect("legacy part fixture should persist");

    let loaded = parts(&session_id, &message_id).await.expect("legacy parts should load");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id(), "legacy-part");

    remove_part(&session_id, &message_id, "legacy-part").await.expect("legacy cleanup");
}

#[tokio::test]
async fn parts_prefer_current_layout_when_legacy_parts_also_exist() {
    let session_id = unique_session_id("parts-current-preferred");
    let message_id = format!("{session_id}-message");
    let current = text_part(&session_id, &message_id, "current-part", "current text");
    let legacy = text_part(&session_id, &message_id, "legacy-part", "legacy text");

    update_part(&current).await.expect("current part should persist");
    storage::write(&["part", &session_id, &message_id, "legacy-part"], &legacy)
        .await
        .expect("legacy part should persist");

    let loaded = parts(&session_id, &message_id).await.expect("current parts should load");
    assert_eq!(loaded.iter().map(Part::id).collect::<Vec<_>>(), vec!["current-part"]);

    remove_part(&session_id, &message_id, "current-part").await.expect("current cleanup");
    storage::remove(&["part", &session_id, &message_id, "legacy-part"])
        .await
        .expect("legacy cleanup");
}

#[tokio::test]
async fn remove_part_cleans_current_and_legacy_storage_layouts() {
    let session_id = unique_session_id("remove-both-layouts");
    let message_id = format!("{session_id}-message");
    let part_id = "shared-part";
    let part = text_part(&session_id, &message_id, part_id, "shared text");

    storage::write(&["part", &message_id, part_id], &part)
        .await
        .expect("current part should persist");
    storage::write(&["part", &session_id, &message_id, part_id], &part)
        .await
        .expect("legacy part should persist");

    remove_part(&session_id, &message_id, part_id).await.expect("both layouts should remove");

    let current_error = storage::read::<Part>(&["part", &message_id, part_id])
        .await
        .expect_err("current part should be absent");
    let legacy_error = storage::read::<Part>(&["part", &session_id, &message_id, part_id])
        .await
        .expect_err("legacy part should be absent");
    assert!(matches!(current_error, storage::Error::NotFound(_)));
    assert!(matches!(legacy_error, storage::Error::NotFound(_)));
}

#[tokio::test]
async fn get_combines_message_info_with_sorted_parts() {
    let session_id = unique_session_id("get-with-parts");
    let message_id = format!("{session_id}-message");
    let info = user_info(&session_id, &message_id);
    let part_b = text_part(&session_id, &message_id, "part-b", "second");
    let part_a = text_part(&session_id, &message_id, "part-a", "first");

    update_message(&info).await.expect("message should persist");
    update_part(&part_b).await.expect("part b should persist");
    update_part(&part_a).await.expect("part a should persist");

    let loaded = get(&session_id, &message_id).await.expect("message with parts should load");
    assert_eq!(loaded.info.id(), message_id.as_str());
    assert_eq!(loaded.parts.iter().map(Part::id).collect::<Vec<_>>(), vec!["part-a", "part-b"]);

    remove_message(&session_id, &message_id).await.expect("message cleanup");
    remove_part(&session_id, &message_id, "part-a").await.expect("part a cleanup");
    remove_part(&session_id, &message_id, "part-b").await.expect("part b cleanup");
}

#[tokio::test]
async fn messages_are_newest_first_limited_and_skip_unreadable_entries() {
    let session_id = unique_session_id("messages");
    let older = user_info(&session_id, "001");
    let newer = assistant_info(&session_id, "002", "001");
    update_message(&older).await.expect("older message should persist");
    update_message(&newer).await.expect("newer message should persist");
    storage::write(&["message", &session_id, "003"], &json!({"role": "bad"}))
        .await
        .expect("bad message fixture should persist");
    storage::write(&["message", &session_id, "nested", "ignored"], &json!({"ignored": true}))
        .await
        .expect("nested message fixture should persist");

    let all = messages(&session_id, None).await.expect("messages should load");
    assert_eq!(all.iter().map(|message| message.info.id()).collect::<Vec<_>>(), vec!["002", "001"]);

    let limited = messages(&session_id, Some(1)).await.expect("limited messages should load");
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].info.id(), "002");

    remove_message(&session_id, "001").await.expect("older cleanup");
    remove_message(&session_id, "002").await.expect("newer cleanup");
    storage::remove(&["message", &session_id, "003"]).await.expect("bad cleanup");
    storage::remove(&["message", &session_id, "nested", "ignored"]).await.expect("nested cleanup");
}

#[tokio::test]
async fn messages_with_zero_limit_return_no_persisted_entries() {
    let session_id = unique_session_id("messages-zero-limit");
    let message = user_info(&session_id, "001");
    update_message(&message).await.expect("message should persist");

    let loaded = messages(&session_id, Some(0)).await.expect("zero limit should be accepted");
    assert!(loaded.is_empty());

    remove_message(&session_id, "001").await.expect("message cleanup");
}
