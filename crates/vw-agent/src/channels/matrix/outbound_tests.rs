// 本文件验证 Matrix Markdown、调度事件与原始事件的出站发送。

use super::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex as StdMutex};
use vw_api_types::agent_scheduler::{
    AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE, AgentSchedulerMatrixDispatchEventDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TASK_POOL_MATRIX_EVENT_TYPE, TaskPoolMatrixActorDto, TaskPoolMatrixEventDto,
    TaskPoolMatrixEventKindDto, TaskPoolStatus,
};

#[derive(Default)]
struct RecordingMatrixRawSender {
    sent: Arc<StdMutex<Vec<(String, serde_json::Value)>>>,
    notices: Arc<StdMutex<Vec<String>>>,
}

struct FailingMatrixRawSender {
    fail_notice: bool,
}

#[async_trait]
impl MatrixRawEventSender for FailingMatrixRawSender {
    async fn send_raw_event(
        &self,
        _event_type: &'static str,
        _content: serde_json::Value,
    ) -> anyhow::Result<String> {
        if self.fail_notice {
            Ok("$sent-before-notice:matrix.example.com".to_string())
        } else {
            anyhow::bail!("raw event rejected")
        }
    }

    async fn send_notice(&self, _message: String) -> anyhow::Result<()> {
        anyhow::bail!("notice rejected")
    }
}

#[async_trait]
impl MatrixRawEventSender for RecordingMatrixRawSender {
    async fn send_raw_event(
        &self,
        event_type: &'static str,
        content: serde_json::Value,
    ) -> anyhow::Result<String> {
        self.sent.lock().expect("recording lock").push((event_type.to_string(), content));
        Ok("$matrix-event:matrix.example.com".to_string())
    }

    async fn send_notice(&self, message: String) -> anyhow::Result<()> {
        self.notices.lock().expect("recording lock").push(message);
        Ok(())
    }
}

fn matrix_task_pool_event() -> TaskPoolMatrixEventDto {
    TaskPoolMatrixEventDto {
        version: 1,
        event_id: "event-1".to_string(),
        task_id: "task-1".to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@alice:matrix.example.com".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1_000,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pool),
            to_status: TaskPoolStatus::Pending,
        },
    }
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
        acp_agent: Some("claude".to_string()),
        reasoning_effort: Some(vw_api_types::ReasoningEffortDto::Max),
        spec_path: "shared/tasks/task-1/spec.md".to_string(),
        metadata_path: "shared/tasks/task-1/meta.json".to_string(),
        created_at_ms: TimestampMs(1_000),
    }
}

#[tokio::test]
async fn send_impl_fails_fast_when_otk_conflict_is_already_detected() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    channel.otk_conflict_detected.store(true, Ordering::SeqCst);
    let message = SendMessage::new("hello", "!room:matrix.example.com");

    let error = channel.send_impl(&message).await.unwrap_err();

    assert!(error.to_string().contains("one-time key upload conflict"));
}

#[test]
fn outbound_markdown_payload_preserves_plain_body_and_html_format() {
    let content = RoomMessageEventContent::text_markdown("**hello**");
    let value = serde_json::to_value(content).unwrap();

    assert_eq!(value["msgtype"], "m.text");
    assert_eq!(value["body"], "**hello**");
    assert_eq!(value["format"], "org.matrix.custom.html");
    assert!(
        value["formatted_body"].as_str().unwrap_or_default().contains("<strong>hello</strong>")
    );
}

#[tokio::test]
async fn matrix_task_pool_raw_sender_uses_custom_event_type_and_payload() {
    let sender = RecordingMatrixRawSender::default();
    let event = matrix_task_pool_event();

    let matrix_event_id =
        MatrixChannel::send_task_pool_matrix_event_with_sender(&sender, &event).await.unwrap();

    assert_eq!(matrix_event_id, "$matrix-event:matrix.example.com");
    let sent = sender.sent.lock().expect("recording lock");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, TASK_POOL_MATRIX_EVENT_TYPE);
    assert_eq!(sent[0].1, serde_json::to_value(event).unwrap());
    let notices = sender.notices.lock().expect("recording lock");
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("VibeWindow 任务池事件"));
    assert!(notices[0].contains("task-1"));
    assert!(notices[0].contains("pool -> pending"));
}

#[tokio::test]
async fn matrix_dispatch_sends_structured_scheduler_event() {
    let sender = RecordingMatrixRawSender::default();
    let event = matrix_dispatch_event();

    let matrix_event_id =
        MatrixChannel::send_agent_scheduler_dispatch_event_with_sender(&sender, &event)
            .await
            .unwrap();

    assert_eq!(matrix_event_id, "$matrix-event:matrix.example.com");
    let sent = sender.sent.lock().expect("recording lock");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE);
    assert_eq!(sent[0].1, serde_json::to_value(event).unwrap());
    assert!(sender.notices.lock().expect("recording lock").is_empty());
}

#[tokio::test]
async fn matrix_task_pool_event_with_sender_rejects_invalid_event() {
    let sender = RecordingMatrixRawSender::default();
    let mut event = matrix_task_pool_event();
    event.version = 2;

    let error =
        MatrixChannel::send_task_pool_matrix_event_with_sender(&sender, &event).await.unwrap_err();

    assert!(error.to_string().contains("version must be 1"));
    assert!(sender.sent.lock().expect("recording lock").is_empty());
    assert!(sender.notices.lock().expect("recording lock").is_empty());
}

