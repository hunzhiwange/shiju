use super::ChatMessage;
use crate::app::TodoPanelPlacement;
use crate::app::components::chat_panel::message_measure::ChatMessageMeasurement;
use crate::app::components::chat_panel::virtualizer::{
    ChatScrollAnchor, ChatScrollMode, ChatVirtualizer,
};
use crate::app::models::{ChatMessage as ChatEntry, ChatRole};
use crate::app::skill_hub::{InstalledSkill, LoadState};
use crate::app::{Message, message};

fn chat_entry(role: ChatRole, content: &str) -> ChatEntry {
    ChatEntry {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn running_child(parent_id: &str, child_id: &str) -> ChatEntry {
    chat_entry(
        ChatRole::Assistant,
        &format!(
            r#"tool AgentTool
{{"status":"running","metadata":{{"parent_session_id":"{parent_id}","child_session_id":"{child_id}","agent":"writer","task":"build badge"}}}}"#
        ),
    )
}

#[test]
fn input_changed_updates_legacy_input_text() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::InputChanged("hello".to_string()));

    assert_eq!(app.input_text, "hello");
}

#[test]
fn agent_sessions_popover_toggle_requires_running_children_and_closes_when_list_clears() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("parent".to_string());

    let _ = super::update(&mut app, ChatMessage::ToggleAgentSessionsPopover);
    assert!(!app.show_agent_sessions_popover);

    app.chat.push(running_child("parent", "child"));
    let _ = super::update(&mut app, ChatMessage::ToggleAgentSessionsPopover);
    assert!(app.show_agent_sessions_popover);

    app.chat.push(chat_entry(
        ChatRole::Assistant,
        r#"tool AgentTool
{"status":"completed","output":"{\"session_id\":\"child\",\"agent\":\"writer\",\"task\":\"build badge\"}"}"#,
    ));
    let _ = super::update(&mut app, ChatMessage::InputChanged(String::new()));
    assert!(!app.show_agent_sessions_popover);
}

#[test]
fn agent_sessions_popover_closes_when_parent_session_changes() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("parent".to_string());
    app.chat.push(running_child("parent", "child"));

    let _ = super::update(&mut app, ChatMessage::ToggleAgentSessionsPopover);
    assert!(app.show_agent_sessions_popover);

    app.active_session_id = Some("other-parent".to_string());
    let _ = super::update(&mut app, ChatMessage::InputChanged(String::new()));
    assert!(!app.show_agent_sessions_popover);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_sessions_popover_open_selected_closes_and_emits_real_session_target() {
    let (mut app, _) = crate::app::App::new();
    app.show_agent_sessions_popover = true;

    let task = super::update(
        &mut app,
        ChatMessage::OpenAgentSession {
            project_path: "/workspace/project".to_string(),
            session_id: "child-session".to_string(),
        },
    );

    assert!(!app.show_agent_sessions_popover);
    let messages = collect_task_messages(task).await;
    assert!(matches!(
        messages.as_slice(),
        [Message::Project(message::ProjectMessage::OpenProjectSessionPressed(path, id))]
            if path == "/workspace/project" && id == "child-session"
    ));
}

