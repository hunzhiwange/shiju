// 本文件验证 Matrix 监听事件过滤、回复缓存与调度载荷分发。

use super::*;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use vw_api_types::agent_scheduler::{
    AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE, AgentSchedulerMatrixDispatchEventDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TASK_POOL_MATRIX_EVENT_TYPE, TaskPoolMatrixActorDto, TaskPoolMatrixEventDto,
    TaskPoolMatrixEventKindDto, TaskPoolStatus,
};

fn cache_with_bot_event(event_id: &str) -> Mutex<(VecDeque<String>, HashSet<String>)> {
    let mut order = VecDeque::new();
    let mut lookup = HashSet::new();
    order.push_back(event_id.to_string());
    lookup.insert(event_id.to_string());
    Mutex::new((order, lookup))
}

fn parse_event(value: serde_json::Value) -> OriginalSyncRoomMessageEvent {
    serde_json::from_value(value).expect("valid Matrix room message event")
}

#[tokio::test]
async fn listen_impl_fails_fast_when_otk_conflict_is_already_detected() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    channel.otk_conflict_detected.store(true, Ordering::SeqCst);
    let (tx, _rx) = mpsc::channel(1);

    let error = channel.listen_impl(tx).await.unwrap_err();

    assert!(error.to_string().contains("one-time key upload conflict"));
    assert!(error.to_string().contains("paused Matrix sync"));
}

#[tokio::test]
async fn reply_cache_detects_replies_to_recent_bot_events() {
    let event = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$reply:matrix.example.com",
        "sender": "@user:matrix.example.com",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "following up",
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": "$bot-message:matrix.example.com"
                }
            }
        }
    }));
    let cache = cache_with_bot_event("$bot-message:matrix.example.com");

    assert!(MatrixChannel::is_reply_to_cached_bot_event(&event, &cache).await);
}

#[tokio::test]
async fn reply_cache_ignores_events_without_reply_relation() {
    let event = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$plain:matrix.example.com",
        "sender": "@user:matrix.example.com",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "plain message"
        }
    }));
    let cache = cache_with_bot_event("$bot-message:matrix.example.com");

    assert!(!MatrixChannel::is_reply_to_cached_bot_event(&event, &cache).await);
}

fn matrix_task_pool_event_content(event_id: &str, task_id: &str) -> serde_json::Value {
    serde_json::to_value(TaskPoolMatrixEventDto {
        version: 1,
        event_id: event_id.to_string(),
        task_id: task_id.to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@alice:matrix.example.com".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1_000,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pool),
            to_status: TaskPoolStatus::Pending,
        },
    })
    .unwrap()
}

fn matrix_task_pool_raw_event(
    matrix_event_id: &str,
    sender: &str,
    content: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "type": TASK_POOL_MATRIX_EVENT_TYPE,
        "event_id": matrix_event_id,
        "sender": sender,
        "origin_server_ts": 1u64,
        "content": content
    })
}

fn matrix_dispatch_event() -> AgentSchedulerMatrixDispatchEventDto {
    AgentSchedulerMatrixDispatchEventDto {
        event_version: 1,
        platform_id: "local".to_string(),
        company_id: "local".to_string(),
        project_id: "/repo/project".to_string(),
        task_id: "task-1".to_string(),
        run_id: "run-task-1-1000".to_string(),
        worker_id: "matrix-test".to_string(),
        worker_role: "tester".to_string(),
        room_id: "!room:matrix.example.com".to_string(),
        matrix_user_id: "@tester:matrix.example.com".to_string(),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        agent: Some("codex".to_string()),
        acp_agent: None,
        reasoning_effort: None,
        spec_path: "shared/tasks/task-1/spec.md".to_string(),
        metadata_path: "shared/tasks/task-1/meta.json".to_string(),
        created_at_ms: TimestampMs(1_000),
    }
}

fn matrix_dispatch_raw_event(
    matrix_event_id: Option<&str>,
    sender: &str,
    content: serde_json::Value,
) -> serde_json::Value {
    let mut event = serde_json::json!({
        "type": AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE,
        "sender": sender,
        "content": content
    });
    if let Some(matrix_event_id) = matrix_event_id {
        event["event_id"] = serde_json::json!(matrix_event_id);
    }
    event
}

