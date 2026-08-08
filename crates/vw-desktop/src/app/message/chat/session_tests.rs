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
    crate::app::models::ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
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
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
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
fn initial_session_title_uses_only_trimmed_first_line() {
    assert_eq!(
        super::initial_session_title_for_input("  第一行标题  \n第二行内容\n第三行内容  "),
        "第一行标题"
    );
    assert_eq!(super::initial_session_title_for_input("\r\n  首行  \r\n次行"), "首行");
    assert_eq!(super::initial_session_title_for_input(" \n\t "), "新会话");
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
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    }
}

#[test]
fn summarize_create_action_starts_ai_summary_and_tracks_its_request() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    let directory = "/workspace/project";
    app.project_path = Some(directory.to_string());
    app.active_session_id = Some(session_id.clone());
    app.sessions = vec![session_info(&session_id, "Current", directory)];
    app.chat = vec![chat_message(crate::app::models::ChatRole::User, "实现快捷消息")];
    app.chat_message_ids = vec![None];

    let _task = super::update(
        &mut app,
        super::ChatMessage::SummarizeAndCreateNewSession { project_path: directory.to_string() },
    );

    let runtime = app.get_session_runtime(&session_id);
    let request = runtime.active_agent_request.as_ref().expect("summary request should start");
    assert!(request.query.contains("只输出这份提示词"));
    assert!(matches!(
        app.quick_message_handoff,
        Some(crate::app::state::QuickMessageHandoffState::WaitingForSummary {
            request_id,
            ref source_session_id,
            ref project_path,
        }) if request_id == request.id
            && source_session_id == &session_id
            && project_path == directory
    ));
    assert_eq!(
        app.chat.last().map(|message| message.role),
        Some(crate::app::models::ChatRole::Assistant)
    );
}

#[test]
fn summarize_create_action_rejects_a_busy_session() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.chat = vec![chat_message(crate::app::models::ChatRole::User, "work")];
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(agent_request(7, &session_id));

    let _task = super::update(
        &mut app,
        super::ChatMessage::SummarizeAndCreateNewSession {
            project_path: "/workspace/project".to_string(),
        },
    );

    assert!(app.quick_message_handoff.is_none());
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("等待当前回复")));
}

#[test]
fn git_commit_quick_action_asks_ai_for_title_and_summary_before_commit() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    let directory = "/workspace/project";
    app.project_path = Some(directory.to_string());
    app.active_session_id = Some(session_id.clone());
    app.sessions = vec![session_info(&session_id, "Current", directory)];
    app.staged_files_selected = vec!["src/lib.rs".to_string()];

    let _task = super::update(
        &mut app,
        super::ChatMessage::GenerateGitCommitMessage {
            project_path: directory.to_string(),
            action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
        },
    );

    let runtime = app.get_session_runtime(&session_id);
    let request = runtime.active_agent_request.as_ref().expect("AI request should start");
    assert!(request.query.contains("\"title\""));
    assert!(request.query.contains("\"summary\""));
    assert!(request.query.contains("src/lib.rs"));
    assert!(request.query.contains("不要执行暂存或提交"));
    assert!(matches!(
        app.quick_message_handoff,
        Some(crate::app::state::QuickMessageHandoffState::WaitingForGitCommitMessage {
            request_id,
            ref source_session_id,
            ref project_path,
            action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
        }) if request_id == request.id
            && source_session_id == &session_id
            && project_path == directory
    ));
    assert!(app.git_commit_in_progress);
}

