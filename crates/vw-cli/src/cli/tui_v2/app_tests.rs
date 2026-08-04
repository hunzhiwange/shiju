//! 覆盖 `app.rs` 中的纯函数和枚举方法。
//! 测试从函数签名出发验证枚举 match 分支、状态转换与格式输出。

use std::cell::RefCell;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::rc::Rc;

use serde_json::json;

use super::app::{
    TodoSessionAccessAction, TuiApp, TuiRunMode, bootstrap_message_base, bootstrap_system_message,
    apply_settings_values_to_state,
    build_exit_confirm_body, comparable_terminal_from_runtime_terminal,
    comparable_usage_from_runtime_terminal, compare_shadow_results, default_session_title,
    delegate_agent_options, empty_session_snapshot, gateway_message_from_chat_message, gateway_role,
    gateway_stream_request_from_state, is_session_unavailable_error, latest_session_preview,
    mcp_server_entry_to_info,
    normalize_catalog_text, parse_mcp_servers_json, provider_name_from_model,
    runtime_event_fallback_overlay, select_restore_session_id, shadow_compare_success_message,
    snapshot_from_preview,
};
use super::commands::settings::SettingsRuntime;
use super::controller::build_prompt_submission;
use super::editor::EditorSelection;
use super::model::permission::PermissionRequest;
use super::model::settings_panel::{SettingsPanel, SettingsValues};
use super::model::{
    PromptSubmission, QueueDrainBoundary, QueuedPromptCommand, QueuedPromptPriority, UiMessage,
    UiOverlay, UiSystemMessageLevel,
};
use super::runtime::permission::FullAccessPermissionRuntime;
use super::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::state::{TuiAction, TuiState, reduce_tui_state};
use super::terminal_capabilities::{TerminalModePlan, TerminalModeSession};
use crate::cli::processor::{SessionProcessorComparableResult, SessionProcessorComparableTerminal};
use crate::app::agent::config::{Config, DelegateAgentConfig};
use vw_gateway_client::GatewayChatUsage;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, TokenUsage,
};

#[derive(Clone, Default)]
struct SharedRecordingWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SharedRecordingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingClipboardWriter;

impl Write for FailingClipboardWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "clipboard writer closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SharedRecordingWriter {
    fn output(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("terminal control output is UTF-8")
    }
}

#[derive(Default)]
struct FullAccessWorkflowRuntime {
    events: Vec<&'static str>,
    pending_error: Option<String>,
    save_error: Option<String>,
}

