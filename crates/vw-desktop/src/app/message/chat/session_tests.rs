#![allow(unused_must_use)]
#[test]
fn session_tests_module_is_wired() {
    assert!(module_path!().ends_with("session_tests"));
}

#[test]
fn chat_reasoning_effort_is_isolated_by_session() {
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-a".to_string());
    let _ = super::update(
        &mut app,
        super::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::High)),
    );

    app.active_session_id = Some("session-b".to_string());
    let _ = super::update(
        &mut app,
        super::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Max)),
    );

    assert_eq!(
        app.get_session_runtime("session-a").reasoning_effort,
        Some(ReasoningEffortDto::High)
    );
    assert_eq!(
        app.get_session_runtime("session-b").reasoning_effort,
        Some(ReasoningEffortDto::Max)
    );

    app.active_session_id = Some("session-a".to_string());
    app.sync_active_session_preferences();
    assert_eq!(app.current_session_runtime().reasoning_effort, Some(ReasoningEffortDto::High));
    assert_eq!(app.reasoning_effort, Some(ReasoningEffortDto::High));
    assert_eq!(app.get_session_runtime("session-c").reasoning_effort, None);
}

fn chat_message(
    role: crate::app::models::ChatRole,
    content: &str,
) -> crate::app::models::ChatMessage {
    crate::app::models::ChatMessage { role, content: content.to_string(), think_timing: Vec::new() }
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

fn queue_item(
    query: &str,
    send_behavior: crate::app::state::ChatSendBehavior,
) -> crate::app::QueueItem {
    crate::app::QueueItem {
        created_ms: 10,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
    }
}

fn agent_request(id: u64, session: &str) -> crate::app::AgentRequest {
    crate::app::AgentRequest {
        id,
        session: session.to_string(),
        query: "active".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        history: Vec::new(),
    }
}

#[test]
fn message_id_for_index_uses_previous_known_id_when_current_is_missing() {
    let ids = vec![Some("msg-1".to_string()), None];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), Some("msg-1".to_string()));
}

#[test]
fn message_id_for_index_handles_gateway_rows_shorter_than_local_chat() {
    let ids = vec![Some("msg-1".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 3), Some("msg-1".to_string()));
}

#[test]
fn message_id_for_index_returns_current_id_before_previous_fallback() {
    let ids = vec![Some("msg-1".to_string()), Some("msg-2".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), Some("msg-2".to_string()));
    assert_eq!(super::message_id_for_index_in_ids(&ids, 0), Some("msg-1".to_string()));
    assert_eq!(super::message_id_for_index_in_ids(&[], 0), None);
}

#[test]
fn message_id_for_index_returns_none_when_current_and_previous_ids_are_missing() {
    let ids = vec![None, None, Some("future".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), None);
}