#[test]
fn agent_sessions_popover_stop_rejects_missing_and_mismatched_plan_run_targets() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::StopPlanRun(" \t ".to_string()));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("task ID 为空")));

    let _ = super::update(
        &mut app,
        ChatMessage::StopPlanRunFinished {
            task_id: "task-requested".to_string(),
            result: Ok(vw_gateway_client::vw_api_types::task::TaskPlanRunStatusResponse {
                task_id: "task-other".to_string(),
                status: vw_gateway_client::vw_api_types::task::TaskPlanRunStatus::Cancelled,
                current_session_id: None,
                terminal_event: None,
            }),
        },
    );
    assert!(
        app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("响应任务不匹配"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn quick_message_selection_dispatches_fill_send_and_session_actions() {
    let (mut app, _) = crate::app::App::new();
    app.quick_messages = vec![
        vw_config_types::ui::QuickMessageConfig {
            text: "继续推进".to_string(),
            group: vw_config_types::ui::QuickMessageGroup::General,
            kind: vw_config_types::ui::QuickMessageKind::Message,
        },
        vw_config_types::ui::QuickMessageConfig {
            text: "仔细检查".to_string(),
            group: vw_config_types::ui::QuickMessageGroup::General,
            kind: vw_config_types::ui::QuickMessageKind::FillInput,
        },
        vw_config_types::ui::QuickMessageConfig {
            text: "创建新会话".to_string(),
            group: vw_config_types::ui::QuickMessageGroup::Git,
            kind: vw_config_types::ui::QuickMessageKind::CreateNewSession,
        },
        vw_config_types::ui::QuickMessageConfig {
            text: "压缩当前会话".to_string(),
            group: vw_config_types::ui::QuickMessageGroup::General,
            kind: vw_config_types::ui::QuickMessageKind::SummarizeAndCreateNewSession,
        },
    ];

    let send_task = super::update(&mut app, ChatMessage::QuickMessageSelected(0));
    assert_eq!(app.current_session_runtime().input_editor.text(), "继续推进");
    assert!(matches!(
        collect_task_messages(send_task).await.as_slice(),
        [Message::Chat(ChatMessage::SendPressed)]
    ));

    let fill_task = super::update(&mut app, ChatMessage::QuickMessageSelected(1));
    assert_eq!(app.current_session_runtime().input_editor.text(), "继续推进\n仔细检查");
    assert!(collect_task_messages(fill_task).await.is_empty());

    app.project_path = Some("/workspace/project".to_string());
    app.active_session_id = Some("session-1".to_string());
    app.sessions.push(vw_shared::session::info::Info {
        id: "session-1".to_string(),
        slug: "session-1".to_string(),
        project_id: "project-1".to_string(),
        directory: "/workspace/session-worktree".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Current".to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    });
    app.quick_message_open_group = Some(vw_config_types::ui::QuickMessageGroup::Git);
    let create_task = super::update(&mut app, ChatMessage::QuickMessageSelected(2));
    assert_eq!(app.quick_message_open_group, None);
    assert!(matches!(
        collect_task_messages(create_task).await.as_slice(),
        [Message::Project(message::ProjectMessage::ProjectCreateSession(path))]
            if path == "/workspace/project"
    ));

    let summarize_task = super::update(&mut app, ChatMessage::QuickMessageSelected(3));
    assert!(matches!(
        collect_task_messages(summarize_task).await.as_slice(),
        [Message::Chat(ChatMessage::CompactSessionRequested { session_id, project_path })]
            if session_id == "session-1" && project_path == "/workspace/session-worktree"
    ));

    app.project_path = None;
    let missing_project_task = super::update(&mut app, ChatMessage::QuickMessageSelected(2));
    assert!(
        collect_task_messages(missing_project_task)
            .await
            .iter()
            .all(|message| !matches!(message, Message::Project(_)))
    );
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("打开项目")));

    let out_of_bounds_task = super::update(&mut app, ChatMessage::QuickMessageSelected(99));
    assert!(collect_task_messages(out_of_bounds_task).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn quick_message_git_actions_reuse_panel_workflows_and_validate_empty_state() {
    use vw_config_types::ui::{QuickMessageConfig, QuickMessageGroup, QuickMessageKind};

    let (mut app, _) = crate::app::App::new();
    app.project_path = Some("/workspace/project".to_string());
    app.quick_messages = [
        QuickMessageKind::RefreshGitChanges,
        QuickMessageKind::SelectAllGitChanges,
        QuickMessageKind::InvertGitSelection,
        QuickMessageKind::StageAllGitChanges,
        QuickMessageKind::StageAllAndCommitGitChanges,
        QuickMessageKind::CommitSelectedGitChanges,
    ]
    .into_iter()
    .map(|kind| QuickMessageConfig {
        text: kind.label().to_string(),
        group: QuickMessageGroup::Git,
        kind,
    })
    .collect();

    let refresh =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(0))).await;
    assert!(matches!(
        refresh.as_slice(),
        [Message::Project(message::ProjectMessage::FileManagerRefreshChanges)]
    ));

    app.git_changed_files = vec!["src/lib.rs".to_string(), "src/main.rs".to_string()];
    let select_all =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(1))).await;
    assert!(matches!(
        select_all.as_slice(),
        [Message::Git(message::GitMessage::SelectAllVisibleFileLines(files))]
            if files == &app.git_changed_files
    ));

    let invert =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(2))).await;
    assert!(matches!(
        invert.as_slice(),
        [Message::Git(message::GitMessage::InvertVisibleFileLines(files))]
            if files == &app.git_changed_files
    ));

    let commit =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(5))).await;
    assert!(matches!(
        commit.as_slice(),
        [Message::Chat(ChatMessage::GenerateGitCommitMessage {
            project_path,
            action: crate::app::state::QuickMessageGitCommitAction::CommitSelected,
        })] if project_path == "/workspace/project"
    ));

    let stage_all =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(3))).await;
    assert!(matches!(stage_all.as_slice(), [Message::Git(message::GitMessage::StageAll)]));

    let stage_and_commit =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(4))).await;
    assert!(matches!(
        stage_and_commit.as_slice(),
        [Message::Chat(ChatMessage::GenerateGitCommitMessage {
            project_path,
            action: crate::app::state::QuickMessageGitCommitAction::StageAllAndCommit,
        })] if project_path == "/workspace/project"
    ));

    app.git_changed_files.clear();
    let empty_selection =
        collect_task_messages(super::update(&mut app, ChatMessage::QuickMessageSelected(1))).await;
    assert!(empty_selection.iter().all(|message| !matches!(message, Message::Git(_))));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("没有可选择")));
}