impl SettingsRuntime for FullAccessWorkflowRuntime {
    fn save(&mut self, _values: &SettingsValues) -> Result<(), String> {
        self.events.push("save");
        match &self.save_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

impl FullAccessPermissionRuntime for FullAccessWorkflowRuntime {
    fn pending_for_current_directory(&mut self) -> Result<Vec<PermissionRequest>, String> {
        self.events.push("pending");
        match &self.pending_error {
            Some(error) => Err(error.clone()),
            None => Ok(Vec::new()),
        }
    }

    fn approve(
        &mut self,
        _request_id: &str,
        _reply: vw_gateway_client::PendingPermissionReplyDto,
    ) -> Result<(), String> {
        self.events.push("reply");
        Ok(())
    }
}

struct DropRestoredTerminal {
    session: TerminalModeSession,
    writer: SharedRecordingWriter,
}

impl Drop for DropRestoredTerminal {
    fn drop(&mut self) {
        self.session.leave(&mut self.writer).expect("panic drop restores terminal modes");
    }
}

fn assert_fullscreen_mode_pairs(output: &str, pair_count: usize) {
    for sequence in ["\u{1b}[?1049h", "\u{1b}[?1000h", "\u{1b}[?2004h", "\u{1b}[?1004h"] {
        assert_eq!(
            output.matches(sequence).count(),
            pair_count,
            "missing or duplicated enter sequence {sequence:?}"
        );
    }
    for sequence in ["\u{1b}[?1004l", "\u{1b}[?2004l", "\u{1b}[?1000l", "\u{1b}[?1049l"] {
        assert_eq!(
            output.matches(sequence).count(),
            pair_count,
            "missing or duplicated leave sequence {sequence:?}"
        );
    }

    let focus_off = output.rfind("\u{1b}[?1004l").expect("focus disabled");
    let paste_off = output.rfind("\u{1b}[?2004l").expect("bracketed paste disabled");
    let mouse_off = output.rfind("\u{1b}[?1000l").expect("mouse disabled");
    let alternate_off = output.rfind("\u{1b}[?1049l").expect("alternate screen left");
    assert!(focus_off < paste_off && paste_off < mouse_off && mouse_off < alternate_off);
}

#[test]
fn full_access_commit_is_not_rolled_back_by_pending_service_failure() {
    let mut panel = SettingsPanel::new(SettingsValues::default());
    panel.pending.full_access_enabled = true;
    let mut offline_runtime = FullAccessWorkflowRuntime {
        pending_error: Some("permission service offline".to_string()),
        ..FullAccessWorkflowRuntime::default()
    };

    let summary = TuiApp::commit_full_access_and_drain(&mut panel, &mut offline_runtime)
        .expect("pending service failure is a recoverable drain warning");
    assert!(panel.original.full_access_enabled, "committed full access remains enabled");
    assert_eq!(summary.approved, 0);
    assert_eq!(summary.failed, 0);
    assert!(summary.warning.as_deref().is_some_and(|warning| warning.contains("offline")));
    assert_eq!(offline_runtime.events, vec!["save", "pending"]);

    let mut rejected_panel = SettingsPanel::new(SettingsValues::default());
    rejected_panel.pending.full_access_enabled = true;
    let mut rejected_runtime = FullAccessWorkflowRuntime {
        save_error: Some("settings write failed".to_string()),
        ..FullAccessWorkflowRuntime::default()
    };
    assert!(
        TuiApp::commit_full_access_and_drain(&mut rejected_panel, &mut rejected_runtime).is_err()
    );
    assert!(!rejected_panel.original.full_access_enabled);
    assert_eq!(
        rejected_runtime.events,
        vec!["save"],
        "a failed settings commit must not approve pending requests"
    );
}

#[test]
fn delegate_agent_catalog_keeps_main_enabled_and_current_disabled_only() {
    let mut config = Config::default();
    config.agents.insert(
        "reviewer".to_string(),
        DelegateAgentConfig {
            label: Some("Code Reviewer".to_string()),
            enabled: true,
            ..DelegateAgentConfig::default()
        },
    );
    config.agents.insert(
        "disabled".to_string(),
        DelegateAgentConfig { enabled: false, ..DelegateAgentConfig::default() },
    );
    config.agents.insert(
        "current-disabled".to_string(),
        DelegateAgentConfig {
            label: Some("Current Disabled".to_string()),
            enabled: false,
            ..DelegateAgentConfig::default()
        },
    );
    config.agents.insert(
        "hidden".to_string(),
        DelegateAgentConfig {
            enabled: true,
            hidden: true,
            ..DelegateAgentConfig::default()
        },
    );

    let catalog = delegate_agent_options(&config, Some("current-disabled"));
    assert_eq!(catalog[0].key, None);
    assert_eq!(catalog[0].label, "main");
    assert!(catalog.iter().any(|agent| agent.key.as_deref() == Some("reviewer")));
    assert!(
        catalog.iter().any(|agent| agent.key.as_deref() == Some("current-disabled"))
    );
    assert!(!catalog.iter().any(|agent| agent.key.as_deref() == Some("disabled")));
    assert!(!catalog.iter().any(|agent| agent.key.as_deref() == Some("hidden")));
}

#[test]
fn tui_terminal_lifecycle_restores_modes_on_all_exit_paths() {
    let plan = TerminalModePlan::for_environment(true, Some("xterm-256color"), Some("WezTerm"));

    let mut normal = Vec::new();
    let mut normal_session = TerminalModeSession::new(plan);
    normal_session.enter(&mut normal).expect("normal enter");
    normal_session.leave(&mut normal).expect("normal leave");
    normal_session.leave(&mut normal).expect("repeated leave is idempotent");
    assert_fullscreen_mode_pairs(&String::from_utf8(normal).expect("normal output"), 1);

    let mut error = Vec::new();
    let mut error_session = TerminalModeSession::new(plan);
    error_session.enter(&mut error).expect("error-path enter");
    error_session.leave(&mut error).expect("business error still restores modes");
    assert_fullscreen_mode_pairs(&String::from_utf8(error).expect("error output"), 1);

    let panic_writer = SharedRecordingWriter::default();
    let panic_capture = panic_writer.clone();
    let panic_result = catch_unwind(AssertUnwindSafe(move || {
        let mut lifecycle =
            DropRestoredTerminal { session: TerminalModeSession::new(plan), writer: panic_writer };
        lifecycle.session.enter(&mut lifecycle.writer).expect("panic-path enter");
        panic!("simulated application panic");
    }));
    assert!(panic_result.is_err());
    assert_fullscreen_mode_pairs(&panic_capture.output(), 1);

    let mut suspend_resume = Vec::new();
    let mut suspended_session = TerminalModeSession::new(plan);
    suspended_session.enter(&mut suspend_resume).expect("initial enter");
    suspended_session.suspend(&mut suspend_resume).expect("SIGTSTP restore");
    suspended_session.resume(&mut suspend_resume).expect("SIGCONT re-enter");
    suspended_session.leave(&mut suspend_resume).expect("final leave");
    assert_fullscreen_mode_pairs(
        &String::from_utf8(suspend_resume).expect("suspend/resume output"),
        2,
    );

    for degraded_plan in [
        TerminalModePlan::for_environment(false, Some("xterm-256color"), Some("WezTerm")),
        TerminalModePlan::for_environment(true, Some("dumb"), Some("WezTerm")),
    ] {
        let mut output = Vec::new();
        let mut session = TerminalModeSession::new(degraded_plan);
        session.enter(&mut output).expect("scrollback enter");
        session.leave(&mut output).expect("scrollback leave");
        assert!(output.is_empty(), "non-TTY/dumb terminals must not receive fullscreen modes");
    }
}

#[test]
fn tui_terminal_lifecycle_enables_shift_enter_in_integrated_terminals() {
    let plan =
        TerminalModePlan::for_environment(true, Some("xterm-256color"), Some("unknown-terminal"));
    let mut output = Vec::new();
    let mut session = TerminalModeSession::new(plan);

    session.enter(&mut output).expect("integrated terminal enter");
    session.leave(&mut output).expect("integrated terminal leave");

    let output = String::from_utf8(output).expect("integrated terminal output");
    assert_eq!(output.matches("\u{1b}[>3u").count(), 1, "keyboard flags must be pushed once");
    assert_eq!(output.matches("\u{1b}[<1u").count(), 1, "keyboard flags must be popped once");

    for degraded_plan in [
        TerminalModePlan::for_environment(false, Some("xterm-256color"), Some("vscode")),
        TerminalModePlan::for_environment(true, Some("dumb"), Some("vscode")),
    ] {
        let mut degraded_output = Vec::new();
        let mut degraded_session = TerminalModeSession::new(degraded_plan);
        degraded_session.enter(&mut degraded_output).expect("degraded terminal enter");
        degraded_session.leave(&mut degraded_output).expect("degraded terminal leave");
        assert!(
            degraded_output.is_empty(),
            "non-TTY/dumb terminals must not receive control sequences"
        );
    }
}

#[test]
fn prompt_selection_copy_feedback_preserves_draft_and_selection() {
    let mut state = TuiState::default();
    reduce_tui_state(
        &mut state,
        TuiAction::PromptValueSet("中e\u{301}🙂\n下一行".to_string()),
    );
    state.prompt.editor.selection = Some(EditorSelection { anchor: 1, active: 6 });
    let prompt_before = state.prompt.clone();
    let mut output = Vec::new();

    TuiApp::copy_prompt_selection_to_writer(&mut state, &mut output, "e\u{301}🙂\n下一");

    assert_eq!(
        output,
        b"\x1b]52;c;ZcyB8J+ZggrkuIvkuIA=\x07"
    );
    assert_eq!(state.prompt, prompt_before);
    assert!(matches!(
        state.messages.last(),
        Some(UiMessage::System(message))
            if message.level == UiSystemMessageLevel::Success
                && message.text == "已复制输入选区。"
    ));

    let mut failed_state = TuiState::default();
    reduce_tui_state(
        &mut failed_state,
        TuiAction::PromptValueSet("保留草稿".to_string()),
    );
    failed_state.prompt.editor.selection = Some(EditorSelection { anchor: 0, active: 2 });
    let failed_prompt_before = failed_state.prompt.clone();
    TuiApp::copy_prompt_selection_to_writer(
        &mut failed_state,
        &mut FailingClipboardWriter,
        "保留",
    );

    assert_eq!(failed_state.prompt, failed_prompt_before);
    assert!(matches!(
        failed_state.overlays.active(),
        Some(UiOverlay::Error(error))
            if error.recoverable
                && error.title == "复制失败"
                && error.message.contains("输入草稿和选区保持不变")
    ));
}

// ── TuiRunMode ──────────────────────────────────────────────────────────

#[test]
fn tui_run_mode_badge_label_standard() {
    assert_eq!(TuiRunMode::Standard.badge_label(), "TUI");
}

#[test]
fn tui_run_mode_shadow_compare_disabled() {
    assert!(!TuiRunMode::Standard.shadow_compare_enabled());
}

// ── TodoSessionAccessAction ─────────────────────────────────────────────

#[test]
fn todo_session_access_action_overlay_title_all_variants() {
    assert_eq!(TodoSessionAccessAction::OpenPanel.overlay_title(), "待办面板不可用");
    assert_eq!(TodoSessionAccessAction::Refresh.overlay_title(), "待办刷新失败");
    assert_eq!(TodoSessionAccessAction::Save.overlay_title(), "待办保存失败");
}

#[test]
fn todo_session_access_action_action_label_all_variants() {
    assert_eq!(TodoSessionAccessAction::OpenPanel.action_label(), "打开待办面板");
    assert_eq!(TodoSessionAccessAction::Refresh.action_label(), "刷新当前会话的待办列表");
    assert_eq!(TodoSessionAccessAction::Save.action_label(), "保存当前会话的待办列表");
}

#[test]
fn session_unavailable_error_matches_markers_case_insensitively() {
    assert!(is_session_unavailable_error("  SESSION NOT FOUND: stale binding  "));
    assert!(is_session_unavailable_error("gateway requires a non-empty session id"));
    assert!(!is_session_unavailable_error(""));
    assert!(!is_session_unavailable_error("gateway connection failed"));
}

// ── build_exit_confirm_body ─────────────────────────────────────────────

#[test]
fn build_exit_confirm_body_without_session_and_messages() {
    let state = TuiState::default();
    let body = build_exit_confirm_body(&state);

    assert!(body.contains("确认离开 TUI 并返回 shell 吗？"));
    assert!(body.contains("Enter 确认退出  Esc 取消"));
    // 没有 session_id → 不显示会话 ID
    assert!(!body.contains("会话 ID"));
    // 没有 user/assistant 消息 → 不显示消息数
    assert!(!body.contains("消息数"));
    // 不 busy → 不显示进行中警告
    assert!(!body.contains("请求正在进行"));
}

#[test]
fn build_exit_confirm_body_with_short_session_id() {
    let mut state = TuiState::default();
    state.session.session_id = Some("abc".to_string());

    let body = build_exit_confirm_body(&state);
    assert!(body.contains("会话 ID  : abc"));
}

#[test]
fn build_exit_confirm_body_with_long_session_id() {
    let mut state = TuiState::default();
    state.session.session_id = Some("abcdefghijklmnopqrstuvwxyz".to_string());

    let body = build_exit_confirm_body(&state);
    // 长 ID 应被截断
    assert!(body.contains("abcdefghijklmnopqrst…"));
}

#[test]
fn build_exit_confirm_body_with_messages() {
    let state = TuiState::from_chat_session(&ChatSession {
        id: "session-msg".to_string(),
        title: "Msg Test".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "问题".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "回答".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("u1".to_string()), Some("a1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 100,
        updated_ms: 200,
    });

    let body = build_exit_confirm_body(&state);
    assert!(body.contains("消息数   : 2 条"));
}

#[test]
fn build_exit_confirm_body_with_busy_prompt() {
    let mut state = TuiState::default();
    state.session.session_id = Some("busy-session".to_string());
    reduce_tui_state(
        &mut state,
        TuiAction::PromptSubmissionStarted(
            PromptSubmission::new("当前请求").with_stream_id(1).with_model("gpt-5.4"),
        ),
    );

    let body = build_exit_confirm_body(&state);
    assert!(body.contains("⚠"));
    assert!(body.contains("请求正在进行"));
}

// ── latest_session_preview ──────────────────────────────────────────────

#[test]
fn latest_session_preview_returns_newest_by_updated_ms() {
    let previews = vec![
        ChatSessionMeta {
            id: "old".to_string(),
            title: "Older".to_string(),
            updated_ms: 10,
            message_count: 0,
            call_count: 0,
            last_content: None,
        },
        ChatSessionMeta {
            id: "new".to_string(),
            title: "Newer".to_string(),
            updated_ms: 20,
            message_count: 0,
            call_count: 0,
            last_content: None,
        },
    ];

    let result = latest_session_preview(&previews);
    assert_eq!(result.map(|p| p.id.as_str()), Some("new"));
}

#[test]
fn latest_session_preview_tiebreaks_by_message_count_then_call_count_then_id() {
    let previews = vec![
        ChatSessionMeta {
            id: "session_a".to_string(),
            title: String::new(),
            updated_ms: 100,
            message_count: 5,
            call_count: 2,
            last_content: None,
        },
        ChatSessionMeta {
            id: "session_b".to_string(),
            title: String::new(),
            updated_ms: 100,
            message_count: 10,
            call_count: 1,
            last_content: None,
        },
    ];

    // session_b 有更多消息 → 应优先
    let result = latest_session_preview(&previews);
    assert_eq!(result.map(|p| p.id.as_str()), Some("session_b"));
}

#[test]
fn latest_session_preview_tiebreaks_same_message_count_by_call_count() {
    let previews = vec![
        ChatSessionMeta {
            id: "low_calls".to_string(),
            title: String::new(),
            updated_ms: 50,
            message_count: 3,
            call_count: 0,
            last_content: None,
        },
        ChatSessionMeta {
            id: "high_calls".to_string(),
            title: String::new(),
            updated_ms: 50,
            message_count: 3,
            call_count: 5,
            last_content: None,
        },
    ];

    let result = latest_session_preview(&previews);
    assert_eq!(result.map(|p| p.id.as_str()), Some("high_calls"));
}

#[test]
fn latest_session_preview_tiebreaks_same_by_id() {
    let previews = vec![
        ChatSessionMeta {
            id: "a_early".to_string(),
            title: String::new(),
            updated_ms: 50,
            message_count: 3,
            call_count: 0,
            last_content: None,
        },
        ChatSessionMeta {
            id: "b_later".to_string(),
            title: String::new(),
            updated_ms: 50,
            message_count: 3,
            call_count: 0,
            last_content: None,
        },
    ];

    let result = latest_session_preview(&previews);
    assert_eq!(result.map(|p| p.id.as_str()), Some("b_later"));
}

#[test]
fn latest_session_preview_empty_returns_none() {
    assert!(latest_session_preview(&[]).is_none());
}

#[test]
fn select_restore_session_id_ignores_blank_explicit_value() {
    let previews = vec![ChatSessionMeta {
        id: "latest".to_string(),
        title: "Latest".to_string(),
        updated_ms: 20,
        message_count: 1,
        call_count: 0,
        last_content: None,
    }];

    assert_eq!(select_restore_session_id(Some(" \t "), &previews), Some("latest".to_string()));
    assert_eq!(
        select_restore_session_id(Some(" explicit "), &previews),
        Some("explicit".to_string())
    );
}

// ── snapshot_from_preview ───────────────────────────────────────────────

#[test]
fn snapshot_from_preview_uses_preview_data() {
    let preview = ChatSessionMeta {
        id: "snap-1".to_string(),
        title: "Snapshot One".to_string(),
        updated_ms: 500,
        message_count: 7,
        call_count: 2,
        last_content: Some("hello".to_string()),
    };

    let snapshot = snapshot_from_preview(&preview);
    assert_eq!(snapshot.id, "snap-1");
    assert_eq!(snapshot.title, "Snapshot One");
    assert_eq!(snapshot.created_ms, 500);
    assert_eq!(snapshot.updated_ms, 500);
    assert!(snapshot.messages.is_empty());
    assert!(snapshot.message_ids.is_empty());
}

// ── empty_session_snapshot ──────────────────────────────────────────────

#[test]
fn empty_session_snapshot_uses_given_id() {
    let snapshot = empty_session_snapshot("empty-123");
    assert_eq!(snapshot.id, "empty-123");
    assert_eq!(snapshot.title, "");
    assert_eq!(snapshot.created_ms, 0);
    assert_eq!(snapshot.updated_ms, 0);
    assert!(snapshot.messages.is_empty());
}

// ── default_session_title ───────────────────────────────────────────────

#[test]
fn default_session_title_uses_directory_name() {
    let title = default_session_title(Path::new("/home/user/my-project"));
    assert_eq!(title, "TUI my-project");
}

#[test]
fn default_session_title_fallback_for_root() {
    let title = default_session_title(Path::new("/"));
    assert_eq!(title, "TUI 会话");
}

#[test]
fn default_session_title_fallback_for_empty_filename() {
    let title = default_session_title(Path::new(""));
    assert_eq!(title, "TUI 会话");
}

// ── bootstrap_message_base ──────────────────────────────────────────────

#[test]
fn bootstrap_message_base_has_local_prefix() {
    let base = bootstrap_message_base();
    assert_eq!(base.id.as_str(), "local:ui-bootstrap-system");
}

// ── bootstrap_system_message ────────────────────────────────────────────

#[test]
fn bootstrap_system_message_standard_is_chinese() {
    let msg = bootstrap_system_message(TuiRunMode::Standard);
    assert!(msg.contains("TUI 已启动"));
    assert!(msg.contains("F1"));
    assert!(msg.contains("F2"));
    assert!(msg.contains("F3"));
    assert!(msg.contains("F4"));
}

// ── provider_name_from_model ────────────────────────────────────────────

#[test]
fn provider_name_from_model_extracts_provider_from_slash() {
    assert_eq!(provider_name_from_model("openai/gpt-5.4").as_deref(), Some("openai"));
}

#[test]
fn provider_name_from_model_returns_none_without_slash() {
    assert!(provider_name_from_model("gpt-5.4").is_none());
}

#[test]
fn provider_name_from_model_returns_none_for_empty() {
    assert!(provider_name_from_model("").is_none());
    assert!(provider_name_from_model("  ").is_none());
}

// ── normalize_catalog_text ──────────────────────────────────────────────

#[test]
fn normalize_catalog_text_preserves_non_empty() {
    assert_eq!(normalize_catalog_text("GPT-4.1", "fallback"), "GPT-4.1");
}

#[test]
fn normalize_catalog_text_trims_whitespace() {
    assert_eq!(normalize_catalog_text("  model-a  ", "fallback"), "model-a");
}

#[test]
fn normalize_catalog_text_fallback_on_empty() {
    assert_eq!(normalize_catalog_text("", "fallback"), "fallback");
    assert_eq!(normalize_catalog_text("   ", "fallback"), "fallback");
}

// ── gateway_message_from_chat_message / gateway_role ────────────────────

#[test]
fn gateway_message_from_chat_message_user() {
    let msg = ChatMessage {
        role: ChatRole::User,
        content: "hello".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let value = gateway_message_from_chat_message(&msg);
    assert_eq!(value["role"], "user");
    assert_eq!(value["content"], "hello");
}

#[test]
fn gateway_message_from_chat_message_assistant() {
    let msg = ChatMessage {
        role: ChatRole::Assistant,
        content: "response".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let value = gateway_message_from_chat_message(&msg);
    assert_eq!(value["role"], "assistant");
    assert_eq!(value["content"], "response");
}

#[test]
fn gateway_message_from_chat_message_maps_system_and_tool_content() {
    for (role, expected) in [(ChatRole::System, "system"), (ChatRole::Tool, "tool")] {
        let message = ChatMessage {
            role,
            content: "payload".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        };

        assert_eq!(
            gateway_message_from_chat_message(&message),
            json!({
                "role": expected,
                "content": "payload",
            })
        );
    }
}

#[test]
fn gateway_role_maps_all_variants() {
    assert_eq!(gateway_role(ChatRole::User), "user");
    assert_eq!(gateway_role(ChatRole::Assistant), "assistant");
    assert_eq!(gateway_role(ChatRole::System), "system");
    assert_eq!(gateway_role(ChatRole::Tool), "tool");
}

// ── comparable_usage_from_runtime_terminal ──────────────────────────────

#[test]
fn comparable_usage_from_done_terminal() {
    let usage = GatewayChatUsage {
        input_tokens: 10,
        output_tokens: 20,
        cached_tokens: 5,
        reasoning_tokens: 3,
    };
    let terminal = UiRuntimeTerminalEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(usage.clone()),
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_usage_from_runtime_terminal(&terminal);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.cached_tokens, 5);
    assert_eq!(result.reasoning_tokens, 3);
}

#[test]
fn comparable_usage_from_cancelled_terminal() {
    let usage = GatewayChatUsage {
        input_tokens: 1,
        output_tokens: 2,
        cached_tokens: 0,
        reasoning_tokens: 0,
    };
    let terminal = UiRuntimeTerminalEvent::Cancelled {
        reason: Some("user cancelled".to_string()),
        usage: Some(usage),
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_usage_from_runtime_terminal(&terminal);
    assert_eq!(result.input_tokens, 1);
}

#[test]
fn comparable_usage_from_timedout_terminal() {
    let usage = GatewayChatUsage {
        input_tokens: 3,
        output_tokens: 4,
        cached_tokens: 1,
        reasoning_tokens: 2,
    };
    let terminal = UiRuntimeTerminalEvent::TimedOut {
        message: "timeout".to_string(),
        usage: Some(usage),
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_usage_from_runtime_terminal(&terminal);
    assert_eq!(result.input_tokens, 3);
}

#[test]
fn comparable_usage_from_done_terminal_no_usage() {
    let terminal = UiRuntimeTerminalEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: None,
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_usage_from_runtime_terminal(&terminal);
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
}

#[test]
fn comparable_usage_from_error_terminal_returns_default() {
    let terminal = UiRuntimeTerminalEvent::Error("network".to_string());
    let result = comparable_usage_from_runtime_terminal(&terminal);
    assert_eq!(result, TokenUsage::default());
}

#[test]
fn comparable_usage_from_done_terminal_with_none_usage_and_timed_out_without() {
    let done = UiRuntimeTerminalEvent::Done {
        finish_reason: None,
        usage: None,
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_usage_from_runtime_terminal(&done);
    assert_eq!(result.input_tokens, 0);
}

// ── comparable_terminal_from_runtime_terminal ───────────────────────────

#[test]
fn comparable_terminal_from_done() {
    let event = UiRuntimeTerminalEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: None,
        message_id: Some("msg-1".to_string()),
        parent_message_id: Some("parent-1".to_string()),
    };
    let result = comparable_terminal_from_runtime_terminal(&event);
    assert_eq!(
        result,
        SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: Some("msg-1".to_string()),
            parent_message_id: Some("parent-1".to_string()),
        }
    );
}

#[test]
fn comparable_terminal_from_cancelled() {
    let event = UiRuntimeTerminalEvent::Cancelled {
        reason: Some("user".to_string()),
        usage: None,
        message_id: None,
        parent_message_id: None,
    };
    let result = comparable_terminal_from_runtime_terminal(&event);
    assert_eq!(
        result,
        SessionProcessorComparableTerminal::Cancelled {
            reason: Some("user".to_string()),
            message_id: None,
            parent_message_id: None,
        }
    );
}

#[test]
fn comparable_terminal_from_timedout() {
    let event = UiRuntimeTerminalEvent::TimedOut {
        message: "deadline".to_string(),
        usage: None,
        message_id: Some("msg-t".to_string()),
        parent_message_id: Some("parent-t".to_string()),
    };
    let result = comparable_terminal_from_runtime_terminal(&event);
    assert_eq!(
        result,
        SessionProcessorComparableTerminal::TimedOut {
            message: "deadline".to_string(),
            message_id: Some("msg-t".to_string()),
            parent_message_id: Some("parent-t".to_string()),
        }
    );
}

#[test]
fn comparable_terminal_from_error() {
    let event = UiRuntimeTerminalEvent::Error("broken".to_string());
    let result = comparable_terminal_from_runtime_terminal(&event);
    assert_eq!(result, SessionProcessorComparableTerminal::Error("broken".to_string()));
}

#[test]
fn runtime_event_fallback_overlay_handles_generic_and_non_error_events() {
    let generic = runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(
        UiRuntimeTerminalEvent::Error("connection reset".to_string()),
    ))
    .expect("generic runtime error should surface an overlay");
    assert_eq!(generic.title, "输出失败");
    assert!(generic.message.contains("connection reset"));

    let unknown = runtime_event_fallback_overlay(&UiRuntimeEvent::Unknown { event_type: None })
        .expect("unknown event should surface a fallback overlay");
    assert!(unknown.message.contains("事件类型: unknown"));

    assert!(runtime_event_fallback_overlay(&UiRuntimeEvent::Delta("chunk".to_string())).is_none());
    assert!(
        runtime_event_fallback_overlay(&UiRuntimeEvent::ContextLoaded {
            paths: vec!["/work/AGENTS.md".to_string()],
        })
        .is_none()
    );
    assert!(runtime_event_fallback_overlay(&UiRuntimeEvent::PostToolRound).is_none());
    assert!(
        runtime_event_fallback_overlay(&UiRuntimeEvent::Terminal(
            UiRuntimeTerminalEvent::Cancelled {
                reason: None,
                usage: None,
                message_id: None,
                parent_message_id: None,
            },
        ))
        .is_none()
    );
}

// ── shadow_compare_success_message ──────────────────────────────────────

#[test]
fn shadow_compare_success_message_format() {
    let result = SessionProcessorComparableResult {
        output: "matched output".to_string(),
        usage: TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            cached_tokens: 5,
            reasoning_tokens: 0,
        },
        step_finishes: 2,
        terminal: SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: None,
            parent_message_id: None,
        },
    };
    let msg = shadow_compare_success_message(&result);
    assert!(msg.contains("Shadow compare matched legacy"));
    assert!(msg.contains("terminal=done"));
    assert!(msg.contains("steps=2"));
    assert!(msg.contains("output="));
}

#[test]
fn compare_shadow_results_reports_only_step_difference() {
    let gateway = SessionProcessorComparableResult {
        output: "same".to_string(),
        usage: TokenUsage::default(),
        step_finishes: 2,
        terminal: SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: None,
            parent_message_id: None,
        },
    };
    let legacy = SessionProcessorComparableResult { step_finishes: 1, ..gateway.clone() };

    assert_eq!(compare_shadow_results(&gateway, &legacy), Err("steps 2 != 1".to_string()));
}

// ── parse_mcp_servers_json / mcp_server_entry_to_info ───────────────────

#[test]
fn parse_mcp_servers_json_object_format() {
    let value = json!({
        "my-server": {
            "type": "stdio",
            "command": "my-tool"
        },
        "web-server": {
            "type": "http",
            "url": "https://example.com/mcp"
        },
        "sse-server": {
            "type": "sse",
            "url": "https://events.example.com/mcp"
        }
    });
    let servers = parse_mcp_servers_json(&value);
    assert_eq!(servers.len(), 3);

    let my = servers.iter().find(|s| s.name == "my-server").unwrap();
    assert_eq!(my.transport.label(), "stdio");
    assert_eq!(my.address, "my-tool");

    let web = servers.iter().find(|s| s.name == "web-server").unwrap();
    assert_eq!(web.transport.label(), "http");
    assert_eq!(web.address, "https://example.com/mcp");

    let sse = servers.iter().find(|s| s.name == "sse-server").unwrap();
    assert_eq!(sse.transport.label(), "sse");
    assert_eq!(sse.address, "https://events.example.com/mcp");
}

#[test]
fn parse_mcp_servers_json_array_format() {
    let value = json!([
        {
            "name": "server-a",
            "type": "stdio",
            "command": "cmd-a"
        },
        {
            "name": "server-b",
            "type": "http",
            "url": "http://localhost:8080/mcp"
        }
    ]);
    let servers = parse_mcp_servers_json(&value);
    assert_eq!(servers.len(), 2);
    assert!(servers.iter().any(|s| s.name == "server-a"));
    assert!(servers.iter().any(|s| s.name == "server-b"));
}

#[test]
fn parse_mcp_servers_json_array_missing_name() {
    let value = json!([
        {
            "type": "stdio",
            "command": "cmd"
        }
    ]);
    let servers = parse_mcp_servers_json(&value);
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "unknown");
}