#[test]
fn parse_inline_mode_command_accepts_known_commands_case_insensitively() {
    assert_eq!(
        super::parse_inline_mode_command("/task build"),
        Some(super::InlineModeCommand::TaskMode)
    );
    assert_eq!(
        super::parse_inline_mode_command(" /NEW topic"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(
        super::parse_inline_mode_command("/clear"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(
        super::parse_inline_mode_command("/session x"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(super::parse_inline_mode_command("/unknown"), None);
    assert_eq!(super::parse_inline_mode_command(""), None);
}

#[test]
fn apply_inline_mode_command_toggles_task_mode_and_clears_runtime_editor() {
    let (mut app, _task) = crate::app::App::new();
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("draft");

    super::apply_inline_mode_command(&mut app, super::InlineModeCommand::TaskMode);

    assert!(app.current_session_runtime().task_mode_enabled);
    assert_eq!(app.current_session_runtime().input_editor.text(), "");

    super::apply_inline_mode_command(&mut app, super::InlineModeCommand::ChatMode);

    assert!(!app.current_session_runtime().task_mode_enabled);
}

#[test]
fn normalize_delegate_agent_trims_and_drops_main_agent() {
    assert_eq!(
        super::normalize_delegate_agent(Some(" worker ".to_string())),
        Some("worker".to_string())
    );
    assert_eq!(
        super::normalize_delegate_agent(Some(crate::app::state::MAIN_AGENT_KEY.to_string())),
        None
    );
    assert_eq!(super::normalize_delegate_agent(Some("   ".to_string())), None);
    assert_eq!(super::normalize_delegate_agent(None), None);
    assert_eq!(
        super::normalize_delegate_agent(Some(format!("  {}  ", crate::app::state::MAIN_AGENT_KEY))),
        None
    );
}

#[test]
fn request_delegate_agent_defaults_to_main_agent() {
    assert_eq!(
        super::request_delegate_agent(Some("worker".to_string())),
        Some("worker".to_string())
    );
    assert_eq!(
        super::request_delegate_agent(None),
        Some(crate::app::state::MAIN_AGENT_KEY.to_string())
    );
}

#[test]
fn resolve_acp_agent_prefers_session_selection_then_app_default() {
    let (mut app, _task) = crate::app::App::new();
    app.acp_agent = Some("default-agent".to_string());

    assert_eq!(
        super::resolve_acp_agent(&app, Some("session-agent".to_string())),
        Some("session-agent".to_string())
    );
    assert_eq!(super::resolve_acp_agent(&app, None), Some("default-agent".to_string()));

    app.acp_agent = None;
    assert_eq!(super::resolve_acp_agent(&app, None), None);
}

#[test]
fn session_agent_selection_normalizes_value_and_resets_tool_context() {
    let (mut app, _task) = crate::app::App::new();
    {
        let selector = &mut app.current_session_runtime_mut().tool_selector;
        selector.toggle_manual_tool("read");
        selector.toggle_manual_skill("review");
        selector.select_tab(crate::app::state::SessionToolSelectorTab::Skills);
    }

    super::update(&mut app, super::ChatMessage::SessionAgentSelected(Some(" worker ".to_string())));

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.agent.as_deref(), Some("worker"));
    assert_eq!(
        runtime.tool_selector.active_tab(),
        crate::app::state::SessionToolSelectorTab::Agent
    );
    assert!(runtime.tool_selector.selected_manual_tools().is_empty());
    assert!(runtime.tool_selector.selected_manual_skills().is_empty());

    super::update(
        &mut app,
        super::ChatMessage::SessionAgentSelected(Some(
            crate::app::state::MAIN_AGENT_KEY.to_string(),
        )),
    );
    assert_eq!(app.current_session_runtime().agent, None);
}

#[test]
fn tool_selector_messages_update_query_scope_group_and_manual_context() {
    let (mut app, _task) = crate::app::App::new();
    let group = crate::app::state::SessionToolGroup::Files;

    super::update(
        &mut app,
        super::ChatMessage::SessionToolSelectorTabSelected(
            crate::app::state::SessionToolSelectorTab::Tools,
        ),
    );
    super::update(
        &mut app,
        super::ChatMessage::SessionToolSelectorSearchChanged("  read  ".to_string()),
    );
    super::update(
        &mut app,
        super::ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(
            crate::app::state::SkillsDirectoryScope::Global,
        ),
    );
    super::update(&mut app, super::ChatMessage::SessionToolGroupCollapsedToggled(group));
    super::update(&mut app, super::ChatMessage::SessionManualToolSelected(" read ".to_string()));
    super::update(&mut app, super::ChatMessage::SessionManualToolSelected("   ".to_string()));
    super::update(&mut app, super::ChatMessage::SessionSkillSelected(" review ".to_string()));
    super::update(&mut app, super::ChatMessage::SessionSkillSelected(String::new()));

    let selector = &app.current_session_runtime().tool_selector;
    assert_eq!(selector.active_tab(), crate::app::state::SessionToolSelectorTab::Tools);
    assert_eq!(selector.query(), "  read  ");
    assert_eq!(selector.skill_directory_scope(), crate::app::state::SkillsDirectoryScope::Global);
    assert!(selector.is_group_collapsed(group));
    assert_eq!(selector.selected_manual_tools(), vec!["read".to_string()]);
    assert_eq!(selector.selected_manual_skills(), vec!["review".to_string()]);
}

#[test]
fn compose_query_with_session_context_appends_selected_tools_and_skills() {
    assert_eq!(super::compose_query_with_session_context("  hi  ", &[], &[]), "hi");

    let composed = super::compose_query_with_session_context(
        "run this",
        &["shell".to_string(), "git".to_string()],
        &["rust".to_string()],
    );

    assert!(composed.starts_with("run this\n\n<session_control_selection>"));
    assert!(composed.contains("工具：shell, git"));
    assert!(composed.contains("技能：rust"));
    assert!(composed.ends_with("</session_control_selection>"));
}

#[test]
fn compose_query_with_session_context_can_stand_alone_without_query() {
    let composed = super::compose_query_with_session_context("   ", &["shell".to_string()], &[]);

    assert!(composed.starts_with("<session_control_selection>"));
    assert!(composed.contains("工具：shell"));
}

#[test]
fn compose_query_with_session_context_supports_skills_without_tools() {
    let composed =
        super::compose_query_with_session_context(" inspect ", &[], &["rust".to_string()]);

    assert!(composed.starts_with("inspect\n\n<session_control_selection>"));
    assert!(!composed.contains("工具："));
    assert!(composed.contains("技能：rust"));
}

#[test]
fn is_supported_image_attachment_matches_known_extensions_case_insensitively() {
    assert!(super::is_supported_image_attachment("/tmp/a.PNG"));
    assert!(super::is_supported_image_attachment("/tmp/a.jpeg"));
    assert!(super::is_supported_image_attachment("/tmp/a.webp"));
    assert!(!super::is_supported_image_attachment("/tmp/a.pdf"));
    assert!(!super::is_supported_image_attachment("/tmp/no-extension"));
}

#[test]
fn compose_query_with_attachments_adds_document_and_image_markers() {
    assert_eq!(super::compose_query_with_attachments("  hello  ", &[]), "hello");
    assert_eq!(
        super::compose_query_with_attachments(
            "",
            &["/tmp/a.png".to_string(), "/tmp/b.pdf".to_string()]
        ),
        "[IMAGE:/tmp/a.png]\n[DOCUMENT:/tmp/b.pdf]"
    );
    assert_eq!(
        super::compose_query_with_attachments("look", &["/tmp/a.gif".to_string()]),
        "look\n\n[IMAGE:/tmp/a.gif]"
    );
}

#[test]
fn compose_query_with_attachments_returns_empty_for_blank_query_without_attachments() {
    assert_eq!(super::compose_query_with_attachments("   ", &[]), "");
}

#[test]
fn push_recovered_message_id_only_emits_pending_content() {
    let mut ids = Vec::new();
    let message_id = Some("msg-1".to_string());
    let mut has_content = false;

    super::push_recovered_message_id(&mut ids, &mut has_content, &message_id);
    assert!(ids.is_empty());

    has_content = true;
    super::push_recovered_message_id(&mut ids, &mut has_content, &message_id);
    assert_eq!(ids, vec![message_id]);
    assert!(!has_content);
}

#[test]
fn gateway_endpoint_defaults_empty_host_to_loopback() {
    let (mut app, _task) = crate::app::App::new();
    app.gateway_settings.host_input = "   ".to_string();
    app.gateway_settings.port = 4321;

    assert_eq!(super::gateway_endpoint(&app), ("127.0.0.1".to_string(), 4321));

    app.gateway_settings.host_input = "0.0.0.0".to_string();
    assert_eq!(super::gateway_endpoint(&app), ("0.0.0.0".to_string(), 4321));

    app.gateway_settings.host_input = "  gateway.local  ".to_string();
    assert_eq!(super::gateway_endpoint(&app), ("gateway.local".to_string(), 4321));
}

#[test]
fn current_session_directory_uses_active_session_directory() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];

    assert_eq!(super::current_session_directory(&app).as_deref(), Some("/repo"));

    app.active_session_id = Some("missing".to_string());
    assert_eq!(super::current_session_directory(&app), None);
}

#[test]
fn restore_cached_chat_for_session_uses_matching_cached_message_ids() {
    let (mut app, _task) = crate::app::App::new();
    app.store_session_chat_snapshot(
        "s1".to_string(),
        crate::app::session::shared_chat_messages(vec![
            chat_message(crate::app::models::ChatRole::User, "hi"),
            chat_message(crate::app::models::ChatRole::Assistant, "hello"),
        ]),
        vec![Some("m1".to_string()), Some("m2".to_string())],
    );

    super::restore_cached_chat_for_session(&mut app, "s1");

    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat_message_ids, vec![Some("m1".to_string()), Some("m2".to_string())]);
}

#[test]
fn restore_cached_chat_for_session_resets_ids_when_cached_ids_have_wrong_len() {
    let (mut app, _task) = crate::app::App::new();
    app.store_session_chat_snapshot(
        "s1".to_string(),
        crate::app::session::shared_chat_messages(vec![
            chat_message(crate::app::models::ChatRole::User, "hi"),
            chat_message(crate::app::models::ChatRole::Assistant, "hello"),
        ]),
        vec![Some("m1".to_string())],
    );

    super::restore_cached_chat_for_session(&mut app, "s1");

    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat_message_ids, vec![None, None]);

    super::restore_cached_chat_for_session(&mut app, "missing");
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
}

#[test]
fn queue_messages_reorder_remove_and_ignore_out_of_bounds_indices() {
    let (mut app, _task) = crate::app::App::new();
    app.current_session_runtime_mut().queue = vec![
        queue_item("first", crate::app::state::ChatSendBehavior::Queue),
        queue_item("second", crate::app::state::ChatSendBehavior::Queue),
        queue_item("third", crate::app::state::ChatSendBehavior::Queue),
    ];

    super::update(&mut app, super::ChatMessage::QueueUp(2));
    super::update(&mut app, super::ChatMessage::QueueUp(0));
    super::update(&mut app, super::ChatMessage::QueueDown(0));
    super::update(&mut app, super::ChatMessage::QueueDown(9));
    super::update(&mut app, super::ChatMessage::QueueRemove(1));
    super::update(&mut app, super::ChatMessage::QueueRemove(9));

    let runtime = app.current_session_runtime();
    let queries = runtime.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["third", "second"]);
}