#[test]
fn quick_message_group_click_opens_and_closes_dropdown_state() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(
        &mut app,
        ChatMessage::QuickMessageGroupChanged(Some(vw_config_types::ui::QuickMessageGroup::Git)),
    );
    assert_eq!(app.quick_message_open_group, Some(vw_config_types::ui::QuickMessageGroup::Git));

    let _ = super::update(&mut app, ChatMessage::QuickMessageGroupChanged(None));
    assert_eq!(app.quick_message_open_group, None);
}

#[test]
fn shiju_skill_selection_validates_catalog_and_toggles_multiple_items() {
    let (mut app, _) = crate::app::App::new();
    app.expert_hub.installed = LoadState::Loaded(vec![
        InstalledSkill {
            skill_id: 42,
            slug: "document-review".to_string(),
            name: "文档审查".to_string(),
            description: "审查文档内容".to_string(),
            installed_version: "1.0.0".to_string(),
            current_version: "1.0.0".to_string(),
            format: "markdown".to_string(),
            upgrade_available: false,
        },
        InstalledSkill {
            skill_id: 43,
            slug: "meeting-notes".to_string(),
            name: "会议纪要".to_string(),
            description: "整理会议纪要".to_string(),
            installed_version: "1.0.0".to_string(),
            current_version: "1.0.0".to_string(),
            format: "markdown".to_string(),
            upgrade_available: false,
        },
    ]);
    app.quick_message_open_group = Some(vw_config_types::ui::QuickMessageGroup::Git);

    let load_task = super::update(&mut app, ChatMessage::ToggleShijuSkillMenu);
    assert!(app.show_shiju_skill_menu);
    assert_eq!(app.quick_message_open_group, None);
    assert_eq!(load_task.units(), 1);

    let _ = super::update(&mut app, ChatMessage::SelectShijuSkill(999));
    assert!(app.current_session_runtime().tool_selector.selected_shiju_skills().is_empty());
    assert!(app.show_shiju_skill_menu);

    let _ = super::update(&mut app, ChatMessage::SelectShijuSkill(42));
    let _ = super::update(&mut app, ChatMessage::SelectShijuSkill(43));
    let selected = app.current_session_runtime().tool_selector.selected_shiju_skills();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].slug(), "document-review");
    assert_eq!(selected[1].slug(), "meeting-notes");
    assert!(app.show_shiju_skill_menu);

    let _ = super::update(&mut app, ChatMessage::SelectShijuSkill(42));
    let selected = app.current_session_runtime().tool_selector.selected_shiju_skills();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].skill_id(), 43);
    assert!(app.show_shiju_skill_menu);
}