#[test]
fn matrix_dispatch_listener_accepts_allowed_valid_event() {
    let dispatch = matrix_dispatch_event();
    let raw_event = matrix_dispatch_raw_event(
        Some("$dispatch-1:matrix.example.com"),
        "@alice:matrix.example.com",
        serde_json::to_value(&dispatch).unwrap(),
    );

    let parsed = MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
        "!room:matrix.example.com",
        "!room:matrix.example.com",
        &raw_event,
        &["@alice:matrix.example.com".to_string()],
    )
    .expect("allowed valid dispatch should be accepted");

    assert_eq!(parsed.matrix_event_id, "$dispatch-1:matrix.example.com");
    assert_eq!(parsed.room_id, "!room:matrix.example.com");
    assert_eq!(parsed.dispatch, dispatch);
}

#[test]
// 调度事件同时校验房间、类型、发送者和标识，任一不匹配即拒绝。
fn matrix_dispatch_listener_rejects_room_type_sender_and_event_id_mismatches() {
    let allowed = ["@alice:matrix.example.com".to_string()];
    let content = serde_json::to_value(matrix_dispatch_event()).unwrap();
    let valid = matrix_dispatch_raw_event(
        Some("$dispatch-1:matrix.example.com"),
        "@alice:matrix.example.com",
        content.clone(),
    );
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!other:matrix.example.com",
            &valid,
            &allowed,
        )
        .is_none()
    );

    let mut wrong_type = valid.clone();
    wrong_type["type"] = serde_json::json!("m.room.message");
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &wrong_type,
            &allowed,
        )
        .is_none()
    );

    let wrong_sender = matrix_dispatch_raw_event(
        Some("$dispatch-2:matrix.example.com"),
        "@mallory:matrix.example.com",
        content.clone(),
    );
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &wrong_sender,
            &allowed,
        )
        .is_none()
    );

    for event_id in [None, Some("   ")] {
        let missing_id =
            matrix_dispatch_raw_event(event_id, "@alice:matrix.example.com", content.clone());
        assert!(
            MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
                "!room:matrix.example.com",
                "!room:matrix.example.com",
                &missing_id,
                &allowed,
            )
            .is_none()
        );
    }
}

#[test]
fn matrix_dispatch_listener_rejects_malformed_and_invalid_payloads() {
    let allowed = ["@alice:matrix.example.com".to_string()];
    let malformed = serde_json::json!({
        "type": AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE,
        "event_id": "$dispatch-malformed:matrix.example.com"
    });
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &malformed,
            &allowed,
        )
        .is_none()
    );

    let unreadable = matrix_dispatch_raw_event(
        Some("$dispatch-unreadable:matrix.example.com"),
        "@alice:matrix.example.com",
        serde_json::json!({"unexpected": "shape"}),
    );
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &unreadable,
            &allowed,
        )
        .is_none()
    );

    let mut invalid_dispatch = matrix_dispatch_event();
    invalid_dispatch.event_version = 2;
    let invalid = matrix_dispatch_raw_event(
        Some("$dispatch-invalid:matrix.example.com"),
        "@alice:matrix.example.com",
        serde_json::to_value(invalid_dispatch).unwrap(),
    );
    assert!(
        MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &invalid,
            &allowed,
        )
        .is_none()
    );
}

#[tokio::test]
async fn matrix_task_pool_listener_accepts_allowed_room_event_and_dedupes() {
    let dedupe = Mutex::new((VecDeque::new(), HashSet::new()));
    let allowed = vec!["@alice:matrix.example.com".to_string()];
    let raw_event = matrix_task_pool_raw_event(
        "$event-1:matrix.example.com",
        "@alice:matrix.example.com",
        matrix_task_pool_event_content("event-1", "task-1"),
    );

    let parsed = MatrixChannel::task_pool_matrix_event_from_raw_timeline(
        "!room:matrix.example.com",
        "!room:matrix.example.com",
        &raw_event,
        &allowed,
        &dedupe,
    )
    .await
    .expect("allowed event should be accepted");

    assert_eq!(parsed.event_id, "event-1");
    assert_eq!(parsed.task_id, "task-1");

    let duplicate = MatrixChannel::task_pool_matrix_event_from_raw_timeline(
        "!room:matrix.example.com",
        "!room:matrix.example.com",
        &raw_event,
        &allowed,
        &dedupe,
    )
    .await;

    assert!(duplicate.is_none());
}