#[test]
fn dispatch_queue_item_appends_queue_requests_and_prepends_guidance() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    {
        let runtime = app.current_session_runtime_mut();
        runtime.is_requesting = true;
        runtime.input_editor = iced::widget::text_editor::Content::with_text("draft");
        runtime.queue.push(queue_item("existing", crate::app::state::ChatSendBehavior::Queue));
    }

    super::dispatch_queue_item(
        &mut app,
        queue_item("queued", crate::app::state::ChatSendBehavior::Queue),
    );
    super::dispatch_queue_item(
        &mut app,
        queue_item("guided", crate::app::state::ChatSendBehavior::Guide),
    );

    let runtime = app.current_session_runtime();
    let queries = runtime.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["guided", "existing", "queued"]);
    assert_eq!(runtime.input_editor.text(), "");
}

#[test]
fn submit_tick_resets_when_idle_and_cycles_when_requesting() {
    let (mut app, _task) = crate::app::App::new();
    app.current_session_runtime_mut().submit_anim = 2;

    super::update(&mut app, super::ChatMessage::SubmitTick);
    assert_eq!(app.current_session_runtime().submit_anim, 0);

    app.current_session_runtime_mut().is_requesting = true;
    for expected in [1, 2, 0] {
        super::update(&mut app, super::ChatMessage::SubmitTick);
        assert_eq!(app.current_session_runtime().submit_anim, expected);
    }
}