#[test]
fn scroll_changed_clamps_offsets_and_updates_autoscroll() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let _ =
        super::update(&mut app, ChatMessage::ScrollChanged { offset_y: 1.5, viewport_h: -10.0 });

    assert_eq!(app.chat_scroll_offset_y, 0.0);
    assert_eq!(app.chat_scroll_viewport_h, 0.0);
    assert!(app.chat_auto_scroll);
}

#[test]
fn chat_scroll_semantics_toggle_think_preserves_manual_anchor() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![
        chat_entry(ChatRole::User, "question"),
        chat_entry(ChatRole::Assistant, "<think>hidden reasoning</think>\nanswer"),
        chat_entry(ChatRole::User, "follow-up"),
        chat_entry(ChatRole::Assistant, "later answer"),
    ];
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[90.0, 130.0, 90.0, 130.0]);
    app.chat_virtualizer.set_viewport_height(180.0);
    app.chat_virtualizer.set_scroll_top_px(105.0);
    app.sync_chat_virtualizer_compat_fields();
    let before_scroll_top = app.chat_virtualizer.scroll_top_px();

    let _task = super::update(&mut app, ChatMessage::ToggleThink(1, 0, false));

    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(app.chat_virtualizer.scroll_top_px(), before_scroll_top);
    assert!(app.chat_think_expanded.contains(&((1_u64 << 32) | 0)));
}

#[test]
fn fullscreen_toggles_clear_competing_git_fullscreen_modes() {
    let (mut app, _) = crate::app::App::new();
    app.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.git_diff_fullscreen = true;
    app.git_diff_half_fullscreen = true;

    let _ = super::update(&mut app, ChatMessage::ToggleFullscreen);

    assert!(app.chat_panel_fullscreen);
    assert!(!app.chat_panel_half_fullscreen);
    assert!(!app.git_diff_fullscreen);
    assert!(!app.git_diff_half_fullscreen);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn half_fullscreen_toggles_clear_fullscreen_mode() {
    let (mut app, _) = crate::app::App::new();
    app.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.chat_panel_fullscreen = true;

    let _ = super::update(&mut app, ChatMessage::ToggleHalfFullscreen);

    assert!(app.chat_panel_half_fullscreen);
    assert!(!app.chat_panel_fullscreen);
    assert!(app.fullscreen_layout_settling);
}

#[test]
fn work_half_fullscreen_toggles_are_mutually_exclusive_and_preserve_code_state() {
    let (mut app, _) = crate::app::App::new();
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.chat_panel_fullscreen = true;
    app.chat_panel_half_fullscreen = false;
    app.work_resource_fullscreen = true;
    app.work_resource_half_fullscreen = true;

    let _ = super::update(&mut app, ChatMessage::ToggleHalfFullscreen);

    assert!(app.work_chat_half_fullscreen);
    assert!(!app.work_chat_fullscreen);
    assert!(!app.work_resource_fullscreen);
    assert!(!app.work_resource_half_fullscreen);
    assert!(!app.is_chat_fullscreen());
    assert!(app.is_chat_half_fullscreen());
    assert!(app.chat_panel_fullscreen);
    assert!(!app.chat_panel_half_fullscreen);

    let _ = super::update(&mut app, ChatMessage::ToggleFullscreen);

    assert!(app.work_chat_fullscreen);
    assert!(app.is_chat_fullscreen());
    assert!(!app.is_chat_half_fullscreen());
    assert!(!app.work_chat_half_fullscreen);
    assert!(app.chat_panel_fullscreen);
    assert!(!app.chat_panel_half_fullscreen);
}

#[test]
fn hover_messages_set_and_clear_state() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ThinkHover(2, 3));
    assert_eq!(app.chat_think_hovered_idx, Some((2_u64 << 32) | 3));
    let _ = super::update(&mut app, ChatMessage::ThinkHoverLeave);
    assert_eq!(app.chat_think_hovered_idx, None);

    let _ = super::update(&mut app, ChatMessage::ToolHover(4, 5));
    assert_eq!(app.chat_tool_hovered_idx, Some((4_u64 << 32) | 5));
    app.chat_tool_file_hovered = Some("file".to_string());
    let _ = super::update(&mut app, ChatMessage::ToolHoverLeave);
    assert_eq!(app.chat_tool_hovered_idx, None);
    assert_eq!(app.chat_tool_file_hovered, None);
}