#[test]
fn parse_mcp_servers_json_other_value_returns_empty() {
    assert!(parse_mcp_servers_json(&json!("string")).is_empty());
    assert!(parse_mcp_servers_json(&json!(42)).is_empty());
    assert!(parse_mcp_servers_json(&serde_json::Value::Null).is_empty());
}

#[test]
fn mcp_server_entry_to_info_http_transport_missing_url() {
    let server = json!({ "type": "http" });
    let info = mcp_server_entry_to_info("http-srv".to_string(), &server);
    assert_eq!(info.transport.label(), "http");
    assert_eq!(info.address, "");
}

#[test]
fn mcp_server_entry_to_info_sse_transport_missing_url() {
    let server = json!({ "type": "sse" });
    let info = mcp_server_entry_to_info("sse-srv".to_string(), &server);
    assert_eq!(info.transport.label(), "sse");
    assert_eq!(info.address, "");
}

#[test]
fn mcp_server_entry_to_info_stdio_transport_missing_command() {
    let server = json!({ "type": "stdio" });
    let info = mcp_server_entry_to_info("stdio-srv".to_string(), &server);
    assert_eq!(info.transport.label(), "stdio");
    assert_eq!(info.address, "");
}

#[test]
fn mcp_server_entry_to_info_default_transport_is_stdio() {
    let server = json!({ "command": "my-cmd" });
    let info = mcp_server_entry_to_info("default-srv".to_string(), &server);
    assert_eq!(info.transport.label(), "stdio");
    assert_eq!(info.address, "my-cmd");
}

