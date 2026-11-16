//! 聊天输入处理模块。
//!
//! 保留 `chat::input` 作为外部入口，并将文件搜索、工具详情、任务模式、
//! 右键菜单和只读编辑器交互拆分到独立子模块，避免单文件继续膨胀。

mod clipboard;
mod context_menus;
mod editor_actions;
mod file_search;
mod shared;
mod slash_palette;
mod task_mode;
mod tool_detail;

#[cfg(test)]
mod file_search_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod shared_tests;
#[cfg(test)]
mod slash_palette_apply_tests;
#[cfg(test)]
mod slash_palette_tests;

pub(crate) use slash_palette::ranked_slash_palette_sections;

use super::ChatMessage;
use super::ClipboardPastePayload;
use crate::app::components::chat_panel::message_measure::ChatMessageMeasurement;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, message};
pub(crate) use file_search::{FileSearchResult, build_ranked_file_search_entries};
use iced::Task;
use std::time::Duration;

pub(crate) use desktop_session::message::chat::input::scroll::ChatScrollUpdateOutcome;

fn git_quick_action_path(app: &App) -> Option<String> {
    desktop_session::message::chat::input::interactions::git_quick_action_path(
        crate::app::components::git_panel::git_repo_path_for_app(app), app.navigation_tabs.project_path.as_deref(),
    )
}

/// Composer edits do not change the transcript or its visible window. Keep them
/// out of transcript chunk preparation, even while the streaming tail is dirty.
pub(crate) fn is_composer_event(message: &ChatMessage) -> bool {
    matches!(
        message,
        ChatMessage::InputChanged(_)
            | ChatMessage::InputEditorAction(_)
            | ChatMessage::OpenInputContextMenu { .. }
            | ChatMessage::CloseInputContextMenu
            | ChatMessage::CopyInputSelection
            | ChatMessage::CutInputSelection
            | ChatMessage::PasteIntoInput
            | ChatMessage::ClipboardPasteResolved(_)
            | ChatMessage::SelectAllInput
    )
}

pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::InputChanged(value) => {
            app.input_text = value;
            Task::none()
        }
        ChatMessage::QuickMessageSelected(index) => {
            app.input_popovers.quick_message_group = None;
            let Some(item) = app.quick_messages.get(index).cloned() else {
                return Task::none();
            };
            use desktop_chat_input::input_panel::quick_actions::{self, Effect};
            let project = app.navigation_tabs.project_path.clone();
            let session = app.active_session_id.clone();
            let directory = session.as_deref().and_then(|id| app.known_session_directory(id));
            let git_path = git_quick_action_path(app);
            let has_git_changes = !app.git.git_changed_files.is_empty();
            let effect = quick_actions::apply(&item, quick_actions::Context {
                project: project.as_deref(), session: session.as_deref(), session_directory: directory.as_deref(),
                git_path: git_path.as_deref(), has_git_changes,
            });
            match effect {
                Effect::Draft { text, send } => {
                    quick_actions::apply_draft(&mut app.current_session_runtime_mut().input_editor, &text, send);
                    shared::sync_global_input_editor_if_needed(app);
                    if send { Task::done(Message::Chat(ChatMessage::SendPressed)) } else { Task::none() }
                },
                Effect::Warning(source) => app.show_localized_toast(crate::app::state::ToastKind::Warning, source, vec![]),
                Effect::CreateSession(path) => Task::done(Message::Project(message::ProjectMessage::ProjectCreateSession(path))),
                Effect::Compact { session, project } => Task::done(Message::Chat(ChatMessage::CompactSessionRequested { session_id: session, project_path: project })),
                Effect::RefreshGit => Task::done(Message::Project(message::ProjectMessage::FileManagerRefreshChanges)),
                Effect::SelectGit => Task::done(Message::Git(message::GitMessage::SelectAllVisibleFileLines(app.git.git_changed_files.clone()))),
                Effect::InvertGit => Task::done(Message::Git(message::GitMessage::InvertVisibleFileLines(app.git.git_changed_files.clone()))),
                Effect::StageGit => Task::done(Message::Git(message::GitMessage::StageAll)),
                Effect::CommitGit { project, stage_all } => Task::done(Message::Chat(ChatMessage::GenerateGitCommitMessage {
                    project_path: project, action: if stage_all { crate::app::state::QuickMessageGitCommitAction::StageAllAndCommit } else { crate::app::state::QuickMessageGitCommitAction::CommitSelected },
                })),
            }
        }
        ChatMessage::RecommendedPromptMenuChanged(open) => {
            app.input_popovers.set_recommended_prompt(open);
            Task::none()
        }
        ChatMessage::QuickMessageGroupChanged(group) => {
            app.input_popovers.set_quick_message_group(group);
            Task::none()
        }
        #[cfg(not(target_arch = "wasm32"))]
        ChatMessage::ToggleShijuSkillMenu => {
            if !app.input_popovers.toggle_shiju_skill() {
                return Task::none();
            }
            Task::batch([
                Task::done(Message::SkillHub(
                    message::SkillHubMessage::EnsureInstalledSkillsLoaded,
                )),
                Task::done(Message::Account(message::AccountMessage::PrepareNativeShijuSkill)),
            ])
        }
        #[cfg(not(target_arch = "wasm32"))]
        ChatMessage::SelectShijuSkill(skill_id) => {
            let selection = match &app.expert_hub.installed {
                crate::app::skill_hub::LoadState::Loaded(skills) => {
                    skills.iter().find(|skill| skill.skill_id == skill_id).map(|skill| {
                        crate::app::state::ShijuSkillSelection::new(
                            skill.skill_id,
                            skill.slug.clone(),
                            skill.name.clone(),
                        )
                    })
                }
                _ => None,
            };
            let Some(selection) = selection else {
                return Task::none();
            };

            app.current_session_runtime_mut().tool_selector.toggle_shiju_skill(selection);
            Task::none()
        }
        #[cfg(target_arch = "wasm32")]
        ChatMessage::ToggleShijuSkillMenu | ChatMessage::SelectShijuSkill(_) => {
            app.show_warning_toast("识矩原生技能管理请使用桌面端")
        }
        ChatMessage::ToggleAgentSessionsPopover => {
            let has_running_children =
                !crate::app::components::input_panel::agent_sessions::running_agent_sessions(app)
                    .is_empty();
            app.input_popovers.toggle_agent_sessions(has_running_children);
            Task::none()
        }
        ChatMessage::CloseAgentSessionsPopover => {
            app.input_popovers.agent_sessions = false;
            Task::none()
        }
        ChatMessage::StopPlanRun(task_id) => {
            let task_id = task_id.trim().to_string();
            if task_id.is_empty() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "无法停止：plan-run task ID 为空",
                    vec![],
                );
            }
            let result_task_id = task_id.clone();
            Task::perform(
                async move {
                    desktop_session::session_gateway::gateway_task_plan_run_stop_async(crate::app::config::gateway_client_endpoint(), &task_id).await
                },
                move |result| {
                    Message::Chat(ChatMessage::StopPlanRunFinished {
                        task_id: result_task_id,
                        result,
                    })
                },
            )
        }
        ChatMessage::StopPlanRunFinished { task_id, result } => match result {
            Ok(status) if status.task_id == task_id => app.show_localized_toast(
                crate::app::state::ToastKind::Info,
                "已请求停止，正在等待运行状态收敛",
                vec![],
            ),
            Ok(status) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "停止 plan-run 失败：响应任务不匹配（请求 %s，响应 %s）",
                vec![task_id.into(), status.task_id.into()],
            ),
            Err(error) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "停止 plan-run 失败：%s",
                vec![format!("{}", error).into()],
            ),
        },
        ChatMessage::OpenInputContextMenu { x, y } => {
            context_menus::handle_open_input_context_menu(app, x, y)
        }
        ChatMessage::CloseInputContextMenu => context_menus::handle_close_input_context_menu(app),
        ChatMessage::CopyInputSelection => context_menus::handle_copy_input_selection(app),
        ChatMessage::CutInputSelection => context_menus::handle_cut_input_selection(app),
        ChatMessage::PasteIntoInput => context_menus::handle_paste_into_input(app),
        ChatMessage::ClipboardPasteResolved(payload) => {
            context_menus::handle_clipboard_paste_resolved(app, payload)
        }
        ChatMessage::SelectAllInput => context_menus::handle_select_all_input(app),
        ChatMessage::InputEditorAction(action) => {
            file_search::handle_input_editor_action(app, action)
        }
        ChatMessage::MessageEditorAction(idx, action) => {
            editor_actions::handle_message_editor_action(app, idx, action)
        }
        ChatMessage::SpecialTextEditorAction(msg_idx, text_idx, action) => {
            editor_actions::handle_special_text_editor_action(app, msg_idx, text_idx, action)
        }
        ChatMessage::ToolTextEditorAction(msg_idx, tool_idx, text_idx, action) => {
            editor_actions::handle_tool_text_editor_action(app, msg_idx, tool_idx, text_idx, action)
        }
        ChatMessage::MarkdownBlockEditorAction(msg_idx, text_idx, block_idx, action) => {
            editor_actions::handle_markdown_block_editor_action(
                app, msg_idx, text_idx, block_idx, action,
            )
        }
        ChatMessage::ThinkEditorAction(msg_idx, think_idx, action) => {
            editor_actions::handle_think_editor_action(app, msg_idx, think_idx, action)
        }
        ChatMessage::ScrollChanged { offset_y: rel_y, absolute_offset_y, viewport_h } => {
            let outcome = apply_chat_scroll_changed(app, rel_y, absolute_offset_y, viewport_h);
            let history_task = if rel_y <= 0.08 {
                app.navigation_tabs.project_path.clone().zip(app.active_session_id.clone()).and_then(
                    |(project_path, session_id)| {
                        let request = app.paged_chat_timeline.as_mut().and_then(|timeline| {
                            let cursor = timeline.before_cursor()?.to_owned();
                            timeline.begin_prepend(&cursor).then(|| (timeline.generation(), cursor))
                        });
                        request.map(|(generation, cursor)| {
                            crate::app::message::project::helpers::load_session_history_page_task(
                                project_path,
                                session_id,
                                generation,
                                cursor,
                            )
                        })
                    },
                )
            } else {
                None
            };
            let chunk_task = if !outcome.pending_chunk_starts.is_empty()
                && let Some(session_id) = app.active_session_id.clone()
            {
                app.mark_chat_ui_chunks_preparing(&outcome.pending_chunk_starts);
                Some(crate::app::message::project::prepare_session_ui_chunks_task(
                    session_id,
                    app.active_shared_chat_messages(),
                    outcome.pending_chunk_starts,
                    None,
                    app.dialogue_flow_show_reasoning_summary,
                ))
            } else {
                None
            };
            let measure_task =
                outcome.measure_visible_messages.then(super::measure_visible_chat_messages_task);

            let tasks =
                [history_task, chunk_task, measure_task].into_iter().flatten().collect::<Vec<_>>();
            if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
        }
        ChatMessage::VisibleMessageMeasurements(measurements) => {
            if apply_chat_message_measurements(app, measurements) {
                super::measure_visible_chat_messages_task()
            } else {
                Task::none()
            }
        }
        ChatMessage::ScrollToBottom => super::scroll_chat_to_bottom_task(app),
        ChatMessage::FollowupScrollToBottom { session_id, generation } => {
            if app.active_session_id != session_id
                || app.chat_autoscroll_generation != generation
                || !app.chat_auto_scroll
            {
                return Task::none();
            }
            // A follow-up must not re-arm the request or enable follow mode.
            iced::widget::operation::snap_to(
                app.chat_scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
            )
            .map(|_: ()| Message::CopyDone)
        }
        ChatMessage::LocateChatMessage(message_id) => shared::locate_chat_message(app, &message_id),
        ChatMessage::LocateChatMessageIndex(target_idx) => {
            shared::locate_chat_message_index(app, target_idx)
        }
        ChatMessage::ToggleFullscreen => {
            match app.navigation_mode.workspace_mode {
                WorkspaceMode::Code => {
                    app.chat_panel_fullscreen = !app.chat_panel_fullscreen;
                    app.chat_panel_half_fullscreen = false;
                    if app.chat_panel_fullscreen {
                        app.git.git_diff_fullscreen = false;
                        app.git.git_diff_half_fullscreen = false;
                    }
                }
                WorkspaceMode::Work => {
                    app.navigation_workspace.work_chat_fullscreen = !app.navigation_workspace.work_chat_fullscreen;
                    app.navigation_workspace.work_chat_half_fullscreen = false;
                    if app.navigation_workspace.work_chat_fullscreen {
                        app.navigation_workspace.work_resource_fullscreen = false;
                        app.navigation_workspace.work_resource_half_fullscreen = false;
                    }
                }
            }
            app.fullscreen_layout_settling = true;
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        ChatMessage::ToggleHalfFullscreen => {
            match app.navigation_mode.workspace_mode {
                WorkspaceMode::Code => {
                    app.chat_panel_half_fullscreen = !app.chat_panel_half_fullscreen;
                    app.chat_panel_fullscreen = false;
                    if app.chat_panel_half_fullscreen {
                        app.git.git_diff_fullscreen = false;
                        app.git.git_diff_half_fullscreen = false;
                    }
                }
                WorkspaceMode::Work => {
                    app.navigation_workspace.work_chat_half_fullscreen = !app.navigation_workspace.work_chat_half_fullscreen;
                    app.navigation_workspace.work_chat_fullscreen = false;
                    if app.navigation_workspace.work_chat_half_fullscreen {
                        app.navigation_workspace.work_resource_fullscreen = false;
                        app.navigation_workspace.work_resource_half_fullscreen = false;
                    }
                }
            }
            app.fullscreen_layout_settling = true;
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        ChatMessage::FullscreenOverlayEntered => {
            app.show_chat_fullscreen_overlay = true;
            Task::none()
        }
        ChatMessage::FullscreenOverlayExited => {
            app.show_chat_fullscreen_overlay = false;
            Task::none()
        }
        ChatMessage::ToggleUserMessage(idx) => {
            if !app
                .chat
                .get(idx)
                .is_some_and(|entry| entry.role == crate::app::models::ChatRole::User)
            {
                return Task::none();
            }
            desktop_session::message::chat::input::interactions::toggle_user_message(&mut app.chat_message_expanded, idx);
            app.chat_message_measured_heights.remove(&idx);
            app.chat_virtualizer.preserve_current_anchor_around();
            app.refine_chat_message_estimated_heights(idx, idx.saturating_add(1));
            super::measure_visible_chat_messages_task()
        }
        ChatMessage::ToggleThink(msg_idx, think_idx, default_expanded) => {
            let next_expanded = desktop_session::message::chat::input::interactions::toggle_think(
                &mut app.chat_think_expanded, &mut app.chat_think_collapsed,
                msg_idx, think_idx, default_expanded,
            );

            if app.chat_auto_scroll {
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
                if next_expanded {
                    return super::scroll_chat_to_bottom_with_followups(app);
                }
                app.chat_virtualizer.snap_to_tail();
                app.sync_chat_virtualizer_compat_fields();
            } else {
                app.chat_virtualizer.preserve_current_anchor_around();
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
            }

            super::measure_visible_chat_messages_task()
        }
        ChatMessage::ThinkHover(msg_idx, think_idx) => {
            let key = desktop_session::message::chat::input::interactions::block_key(msg_idx, think_idx);
            app.chat_think_hovered_idx = Some(key);
            Task::none()
        }
        ChatMessage::ThinkHoverLeave => {
            app.chat_think_hovered_idx = None;
            Task::none()
        }
        ChatMessage::ToggleToolFile(msg_idx, tool_idx, path) => {
            desktop_session::message::chat::input::interactions::toggle_tool_file(
                &mut app.chat_tool_file_expanded, msg_idx, tool_idx, &path,
            );
            // 展开/收起内联 diff 会显著改变消息高度，需与 ToggleThink 一样重测，
            // 否则虚拟列表 spacer 仍按旧高度计算，表现为大片空白。
            app.chat_message_measured_heights.remove(&msg_idx);
            if app.chat_auto_scroll {
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
                app.chat_virtualizer.snap_to_tail();
                app.sync_chat_virtualizer_compat_fields();
            } else {
                app.chat_virtualizer.preserve_current_anchor_around();
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
            }
            super::measure_visible_chat_messages_task()
        }
        ChatMessage::ToggleTool(msg_idx, tool_idx) => {
            desktop_session::message::chat::input::interactions::toggle_tool_block(
                &mut app.chat_tool_expanded, msg_idx, tool_idx,
            );
            app.chat_message_measured_heights.remove(&msg_idx);
            if app.chat_auto_scroll {
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
                app.chat_virtualizer.snap_to_tail();
                app.sync_chat_virtualizer_compat_fields();
            } else {
                app.chat_virtualizer.preserve_current_anchor_around();
                app.refine_chat_message_estimated_heights(msg_idx, msg_idx.saturating_add(1));
            }
            super::measure_visible_chat_messages_task()
        }
        ChatMessage::ToolFileHover(key) => {
            app.chat_tool_file_hovered = Some(key);
            Task::none()
        }
        ChatMessage::ToolFileHoverLeave => {
            app.chat_tool_file_hovered = None;
            Task::none()
        }
        ChatMessage::ToolHover(msg_idx, tool_idx) => {
            let key = desktop_session::message::chat::input::interactions::block_key(msg_idx, tool_idx);
            app.chat_tool_hovered_idx = Some(key);
            Task::none()
        }
        ChatMessage::ToolHoverLeave => {
            app.chat_tool_hovered_idx = None;
            app.chat_tool_file_hovered = None;
            Task::none()
        }
        ChatMessage::ToggleExploreSummary(msg_idx, tool_idx) => {
            desktop_session::message::chat::input::interactions::toggle_tool_block(
                &mut app.chat_explore_expanded, msg_idx, tool_idx,
            );
            Task::none()
        }
        ChatMessage::OpenToolDetail(msg_idx, tool_idx, raw) => {
            tool_detail::handle_open_tool_detail(app, msg_idx, tool_idx, raw)
        }
        ChatMessage::ToolDetailEditorAction(action) => {
            tool_detail::handle_tool_detail_editor_action(app, action)
        }
        ChatMessage::ToolDetailUndo => tool_detail::handle_tool_detail_undo(app),
        ChatMessage::ToolDetailRedo => tool_detail::handle_tool_detail_redo(app),
        ChatMessage::ToolDetailOpenContextMenu { x, y } => {
            tool_detail::handle_tool_detail_open_context_menu(app, x, y)
        }
        ChatMessage::ToolDetailCloseContextMenu => {
            tool_detail::handle_tool_detail_close_context_menu(app)
        }
        ChatMessage::ToolDetailContextMenuCopy => {
            tool_detail::handle_tool_detail_context_menu_copy(app)
        }
        ChatMessage::ToolDetailContextMenuCut => {
            tool_detail::handle_tool_detail_context_menu_cut(app)
        }
        ChatMessage::ToolDetailContextMenuPaste => {
            tool_detail::handle_tool_detail_context_menu_paste(app)
        }
        ChatMessage::ToolDetailContextMenuDelete => {
            tool_detail::handle_tool_detail_context_menu_delete(app)
        }
        ChatMessage::ToolDetailEditorWheelScrolled { delta, viewport_height } => {
            tool_detail::handle_tool_detail_editor_wheel_scrolled(app, delta, viewport_height)
        }
        ChatMessage::ToolDetailScrollbarChanged { top_line, viewport_height } => {
            tool_detail::handle_tool_detail_scrollbar_changed(app, top_line, viewport_height)
        }
        ChatMessage::CloseToolDetail => tool_detail::handle_close_tool_detail(app),
        ChatMessage::ToggleTodoPanel => {
            if desktop_session::message::chat::input::interactions::toggle_todo_panel(
                &mut app.chat_todo_expanded, &mut app.chat_todo_anim,
            ) {
                message::after(Duration::from_millis(16), Message::Chat(ChatMessage::TodoAnimTick))
            } else {
                Task::none()
            }
        }

        ChatMessage::SetTodoPanelPlacement(placement) => {
            app.chat_todo_placement = placement;
            desktop_session::message::chat::input::interactions::show_todo_panel(&mut app.chat_todo_expanded, &mut app.chat_todo_anim);
            Task::none()
        }
        ChatMessage::TodoAnimTick => {
            if desktop_session::message::chat::input::interactions::tick_todo_panel(
                app.chat_todo_expanded, &mut app.chat_todo_anim,
            ) {
                message::after(Duration::from_millis(16), Message::Chat(ChatMessage::TodoAnimTick))
            } else {
                Task::none()
            }
        }

        ChatMessage::FileSearchInputChanged(value) => {
            file_search::handle_file_search_input_changed(app, value)
        }
        ChatMessage::FileSearchNavigateUp => file_search::handle_file_search_navigate_up(app),
        ChatMessage::FileSearchNavigateDown => file_search::handle_file_search_navigate_down(app),
        ChatMessage::FileSearchSelectCurrent => file_search::handle_file_search_select_current(app),
        ChatMessage::FileSearchSelect(path) => file_search::handle_file_search_select(app, path),
        ChatMessage::SlashPaletteClosed => slash_palette::handle_slash_palette_closed(app),
        ChatMessage::SlashPaletteNavigateUp => slash_palette::handle_slash_palette_navigate_up(app),
        ChatMessage::SlashPaletteNavigateDown => {
            slash_palette::handle_slash_palette_navigate_down(app)
        }
        ChatMessage::SlashPaletteSelectCurrent => {
            slash_palette::handle_slash_palette_select_current(app)
        }
        ChatMessage::SlashPaletteSelect { kind, id } => {
            slash_palette::handle_slash_palette_select(app, kind, id)
        }
        ChatMessage::SlashPaletteExpandKind(kind) => {
            slash_palette::handle_slash_palette_expand_kind(app, kind)
        }
        ChatMessage::RemoveFileReference(file_path) => {
            file_search::handle_remove_file_reference(app, file_path)
        }
        ChatMessage::FileReferenceHoverChanged(value) => {
            app.input_suggestions.file_ref_hovered_index = value;
            Task::none()
        }
        ChatMessage::ToolFilesFilterChanged(value) => {
            app.tool_files_filter = value;
            Task::none()
        }
        ChatMessage::TaskModeToggled(value) => task_mode::handle_task_mode_toggled(app, value),
        ChatMessage::WorkflowModeToggled(value) => {
            task_mode::handle_workflow_mode_toggled(app, value)
        }
        ChatMessage::GoalModeToggled(value) => task_mode::handle_goal_mode_toggled(app, value),
        ChatMessage::TaskModePriorityChanged(value) => {
            task_mode::handle_task_mode_priority_changed(app, value)
        }
        ChatMessage::TaskModeModelChanged(model) => {
            task_mode::handle_task_mode_model_changed(app, model)
        }
        ChatMessage::TaskModeTaskSplitModeChanged(mode) => {
            task_mode::handle_task_mode_task_split_mode_changed(app, mode)
        }
        ChatMessage::TaskModeExecutorChanged(executor) => {
            task_mode::handle_task_mode_executor_changed(app, executor)
        }
        ChatMessage::TaskModeReasoningEffortChanged(reasoning_effort) => {
            task_mode::handle_task_mode_reasoning_effort_changed(app, reasoning_effort)
        }
        ChatMessage::TaskModeSubtaskChanged { index, value } => {
            task_mode::handle_task_mode_subtask_changed(app, index, value)
        }
        ChatMessage::TaskModeSubtaskEditorAction { index, action } => {
            task_mode::handle_task_mode_subtask_editor_action(app, index, action)
        }
        ChatMessage::TaskModeAddSubtask => task_mode::handle_task_mode_add_subtask(app),
        ChatMessage::TaskModeRemoveSubtask(index) => {
            task_mode::handle_task_mode_remove_subtask(app, index)
        }
        ChatMessage::TaskModeMoveSubtaskUp(index) => {
            task_mode::handle_task_mode_move_subtask_up(app, index)
        }
        ChatMessage::TaskModeMoveSubtaskDown(index) => {
            task_mode::handle_task_mode_move_subtask_down(app, index)
        }
        ChatMessage::InputAreaDragDrop => file_search::handle_input_area_drag_drop(app),
        ChatMessage::InputAreaDragHoverChanged(value) => {
            app.file_tree_interaction.input_drop_hovered = value;
            Task::none()
        }
        ChatMessage::OpenMessageContextMenu { target, x, y, text } => {
            context_menus::handle_open_message_context_menu(app, target, x, y, text)
        }
        ChatMessage::OpenChatMessageContextMenu { target, x, y, session_id, message_idx } => {
            if app.active_session_id.as_deref() != Some(session_id.as_str()) {
                return Task::none();
            }
            let Some(text) =
                crate::app::components::chat_panel::message_view::chat_message_action_text(
                    app,
                    message_idx,
                    true,
                )
            else {
                return Task::none();
            };
            context_menus::handle_open_message_context_menu(app, target, x, y, text)
        }
        ChatMessage::CloseMessageContextMenu => {
            context_menus::handle_close_message_context_menu(app)
        }
        ChatMessage::OpenForkSessionDialog(anchor) => {
            context_menus::handle_open_fork_session_dialog(app, anchor)
        }
        ChatMessage::CloseForkSessionDialog => context_menus::handle_close_fork_session_dialog(app),
        ChatMessage::ToggleResetMenu(msg_idx) => {
            context_menus::handle_toggle_reset_menu(app, msg_idx)
        }
        ChatMessage::CloseResetMenu => context_menus::handle_close_reset_menu(app),
        ChatMessage::CopyContextMenuText => context_menus::handle_copy_context_menu_text(app),
        ChatMessage::AppendContextMenuText => context_menus::handle_append_context_menu_text(app),
        ChatMessage::SearchContextMenuWithBaidu => {
            context_menus::handle_search_context_menu_with_baidu(app)
        }
        ChatMessage::SearchContextMenuWithGoogle => {
            context_menus::handle_search_context_menu_with_google(app)
        }
        ChatMessage::SearchContextMenuWithBing => {
            context_menus::handle_search_context_menu_with_bing(app)
        }
        _ => Task::none(),
    }
}