#[test]
fn recovered_message_ids_ignore_stale_session_and_store_exact_match() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = vec![
        chat_message(crate::app::models::ChatRole::User, "one"),
        chat_message(crate::app::models::ChatRole::Assistant, "two"),
    ];
    app.chat_message_ids = vec![None, None];

    super::update(
        &mut app,
        super::ChatMessage::MessageIdsRecoveredForAction {
            session_id: "stale".to_string(),
            msg_idx: 1,
            action: super::MessageIndexedSessionAction::Reset { revert_code: false },
            result: Ok(vec![Some("stale-1".to_string()), Some("stale-2".to_string())]),
        },
    );
    assert_eq!(app.chat_message_ids, vec![None, None]);

    let recovered = vec![Some("msg-1".to_string()), Some("msg-2".to_string())];
    super::update(
        &mut app,
        super::ChatMessage::MessageIdsRecoveredForAction {
            session_id: "active".to_string(),
            msg_idx: 1,
            action: super::MessageIndexedSessionAction::Fork(super::ForkSessionTarget::Local),
            result: Ok(recovered.clone()),
        },
    );
    assert_eq!(app.chat_message_ids, recovered);
    assert_eq!(app.cached_chat_message_ids("active"), Some(recovered));
}

#[test]
fn session_action_failures_report_operation_specific_notifications() {
    let (mut app, _task) = crate::app::App::new();

    super::update(
        &mut app,
        super::ChatMessage::ForkSessionFinished { result: Err("fork-error".to_string()) },
    );
    super::update(
        &mut app,
        super::ChatMessage::LocalForkSessionCreated {
            result: Err("local-error".to_string()),
            target: super::ForkSessionTarget::Local,
            retained_chat: Vec::new(),
            retained_message_ids: Vec::new(),
        },
    );
    super::update(
        &mut app,
        super::ChatMessage::ResetSessionFinished {
            result: Err("reset-error".to_string()),
            session_id: "s1".to_string(),
            retained_chat: Vec::new(),
            retained_message_ids: Vec::new(),
            directory: None,
        },
    );

    let messages = app
        .notifications
        .iter()
        .map(|notification| notification.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        messages,
        vec![
            "分叉会话失败: fork-error",
            "本地分叉会话失败: local-error",
            "重置会话失败: reset-error",
        ]
    );
}