#[test]
fn selected_git_commit_quick_action_requires_a_selection_before_ai_request() {
    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace/project".to_string());

    let _task = super::update(
        &mut app,
        super::ChatMessage::GenerateGitCommitMessage {
            project_path: "/workspace/project".to_string(),
            action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
        },
    );

    assert!(app.quick_message_handoff.is_none());
    assert!(!app.git_commit_in_progress);
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("先选择")));
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
fn open_session_tool_selector_tab_selects_and_toggles_popover() {
    use crate::app::state::SessionToolSelectorTab;

    let (mut app, _task) = crate::app::App::new();
    app.show_model_popover = true;
    app.show_acp_popover = true;

    let _ = super::update(
        &mut app,
        super::ChatMessage::OpenSessionToolSelectorTab(SessionToolSelectorTab::Agent),
    );
    assert!(app.show_session_tool_selector_popover);
    assert!(!app.show_model_popover);
    assert!(!app.show_acp_popover);
    assert_eq!(
        app.current_session_runtime().tool_selector.active_tab(),
        SessionToolSelectorTab::Agent
    );

    let _ = super::update(
        &mut app,
        super::ChatMessage::OpenSessionToolSelectorTab(SessionToolSelectorTab::Tools),
    );
    assert!(app.show_session_tool_selector_popover);
    assert_eq!(
        app.current_session_runtime().tool_selector.active_tab(),
        SessionToolSelectorTab::Tools
    );

    let _ = super::update(
        &mut app,
        super::ChatMessage::OpenSessionToolSelectorTab(SessionToolSelectorTab::Tools),
    );
    assert!(!app.show_session_tool_selector_popover);
    assert_eq!(
        app.current_session_runtime().tool_selector.active_tab(),
        SessionToolSelectorTab::Tools
    );

    let _ = super::update(
        &mut app,
        super::ChatMessage::OpenSessionToolSelectorTab(SessionToolSelectorTab::Skills),
    );
    assert!(app.show_session_tool_selector_popover);
    assert_eq!(
        app.current_session_runtime().tool_selector.active_tab(),
        SessionToolSelectorTab::Skills
    );
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
    assert_eq!(super::compose_query_with_session_context("  hi  ", &[], &[], &[]), "hi");

    let composed = super::compose_query_with_session_context(
        "run this",
        &["shell".to_string(), "git".to_string()],
        &["rust".to_string()],
        &[],
    );

    assert!(composed.starts_with("run this\n\n<session_control_selection>"));
    assert!(composed.contains("工具：shell, git"));
    assert!(composed.contains("技能：rust"));
    assert!(composed.ends_with("</session_control_selection>"));
}

#[test]
fn compose_query_with_session_context_can_stand_alone_without_query() {
    let composed =
        super::compose_query_with_session_context("   ", &["shell".to_string()], &[], &[]);

    assert!(composed.starts_with("<session_control_selection>"));
    assert!(composed.contains("工具：shell"));
}

#[test]
fn compose_query_with_session_context_supports_skills_without_tools() {
    let composed = super::compose_query_with_session_context(
        " inspect ",
        &[],
        &["rust".to_string()],
        &[],
    );

    assert!(composed.starts_with("inspect\n\n<session_control_selection>"));
    assert!(!composed.contains("工具："));
    assert!(composed.contains("技能：rust"));
}

#[test]
fn compose_query_maps_multiple_shiju_functions_to_skill_context() {
    let selections = [
        crate::app::state::ShijuSkillSelection::new(
            42,
            "document-review".to_string(),
            "文档审查".to_string(),
        ),
        crate::app::state::ShijuSkillSelection::new(
            43,
            "meeting-notes".to_string(),
            "会议纪要".to_string(),
        ),
    ];
    let composed = super::compose_query_with_session_context(
        "   ",
        &[],
        &["shiju".to_string(), "rust".to_string()],
        &selections,
    );

    assert!(composed.starts_with("<session_control_selection>"));
    assert_eq!(composed.matches("技能：shiju, rust").count(), 1);
    assert!(composed.contains("识矩功能：document-review, meeting-notes"));
    assert!(composed.contains(
        "默认将用户的问题理解为针对上述已选识矩功能；不要回答成对识矩本身的介绍或使用说明"
    ));
    assert!(composed.ends_with("</session_control_selection>"));
}

