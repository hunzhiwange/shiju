//! 代理会话状态测试，覆盖运行权限默认值、交接消费和聊天发送策略解析。
//! 同时验证步骤时间元数据及历史回放模式具有稳定键值和兼容别名。

use super::{
    AcpHistoryReplayMode, ChatSendBehavior, SessionRuntimeState, StepUiMeta,
    clear_pending_guide_handoff, mark_pending_guide_handoff, take_pending_guide_handoff,
};
use crate::app::models::ChatSessionStep;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(u64::MAX / 2);

fn unique_request_id() -> u64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn session_step(finished_ms: Option<u64>) -> ChatSessionStep {
    ChatSessionStep {
        index: 7,
        turn_id: None,
        started_ms: 1_000,
        finished_ms,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: Default::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some("model-a".to_string()),
        tool_timings: Vec::new(),
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("agent_tests"));
}

#[test]
fn session_runtime_defaults_to_full_access() {
    let runtime = SessionRuntimeState::new();

    assert!(runtime.full_access_enabled);
}

#[test]
fn pending_guide_handoff_is_consumed_once() {
    let request_id = unique_request_id();
    clear_pending_guide_handoff(request_id);

    mark_pending_guide_handoff(request_id);
    mark_pending_guide_handoff(request_id);

    assert!(take_pending_guide_handoff(request_id));
    assert!(!take_pending_guide_handoff(request_id));
}

#[test]
fn clearing_pending_guide_handoff_prevents_consumption() {
    let request_id = unique_request_id();
    mark_pending_guide_handoff(request_id);

    clear_pending_guide_handoff(request_id);

    assert!(!take_pending_guide_handoff(request_id));
}

#[test]
fn acp_history_replay_modes_expose_stable_keys() {
    let cases = [
        (AcpHistoryReplayMode::Discard, "discard"),
        (AcpHistoryReplayMode::Full, "full"),
        (AcpHistoryReplayMode::Summary, "summary"),
        (AcpHistoryReplayMode::Recent, "recent"),
    ];

    for (mode, key) in cases {
        assert_eq!(mode.as_str(), key);
    }
}

#[test]
fn acp_history_replay_mode_parsing_normalizes_input_and_rejects_unknown_values() {
    assert_eq!(AcpHistoryReplayMode::from_str(" DISCARD "), AcpHistoryReplayMode::Discard);
    assert_eq!(AcpHistoryReplayMode::from_str("Full"), AcpHistoryReplayMode::Full);
    assert_eq!(AcpHistoryReplayMode::from_str("summary"), AcpHistoryReplayMode::Summary);
    assert_eq!(AcpHistoryReplayMode::from_str("RECENT"), AcpHistoryReplayMode::Recent);
    assert_eq!(AcpHistoryReplayMode::from_str("unsupported"), AcpHistoryReplayMode::Discard);
    assert_eq!(AcpHistoryReplayMode::from_str("   "), AcpHistoryReplayMode::Discard);
}

#[test]
fn chat_send_behaviors_expose_stable_metadata() {
    let cases = [
        (ChatSendBehavior::Queue, "queue", "添加到队列", "发送到队列，保持当前排队逻辑。"),
        (
            ChatSendBehavior::StopAndSend,
            "stop_send",
            "停止并发送",
            "立刻停止当前请求，并开始发送。",
        ),
        (ChatSendBehavior::Guide, "guide", "通过消息引导", "等待当前一轮工具完成，下一轮请求前。"),
    ];

    for (behavior, key, label, description) in cases {
        assert_eq!(behavior.as_str(), key);
        assert_eq!(behavior.label(), label);
        assert_eq!(behavior.description(), description);
    }
}