#[test]
fn toggle_sets_add_and_remove_keys() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ToggleToolFile(1, 2, "src/main.rs".to_string()));
    assert!(app.chat_tool_file_expanded.contains("1:2:src/main.rs"));
    let _ = super::update(&mut app, ChatMessage::ToggleToolFile(1, 2, "src/main.rs".to_string()));
    assert!(!app.chat_tool_file_expanded.contains("1:2:src/main.rs"));

    let _ = super::update(&mut app, ChatMessage::ToggleTool(3, 4));
    assert!(app.chat_tool_expanded.contains(&((3_u64 << 32) | 4)));
    let _ = super::update(&mut app, ChatMessage::ToggleTool(3, 4));
    assert!(!app.chat_tool_expanded.contains(&((3_u64 << 32) | 4)));

    let _ = super::update(&mut app, ChatMessage::ToggleExploreSummary(5, 6));
    assert!(app.chat_explore_expanded.contains(&((5_u64 << 32) | 6)));
    let _ = super::update(&mut app, ChatMessage::ToggleExploreSummary(5, 6));
    assert!(!app.chat_explore_expanded.contains(&((5_u64 << 32) | 6)));
}

#[test]
fn toggle_tool_file_preserves_scroll_anchor_and_clears_stale_height() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "tool apply_patch\n{}")];
    app.chat_auto_scroll = false;
    app.chat_message_measured_heights.insert(0, 88.0);
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[88.0]);
    app.chat_virtualizer.set_viewport_height(200.0);
    app.chat_virtualizer.preserve_anchor(ChatScrollAnchor::new(0, 12.0));
    app.sync_chat_virtualizer_compat_fields();

    let _ = super::update(&mut app, ChatMessage::ToggleToolFile(0, 0, "src/main.rs".to_string()));
    assert!(app.chat_tool_file_expanded.contains("0:0:src/main.rs"));
    assert!(!app.chat_message_measured_heights.contains_key(&0));
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(app.chat_virtualizer.anchor().map(|anchor| anchor.index), Some(0));
}

#[test]
fn todo_panel_messages_update_state_and_animation() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(
        &mut app,
        ChatMessage::SetTodoPanelPlacement(TodoPanelPlacement::ChatTopRight),
    );
    assert_eq!(app.chat_todo_placement, TodoPanelPlacement::ChatTopRight);
    assert!(app.chat_todo_expanded);
    assert_eq!(app.chat_todo_anim, 1.0);

    let _ = super::update(&mut app, ChatMessage::ToggleTodoPanel);
    assert!(!app.chat_todo_expanded);
    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert!(app.chat_todo_anim < 1.0);
}

#[test]
fn simple_input_flags_update_directly() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::InputAreaDragHoverChanged(true));
    assert!(app.input_drop_hovered);
    let _ = super::update(&mut app, ChatMessage::FileReferenceHoverChanged(Some(2)));
    assert_eq!(app.file_ref_hovered_index, Some(2));
    let _ = super::update(&mut app, ChatMessage::ToolFilesFilterChanged("rs".to_string()));
    assert_eq!(app.tool_files_filter, "rs");
    let _ = super::update(&mut app, ChatMessage::FullscreenOverlayEntered);
    assert!(app.show_chat_fullscreen_overlay);
    let _ = super::update(&mut app, ChatMessage::FullscreenOverlayExited);
    assert!(!app.show_chat_fullscreen_overlay);
}