#[test]
fn compose_query_omits_shiju_default_subject_hint_without_shiju_functions() {
    let composed =
        super::compose_query_with_session_context("inspect", &[], &["rust".to_string()], &[]);

    assert!(!composed.contains("默认将用户的问题理解为针对上述已选识矩功能"));
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
fn queue_copy_keeps_items_and_ignores_out_of_bounds_or_empty_queries() {
    let (mut app, _task) = crate::app::App::new();
    app.current_session_runtime_mut().queue = vec![
        queue_item("copy me", crate::app::state::ChatSendBehavior::Queue),
        queue_item("", crate::app::state::ChatSendBehavior::Queue),
    ];

    let _ = super::update(&mut app, super::ChatMessage::QueueCopy(0));
    let _ = super::update(&mut app, super::ChatMessage::QueueCopy(1));
    let _ = super::update(&mut app, super::ChatMessage::QueueCopy(9));

    let runtime = app.current_session_runtime();
    let queries = runtime.queue.iter().map(|item| item.query.as_str()).collect::<Vec<_>>();
    assert_eq!(queries, vec!["copy me", ""]);
}

#[test]
fn queue_edit_restores_only_text_and_removes_the_selected_item() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    let mut editable = queue_item("edit this text", crate::app::state::ChatSendBehavior::Queue);
    editable.attachments = vec!["/tmp/queued-image.png".to_string()];
    {
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor = iced::widget::text_editor::Content::with_text("replace this draft");
        runtime.queue =
            vec![queue_item("keep", crate::app::state::ChatSendBehavior::Queue), editable];
    }

    super::update(&mut app, super::ChatMessage::QueueEdit(1));

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.input_editor.text(), "edit this text");
    assert_eq!(runtime.queue.len(), 1);
    assert_eq!(runtime.queue[0].query, "keep");
    assert!(app.files.is_empty(), "queued attachments must not be restored for text editing");

    super::update(&mut app, super::ChatMessage::QueueEdit(9));
    assert_eq!(app.current_session_runtime().input_editor.text(), "edit this text");
}

#[test]
fn queue_guide_removes_item_and_dispatches_with_stop_and_send() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    crate::app::state::clear_pending_guide_handoff(42);
    {
        let runtime = app.current_session_runtime_mut();
        runtime.is_requesting = true;
        runtime.active_agent_request = Some(agent_request(42, "s1"));
        runtime.queue = vec![
            queue_item("waiting", crate::app::state::ChatSendBehavior::Queue),
            queue_item("guide now", crate::app::state::ChatSendBehavior::Queue),
        ];
    }

    super::update(&mut app, super::ChatMessage::QueueGuide(1));

    let runtime = app.current_session_runtime();
    assert_eq!(runtime.queue.len(), 1);
    assert_eq!(runtime.queue[0].query, "waiting");
    assert!(!crate::app::state::take_pending_guide_handoff(42));

    super::update(&mut app, super::ChatMessage::QueueGuide(9));
    assert_eq!(app.current_session_runtime().queue.len(), 1);
    assert!(!crate::app::state::take_pending_guide_handoff(42));
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
fn explicit_model_selection_cancels_pending_history_restore() {
    let (mut app, _task) = crate::app::App::new();
    app.prepare_historical_session_model("history");
    app.active_session_id = Some("history".to_string());

    super::update(
        &mut app,
        super::ChatMessage::ModelSelected("provider/user-selection".to_string()),
    );

    assert!(!app.restore_session_model_from_history("history", "provider/history-model"));
    assert_eq!(app.model, "provider/user-selection");
    assert_eq!(app.get_session_runtime("history").model, "provider/user-selection");
}

#[test]
fn model_selection_does_not_modify_historical_message_models() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("history".to_string());
    app.chat = vec![
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: "question".to_string(),
            model: Some("provider/history-model".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Assistant,
            content: "answer".to_string(),
            model: Some("provider/history-model".to_string()),
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    super::update(&mut app, super::ChatMessage::ModelSelected("provider/new-model".to_string()));

    assert_eq!(
        app.chat.iter().map(|message| message.model.as_deref()).collect::<Vec<_>>(),
        vec![Some("provider/history-model"), Some("provider/history-model")]
    );
    assert_eq!(app.model, "provider/new-model");
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
fn cancel_pressed_schedules_session_save_without_blocking_update() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];
    {
        let runtime = app.get_session_runtime_mut("s1");
        runtime.is_requesting = true;
        runtime.active_agent_request = Some(agent_request(41, "s1"));
    }

    let task = super::update(&mut app, super::ChatMessage::CancelPressed);

    assert_eq!(task.units(), 1);
    assert!(!app.get_session_runtime("s1").is_requesting);
}