#[test]
fn chat_send_behavior_parsing_accepts_stop_aliases_and_defaults_to_queue() {
    for value in ["stop_send", "stop-and-send", "stop_and_send", "stop", " STOP "] {
        assert_eq!(ChatSendBehavior::from_str(value), ChatSendBehavior::StopAndSend);
    }
    assert_eq!(ChatSendBehavior::from_str(" GUIDE "), ChatSendBehavior::Guide);
    assert_eq!(ChatSendBehavior::from_str("queue"), ChatSendBehavior::Queue);
    assert_eq!(ChatSendBehavior::from_str("unsupported"), ChatSendBehavior::Queue);
    assert_eq!(ChatSendBehavior::from_str(""), ChatSendBehavior::Queue);
}

#[test]
fn step_ui_meta_uses_finished_time_for_completed_step() {
    let step = session_step(Some(1_250));

    let meta = StepUiMeta::from(&step);

    assert_eq!(meta.model.as_deref(), Some("model-a"));
    assert_eq!(meta.started_ms, 1_000);
    assert_eq!(meta.finished_ms, Some(1_250));
    assert_eq!(meta.display_time_ms, 1_250);
}

#[test]
fn step_ui_meta_uses_started_time_for_unfinished_step() {
    let step = session_step(None);

    let meta = StepUiMeta::from(&step);

    assert_eq!(meta.finished_ms, None);
    assert_eq!(meta.display_time_ms, 1_000);
}

#[test]
fn step_ui_meta_preserves_missing_model_without_affecting_display_time() {
    let mut step = session_step(Some(1_500));
    step.model = None;

    let meta = StepUiMeta::from(&step);

    assert_eq!(meta.model, None);
    assert_eq!(meta.started_ms, 1_000);
    assert_eq!(meta.display_time_ms, 1_500);
}

#[test]
fn session_runtime_with_defaults_preserves_model_and_initializes_idle_state() {
    let runtime = SessionRuntimeState::with_defaults("model-b".to_string(), false);

    assert_eq!(runtime.model, "model-b");
    assert!(!runtime.auto_model);
    assert_eq!(runtime.task_mode_model, "model-b");
    assert!(!runtime.is_requesting);
    assert_eq!(runtime.request_started_ms, 0);
    assert_eq!(runtime.submit_anim, 0);
    assert!(!runtime.has_unseen_success);
    assert!(runtime.active_agent_request.is_none());
    assert!(runtime.queue.is_empty());
    assert_eq!(runtime.input_editor.text(), "");
    assert!(!runtime.task_mode_enabled);
    assert!(!runtime.workflow_mode_enabled);
    assert!(runtime.goal_mode_enabled);
    assert_eq!(runtime.task_mode_priority, "999");
    assert_eq!(runtime.task_mode_subtasks, ["", "", ""]);
    assert_eq!(runtime.task_mode_subtask_editors.len(), 3);
    assert!(runtime.task_mode_subtask_editors.iter().all(|editor| editor.text().is_empty()));
    assert_eq!(runtime.task_mode_task_split_mode, crate::app::task::TaskSplitMode::FollowSystem);
    assert_eq!(runtime.acp_history_mode, AcpHistoryReplayMode::Discard);
    assert_eq!(runtime.acp_recent_count, 3);
    assert!(runtime.full_access_enabled);
    assert!(!runtime.acp_rebuild_required);
}

#[test]
fn session_runtime_default_matches_new_auto_model_defaults() {
    let runtime = SessionRuntimeState::default();

    assert_eq!(runtime.model, "auto");
    assert!(runtime.auto_model);
    assert_eq!(runtime.task_mode_model, "auto");
    assert!(runtime.reasoning_effort.is_none());
    assert!(runtime.task_mode_executor.is_none());
    assert!(runtime.task_mode_reasoning_effort.is_none());
    assert!(runtime.agent.is_none());
    assert!(runtime.acp_agent.is_none());
    assert!(runtime.last_effective_acp_agent.is_none());
}

#[test]
fn session_runtime_defaults_goal_mode_enabled() {
    let runtime = SessionRuntimeState::default();
    assert!(runtime.goal_mode_enabled);
    assert!(!runtime.workflow_mode_enabled);

    let with_model = SessionRuntimeState::with_defaults("gpt-5".to_string(), true);
    assert!(with_model.goal_mode_enabled);
    assert!(!with_model.workflow_mode_enabled);
}
