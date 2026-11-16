#![allow(unused_must_use)]
use crate::message::chat::send_input::apply_inline_mode_command;
use crate::message::chat::session::InlineModeCommand;

#[test]
fn apply_inline_mode_command_toggles_task_mode_and_clears_runtime_editor() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    runtime.input_editor = iced::widget::text_editor::Content::with_text("draft");

    apply_inline_mode_command(&mut runtime, InlineModeCommand::TaskMode);

    assert!(runtime.task_mode_enabled);
    assert_eq!(runtime.input_editor.text(), "");

    apply_inline_mode_command(&mut runtime, InlineModeCommand::ChatMode);

    assert!(!runtime.task_mode_enabled);
}

use crate::message::chat::lifecycle::dispatch;
use crate::message::chat::request::{ChatSendBehavior, QueueAction, update_queue};
fn queue_item(
    query: &str,
    send_behavior: crate::message::chat::request::ChatSendBehavior,
) -> crate::message::chat::request::QueueItem {
    crate::message::chat::request::QueueItem {
        created_ms: 10,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::message::chat::request::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

#[test]
fn queue_messages_reorder_remove_and_ignore_out_of_bounds_indices() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    runtime.request_lifecycle.queue = vec![
        queue_item("first", ChatSendBehavior::Queue),
        queue_item("second", ChatSendBehavior::Queue),
        queue_item("third", ChatSendBehavior::Queue),
    ];

    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Up(2));
    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Up(0));
    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Down(0));
    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Down(9));
    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Remove(1));
    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Remove(9));

    let queries =
        runtime.request_lifecycle.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["third", "second"]);
}

#[test]
fn queue_copy_keeps_items_and_ignores_out_of_bounds_or_empty_queries() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    runtime.request_lifecycle.queue = vec![
        queue_item("copy me", ChatSendBehavior::Queue),
        queue_item("", ChatSendBehavior::Queue),
    ];

    let _ = update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Copy(0));
    let _ = update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Copy(1));
    let _ = update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Copy(9));

    let queries =
        runtime.request_lifecycle.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["copy me", ""]);
}

#[test]
fn submit_tick_resets_when_idle_and_cycles_when_requesting() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    runtime.request_lifecycle.submit_anim = 2;

    runtime.request_lifecycle.submit_tick(runtime.request_lifecycle.is_requesting);
    assert_eq!(runtime.request_lifecycle.submit_anim, 0);

    runtime.request_lifecycle.is_requesting = true;
    for expected in [1, 2, 0] {
        runtime.request_lifecycle.submit_tick(runtime.request_lifecycle.is_requesting);
        assert_eq!(runtime.request_lifecycle.submit_anim, expected);
    }
}

#[test]
fn dispatch_queue_item_appends_queue_requests_and_prepends_guidance() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    {
        runtime.request_lifecycle.is_requesting = true;
        runtime.input_editor = iced::widget::text_editor::Content::with_text("draft");
        runtime.request_lifecycle.queue.push(queue_item("existing", ChatSendBehavior::Queue));
    }

    dispatch(&mut runtime, queue_item("queued", ChatSendBehavior::Queue), true);
    dispatch(&mut runtime, queue_item("guided", ChatSendBehavior::Guide), true);

    let queries =
        runtime.request_lifecycle.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["guided", "existing", "queued"]);
    assert_eq!(runtime.input_editor.text(), "");
}

#[test]
fn gateway_endpoint_defaults_empty_host_to_loopback() {
    let mut host = "   ".to_string();
    let port = 4321;

    assert_eq!(
        crate::message::chat::session::gateway_endpoint(&host, port),
        ("127.0.0.1".to_string(), 4321)
    );

    host = "0.0.0.0".to_string();
    assert_eq!(
        crate::message::chat::session::gateway_endpoint(&host, port),
        ("0.0.0.0".to_string(), 4321)
    );

    host = "  gateway.local  ".to_string();
    assert_eq!(
        crate::message::chat::session::gateway_endpoint(&host, port),
        ("gateway.local".to_string(), 4321)
    );
}