#[tokio::test]
// 原始事件和通知失败都须向上传播，不能伪装成已投递。
async fn matrix_task_pool_sender_propagates_raw_event_and_notice_failures() {
    let event = matrix_task_pool_event();

    let raw_error = MatrixChannel::send_task_pool_matrix_event_with_sender(
        &FailingMatrixRawSender { fail_notice: false },
        &event,
    )
    .await
    .unwrap_err();
    assert!(raw_error.to_string().contains("raw event rejected"));

    let notice_error = MatrixChannel::send_task_pool_matrix_event_with_sender(
        &FailingMatrixRawSender { fail_notice: true },
        &event,
    )
    .await
    .unwrap_err();
    assert!(notice_error.to_string().contains("notice rejected"));
}

#[tokio::test]
async fn matrix_dispatch_sender_rejects_invalid_event_before_sending() {
    let sender = RecordingMatrixRawSender::default();
    let mut event = matrix_dispatch_event();
    event.event_version = 2;

    let error = MatrixChannel::send_agent_scheduler_dispatch_event_with_sender(&sender, &event)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("event_version must be 1"));
    assert!(sender.sent.lock().expect("recording lock").is_empty());
}

#[tokio::test]
async fn task_pool_wrapper_rejects_invalid_event_before_room_lookup() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    let mut event = matrix_task_pool_event();
    event.version = 2;

    let error = channel.send_task_pool_matrix_event(&event).await.unwrap_err();

    assert!(error.to_string().contains("version must be 1"));
}

#[tokio::test]
async fn scheduler_dispatch_wrapper_rejects_invalid_event_before_room_lookup() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    let mut event = matrix_dispatch_event();
    event.event_version = 2;

    let error = channel.send_agent_scheduler_dispatch_matrix_event(&event).await.unwrap_err();

    assert!(error.to_string().contains("event_version must be 1"));
}

#[tokio::test]
async fn task_pool_wrapper_reports_otk_conflict_after_validation() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    channel.otk_conflict_detected.store(true, Ordering::SeqCst);

    let error = channel.send_task_pool_matrix_event(&matrix_task_pool_event()).await.unwrap_err();

    assert!(error.to_string().contains("one-time key upload conflict"));
}

#[tokio::test]
async fn scheduler_dispatch_wrapper_reports_otk_conflict_after_validation() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    channel.otk_conflict_detected.store(true, Ordering::SeqCst);

    let error = channel
        .send_agent_scheduler_dispatch_matrix_event(&matrix_dispatch_event())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("one-time key upload conflict"));
}

#[test]
fn missing_joined_room_message_points_to_room_config_and_bot_membership() {
    let message = matrix_join_required_message("!room-id:localhost");

    assert!(message.contains("!room-id:localhost"));
    assert!(message.contains("channels.matrix.room_id"));
    assert!(message.contains("invite or join the bot account"));
}

fn notice_with_kind(kind: TaskPoolMatrixEventKindDto) -> String {
    let mut event = matrix_task_pool_event();
    event.kind = kind;
    task_pool_matrix_notice(&event)
}

#[test]
fn notice_renders_status_changed_without_from_status() {
    let notice = notice_with_kind(TaskPoolMatrixEventKindDto::TaskStatusChanged {
        from_status: None,
        to_status: TaskPoolStatus::Running,
    });

    assert!(notice.contains("状态 -> running"));
}

#[test]
fn notice_renders_task_claimed_note_and_blocked_kinds() {
    let claimed = notice_with_kind(TaskPoolMatrixEventKindDto::TaskClaimed {
        assignee: "@bob:matrix.example.com".to_string(),
    });
    assert!(claimed.contains("认领给 @bob:matrix.example.com"));

    let note = notice_with_kind(TaskPoolMatrixEventKindDto::TaskNote {
        message: "  needs review  ".to_string(),
    });
    assert!(note.contains("备注：needs review"));

    let blocked = notice_with_kind(TaskPoolMatrixEventKindDto::TaskBlocked {
        reason: "  waiting on CI  ".to_string(),
    });
    assert!(blocked.contains("阻塞：waiting on CI"));
}

#[test]
fn notice_prefers_trimmed_display_name_then_falls_back_to_user_id() {
    let mut event = matrix_task_pool_event();
    event.actor.display_name = Some("  Alice  ".to_string());
    assert!(task_pool_matrix_notice(&event).ends_with("Alice"));

    event.actor.display_name = Some("   ".to_string());
    assert!(task_pool_matrix_notice(&event).ends_with("@alice:matrix.example.com"));

    event.actor.display_name = None;
    assert!(task_pool_matrix_notice(&event).ends_with("@alice:matrix.example.com"));
}

#[test]
fn status_label_covers_all_variants() {
    let cases = [
        (TaskPoolStatus::Pool, "pool"),
        (TaskPoolStatus::Pending, "pending"),
        (TaskPoolStatus::Planning, "planning"),
        (TaskPoolStatus::Running, "running"),
        (TaskPoolStatus::Failed, "failed"),
        (TaskPoolStatus::Paused, "paused"),
        (TaskPoolStatus::CodeComplete, "code_complete"),
        (TaskPoolStatus::CodeReview, "code_review"),
        (TaskPoolStatus::PrSubmitted, "pr_submitted"),
        (TaskPoolStatus::Completed, "completed"),
        (TaskPoolStatus::Archived, "archived"),
    ];

    for (status, expected) in cases {
        assert_eq!(task_pool_status_label(status), expected);
    }
}