#[test]
fn fork_to_worktree_without_project_directory_is_rejected_before_gateway_access() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.chat = vec![chat_message(crate::app::models::ChatRole::User, "question")];
    app.chat_message_ids = vec![Some("msg-1".to_string())];

    super::update(
        &mut app,
        super::ChatMessage::ForkSessionAt {
            msg_idx: 0,
            target: super::ForkSessionTarget::NewWorktree,
        },
    );

    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("当前项目目录为空，无法创建新 worktree")
    );
}

#[test]
fn reset_session_locally_marks_boundary_and_next_send_uses_trimmed_history() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.project_path = Some("/repo".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];
    app.chat = vec![
        chat_message(crate::app::models::ChatRole::User, "keep"),
        chat_message(crate::app::models::ChatRole::Assistant, "old answer"),
        chat_message(crate::app::models::ChatRole::User, "old follow-up"),
    ];
    app.chat_message_ids = vec![
        Some("m-keep".to_string()),
        Some("m-old-answer".to_string()),
        Some("m-old-follow-up".to_string()),
    ];

    let _ = super::reset_session_locally(&mut app, "s1".to_string(), 0);

    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.session_history_reset_boundary("s1").unwrap(), &[Some("m-keep".to_string())]);

    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("new question");

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("s1");
    let request = runtime.active_agent_request.as_ref().unwrap();
    assert_eq!(request.history.len(), 1);
    assert_eq!(request.history[0].content, "keep");
}

#[test]
fn preference_messages_update_runtime_and_global_chat_defaults() {
    let (mut app, _task) = crate::app::App::new();
    app.show_model_popover = true;
    app.show_mode_popover = true;
    app.show_acp_popover = true;
    app.show_reasoning_effort_popover = true;
    app.current_session_runtime_mut().task_mode_model = "manual-model".to_string();

    super::update(&mut app, super::ChatMessage::AutoModelToggled(true));
    assert!(app.auto_model);
    assert!(app.current_session_runtime().auto_model);
    assert_eq!(app.current_session_runtime().task_mode_model, "auto");

    super::update(&mut app, super::ChatMessage::ModelSelected("  custom/model  ".to_string()));
    assert_eq!(app.model, "custom/model");
    assert!(!app.auto_model);
    assert_eq!(app.current_session_runtime().task_mode_model, "custom/model");
    assert!(!app.show_model_popover);
    assert!(!app.show_mode_popover);

    super::update(&mut app, super::ChatMessage::ModelInputChanged("   ".to_string()));
    assert_eq!(app.model, "auto");
    assert!(app.auto_model);
    assert_eq!(app.current_session_runtime().task_mode_model, "auto");

    super::update(&mut app, super::ChatMessage::AcpAgentSelected(Some("agent-a".to_string())));
    assert_eq!(app.acp_agent.as_deref(), Some("agent-a"));
    assert_eq!(app.current_session_runtime().acp_agent.as_deref(), Some("agent-a"));
    assert!(!app.show_acp_popover);
    assert!(!app.show_reasoning_effort_popover);

    super::update(&mut app, super::ChatMessage::AcpHistoryRecentCountChanged(" 200 ".to_string()));
    assert_eq!(app.acp_recent_count, 20);
    assert_eq!(app.current_session_runtime().acp_recent_count, 20);

    super::update(
        &mut app,
        super::ChatMessage::AcpHistoryRecentCountChanged("not-a-number".to_string()),
    );
    assert_eq!(app.acp_recent_count, 3);
    assert_eq!(app.current_session_runtime().acp_recent_count, 3);
}