fn send_runtime(runtime: &mut crate::session::agent::SessionRuntimeState<()>, child: bool) {
    use crate::message::chat::send_input::{SendInputEffect, prepare_send_input};
    let effect = prepare_send_input(
        runtime,
        &mut Vec::new(),
        Some("/repo".into()),
        None,
        child,
        ChatSendBehavior::Queue,
        100,
    );
    let SendInputEffect::Send(item) = effect else {
        panic!("expected request");
    };
    let prepared = crate::message::chat::lifecycle::prepare_request(
        runtime,
        item,
        &mut Vec::new(),
        &mut Vec::new(),
        100,
    );
    crate::message::chat::lifecycle::begin_request(
        runtime,
        prepared.item,
        1,
        "session".into(),
        prepared.request_query,
        prepared.request_history,
        prepared.is_draft,
        100,
        false,
    );
}
#[test]
fn send_current_session_input_disables_workflow_mode_for_plan_run_child_session() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    {
        runtime.input_editor = iced::widget::text_editor::Content::with_text("continue");
        runtime.workflow_mode_enabled = true;
        runtime.goal_mode_enabled = true;
    }

    send_runtime(&mut runtime, true);
    let request =
        runtime.request_lifecycle.active_agent_request.as_ref().expect("request should start");
    assert!(
        !request.workflow_mode_enabled,
        "child session send must not carry workflow_mode into plan-run"
    );
    assert!(
        !request.goal_mode_enabled,
        "child session send must not carry goal_mode into plan-run"
    );
    // UI 开关可仍为 true；发送路径强制隔离即可。
    assert!(runtime.workflow_mode_enabled);
    assert!(runtime.goal_mode_enabled);
}

#[test]
fn send_current_session_input_keeps_workflow_mode_for_root_session() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    {
        runtime.input_editor = iced::widget::text_editor::Content::with_text("plan this");
        runtime.workflow_mode_enabled = true;
    }

    send_runtime(&mut runtime, false);
    let request =
        runtime.request_lifecycle.active_agent_request.as_ref().expect("request should start");
    assert!(request.workflow_mode_enabled);
}

#[test]
fn send_current_session_input_keeps_workflow_mode_for_any_query_when_toggle_on() {
    for (_index, query) in [
        "这个模块是做什么的？",
        "为什么测试会失败",
        "解释一下当前工作流",
        "优化登录性能",
        "继续刚才的任务",
        "How does this workflow work?",
    ]
    .into_iter()
    .enumerate()
    {
        let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
        {
            runtime.input_editor = iced::widget::text_editor::Content::with_text(query);
            runtime.workflow_mode_enabled = true;
        }

        send_runtime(&mut runtime, false);
        let request =
            runtime.request_lifecycle.active_agent_request.as_ref().expect("request should start");
        assert!(
            request.workflow_mode_enabled,
            "workflow toggle must enter plan-run for any query: {query}"
        );
        assert!(runtime.workflow_mode_enabled, "sending must not turn off the workflow toggle");
    }
}

#[test]
fn send_current_session_input_keeps_explicit_actions_in_workflow_mode() {
    for (_index, query) in [
        "修复工作流重试问题",
        "请实现普通问答分流",
        "麻烦帮我修复工作流",
        "帮我重构这个模块",
        "把重试次数改成三次",
        "工作流需要重试失败子任务",
        "Fix the workflow retry bug",
        "Please implement workflow routing",
    ]
    .into_iter()
    .enumerate()
    {
        let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
        {
            runtime.input_editor = iced::widget::text_editor::Content::with_text(query);
            runtime.workflow_mode_enabled = true;
        }

        send_runtime(&mut runtime, false);
        let request =
            runtime.request_lifecycle.active_agent_request.as_ref().expect("request should start");
        assert!(request.workflow_mode_enabled, "explicit action must use plan-run: {query}");
    }
}

#[test]
fn send_current_session_input_keeps_goal_mode_for_root_session() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    {
        runtime.input_editor = iced::widget::text_editor::Content::with_text("reach this goal");
        runtime.goal_mode_enabled = true;
    }

    send_runtime(&mut runtime, false);
    let request =
        runtime.request_lifecycle.active_agent_request.as_ref().expect("request should start");
    assert!(request.goal_mode_enabled);
    assert!(!request.workflow_mode_enabled);
}

#[test]
fn restore_cached_chat_for_session_uses_matching_cached_message_ids() {
    let cached = vec![
        crate::test_messages::message(session_ui_types::ChatRole::User, "hi"),
        crate::test_messages::message(session_ui_types::ChatRole::Assistant, "hello"),
    ];
    let mut chat = Vec::new();
    let mut message_ids = Vec::new();
    crate::message::chat::lifecycle::restore_cached_chat(
        &mut chat,
        &mut message_ids,
        Some(&cached),
        Some(vec![Some("m1".to_string()), Some("m2".to_string())]),
    );
    assert_eq!(chat.len(), 2);
    assert_eq!(message_ids, vec![Some("m1".to_string()), Some("m2".to_string())]);
}