#[test]
fn toggle_think_resets_existing_override_to_default_state() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "reasoning")];
    app.chat_auto_scroll = false;
    let key = 11;
    app.chat_think_collapsed.insert(key);

    let _ = super::update(&mut app, ChatMessage::ToggleThink(0, 11, true));

    assert!(!app.chat_think_collapsed.contains(&key));
    assert!(!app.chat_think_expanded.contains(&key));
}

#[test]
fn tool_file_hover_messages_set_and_clear_path() {
    let (mut app, _) = crate::app::App::new();

    let _ = super::update(&mut app, ChatMessage::ToolFileHover("src/lib.rs".to_string()));
    assert_eq!(app.chat_tool_file_hovered.as_deref(), Some("src/lib.rs"));

    let _ = super::update(&mut app, ChatMessage::ToolFileHoverLeave);
    assert_eq!(app.chat_tool_file_hovered, None);
}

#[test]
fn todo_animation_tick_snaps_both_directions_at_completion_threshold() {
    let (mut app, _) = crate::app::App::new();
    app.chat_todo_expanded = false;
    app.chat_todo_anim = 0.001;

    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert_eq!(app.chat_todo_anim, 0.0);

    app.chat_todo_expanded = true;
    app.chat_todo_anim = 0.999;
    let _ = super::update(&mut app, ChatMessage::TodoAnimTick);
    assert_eq!(app.chat_todo_anim, 1.0);
}

#[test]
fn unchanged_scroll_geometry_reports_no_viewport_or_offset_change() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let outcome = super::input::apply_chat_scroll_changed(&mut app, 0.0, 0.0);

    assert!(!outcome.viewport_or_offset_changed);
    assert!(outcome.pending_chunk_starts.is_empty());
    assert!(!outcome.measure_visible_messages);
}

#[test]
fn scroll_within_rendered_window_skips_redundant_measurement() {
    let (mut app, _) = crate::app::App::new();
    app.chat =
        (0..40).map(|idx| chat_entry(ChatRole::Assistant, &format!("message {idx}"))).collect();
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&vec![100.0; app.chat.len()]);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.set_scroll_top_px(1_500.0);
    app.chat_auto_scroll = false;

    let max_scroll_top = app.chat_virtualizer.max_scroll_top_px();
    let outcome =
        super::input::apply_chat_scroll_changed(&mut app, 1_510.0 / max_scroll_top, 300.0);

    assert!(outcome.viewport_or_offset_changed);
    assert!(!outcome.measure_visible_messages);
}

#[test]
fn scroll_across_rendered_window_boundary_requests_measurement() {
    let (mut app, _) = crate::app::App::new();
    app.chat =
        (0..40).map(|idx| chat_entry(ChatRole::Assistant, &format!("message {idx}"))).collect();
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&vec![100.0; app.chat.len()]);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_auto_scroll = false;

    let max_scroll_top = app.chat_virtualizer.max_scroll_top_px();
    let outcome =
        super::input::apply_chat_scroll_changed(&mut app, 1_200.0 / max_scroll_top, 300.0);

    assert!(outcome.measure_visible_messages);
}

#[test]
fn scroll_autoscroll_threshold_is_relaxed_during_hold_window() {
    let (mut held_app, _) = crate::app::App::new();
    held_app.chat_auto_scroll = true;
    held_app.chat_autoscroll_hold_until_ms = u64::MAX;

    let _ = super::input::apply_chat_scroll_changed(&mut held_app, 0.85, 100.0);
    assert!(held_app.chat_auto_scroll);

    let (mut regular_app, _) = crate::app::App::new();
    regular_app.chat_auto_scroll = true;
    regular_app.chat_autoscroll_hold_until_ms = 0;

    let _ = super::input::apply_chat_scroll_changed(&mut regular_app, 0.85, 100.0);
    assert!(!regular_app.chat_auto_scroll);
}