#[test]
fn processing_timing_start_uses_runtime_request_start_and_cancel_creates_independent_next_cycle() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "Session", "/repo")];

    let _ = super::start(
        &mut app,
        queue_item("first request", crate::app::state::ChatSendBehavior::Queue),
        true,
    );
    let first_started = app
        .chat
        .last()
        .and_then(|message| message.processing_timing.as_ref())
        .map(|timing| timing.started_ms)
        .expect("first assistant cycle should start");
    assert_eq!(app.get_session_runtime("s1").request_started_ms, first_started);

    app.get_session_runtime_mut("s1")
        .queue
        .push(queue_item("next request", crate::app::state::ChatSendBehavior::Queue));
    let _ = super::update(&mut app, super::ChatMessage::CancelPressed);

    let assistant_timings = app
        .chat
        .iter()
        .filter_map(|message| message.processing_timing.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(assistant_timings.len(), 2);
    assert!(assistant_timings[0].completed_ms.is_some());
    assert!(assistant_timings[1].completed_ms.is_none());
    assert_eq!(
        app.get_session_runtime("s1").request_started_ms,
        assistant_timings[1].started_ms
    );
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
        runtime.tool_selector.toggle_shiju_skill(crate::app::state::ShijuSkillSelection::new(
            7,
            "code-review".to_string(),
            "Code Review".to_string(),
        ));
    }
    app.files = vec!["/tmp/diagram.PNG".to_string(), "/tmp/notes.md".to_string()];

    let _ = super::send_current_session_input(&mut app);

    assert!(app.files.is_empty());
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat[0].role, crate::app::models::ChatRole::User);
    assert!(app.chat[0].content.starts_with("inspect\n\n<session_control_selection>"));
    assert!(app.chat[0].content.contains("工具：shell"));
    assert!(app.chat[0].content.contains("技能：rust"));
    assert!(app.chat[0].content.contains("识矩功能：code-review"));
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
    assert!(!runtime.tool_selector.has_manual_context_selection());
    assert!(runtime.tool_selector.selected_manual_tools().is_empty());
    assert!(runtime.tool_selector.selected_manual_skills().is_empty());
    assert!(runtime.tool_selector.selected_shiju_skills().is_empty());
}

#[test]
fn send_current_session_input_disables_workflow_mode_for_plan_run_child_session() {
    let (mut app, _task) = crate::app::App::new();
    let mut child = session_info("child", "Child", "/repo");
    child.parent_id = Some("parent".to_string());
    app.active_session_id = Some("child".to_string());
    app.sessions = vec![child];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    {
        let runtime = app.get_session_runtime_mut("child");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("continue");
        runtime.workflow_mode_enabled = true;
        runtime.goal_mode_enabled = true;
    }

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("child");
    let request = runtime.active_agent_request.as_ref().expect("request should start");
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
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("root".to_string());
    app.sessions = vec![session_info("root", "Root", "/repo")];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    {
        let runtime = app.get_session_runtime_mut("root");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("plan this");
        runtime.workflow_mode_enabled = true;
    }

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("root");
    let request = runtime.active_agent_request.as_ref().expect("request should start");
    assert!(request.workflow_mode_enabled);
}

#[test]
fn send_current_session_input_keeps_workflow_mode_for_any_query_when_toggle_on() {
    for (index, query) in [
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
        let (mut app, _task) = crate::app::App::new();
        let session_id = format!("root-question-{index}");
        app.active_session_id = Some(session_id.clone());
        app.sessions = vec![session_info(&session_id, "Root", "/repo")];
        app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
        {
            let runtime = app.get_session_runtime_mut(&session_id);
            runtime.input_editor = iced::widget::text_editor::Content::with_text(query);
            runtime.workflow_mode_enabled = true;
        }

        let _ = super::send_current_session_input(&mut app);

        let runtime = app.get_session_runtime(&session_id);
        let request = runtime.active_agent_request.as_ref().expect("request should start");
        assert!(
            request.workflow_mode_enabled,
            "workflow toggle must enter plan-run for any query: {query}"
        );
        assert!(
            runtime.workflow_mode_enabled,
            "sending must not turn off the workflow toggle"
        );
    }
}