#[test]
fn mcp_server_entry_to_info_unknown_transport_falls_back_to_stdio() {
    let server = json!({ "type": "websocket", "command": "bridge" });

    let info = mcp_server_entry_to_info("fallback-srv".to_string(), &server);

    assert_eq!(info.transport.label(), "stdio");
    assert_eq!(info.address, "bridge");
}

#[test]
fn tui_busy_queue_drains_priorities_and_preserves_editable_input() {
    let mut prompt = super::model::PromptState::new("local draft");
    assert!(prompt.stash_draft());
    assert!(!prompt.stash_draft(), "an existing stash is never overwritten");

    for (raw, priority, session_id) in [
        ("turn", QueuedPromptPriority::EndTurn, "session-a"),
        ("tool", QueuedPromptPriority::NextTool, "session-a"),
        ("now", QueuedPromptPriority::Now, "session-a"),
        ("foreign", QueuedPromptPriority::Now, "session-b"),
    ] {
        prompt.queue_command(QueuedPromptCommand {
            raw: raw.to_string(),
            priority,
            session_id: Some(session_id.to_string()),
            ..QueuedPromptCommand::default()
        });
    }

    assert_eq!(
        prompt
            .drain_queued_for(QueueDrainBoundary::Immediate, Some("session-a"))
            .map(|item| item.raw),
        Some("now".to_string())
    );
    assert_eq!(
        prompt
            .drain_queued_for(QueueDrainBoundary::Tool, Some("session-a"))
            .map(|item| item.raw),
        Some("tool".to_string())
    );
    assert_eq!(
        prompt
            .drain_queued_for(QueueDrainBoundary::Turn, Some("session-a"))
            .map(|item| item.raw),
        Some("turn".to_string())
    );
    assert_eq!(prompt.queued_commands[0].raw, "foreign");

    prompt.value.clear();
    assert!(prompt.restore_stashed_draft());
    assert_eq!(prompt.value, "local draft");
    let reclaimed = prompt
        .reclaim_latest_queued(Some("session-b"))
        .expect("foreign session item remains editable");
    assert_eq!(reclaimed.raw, "foreign");
}

