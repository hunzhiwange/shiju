//! 覆盖 `app.rs` 中的纯函数和枚举方法。
//! 测试从函数签名出发验证枚举 match 分支、状态转换与格式输出。

use std::path::Path;

use serde_json::json;

use super::app::{
    TodoSessionAccessAction, TuiRunMode, bootstrap_message_base, bootstrap_system_message,
    build_exit_confirm_body, comparable_terminal_from_runtime_terminal,
    comparable_usage_from_runtime_terminal, compare_shadow_results, default_session_title,
    empty_session_snapshot, gateway_message_from_chat_message, gateway_role,
    is_session_unavailable_error, latest_session_preview, mcp_server_entry_to_info,
    normalize_catalog_text, parse_mcp_servers_json, provider_name_from_model,
    runtime_event_fallback_overlay, select_restore_session_id, shadow_compare_success_message,
    snapshot_from_preview,
};
use super::model::PromptSubmission;
use super::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::state::{TuiAction, TuiState, reduce_tui_state};
use crate::cli::processor::{SessionProcessorComparableResult, SessionProcessorComparableTerminal};
use vw_gateway_client::GatewayChatUsage;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, TokenUsage,
};

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
                think_timing: Vec::new(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "回答".to_string(),
                think_timing: Vec::new(),
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
        think_timing: Vec::new(),
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
        think_timing: Vec::new(),
    };
    let value = gateway_message_from_chat_message(&msg);
    assert_eq!(value["role"], "assistant");
    assert_eq!(value["content"], "response");
}

#[test]
fn gateway_message_from_chat_message_maps_system_and_tool_content() {
    for (role, expected) in [(ChatRole::System, "system"), (ChatRole::Tool, "tool")] {
        let message =
            ChatMessage { role, content: "payload".to_string(), think_timing: Vec::new() };

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