#[test]
fn manual_scroll_requires_tail_threshold_to_resume_autoscroll() {
    let (mut app, _) = crate::app::App::new();
    app.chat_auto_scroll = false;

    let _ = super::input::apply_chat_scroll_changed(&mut app, 0.974, 100.0);
    assert!(!app.chat_auto_scroll);

    let _ = super::input::apply_chat_scroll_changed(&mut app, 0.975, 100.0);
    assert!(app.chat_auto_scroll);
}

#[test]
fn matching_scroll_callback_preserves_turn_focus() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![
        chat_entry(ChatRole::Assistant, "history"),
        chat_entry(ChatRole::User, "question"),
        chat_entry(ChatRole::Assistant, ""),
    ];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[120.0, 100.0, 0.0]);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.focus_turn_on_user(1);
    app.chat_auto_scroll = false;
    let expected_relative_y = app.chat_virtualizer.relative_y();

    let _ =
        super::input::apply_chat_scroll_changed(&mut app, expected_relative_y, 300.0);

    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), Some(1));
}

#[test]
fn manual_scroll_and_explicit_navigation_release_turn_focus() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![
        chat_entry(ChatRole::Assistant, "history"),
        chat_entry(ChatRole::User, "question"),
        chat_entry(ChatRole::Assistant, ""),
    ];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[120.0, 100.0, 0.0]);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.focus_turn_on_user(1);
    app.chat_auto_scroll = false;

    let _ = super::input::apply_chat_scroll_changed(&mut app, 0.5, 300.0);

    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), None);

    app.chat_virtualizer.focus_turn_on_user(1);
    let _task = super::scroll_chat_to_bottom_task(&mut app);

    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), None);
}

#[test]
fn message_measurements_ignore_empty_invalid_and_out_of_range_values() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    assert!(!super::input::apply_chat_message_measurements(&mut app, Vec::new()));
    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![
            ChatMessageMeasurement { message_idx: 0, y: 0.0, height: f32::NAN },
            ChatMessageMeasurement { message_idx: 1, y: 0.0, height: 120.0 },
        ],
    ));
    assert!(app.chat_message_measured_heights.is_empty());
}

#[test]
fn message_measurements_clamp_negative_height_and_sync_virtualizer_length() {
    let (mut app, _) = crate::app::App::new();
    app.chat =
        vec![chat_entry(ChatRole::User, "question"), chat_entry(ChatRole::Assistant, "answer")];
    assert_eq!(app.chat_virtualizer.len(), 0);

    let changed = super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 1, y: 0.0, height: -12.0 }],
    );

    assert!(changed);
    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_message_measured_heights.get(&1), Some(&0.0));
}

#[test]
fn message_measurements_ignore_epsilon_changes_and_apply_larger_updates() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 80.5 }],
    ));
    assert!(app.chat_message_measured_heights.is_empty());

    assert!(super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 82.0 }],
    ));
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&82.0));

    assert!(!super::input::apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 0.0, height: 82.0 }],
    ));
}

#[test]
fn visible_message_measurements_message_updates_virtualizer_when_height_changes() {
    let (mut app, _) = crate::app::App::new();
    app.chat = vec![chat_entry(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0]);

    let _ = super::update(&mut app, ChatMessage::VisibleMessageMeasurements(Vec::new()));
    assert!(app.chat_message_measured_heights.is_empty());

    let _ = super::update(
        &mut app,
        ChatMessage::VisibleMessageMeasurements(vec![ChatMessageMeasurement {
            message_idx: 0,
            y: 0.0,
            height: 104.0,
        }]),
    );

    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&104.0));
}

#[test]
fn wasm_focus_message_is_noop_on_native_targets() {
    let (mut app, _) = crate::app::App::new();
    app.input_text = "unchanged".to_string();

    let _ = super::update(&mut app, ChatMessage::WasmImeFocus);

    assert_eq!(app.input_text, "unchanged");
}