#[test]
fn subagent_mode_requires_agent_tool_for_snapshot_request() {
    let mut state = TuiState::default();
    state.session_options.permission_mode = super::model::PermissionMode::Plan;
    state.session_options.set_subagent_mode(true);
    let mut queued = QueuedPromptCommand {
        raw: "delegate substantive work".to_string(),
        execution: Box::new(state.session_options.execution_snapshot()),
        ..QueuedPromptCommand::default()
    };
    state.session_options.set_subagent_mode(false);
    let mut submission = PromptSubmission::new(queued.raw.clone());
    submission.execution = std::mem::take(&mut queued.execution);
    let request = gateway_stream_request_from_state(&state, &submission);

    let tools = request.allowed_tools.as_ref().expect("subagent tool restriction");
    assert_eq!(tools.iter().filter(|tool| *tool == "AgentTool").count(), 1);
    assert!(tools.iter().any(|tool| tool == "todowrite"));
    let options = request.options.as_ref().expect("subagent options");
    assert_eq!(options["require_agent_tool"], true);
    assert_eq!(
        options["allowed_tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|tool| tool.as_str() == Some("AgentTool"))
            .count(),
        1
    );
    let system = request.system.as_ref().expect("subagent system constraint");
    assert_eq!(system.len(), 1);
    assert_eq!(system.iter().filter(|prompt| prompt.contains("AgentTool-First Mode")).count(), 1);

    let normal = PromptSubmission {
        execution: Box::new(state.session_options.execution_snapshot()),
        ..PromptSubmission::new("normal request")
    };
    let normal_request = gateway_stream_request_from_state(&state, &normal);
    assert_eq!(normal_request.allowed_tools, Some(vec!["todowrite".to_string()]));
    assert_eq!(normal_request.system, None);
    assert!(
        normal_request
            .options
            .as_ref()
            .is_none_or(|options| options.get("require_agent_tool").is_none())
    );
}

#[test]
fn model_and_acp_settings_update_state_and_next_request_snapshot() {
    let mut state = TuiState::default();
    state.project.workspace_root = Some("/tmp/tui-settings".into());
    let values = SettingsValues {
        model: Some("openai/gpt-5.6".to_string()),
        acp_backend: Some("codex".to_string()),
        full_access_enabled: state.session_options.full_access_enabled(),
        ..SettingsValues::default()
    };

    apply_settings_values_to_state(&mut state, &values);

    assert_eq!(state.session_options.model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(state.status.provider_name.as_deref(), Some("openai"));
    assert_eq!(state.status.model_name.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(state.session_options.acp_backend.as_deref(), Some("codex"));
    assert!(state.session_options.acp_force_new_session());

    let submission = build_prompt_submission(&state, "use the selected backend")
        .expect("settings produce a prompt submission");
    let request = gateway_stream_request_from_state(&state, &submission);
    assert_eq!(request.model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(
        request.options.expect("ACP request options")["acp_force_new_session"],
        true
    );
}
