use super::{
    TaskPetAvatarKind, TaskPetItem, TaskPetStatus, current_open_think_body, find_tool_start,
    newest_open_thinking_text, task_pet_compact_detail, task_pet_detail, task_pet_project_label,
    task_pet_title, task_pet_window_size_for_progress, truncate_for_pet, valid_think_tag_suffix,
};
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::{AcpHistoryReplayMode, AgentRequest, SessionRuntimeState};

fn app() -> App {
    App::new().0
}

fn chat_message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}

fn agent_request(id: u64, session: &str, query: &str, root: Option<&str>) -> AgentRequest {
    AgentRequest {
        id,
        session: session.to_string(),
        query: query.to_string(),
        root: root.map(str::to_string),
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    }
}

fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

#[test]
fn task_pet_title_uses_first_non_empty_line() {
    let title = task_pet_title("\n\n  修复网关重启配置丢失\n后续内容");

    assert_eq!(title, "修复网关重启配置丢失");
}

#[test]
fn truncate_for_pet_marks_long_text() {
    let value = truncate_for_pet("abcdefghijklmnopqrstuvwxyz", 5);

    assert_eq!(value, "abcde...");
}

#[test]
fn truncate_for_pet_preserves_unicode_at_character_boundary() {
    assert_eq!(truncate_for_pet("你好世界", 3), "你好世...");
    assert_eq!(truncate_for_pet("你好世界", 4), "你好世界");
}

#[test]
fn task_pet_detail_compacts_followup_lines_and_defaults_blank_input() {
    assert_eq!(
        task_pet_compact_detail(" heading \n detail one \n\n detail two "),
        "detail one detail two"
    );
    assert_eq!(task_pet_detail(" heading \n detail one \n detail two "), "detail one detail two");
    assert_eq!(task_pet_detail(" \n\t "), "思考中...");
}

#[test]
fn task_pet_project_label_rejects_blank_and_truncates_leaf_name() {
    assert_eq!(task_pet_project_label(None), None);
    assert_eq!(task_pet_project_label(Some(" \n ")), None);
    assert_eq!(task_pet_project_label(Some(" project-name ")).as_deref(), Some("project-name"));
    assert_eq!(
        task_pet_project_label(Some("abcdefghijklmnopqrs")).as_deref(),
        Some("abcdefghijklmnopqr...")
    );
}

#[test]
fn find_tool_start_requires_tool_at_line_start() {
    assert_eq!(find_tool_start("tool search"), Some(0));
    assert_eq!(find_tool_start("analysis\ntool search"), Some(9));
    assert_eq!(find_tool_start("analysis tool search"), None);
}

#[test]
fn think_tag_suffix_accepts_only_empty_or_whitespace_prefix() {
    assert!(valid_think_tag_suffix(""));
    assert!(valid_think_tag_suffix(" reason=\"work\""));
    assert!(!valid_think_tag_suffix("ing"));
}

#[test]
fn current_open_think_body_uses_new_empty_think() {
    let body = current_open_think_body("<think>旧思考</think>\n普通回复\n<think>");

    assert_eq!(body, Some(String::new()));
}

#[test]
fn current_open_think_body_stops_before_tool_output() {
    let body = current_open_think_body("<think>inspect state\ntool search\nresult");

    assert_eq!(body.as_deref(), Some("inspect state"));
}

#[test]
fn newest_open_thinking_text_ignores_visible_answer() {
    let messages = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>分析实现路径".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let detail = newest_open_thinking_text(messages.iter());

    assert_eq!(detail, Some("分析实现路径".to_string()));
}

#[test]
fn newest_open_thinking_text_returns_none_without_open_think() {
    let messages = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>分析完成</think>\n最终回答".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let detail = newest_open_thinking_text(messages.iter());

    assert_eq!(detail, None);
}

#[test]
fn newest_open_thinking_text_skips_user_and_uses_newest_open_assistant() {
    let messages = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "<think>older analysis".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: "<think>user text".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "<think>newest analysis".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert_eq!(newest_open_thinking_text(messages.iter()).as_deref(), Some("newest analysis"));
}

