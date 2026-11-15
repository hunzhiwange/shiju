//! 会话加载测试，验证共享消息转换、冷加载分块和历史预览生成。
//! 预览文本需处理空白、换行、长度边界与重复会话标识，同时保持原消息顺序。

use crate::app::models::{
    ChatMessage, ChatRole, TokenUsage,
};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session_info(id: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: "/workspace/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn cold_session_load_prepares_base_chunk_off_ui_thread() {
    let messages =
        (0..33).map(|index| message(ChatRole::Assistant, &format!("message {index}"))).collect();
    let prepared = super::prepare_session_open(
        "session-1".to_string(),
        messages,
        (0..33).map(|index| Some(format!("message-{index}"))).collect(),
        TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        20,
        None,
        false,
    );

    let chunk = prepared.base_ui_chunk.expect("first UI chunk should be prepared");
    assert_eq!(chunk.chunk_start_idx, 0);
    assert_eq!(chunk.chunk_end_idx, 32);
    assert_eq!(chunk.render_cache.len(), 32);
    assert_eq!(prepared.messages.len(), 33);
}

#[test]
fn reload_sessions_for_project_clears_current_session_state_before_loading() {
    let (mut app, _startup_task) = crate::app::App::new();
    app.session_previews.insert("session-1".to_string(), "preview".to_string());
    app.archived_session_ids.insert("session-1".to_string());
    app.sessions.push(session_info("session-1"));
    app.active_session_id = Some("session-1".to_string());
    app.chat.push(message(ChatRole::User, "stale message"));
    app.usage =
        TokenUsage { input_tokens: 10, output_tokens: 20, cached_tokens: 3, reasoning_tokens: 4 };
    app.chat_visible_text_cache.push(Some("stale text".to_string()));
    app.chat_message_expanded.insert(0);

    let _reload_task = app.reload_sessions_for_project(None);

    assert!(app.session_previews.is_empty());
    assert!(app.archived_session_ids.is_empty());
    assert!(app.sessions.is_empty());
    assert_eq!(app.active_session_id, None);
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, TokenUsage::default());
    assert!(app.chat_visible_text_cache.is_empty());
    assert!(app.chat_message_expanded.is_empty());
}

#[test]
fn reload_sessions_for_project_with_path_returns_runnable_task() {
    let (mut app, _startup_task) = crate::app::App::new();
    let project_path = "/workspace/project".to_string();

    let reload_task = app.reload_sessions_for_project(Some(project_path.clone()));

    assert!(iced_runtime::task::into_stream(reload_task.collect()).is_some());
    assert!(app.project_sessions_loading.contains(&project_path));
}
use session_ui_types::SessionMessageRecord;

fn bounded_summary(message_count: usize) -> vw_api_session::GatewaySessionUiSummary {
    vw_api_session::GatewaySessionUiSummary {
        agent_binding: None,
        full_access: None,
        id: "session-page".to_string(),
        title: "Paged".to_string(),
        updated_ms: 42,
        message_count,
        call_count: 0,
        last_content: None,
    }
}

fn page_record(seq_no: u64) -> SessionMessageRecord {
    SessionMessageRecord {
        seq_no,
        message_id: Some(format!("message-{seq_no}")),
        message: ChatMessage {
            role: session_ui_types::ChatRole::User,
            content: format!("message {seq_no}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    }
}

#[test]
fn session_open_page_keeps_initial_history_bounded() {
    let page = vw_api_session::GatewaySessionMessagePage {
        items: (9_937..=10_000).map(page_record).collect(),
        next_cursor: Some("older-page".to_string()),
        previous_cursor: None,
        total_count: 10_000,
    };

    let prepared = super::prepare_bounded_session_open(bounded_summary(10_000), page, false);

    assert_eq!(prepared.messages.len(), 64);
    let timeline = prepared.timeline_page.expect("bounded open should carry timeline page");
    assert_eq!(timeline.entries.len(), 64);
    assert_eq!(timeline.total_count, 10_000);
    assert_eq!(timeline.before_cursor.as_deref(), Some("older-page"));
}

#[test]
fn session_open_page_becomes_ready_from_latest_page_without_heavy_state() {
    let page = vw_api_session::GatewaySessionMessagePage {
        items: vec![page_record(10_000)],
        next_cursor: None,
        previous_cursor: None,
        total_count: 10_000,
    };

    let prepared = super::prepare_bounded_session_open(bounded_summary(10_000), page, false);

    assert!(prepared.base_ui_chunk.is_some());
    assert!(prepared.steps.is_empty());
    assert!(prepared.calls.is_empty());
    assert_eq!(prepared.updated_ms, 42);
}

#[test]
fn agent_unified_selection_bounded_load_preserves_trusted_identity() {
    let mut summary = bounded_summary(0);
    summary.agent_binding = Some(vw_api_chat_identity::SessionAgentIdentity {
        agent: "acp-codex".into(),
        label: Some("编程助手".into()),
        acp_agent: Some("codex".into()),
    });
    let prepared = super::prepare_bounded_session_open(
        summary,
        vw_api_session::GatewaySessionMessagePage {
            items: vec![],
            next_cursor: None,
            previous_cursor: None,
            total_count: 0,
        },
        false,
    );
    assert_eq!(prepared.agent_binding.unwrap().acp_agent.as_deref(), Some("codex"));
}