#[test]
fn restore_cached_chat_for_session_resets_ids_when_cached_ids_have_wrong_len() {
    let cached = vec![
        crate::test_messages::message(session_ui_types::ChatRole::User, "hi"),
        crate::test_messages::message(session_ui_types::ChatRole::Assistant, "hello"),
    ];
    let mut chat = Vec::new();
    let mut message_ids = Vec::new();
    crate::message::chat::lifecycle::restore_cached_chat(
        &mut chat,
        &mut message_ids,
        Some(&cached),
        Some(vec![Some("m1".to_string())]),
    );
    assert_eq!(chat.len(), 2);
    assert_eq!(message_ids, vec![None, None]);

    crate::message::chat::lifecycle::restore_cached_chat(&mut chat, &mut message_ids, None, None);
    assert!(chat.is_empty());
    assert!(message_ids.is_empty());
}

#[test]
fn expert_package_selection_is_consumed_once() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    let id = "skillhub--skillset--fixture-suite";
    {
        runtime.input_editor = iced::widget::text_editor::Content::with_text("review this");
        runtime.tool_selector.select_manual_skill(id);
    }
    let effect = crate::message::chat::send_input::prepare_send_input(
        &mut runtime,
        &mut Vec::new(),
        Some("/repo".into()),
        None,
        false,
        ChatSendBehavior::Queue,
        100,
    );
    let crate::message::chat::send_input::SendInputEffect::Send(item) = effect else {
        panic!("expected send");
    };
    assert!(item.query.contains(&format!("技能：{id}")));
    assert!(!item.query.contains("技能：skillhub--public--member"));
    assert!(runtime.tool_selector.selected_manual_skills().is_empty());
}

#[test]
fn tool_selector_messages_update_query_scope_group_and_manual_context() {
    use crate::message::chat::selection::{SelectionMessage, update};
    use desktop_chat_tools::state::{SessionToolGroup, SessionToolSelectorTab};
    use desktop_settings_skills::types::SkillsDirectoryScope;
    let mut selector = desktop_chat_tools::state::SessionToolSelectorState::default();
    let group = SessionToolGroup::Files;
    selector.select_tab(SessionToolSelectorTab::Tools);
    for message in [
        SelectionMessage::Search("  read  ".into()),
        SelectionMessage::Scope(SkillsDirectoryScope::Global),
        SelectionMessage::Collapse(group),
        SelectionMessage::ManualTool(" read ".into()),
        SelectionMessage::ManualTool("   ".into()),
        SelectionMessage::Skill(" review ".into()),
        SelectionMessage::Skill(String::new()),
    ] {
        update(&mut selector, &[], message);
    }
    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Tools);
    assert_eq!(selector.query(), "  read  ");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::Global);
    assert!(selector.is_group_collapsed(group));
    assert_eq!(selector.selected_manual_tools(), vec!["read".to_string()]);
    assert_eq!(selector.selected_manual_skills(), vec!["review".to_string()]);
}

#[test]
fn expert_package_selection_does_not_leak_between_sessions() {
    let mut first = crate::session::agent::SessionRuntimeState::<()>::default();
    let second = crate::session::agent::SessionRuntimeState::<()>::default();
    first.tool_selector.select_manual_skill("skillhub--skillset--fixture-suite");
    assert!(second.tool_selector.selected_manual_skills().is_empty());
    assert_eq!(
        first.tool_selector.selected_manual_skills(),
        vec!["skillhub--skillset--fixture-suite".to_string()]
    );
}

fn session_info(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo { created: 10, updated: 20, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn active_session_is_plan_run_child_uses_persisted_source() {
    use crate::message::chat::preferences::is_plan_run_child;
    assert!(!is_plan_run_child(None, false, [].iter()));
    let mut child = session_info("child", "Child", "/repo");
    child.parent_id = Some("parent".to_string());
    let mut sessions = vec![child];
    assert!(!is_plan_run_child(Some("child"), false, sessions.iter()));
    sessions[0].slug = session_info::Info::plan_run_slug(&sessions[0].slug);
    assert!(is_plan_run_child(Some("child"), false, sessions.iter()));
}

#[test]
fn current_session_directory_uses_active_session_directory() {
    use crate::session::cache::known_session_directory;
    let sessions = vec![session_info("s1", "Session", "/repo")];
    let projects = std::collections::HashMap::new();
    let states =
        std::collections::HashMap::<String, crate::session::agent::SessionRuntimeState<()>>::new();
    assert_eq!(
        known_session_directory("s1", &sessions, &projects, &states).as_deref(),
        Some("/repo")
    );
    assert_eq!(known_session_directory("missing", &sessions, &projects, &states), None);
}

mod selection_tests;