#[tokio::test]
async fn matrix_task_pool_listener_rejects_wrong_room_sender_type_and_payload() {
    let dedupe = Mutex::new((VecDeque::new(), HashSet::new()));
    let allowed = vec!["@alice:matrix.example.com".to_string()];
    let valid_content = matrix_task_pool_event_content("event-1", "task-1");

    let wrong_room = matrix_task_pool_raw_event(
        "$event-wrong-room:matrix.example.com",
        "@alice:matrix.example.com",
        valid_content.clone(),
    );
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!other:matrix.example.com",
            &wrong_room,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );

    let wrong_sender = matrix_task_pool_raw_event(
        "$event-wrong-sender:matrix.example.com",
        "@mallory:matrix.example.com",
        valid_content.clone(),
    );
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &wrong_sender,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );

    let wrong_type = serde_json::json!({
        "type": "m.room.message",
        "event_id": "$event-wrong-type:matrix.example.com",
        "sender": "@alice:matrix.example.com",
        "origin_server_ts": 1u64,
        "content": valid_content
    });
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &wrong_type,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );

    let invalid_payload = matrix_task_pool_raw_event(
        "$event-invalid-payload:matrix.example.com",
        "@alice:matrix.example.com",
        matrix_task_pool_event_content("event-invalid", ""),
    );
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &invalid_payload,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );
}

#[tokio::test]
async fn matrix_task_pool_listener_rejects_malformed_raw_envelope() {
    let dedupe = Mutex::new((VecDeque::new(), HashSet::new()));
    let allowed = vec!["@alice:matrix.example.com".to_string()];

    // `sender` missing -> RawTaskPoolMatrixTimelineEvent fails to deserialize.
    let malformed = serde_json::json!({
        "type": TASK_POOL_MATRIX_EVENT_TYPE,
        "event_id": "$event-malformed:matrix.example.com",
    });

    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &malformed,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );
}

#[tokio::test]
async fn matrix_task_pool_listener_rejects_missing_or_blank_matrix_event_id() {
    let dedupe = Mutex::new((VecDeque::new(), HashSet::new()));
    let allowed = vec!["@alice:matrix.example.com".to_string()];
    let content = matrix_task_pool_event_content("event-1", "task-1");

    let missing_event_id = serde_json::json!({
        "type": TASK_POOL_MATRIX_EVENT_TYPE,
        "sender": "@alice:matrix.example.com",
        "origin_server_ts": 1u64,
        "content": content.clone()
    });
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &missing_event_id,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );

    let blank_event_id = matrix_task_pool_raw_event("   ", "@alice:matrix.example.com", content);
    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &blank_event_id,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );
}

#[tokio::test]
async fn matrix_task_pool_listener_rejects_unreadable_content_payload() {
    let dedupe = Mutex::new((VecDeque::new(), HashSet::new()));
    let allowed = vec!["@alice:matrix.example.com".to_string()];

    // Correct envelope/type/sender, but `content` is not a TaskPoolMatrixEventDto.
    let unreadable = matrix_task_pool_raw_event(
        "$event-unreadable:matrix.example.com",
        "@alice:matrix.example.com",
        serde_json::json!({"unexpected": "shape"}),
    );

    assert!(
        MatrixChannel::task_pool_matrix_event_from_raw_timeline(
            "!room:matrix.example.com",
            "!room:matrix.example.com",
            &unreadable,
            &allowed,
            &dedupe,
        )
        .await
        .is_none()
    );
}

#[test]
fn mark_otk_conflict_sets_flag_and_is_idempotent() {
    let flag = Arc::new(AtomicBool::new(false));

    MatrixChannel::mark_otk_conflict(&flag);
    assert!(flag.load(Ordering::SeqCst));

    // Second call must keep the flag set without panicking (already detected).
    MatrixChannel::mark_otk_conflict(&flag);
    assert!(flag.load(Ordering::SeqCst));
}