#[test]
fn cancel_pressed_ignores_idle_sessions_and_dispatches_next_queued_item_when_busy() {
    let (mut app, _task) = crate::app::App::new();

    super::update(&mut app, super::ChatMessage::CancelPressed);
    assert!(app.current_session_runtime().active_agent_request.is_none());

    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];
    super::update(&mut app, super::ChatMessage::CancelPressed);
    assert!(app.get_session_runtime("s1").active_agent_request.is_none());

    {
        let runtime = app.get_session_runtime_mut("s1");
        runtime.is_requesting = true;
        runtime.submit_anim = 2;
        runtime.active_agent_request = Some(agent_request(41, "s1"));
        runtime.queue.push(queue_item("next request", crate::app::state::ChatSendBehavior::Queue));
    }

    super::update(&mut app, super::ChatMessage::CancelPressed);

    let runtime = app.get_session_runtime("s1");
    assert!(runtime.is_requesting);
    assert_eq!(runtime.submit_anim, 0);
    assert!(runtime.queue.is_empty());
    let active = runtime.active_agent_request.as_ref().expect("queued item should start");
    assert_eq!(active.query, "next request");
    assert_ne!(active.id, 41);
    assert_eq!(
        app.chat.last().map(|message| message.role),
        Some(crate::app::models::ChatRole::Assistant)
    );
    assert!(!app.show_send_mode_popover);
}

#[test]
fn start_without_project_reports_error_before_creating_session() {
    let (mut app, _task) = crate::app::App::new();

    super::dispatch_queue_item(
        &mut app,
        queue_item("new conversation", crate::app::state::ChatSendBehavior::Queue),
    );

    assert_eq!(app.error_message.as_deref(), Some("未选择项目，无法创建会话"));
    assert!(app.active_session_id.is_none());
    assert!(app.sessions.is_empty());
    assert!(app.chat.is_empty());
}

#[test]
fn send_current_session_input_task_mode_clears_files_and_leaves_chat_unstarted() {
    let (mut app, _task) = crate::app::App::new();
    {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_enabled = true;
        runtime.task_mode_priority = "7".to_string();
        runtime.input_editor = iced::widget::text_editor::Content::with_text("build feature");
    }
    app.files = vec!["/tmp/spec.pdf".to_string(), "/tmp/mock.png".to_string()];

    let task = super::send_current_session_input(&mut app);

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    assert!(app.files.is_empty());
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
    assert_eq!(app.current_session_runtime().input_editor.text(), "build feature");
}

#[test]
fn send_current_session_input_starts_existing_session_with_selected_context_and_attachments() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    app.agents_settings.available_tools = vec!["shell".to_string(), "write".to_string()];
    {
        let runtime = app.get_session_runtime_mut("s1");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("inspect");
        runtime.auto_model = false;
        runtime.model = "gpt-test".to_string();
        runtime.reasoning_effort = Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High);
        runtime.acp_agent = Some("codex".to_string());
        runtime.acp_recent_count = 99;
        runtime.tool_selector.toggle_manual_tool("shell");
        runtime.tool_selector.toggle_manual_skill("rust");
    }
    app.files = vec!["/tmp/diagram.PNG".to_string(), "/tmp/notes.md".to_string()];

    let _ = super::send_current_session_input(&mut app);

    assert!(app.files.is_empty());
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat[0].role, crate::app::models::ChatRole::User);
    assert!(app.chat[0].content.starts_with("inspect\n\n<session_control_selection>"));
    assert!(app.chat[0].content.contains("工具：shell"));
    assert!(app.chat[0].content.contains("技能：rust"));
    assert!(app.chat[0].content.contains("[IMAGE:/tmp/diagram.PNG]"));
    assert!(app.chat[0].content.contains("[DOCUMENT:/tmp/notes.md]"));
    assert_eq!(app.chat[1].role, crate::app::models::ChatRole::Assistant);

    let runtime = app.get_session_runtime("s1");
    let request = runtime.active_agent_request.as_ref().expect("request should start");
    assert_eq!(request.model.as_deref(), Some("gpt-test"));
    assert_eq!(
        request.reasoning_effort,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High)
    );
    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(request.acp_recent_count, 20);
    assert!(request.acp_force_new_session);
    assert_eq!(request.allowed_tools, None);
    assert_eq!(request.agent.as_deref(), Some(crate::app::state::MAIN_AGENT_KEY));
    assert_eq!(runtime.input_editor.text(), "");
    assert_eq!(runtime.last_effective_acp_agent.as_deref(), Some("codex"));
}