#[test]
fn task_pet_window_size_clamps_progress_and_eases_midpoint() {
    let collapsed = task_pet_window_size_for_progress(-1.0);
    let midpoint = task_pet_window_size_for_progress(0.5);
    let expanded = task_pet_window_size_for_progress(2.0);

    assert!((collapsed.width - 112.0).abs() < f32::EPSILON);
    assert!((collapsed.height - 104.0).abs() < f32::EPSILON);
    assert!((midpoint.width - 236.0).abs() < f32::EPSILON);
    assert!((midpoint.height - 225.0).abs() < f32::EPSILON);
    assert!((expanded.width - 360.0).abs() < f32::EPSILON);
    assert!((expanded.height - 346.0).abs() < f32::EPSILON);
}

#[test]
fn task_pet_avatar_next_cycles_all_variants() {
    assert_eq!(TaskPetAvatarKind::Robot.next(), TaskPetAvatarKind::Beauty);
    assert_eq!(TaskPetAvatarKind::Beauty.next(), TaskPetAvatarKind::Handsome);
    assert_eq!(TaskPetAvatarKind::Handsome.next(), TaskPetAvatarKind::Robot);
}

#[test]
fn sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items() {
    let mut app = app();
    app.session_runtime_states.clear();
    app.sessions.push(session_info("s1", "  Session Title  ", "/tmp/project-one"));
    app.active_session_id = Some("s1".to_string());
    app.chat = vec![chat_message(ChatRole::Assistant, "<think>正在检查文件\ntool read\nresult")];

    let mut runtime = SessionRuntimeState::default();
    runtime.is_requesting = true;
    runtime.active_agent_request =
        Some(agent_request(7, "s1", "fallback title", Some("/tmp/project-one")));
    app.session_runtime_states.insert("s1".to_string(), runtime);

    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_active_count(), 1);
    assert_eq!(app.task_pet_visible_count(), 1);
    assert_eq!(app.task_pet_items[0].request_id, 7);
    assert_eq!(app.task_pet_items[0].title, "Session Title");
    assert_eq!(app.task_pet_items[0].detail, "正在检查文件");
    assert_eq!(app.task_pet_items[0].project.as_deref(), Some("project-one"));

    let runtime = app.session_runtime_states.get_mut("s1").expect("runtime should exist");
    runtime.active_agent_request =
        Some(agent_request(8, "s1", "replacement query", Some("/tmp/project-two")));
    app.chat.clear();

    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_items.len(), 1);
    assert_eq!(app.task_pet_items[0].request_id, 8);
    assert_eq!(app.task_pet_items[0].detail, "正在检查文件");
    assert_eq!(app.task_pet_items[0].project.as_deref(), Some("project-two"));

    app.session_runtime_states.get_mut("s1").expect("runtime should exist").is_requesting = false;
    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_active_count(), 0);
    assert_eq!(app.task_pet_items[0].status, TaskPetStatus::Completed);

    let mut other = SessionRuntimeState::default();
    other.is_requesting = true;
    other.active_agent_request = Some(agent_request(10, "s2", "newer task", None));
    app.session_runtime_states.insert("s2".to_string(), other);
    app.sync_task_pet_from_runtime();

    assert_eq!(app.task_pet_items[0].request_id, 10);
    assert_eq!(app.task_pet_items[0].status, TaskPetStatus::Running);
    assert_eq!(app.task_pet_items[1].request_id, 8);
}