#[test]
fn send_current_session_input_keeps_explicit_actions_in_workflow_mode() {
    for (index, query) in [
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
        let (mut app, _task) = crate::app::App::new();
        let session_id = format!("root-action-{index}");
        app.active_session_id = Some(session_id.clone());
        app.sessions = vec![session_info(&session_id, "Root", "/repo")];
        app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
        {
            let runtime = app.get_session_runtime_mut(&session_id);
            runtime.input_editor = iced::widget::text_editor::Content::with_text(query);
            runtime.workflow_mode_enabled = true;
        }

        let _ = super::send_current_session_input(&mut app);

        let runtime = app.get_session_runtime(&session_id);
        let request = runtime.active_agent_request.as_ref().expect("request should start");
        assert!(request.workflow_mode_enabled, "explicit action must use plan-run: {query}");
    }
}

#[test]
fn send_current_session_input_keeps_goal_mode_for_root_session() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("root-goal".to_string());
    app.sessions = vec![session_info("root-goal", "Root Goal", "/repo")];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    {
        let runtime = app.get_session_runtime_mut("root-goal");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("reach this goal");
        runtime.goal_mode_enabled = true;
    }

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("root-goal");
    let request = runtime
        .active_agent_request
        .as_ref()
        .expect("request should start");
    assert!(request.goal_mode_enabled);
    assert!(!request.workflow_mode_enabled);
}

#[test]
fn subagent_mode_request_preserves_agent_tool_without_requirement() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("root-subagent".to_string());
    app.sessions = vec![session_info("root-subagent", "Root Subagent", "/repo")];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    app.agents_settings.available_tools = vec!["shell".to_string(), "AgentTool".to_string()];
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == crate::app::state::MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string()];
    {
        let runtime = app.get_session_runtime_mut("root-subagent");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("delegate this work");
        runtime.subagent_mode_enabled = true;
    }

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("root-subagent");
    let request = runtime.active_agent_request.as_ref().expect("request should start");
    let injected = request.history.first().expect("subagent system prompt");
    assert_eq!(injected.role, crate::app::models::ChatRole::System);
    assert_eq!(injected.content, super::SUBAGENT_MODE_SYSTEM_PROMPT);
    assert!(injected.content.contains("AgentTool"));
    assert!(!injected.content.contains("required primary execution path"));
    assert!(app.chat.iter().all(|message| message.content != injected.content));
    assert!(!request.require_agent_tool);
    assert_eq!(
        request.allowed_tools.as_ref().expect("restricted tools"),
        &["AgentTool".to_string(), "shell".to_string()]
    );
}

#[test]
fn subagent_mode_request_disabled_preserves_restricted_tools() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("root-standard".to_string());
    app.sessions = vec![session_info("root-standard", "Root Standard", "/repo")];
    app.chat_send_behavior = crate::app::state::ChatSendBehavior::Queue;
    app.agents_settings.available_tools = vec!["shell".to_string(), "AgentTool".to_string()];
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == crate::app::state::MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string()];
    {
        let runtime = app.get_session_runtime_mut("root-standard");
        runtime.input_editor = iced::widget::text_editor::Content::with_text("handle this work");
        runtime.subagent_mode_enabled = false;
    }

    let _ = super::send_current_session_input(&mut app);

    let runtime = app.get_session_runtime("root-standard");
    let request = runtime.active_agent_request.as_ref().expect("request should start");
    assert!(!request.require_agent_tool);
    assert_eq!(
        request.allowed_tools.as_ref().expect("restricted tools"),
        &["shell".to_string()]
    );
    assert!(
        request
            .history
            .iter()
            .all(|message| message.content != super::SUBAGENT_MODE_SYSTEM_PROMPT)
    );
}

#[test]
fn active_session_is_plan_run_child_detects_parent_id() {
    let (mut app, _task) = crate::app::App::new();
    assert!(!super::active_session_is_plan_run_child(&app));

    app.active_session_id = Some("child".to_string());
    let mut child = session_info("child", "Child", "/repo");
    child.parent_id = Some("parent".to_string());
    app.sessions = vec![child];
    assert!(super::active_session_is_plan_run_child(&app));

    app.sessions[0].parent_id = Some("   ".to_string());
    assert!(!super::active_session_is_plan_run_child(&app));
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