#[test]
fn fork_and_local_fork_success_update_sessions_chat_cache_and_notifications() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("source".to_string());
    app.chat = vec![chat_message(crate::app::models::ChatRole::User, "source")];
    app.chat_message_ids = vec![Some("source-msg".to_string())];
    let fork_info = session_info("forked", "Forked", "/repo");

    super::update(
        &mut app,
        super::ChatMessage::ForkSessionFinished {
            result: Ok(super::ForkSessionLoaded {
                info: fork_info.clone(),
                target: super::ForkSessionTarget::Local,
                messages: Vec::new(),
                usage: crate::app::models::TokenUsage {
                    input_tokens: 1,
                    output_tokens: 2,
                    cached_tokens: 3,
                    reasoning_tokens: 4,
                },
            }),
        },
    );

    assert_eq!(app.active_session_id.as_deref(), Some("forked"));
    assert!(app.chat.is_empty());
    assert!(app.chat_message_ids.is_empty());
    assert_eq!(app.usage.input_tokens, 1);
    assert!(app.sessions.iter().any(|session| session.id == "forked"));
    assert!(
        app.project_sessions.get("/repo").unwrap().iter().any(|session| session.id == "forked")
    );
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("已分叉到新会话")
    );

    let retained_chat = vec![chat_message(crate::app::models::ChatRole::User, "retained")];
    let local_info = session_info("local-fork", "Local Fork", "/repo");
    super::update(
        &mut app,
        super::ChatMessage::LocalForkSessionCreated {
            result: Ok((local_info.clone(), Some("/repo".to_string()))),
            target: super::ForkSessionTarget::NewWorktree,
            retained_chat,
            retained_message_ids: vec![Some("wrong-len".to_string()), None],
        },
    );

    assert_eq!(app.active_session_id.as_deref(), Some("local-fork"));
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat_message_ids, vec![None]);
    assert!(app.sessions.iter().any(|session| session.id == "local-fork"));
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("已从本地历史分叉到新工作树会话")
    );
}

#[test]
fn recovered_reset_mismatch_falls_back_to_local_history_with_revert_notice() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];
    app.chat = vec![
        chat_message(crate::app::models::ChatRole::User, "keep"),
        chat_message(crate::app::models::ChatRole::Assistant, "drop"),
    ];
    app.chat_message_ids = vec![None, None];

    super::update(
        &mut app,
        super::ChatMessage::MessageIdsRecoveredForAction {
            session_id: "s1".to_string(),
            msg_idx: 0,
            action: super::MessageIndexedSessionAction::Reset { revert_code: true },
            result: Ok(Vec::new()),
        },
    );

    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "keep");
    assert_eq!(app.session_history_reset_boundary("s1").unwrap(), &[None]);
    assert_eq!(
        app.notifications.last().map(|notification| notification.message.as_str()),
        Some("网关没有可回滚历史，已仅重置本地会话历史")
    );
}

#[test]
fn reset_session_finished_success_updates_session_indexes_and_cached_chat() {
    let (mut app, _task) = crate::app::App::new();
    let original = session_info("s1", "Before", "/repo");
    let mut updated = session_info("s1", "After", "/repo");
    updated.time.updated = 99;
    app.active_session_id = Some("other".to_string());
    app.sessions = vec![original.clone()];
    app.project_sessions.insert("/repo".to_string(), vec![original]);
    let retained_chat = vec![chat_message(crate::app::models::ChatRole::User, "kept")];
    let retained_message_ids = vec![Some("m1".to_string())];

    super::update(
        &mut app,
        super::ChatMessage::ResetSessionFinished {
            result: Ok(updated.clone()),
            session_id: "s1".to_string(),
            retained_chat: retained_chat.clone(),
            retained_message_ids: retained_message_ids.clone(),
            directory: Some("/repo".to_string()),
        },
    );

    assert_eq!(app.active_session_id.as_deref(), Some("s1"));
    assert_eq!(app.sessions[0].title, "After");
    assert_eq!(app.project_sessions.get("/repo").unwrap()[0].title, "After");
    assert_eq!(app.chat, retained_chat);
    assert_eq!(app.chat_message_ids, retained_message_ids);
    assert_eq!(app.cached_chat_message_ids("s1"), Some(retained_message_ids));
    assert!(app.chat_reset_menu_idx.is_none());
}