#[test]
fn task_pet_window_animation_drag_and_avatar_state_transitions() {
    let mut app = app();
    app.window_size = (500.0, 420.0);
    app.task_pet_position = iced::Point::new(300.0, 220.0);
    app.task_pet_expand_progress = 1.0;
    app.task_pet_collapsed = false;

    app.toggle_task_pet_collapsed();
    assert!(app.task_pet_collapsed);
    assert!(!app.task_pet_render_collapsed());

    let (size, position) = app.advance_task_pet_expand_animation().unwrap();
    assert!(size.width < 360.0);
    assert!(position.x > 300.0);
    assert!(app.task_pet_expand_target.is_some());

    app.task_pet_expand_progress = 0.1;
    app.task_pet_expand_target = Some(0.0);
    let _ = app.advance_task_pet_expand_animation().unwrap();
    assert_eq!(app.task_pet_expand_progress, 0.0);
    assert!(app.task_pet_expand_target.is_none());
    assert!(app.task_pet_render_collapsed());

    assert!(app.advance_task_pet_expand_animation().is_none());

    app.start_task_pet_drag();
    assert!(app.task_pet_dragging);
    app.drag_task_pet_to(100.0, 100.0);
    assert_eq!(app.task_pet_drag_anchor, Some(iced::Point::new(100.0, 100.0)));
    let before = app.task_pet_position;
    app.drag_task_pet_to(-200.0, 600.0);
    assert!(app.task_pet_position.x <= before.x);
    assert!(app.task_pet_facing_left());
    assert!(app.task_pet_is_walking());
    app.finish_task_pet_drag();
    assert!(!app.task_pet_dragging);
    assert!(app.task_pet_drag_anchor.is_none());

    app.set_task_pet_robot_hovered(true);
    assert!(app.task_pet_robot_hovered);
    app.cycle_task_pet_avatar();
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::Beauty);
}

#[test]
fn task_pet_reply_dismiss_refresh_and_completed_click_behaviors() {
    let mut app = app();
    app.task_pet_items = vec![
        TaskPetItem {
            request_id: 1,
            session_id: "s1".to_string(),
            title: "Old".to_string(),
            detail: String::new(),
            project: None,
            status: TaskPetStatus::Running,
            last_click_at: None,
        },
        TaskPetItem {
            request_id: 2,
            session_id: "s2".to_string(),
            title: "Done".to_string(),
            detail: "finished".to_string(),
            project: None,
            status: TaskPetStatus::Completed,
            last_click_at: None,
        },
    ];

    app.refresh_task_pet_session_title("s1", "\n Refreshed title");
    assert_eq!(app.task_pet_items[0].title, "Refreshed title");

    app.set_task_pet_hovered(Some(1));
    app.open_task_pet_reply(1);
    assert_eq!(app.task_pet_reply_request_id, Some(1));
    assert_eq!(app.task_pet_hovered_request_id, Some(1));
    app.update_task_pet_reply_input(" \n ".to_string());
    assert_eq!(app.take_task_pet_reply(), None);
    app.update_task_pet_reply_input(" continue please ".to_string());
    assert_eq!(app.take_task_pet_reply(), Some(("s1".to_string(), "continue please".to_string())));
    assert!(app.task_pet_reply_input.is_empty());

    app.open_task_pet_reply(1);
    app.update_task_pet_reply_input("draft".to_string());
    app.open_task_pet_reply(1);
    assert!(app.task_pet_reply_request_id.is_none());
    assert!(app.task_pet_reply_input.is_empty());

    assert_eq!(app.task_pet_item_clicked(99), None);
    assert_eq!(app.task_pet_item_clicked(1), Some("s1".to_string()));
    assert_eq!(app.task_pet_item_clicked(2), Some("s2".to_string()));
    assert_eq!(app.task_pet_item_clicked(2), None);
    assert!(app.task_pet_items.iter().all(|item| item.request_id != 2));

    app.open_task_pet_reply(1);
    app.update_task_pet_reply_input("draft".to_string());
    app.dismiss_task_pet_item(1);
    assert!(app.task_pet_dismissed_request_ids.contains(&1));
    assert!(app.task_pet_items.is_empty());
    assert!(app.task_pet_hovered_request_id.is_none());
    assert!(app.task_pet_reply_request_id.is_none());
    assert!(app.task_pet_reply_input.is_empty());
}