pub(crate) fn apply_chat_scroll_changed(
    app: &mut App,
    rel_y: f32,
    absolute_offset_y: f32,
    viewport_h: f32,
) -> ChatScrollUpdateOutcome {
    if app.chat_virtualizer.len() != app.chat.len() {
        app.sync_chat_message_estimated_heights_len();
    }
    let outcome = desktop_chat_view::scroll_update::apply_scroll_changed(
        &mut app.chat_virtualizer, &mut app.chat_auto_scroll, &mut app.chat_autoscroll_generation,
        app.chat_autoscroll_hold_until_ms, rel_y, absolute_offset_y, viewport_h, super::now_ms,
    );
    app.sync_chat_scroll_compat_fields();
    let visible_range = outcome.visible_range;
    let pending_chunk_starts = if outcome.visible_range_changed || outcome.viewport_changed {
        app.prune_chat_heavy_editor_caches(visible_range.0, visible_range.1);
        app.pending_chat_ui_chunk_starts(visible_range.0, visible_range.1, true)
    } else {
        Vec::new()
    };

    ChatScrollUpdateOutcome {
        visible_range,
        pending_chunk_starts,
        measure_visible_messages: outcome.measure_visible_messages,
        viewport_or_offset_changed: outcome.viewport_or_offset_changed,
    }
}

pub(crate) fn apply_chat_message_measurements(
    app: &mut App,
    measurements: Vec<ChatMessageMeasurement>,
) -> bool {
    if measurements.is_empty() {
        return false;
    }

    if app.chat_virtualizer.len() != app.chat.len() {
        app.sync_chat_message_estimated_heights_len();
    }

    let changed = desktop_chat_view::scroll_update::apply_targeted_message_measurements(
        &mut app.chat_virtualizer, &mut app.chat_message_measured_heights, app.chat.len(),
        app.paged_chat_timeline.as_ref(), measurements,
    );

    if changed {
        app.sync_chat_virtualizer_compat_fields();
    }

    changed
}
