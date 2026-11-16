//! 处理聊天会话相关应用消息。
//! 本模块协调用户输入、会话缓存和网关持久化。

use desktop_session::message::chat::session::{
    message_id_for_index_in_ids, normalize_delegate_agent,
};

use super::ChatMessage;
use super::ForkSessionLoaded;
use super::ForkSessionTarget;

use crate::app::message::TaskBoardMessage;
use crate::app::message::chat::stream;
use crate::app::state::{MAIN_AGENT_KEY, QuickMessageGitCommitAction, SessionToolSelectorTab};
use crate::app::{App, Message, QueueItem, message, models};
use iced::Task;
use ordered_id as id;
use std::time::Duration;
use vw_api_acp_model::acp_model_source::AcpModelSource;

pub use desktop_session::message::chat::session::PendingSessionSend;

fn clear_quick_message_handoff_for_request(app: &mut App, request_id: u64) {
    use desktop_session::message::chat::git_handoff::{GitHandoffEffect, clear_for_request};
    if matches!(
        clear_for_request(&mut app.quick_message_handoff, request_id),
        GitHandoffEffect::ClearCommitInProgress
    ) {
        app.git.git_commit_in_progress = false;
    }
}

fn current_session_directory(app: &App) -> Option<String> {
    app.active_session_id.as_deref().and_then(|id| app.known_session_directory(id))
}

fn base_tools_for_selection(app: &mut App) -> Vec<String> {
    let started = app.current_session_has_started();
    desktop_session::message::chat::preferences::prepare_tool_selection(
        app.current_session_runtime_mut(),
        started,
    );
    app.session_tool_inventory(&app.current_session_runtime()).base_tools
}

fn dispatch_queue_item(app: &mut App, item: QueueItem) -> Task<Message> {
    use desktop_session::message::chat::lifecycle::{DispatchEffect, dispatch};
    let active = app.active_session_id.is_some();
    match dispatch(app.current_session_runtime_mut(), item, active) {
        DispatchEffect::None => Task::none(),
        DispatchEffect::StopDraft => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            stream::stop_agent_stream(app, &session_id)
        }
        DispatchEffect::Start { item, replaced_request, replace } => {
            if let Some(request_id) = replaced_request {
                clear_quick_message_handoff_for_request(app, request_id);
            }
            if replace {
                app.sync_task_pet_from_runtime();
                app.popover_visibility.send_mode = false;
            }
            start(app, item, false)
        }
    }
}

fn selection_message(
    app: &mut App,
    message: desktop_session::message::chat::selection::SelectionMessage,
    needs_inventory: bool,
) -> Task<Message> {
    let base_tools = if needs_inventory { base_tools_for_selection(app) } else { Vec::new() };
    if let Some(message) = desktop_session::message::chat::selection::update(
        &mut app.current_session_runtime_mut().tool_selector,
        &base_tools,
        message,
    ) {
        app.push_localized_notification(message, vec![]);
    }
    Task::none()
}

fn persist_acp_history_preferences(
    app: &mut App,
    mode: crate::app::state::AcpHistoryReplayMode,
    recent_count: usize,
) -> Task<Message> {
    app.acp_history_mode = mode;
    app.acp_recent_count = recent_count.clamp(1, 20);

    app.save_desktop_preferences(
        desktop_session::message::chat::preferences::acp_history_preferences(
            mode,
            app.acp_recent_count,
        ),
    )
}

fn persist_session_reasoning_effort(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.active_session_id.clone() else {
        return Task::none();
    };
    let effort = app.current_session_runtime().reasoning_effort;
    app.session_reasoning_efforts.insert(session_id.clone(), effort);
    app.save_desktop_preference(
        "session_reasoning_efforts",
        serde_json::json!({session_id: effort}),
    )
}

fn message_id_for_index(app: &App, msg_idx: usize) -> Option<String> {
    message_id_for_index_in_ids(&app.chat_message_ids, msg_idx)
}

fn recover_reset_message_ids_task(
    session_id: String,
    directory: Option<String>,
    msg_idx: usize,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = match crate::app::gateway_client() {
                Ok(client) => {
                    desktop_session::message::chat::session::recover_reset_message_ids(
                        client,
                        &session_id,
                        directory.as_deref(),
                    )
                    .await
                }
                Err(error) => Err(error),
            };
            ChatMessage::ResetMessageIdsRecovered { session_id, msg_idx, result }
        },
        Message::Chat,
    )
}

fn reset_runtime_history_snapshots(
    app: &mut App,
    session_id: &str,
    retained_chat: &[models::ChatMessage],
) {
    let runtime = app.get_session_runtime_mut(session_id);
    desktop_session::message::chat::request::reset_history_snapshots(
        &mut runtime.request_lifecycle.active_agent_request,
        &mut runtime.request_lifecycle.queue,
        retained_chat,
    );
}

fn reset_session_locally(app: &mut App, session_id: String, msg_idx: usize) -> Task<Message> {
    let root = current_session_directory(app).or_else(|| app.navigation_tabs.project_path.clone());
    let (retained_chat, retained_message_ids) =
        desktop_session::message::chat::lifecycle::retain_history(
            &app.chat,
            &app.chat_message_ids,
            msg_idx,
        );

    app.chat_reset_menu_idx = None;
    app.chat_fork_dialog = None;
    app.mark_session_acp_rebuild_required(&session_id);
    app.active_session_id = Some(session_id.clone());
    app.chat = retained_chat;
    app.chat_message_ids = retained_message_ids;
    app.mark_session_history_reset_boundary(session_id.clone(), app.chat_message_ids.clone());
    let retained_history = app.chat.clone();
    reset_runtime_history_snapshots(app, &session_id, &retained_history);
    app.invalidate_chat_ui_state();
    app.sync_active_session_preferences();
    app.rebuild_active_session_message_meta();
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );
    app.sync_active_session_from_chat();
    let history_len = app.chat.len();
    if let Some(draft) = app.get_session_runtime_mut(&session_id).draft.as_mut() {
        draft.history_len = draft.history_len.min(history_len);
        return Task::none();
    }

    let now =
        crate::app::time::now_ms().max(app.active_session_view_state.updated_ms.saturating_add(1));
    app.active_session_view_state.updated_ms = now;
    let seq_base = app.get_session_runtime(&session_id).stream_save.seq_base;
    let boundary = app.chat.last().cloned();
    let boundary_id = app.chat_message_ids.last().cloned().flatten();
    Task::perform(
        async move {
            let reset = desktop_session::message::chat::session::HistoryReset::new(
                boundary,
                boundary_id,
                seq_base,
                history_len,
                now,
            )?;
            let client = crate::app::gateway_client()?;
            reset.persist(client, &session_id, root.as_deref()).await
        },
        |result| {
            Message::Chat(match result {
                Ok(()) => ChatMessage::SessionSaveAck,
                Err(error) => ChatMessage::SessionSaveFailed(error),
            })
        },
    )
}

fn save_session_task(session: models::ChatSession, directory: Option<String>) -> Task<Message> {
    // Stream updates and lifecycle events can enqueue snapshots at the same time.
    // Route this path through the same per-session coordinator so an older
    // snapshot cannot finish after a newer one and overwrite the history.
    stream::save_session_task(session, directory)
}

fn restore_cached_chat_for_session(app: &mut App, session_id: &str) {
    let cached = app.cached_chat_messages(session_id).map(|chat| chat.to_vec());
    let cached_ids = app.cached_chat_message_ids(session_id);
    desktop_session::message::chat::lifecycle::restore_cached_chat(
        &mut app.chat,
        &mut app.chat_message_ids,
        cached.as_deref(),
        cached_ids,
    );
}

fn send_to_existing_session(app: &mut App, session_id: String, input: String) -> Task<Message> {
    let input = input.trim().to_string();
    if input.is_empty() {
        return Task::none();
    }

    if app.active_session_id.as_deref() != Some(session_id.as_str())
        && app.cached_chat_messages(&session_id).is_none()
    {
        let Some(directory) = app.known_session_directory(&session_id) else {
            return app.show_error_toast("无法确定会话目录，未发送消息");
        };
        return Task::perform(
            async move {
                let result = match crate::app::gateway_client() {
                    Ok(client) => {
                        desktop_session::message::chat::lifecycle::load_send_history(
                            client,
                            &session_id,
                            &directory,
                        )
                        .await
                    }
                    Err(error) => Err(error),
                };
                ChatMessage::SessionSendHistoryLoaded { session_id, input, result }
            },
            Message::Chat,
        );
    }

    if app.active_session_id.as_deref() == Some(session_id.as_str()) {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.input_editor = iced::widget::text_editor::Content::with_text(&input);
        return send_current_session_text_only(app);
    }

    let previous_session_id = app.active_session_id.clone();
    let previous_chat = app.chat.clone();
    let previous_message_ids = app.chat_message_ids.clone();
    if let Some(previous_id) = previous_session_id.clone() {
        app.store_session_chat_snapshot(
            previous_id,
            crate::app::session::shared_chat_messages(previous_chat.clone()),
            previous_message_ids.clone(),
        );
    }

    app.active_session_id = Some(session_id.clone());
    restore_cached_chat_for_session(app, &session_id);
    app.invalidate_chat_ui_state();
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.input_editor = iced::widget::text_editor::Content::with_text(&input);

    let task = send_current_session_text_only(app);
    app.store_session_chat_snapshot(
        session_id,
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );

    app.active_session_id = previous_session_id;
    app.chat = previous_chat;
    app.chat_message_ids = previous_message_ids;
    if let Some(previous_id) = app.active_session_id.clone() {
        app.store_session_chat_snapshot(
            previous_id,
            crate::app::session::shared_chat_messages(app.chat.clone()),
            app.chat_message_ids.clone(),
        );
    }
    app.invalidate_chat_ui_state();
    app.sync_active_session_from_chat();

    task
}

fn send_current_session_text_only(app: &mut App) -> Task<Message> {
    let previous_files = std::mem::take(&mut app.files);
    let task = send_current_session_input(app);
    app.files = previous_files;
    task
}

fn session_info_is_compacting(app: &App, session_id: &str) -> bool {
    desktop_session::message::chat::session::session_info_is_compacting(
        app.sessions.iter().chain(app.project_sessions.values().flatten()),
        session_id,
    )
}

fn set_session_compacting(app: &mut App, session_id: &str, compacting: Option<u64>) {
    desktop_session::message::chat::session::set_session_compacting(
        app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()),
        session_id,
        compacting,
    );
}

fn request_session_compaction_confirmation(
    app: &mut App,
    session_id: String,
    project_path: String,
) -> Task<Message> {
    use desktop_session::message::chat::session::{CompactionGate, compaction_gate};
    let feedback = match compaction_gate(
        &session_id,
        app.session_is_requesting(&session_id),
        session_info_is_compacting(app, &session_id),
        true,
    ) {
        CompactionGate::Ready => None,
        CompactionGate::NoSession => {
            Some((crate::app::state::ToastKind::Warning, "当前没有可压缩的会话"))
        }
        CompactionGate::Requesting => {
            Some((crate::app::state::ToastKind::Warning, "请等待当前回复完成后再压缩会话"))
        }
        CompactionGate::Compacting => {
            Some((crate::app::state::ToastKind::Info, "会话压缩正在进行中"))
        }
    };
    if let Some((kind, message)) = feedback {
        return app.show_localized_toast(kind, message, vec![]);
    }

    app.popover_visibility.session_actions = false;
    app.session_menu_id = None;
    app.session_menu_anchor = None;
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    Task::perform(
        async move {
            let result =
                rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title(i18n.__("压缩当前会话"))
                    .set_description(i18n.__(
                        "压缩会生成摘要检查点以缩短模型上下文，原始聊天记录保持完整。确认继续吗？",
                    ))
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show()
                    .await;
            (session_id, project_path, matches!(result, rfd::MessageDialogResult::Yes))
        },
        |(session_id, project_path, confirmed)| {
            Message::Chat(ChatMessage::CompactSessionConfirmationResolved {
                session_id,
                project_path,
                confirmed,
            })
        },
    )
}

fn compact_session(
    app: &mut App,
    session_id: String,
    project_path: String,
    confirmed: bool,
) -> Task<Message> {
    if !confirmed {
        return Task::none();
    }
    use desktop_session::message::chat::session::{CompactionGate, compaction_gate};
    let feedback = match compaction_gate(
        &session_id,
        app.session_is_requesting(&session_id),
        session_info_is_compacting(app, &session_id),
        false,
    ) {
        CompactionGate::Ready => None,
        CompactionGate::NoSession => {
            Some((crate::app::state::ToastKind::Warning, "当前没有可压缩的会话"))
        }
        CompactionGate::Requesting => {
            Some((crate::app::state::ToastKind::Warning, "请等待当前回复完成后再压缩会话"))
        }
        CompactionGate::Compacting => {
            Some((crate::app::state::ToastKind::Info, "会话压缩正在进行中"))
        }
    };
    if let Some((kind, message)) = feedback {
        return app.show_localized_toast(kind, message, vec![]);
    }

    set_session_compacting(app, &session_id, Some(crate::app::time::now_ms()));
    Task::perform(
        async move {
            let result = match crate::app::gateway_client() {
                Ok(client) => {
                    desktop_session::message::chat::session::summarize_session(
                        client,
                        &session_id,
                        &project_path,
                    )
                    .await
                }
                Err(error) => Err(error),
            };
            (session_id, project_path, result)
        },
        |(session_id, project_path, result)| {
            Message::Chat(ChatMessage::CompactSessionFinished { session_id, project_path, result })
        },
    )
}

fn finish_session_compaction(
    app: &mut App,
    session_id: String,
    _project_path: String,
    result: Result<bool, String>,
) -> Task<Message> {
    set_session_compacting(app, &session_id, None);
    match result {
        Ok(true) => {
            app.show_localized_toast(crate::app::state::ToastKind::Success, "会话压缩完成", vec![])
        }
        Ok(false) => app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "当前会话没有需要压缩的较早历史",
            vec![],
        ),
        Err(error) => app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "会话压缩失败：%s",
            vec![format!("{}", error).into()],
        ),
    }
}

fn generate_git_commit_message(
    app: &mut App,
    project_path: String,
    action: QuickMessageGitCommitAction,
) -> Task<Message> {
    use desktop_session::message::chat::git_handoff::{GitCommitPreparation, prepare_git_commit};
    let prepared = prepare_git_commit(
        &project_path,
        &action,
        app.git.git_commit_in_progress,
        app.current_session_is_requesting(),
        app.quick_message_handoff.is_some(),
        app.current_session_runtime().task_mode_enabled,
        &app.git.staged_files_selected,
        &app.git.staged_hunks_selected,
        &app.git.staged_lines_selected,
        &app.git.staged_old_lines_selected,
    );
    let prompt = match prepared {
        GitCommitPreparation::Warning(message) => {
            return app.show_localized_toast(
                crate::app::state::ToastKind::Warning,
                message,
                vec![],
            );
        }
        GitCommitPreparation::Send(prompt) => prompt,
    };
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text(&prompt);
    let task = send_current_session_text_only(app);
    let Some(source_session_id) = app.active_session_id.clone() else {
        return task;
    };
    let request_id = app
        .get_session_runtime(&source_session_id)
        .request_lifecycle
        .active_agent_request
        .as_ref()
        .map(|request| request.id);
    use desktop_session::message::chat::git_handoff::{GitHandoffEffect, begin_handoff};
    if let GitHandoffEffect::Waiting(state) =
        begin_handoff(request_id, source_session_id, project_path, action)
    {
        app.quick_message_handoff = Some(state);
        app.git.git_commit_in_progress = true;
    }
    task
}

/// 当前活动会话是否由 plan-run 创建。
fn active_session_is_plan_run_child(app: &App) -> bool {
    desktop_session::message::chat::preferences::is_plan_run_child(
        app.active_session_id.as_deref(),
        app.current_session_runtime_ref().is_some_and(|runtime| runtime.is_plan_run_session),
        app.sessions.iter().chain(app.project_sessions.values().flatten()),
    )
}

fn check_agent_installation(app: &App, backend: String, query: Option<String>) -> Task<Message> {
    let session_id = app.active_session_id.clone();
    let agent = app.current_session_runtime().agent.clone();
    Task::perform(
        async {
            gateway_config::GatewayConfigClient::new(
                crate::app::gateway_client()?.transport().clone(),
            )
            .global_acp_installed_get()
            .await
        },
        move |result| {
            Message::Chat(ChatMessage::AgentInstallationChecked {
                session_id: session_id.clone(),
                agent: agent.clone(),
                backend: backend.clone(),
                query: query.clone(),
                result,
            })
        },
    )
}

fn send_current_session_input(app: &mut App) -> Task<Message> {
    let runtime = app.current_session_runtime();
    if let Some(backend) = runtime.acp_agent.clone() {
        return check_agent_installation(app, backend, Some(runtime.input_editor.text()));
    }
    send_current_session_input_checked(app)
}

fn send_current_session_input_checked(app: &mut App) -> Task<Message> {
    if let Some(message) = desktop_session::message::chat::lifecycle::send_gate(
        app.current_session_runtime_ref(),
        app.active_session_id.is_some(),
        app.active_session_view_state.ui_preparing,
        app.active_session_view_state.base_ready,
    ) {
        return app.show_error_toast(message);
    }
    use desktop_session::message::chat::send_input::{SendInputEffect, prepare_send_input};
    let root = current_session_directory(app).or_else(|| app.navigation_tabs.project_path.clone());
    let mut request_runtime = app.current_session_runtime();
    request_runtime.tool_selector.clear_manual_context_selection();
    let allowed_tools = app.session_allowed_tools_for_request(&request_runtime);
    let plan_run_child = active_session_is_plan_run_child(app);
    let behavior = app.chat_send_behavior;
    let mut files = std::mem::take(&mut app.files);
    let effect = prepare_send_input(
        app.current_session_runtime_mut(),
        &mut files,
        root,
        allowed_tools,
        plan_run_child,
        behavior,
        crate::app::time::now_ms(),
    );
    app.files = files;
    match effect {
        SendInputEffect::None => Task::none(),
        SendInputEffect::InlineMode => {
            if app.active_session_id.is_none() {
                app.input_editor = app.current_session_runtime().input_editor;
            }
            Task::none()
        }
        SendInputEffect::AddTask {
            content,
            priority,
            model,
            reasoning_effort,
            task_split_mode,
            subtasks,
        } => Task::done(Message::TaskBoard(TaskBoardMessage::AddTaskFromInputWithOptions {
            content,
            priority,
            model,
            reasoning_effort,
            task_split_mode,
            subtasks,
        })),
        SendInputEffect::Send(item) => dispatch_queue_item(app, item),
    }
}

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    if matches!(
        message,
        ChatMessage::AutoModelToggled(_)
            | ChatMessage::ModelSelected(_)
            | ChatMessage::ModelInputChanged(_)
            | ChatMessage::AcpModelSourceSelected(_)
    ) && app.current_session_has_started()
    {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "会话开始后不能切换模型",
            vec![],
        );
    }
    match message {
        ChatMessage::SendPressed => {
            if app.current_session_runtime().acp_options_state.source_pending.is_some()
                || app.current_session_runtime().acp_options_state.pending
            {
                return app.show_error_toast("模型来源正在保存，请稍后发送");
            }
            send_current_session_input(app)
        }
        ChatMessage::NewSessionProjectResolved { project_path, stream_id, pending, result } => {
            use desktop_session::message::chat::lifecycle::{
                ProjectResolutionEffect, pending_project_is_current, resolve_pending_send,
            };
            let current = pending_project_is_current(
                app.navigation_tabs.project_path.as_deref(),
                &project_path,
                app.agent_stream_id,
                stream_id,
                app.active_session_id.as_deref(),
            );
            match resolve_pending_send(current, *pending, result) {
                ProjectResolutionEffect::Ignore => Task::none(),
                ProjectResolutionEffect::Resume { project_id, pending } => {
                    app.project_id = Some(project_id);
                    start(app, pending.item, pending.keep)
                }
                ProjectResolutionEffect::Failed { attachments, error } => {
                    app.files.extend(attachments);
                    app.push_localized_notification(
                        "解析项目失败，无法创建会话: %s",
                        vec![error.into()],
                    );
                    Task::none()
                }
            }
        }
        ChatMessage::SendToSession { session_id, input } => {
            send_to_existing_session(app, session_id, input)
        }
        ChatMessage::SessionSendHistoryLoaded { session_id, input, result } => {
            use desktop_session::message::chat::lifecycle::{
                HistoryLoadEffect, resolve_history_load,
            };
            let effect = resolve_history_load(
                &session_id,
                app.cached_chat_messages(&session_id).is_some(),
                app.session_is_requesting(&session_id),
                result,
            );
            let loaded = match effect {
                HistoryLoadEffect::Error(error) => return app.show_error_toast(error),
                HistoryLoadEffect::UseCached => None,
                HistoryLoadEffect::Store(loaded) => Some(loaded),
            };
            if let Some(loaded) = loaded {
                let runtime = app.get_session_runtime_mut(&session_id);
                runtime.stream_save.seq_base = 0;
                runtime.stream_save.expected_updated_ms = loaded.updated_ms;
                if app.active_session_id.as_deref() == Some(session_id.as_str()) {
                    app.chat = loaded.messages.clone();
                    app.chat_message_ids = loaded.message_ids.clone();
                    app.invalidate_chat_ui_state();
                }
                app.store_session_chat_snapshot(
                    session_id.clone(),
                    crate::app::session::shared_chat_messages(loaded.messages),
                    loaded.message_ids,
                );
            }
            send_to_existing_session(app, session_id, input)
        }
        ChatMessage::CompactSessionRequested { session_id, project_path } => {
            request_session_compaction_confirmation(app, session_id, project_path)
        }
        ChatMessage::CompactSessionConfirmationResolved { session_id, project_path, confirmed } => {
            compact_session(app, session_id, project_path, confirmed)
        }
        ChatMessage::CompactSessionFinished { session_id, project_path, result } => {
            finish_session_compaction(app, session_id, project_path, result)
        }
        ChatMessage::GenerateGitCommitMessage { project_path, action } => {
            generate_git_commit_message(app, project_path, action)
        }
        ChatMessage::CancelPressed => {
            let session_id = match &app.active_session_id {
                Some(id) => id.clone(),
                None => return Task::none(),
            };

            let is_requesting = app.current_session_is_requesting();
            if !is_requesting {
                return Task::none();
            }

            let request_id = app
                .get_session_runtime(&session_id)
                .request_lifecycle
                .active_agent_request
                .as_ref()
                .map(|request| request.id);
            if let Some(request_id) = request_id {
                crate::app::state::clear_pending_guide_handoff(request_id);
                clear_quick_message_handoff_for_request(app, request_id);
            }
            let stop_task = stream::stop_agent_stream(app, &session_id);
            if app.current_session_runtime().draft.is_some() {
                return stop_task;
            }
            let completed_ms = crate::app::time::now_ms();
            let mut chat = std::mem::take(&mut app.chat);
            desktop_session::message::chat::lifecycle::complete_cancel(
                app.get_session_runtime_mut(&session_id),
                &mut chat,
                completed_ms,
            );
            app.chat = chat;
            app.store_session_chat_snapshot(
                session_id.clone(),
                crate::app::session::shared_chat_messages(app.chat.clone()),
                app.chat_message_ids.clone(),
            );
            let mut session = stream::load_session_or_default(app, session_id.clone());
            session.messages = app.chat.clone();
            session.message_ids = app.chat_message_ids.clone();
            session.updated_ms = completed_ms;
            let save_task = Task::batch([
                stop_task,
                save_session_task(session, stream::session_directory_for_save(app, &session_id)),
            ]);
            {
                let runtime = app.get_session_runtime_mut(&session_id);
                runtime.request_lifecycle.clear_request();
            }
            app.sync_task_pet_from_runtime();
            app.popover_visibility.send_mode = false;
            app.finalize_stream_text();
            app.sync_active_session_from_chat();

            match desktop_session::message::chat::lifecycle::next_queued(
                app.get_session_runtime_mut(&session_id),
            ) {
                None => save_task,
                Some(item) => save_task.chain(start(app, item, true)),
            }
        }

        ChatMessage::AcpModelSourceSelected(source) => {
            super::acp_options::select_source(app, source, None)
        }
        ChatMessage::AutoModelToggled(b) => {
            if app.current_session_runtime().acp_agent.is_some() {
                let model =
                    if b { "auto".to_string() } else { app.current_session_runtime().model };
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(model),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model) = {
                let runtime = app.current_session_runtime_mut();
                desktop_session::message::chat::preferences::toggle_auto_model(runtime, b)
            };
            app.auto_model = auto_model;
            app.save_chat_model_preferences(model, auto_model)
        }
        ChatMessage::ReasoningEffortSelected(reasoning_effort) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return Task::none();
            }
            app.current_session_runtime_mut().reasoning_effort = reasoning_effort;
            app.reasoning_effort = reasoning_effort;
            persist_session_reasoning_effort(app)
        }
        ChatMessage::AgentInstallationStatusesLoaded(result) => {
            if let Ok(statuses) = result {
                app.acp_settings.installed = statuses;
            }
            Task::none()
        }
        ChatMessage::AgentInstallationChecked { session_id, agent, backend, query, result } => {
            use desktop_session::message::chat::lifecycle::{
                InstallationEffect, resolve_installation,
            };
            let runtime = app.current_session_runtime();
            match resolve_installation(
                app.active_session_id.as_deref(),
                session_id.as_deref(),
                &runtime,
                agent.as_deref(),
                &backend,
                query,
                result,
            ) {
                InstallationEffect::Ignore => Task::none(),
                InstallationEffect::Error(error) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "检测运行环境失败：%s",
                    vec![error.into()],
                ),
                InstallationEffect::Checked { statuses, warning, send } => {
                    app.acp_settings.installed = statuses;
                    if let Some(message) = warning {
                        app.show_localized_toast(
                            crate::app::state::ToastKind::Warning,
                            message,
                            vec![],
                        )
                    } else if send {
                        send_current_session_input_checked(app)
                    } else {
                        Task::none()
                    }
                }
            }
        }
        ChatMessage::AgentRecommendedPromptSelected(index) => {
            app.input_popovers.recommended_prompt = false;
            let runtime = app.current_session_runtime();
            let key = runtime.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
            let prompt = app
                .agents_settings
                .entries
                .iter()
                .find(|entry| entry.key == key)
                .and_then(|entry| entry.recommended_prompts.get(index))
                .cloned();
            match desktop_chat_input::input_panel::quick_actions::fill_recommendation(
                &mut app.current_session_runtime_mut().input_editor, prompt.as_deref(),
            ) {
                Ok(Some(prompt)) => app.input_text = prompt,
                Ok(None) => {},
                Err(source) => return app.show_localized_toast(crate::app::state::ToastKind::Warning, source, vec![]),
            }
            Task::none()
        }
        ChatMessage::SessionAgentSelected(agent) => {
            app.input_popovers.recommended_prompt = false;
            if app.current_session_has_started() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "会话开始后不能切换智能体",
                    vec![],
                );
            }
            let normalized_agent = normalize_delegate_agent(agent);
            let key = normalized_agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
            let Some(entry) = app
                .agents_settings
                .entries
                .iter()
                .find(|entry| entry.key == key && entry.can_start_session())
            else {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "智能体未启用或不支持当前入口",
                    vec![],
                );
            };
            let full_access = match &entry.runtime {
                config_agent_profile::AgentRuntime::Native => {
                    app.last_llm_full_access.unwrap_or(entry.defaults.full_access)
                }
                config_agent_profile::AgentRuntime::Acp { .. } => entry.defaults.full_access,
            };
            let agent_label = Some(entry.label.clone());
            let acp_agent = match &entry.runtime {
                config_agent_profile::AgentRuntime::Native => None,
                config_agent_profile::AgentRuntime::Acp { backend } => Some(backend.clone()),
            };
            desktop_session::message::chat::preferences::select_agent(
                app.current_session_runtime_mut(),
                normalized_agent,
                agent_label,
                acp_agent.clone(),
                full_access,
            );
            app.invalidate_project_chat_preferences_read();
            app.acp_agent = acp_agent.clone();
            app.popover_visibility.acp = false;
            app.popover_visibility.reasoning_effort = false;
            app.popover_visibility.session_tool_selector = false;
            if let Some(backend) = acp_agent {
                Task::batch([
                    check_agent_installation(app, backend, None),
                    super::acp_options::update(app, ChatMessage::AcpSourceRefresh),
                ])
            } else {
                Task::none()
            }
        }
        ChatMessage::AcpHistoryModeSelected(mode) => {
            let recent_count = {
                let runtime = app.current_session_runtime_mut();
                runtime.acp_history_mode = mode;
                runtime.acp_recent_count.clamp(1, 20)
            };
            persist_acp_history_preferences(app, mode, recent_count)
        }
        ChatMessage::AcpHistoryRecentCountChanged(raw) => {
            let parsed = desktop_session::message::chat::session::history_recent_count(&raw);
            {
                let runtime = app.current_session_runtime_mut();
                runtime.acp_recent_count = parsed;
            }
            let mode = app.current_session_runtime().acp_history_mode;
            persist_acp_history_preferences(app, mode, parsed)
        }
        ChatMessage::ModelSelected(m) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(m),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model): (String, bool) = {
                let runtime = app.current_session_runtime_mut();
                desktop_session::message::chat::preferences::select_model(runtime, &m, false)
            };
            let options = crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &model);
            let runtime = app.current_session_runtime_mut();
            let reset_effort = desktop_session::message::chat::session::reconcile_reasoning_effort(
                &mut runtime.reasoning_effort,
                &mut runtime.task_mode_reasoning_effort,
                runtime.task_mode_executor.is_some(),
                &options,
            );
            app.model = model.clone();
            app.auto_model = auto_model;
            app.popover_visibility.model = false;
            app.popover_visibility.mode = false;
            app.reasoning_effort = app.current_session_runtime().reasoning_effort;
            let reasoning_task =
                if reset_effort { persist_session_reasoning_effort(app) } else { Task::none() };
            Task::batch([reasoning_task, app.save_chat_model_preferences(model, auto_model)])
        }
        ChatMessage::ModelInputChanged(m) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(m),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model): (String, bool) = {
                let runtime = app.current_session_runtime_mut();
                desktop_session::message::chat::preferences::select_model(runtime, &m, true)
            };
            let options = crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &model);
            let runtime = app.current_session_runtime_mut();
            let reset_effort = desktop_session::message::chat::session::reconcile_reasoning_effort(
                &mut runtime.reasoning_effort,
                &mut runtime.task_mode_reasoning_effort,
                runtime.task_mode_executor.is_some(),
                &options,
            );
            app.model = model.clone();
            app.auto_model = auto_model;
            app.reasoning_effort = app.current_session_runtime().reasoning_effort;
            let reasoning_task =
                if reset_effort { persist_session_reasoning_effort(app) } else { Task::none() };
            Task::batch([reasoning_task, app.save_chat_model_preferences(model, auto_model)])
        }
        ChatMessage::SessionToolBucketToggled(bucket) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Bucket(bucket),
            true,
        ),
        ChatMessage::SessionToolSelectorTabSelected(tab) => {
            {
                let runtime = app.current_session_runtime_mut();
                runtime.tool_selector.select_tab(tab);
            }
            if tab == SessionToolSelectorTab::Agent {
                let status = Task::perform(
                    async {
                        gateway_config::GatewayConfigClient::new(
                            crate::app::gateway_client()?.transport().clone(),
                        )
                        .global_acp_installed_get()
                        .await
                    },
                    |result| Message::Chat(ChatMessage::AgentInstallationStatusesLoaded(result)),
                );
                let profiles = if !app.agents_settings.dirty && !app.agents_settings.save_in_flight
                {
                    crate::app::message::settings::update(
                        app,
                        message::SettingsMessage::Agents(message::settings::AgentsMessage::Refresh),
                    )
                } else {
                    Task::none()
                };
                return Task::batch([status, profiles]);
            }
            if tab == SessionToolSelectorTab::Skills
                && app.skills_settings.catalog.is_empty()
                && !app.skills_settings.loading
            {
                Task::done(Message::Settings(message::SettingsMessage::SkillsRefresh))
            } else {
                Task::none()
            }
        }
        ChatMessage::OpenSessionToolSelectorTab(tab) => {
            let same_tab_open = app.popover_visibility.session_tool_selector
                && app.current_session_runtime().visible_session_tool_selector_tab() == tab;
            {
                let runtime = app.current_session_runtime_mut();
                runtime.tool_selector.select_tab(tab);
            }
            if same_tab_open {
                app.popover_visibility.session_tool_selector = false;
                return Task::none();
            }

            app.popover_visibility.session_tool_selector = true;
            app.popover_visibility.model = false;
            app.popover_visibility.mode = false;
            app.popover_visibility.send_mode = false;
            app.popover_visibility.file = false;
            app.popover_visibility.acp = false;
            app.popover_visibility.reasoning_effort = false;
            app.popover_visibility.usage = false;
            app.popover_visibility.session_actions = false;
            app.popover_visibility.executor = false;

            if tab == SessionToolSelectorTab::Agent {
                let status = Task::perform(
                    async {
                        gateway_config::GatewayConfigClient::new(
                            crate::app::gateway_client()?.transport().clone(),
                        )
                        .global_acp_installed_get()
                        .await
                    },
                    |result| Message::Chat(ChatMessage::AgentInstallationStatusesLoaded(result)),
                );
                let profiles = if !app.agents_settings.dirty && !app.agents_settings.save_in_flight
                {
                    crate::app::message::settings::update(
                        app,
                        message::SettingsMessage::Agents(message::settings::AgentsMessage::Refresh),
                    )
                } else {
                    Task::none()
                };
                return Task::batch([status, profiles]);
            }
            if tab == SessionToolSelectorTab::Skills
                && app.skills_settings.catalog.is_empty()
                && !app.skills_settings.loading
            {
                Task::done(Message::Settings(message::SettingsMessage::SkillsRefresh))
            } else {
                Task::none()
            }
        }
        ChatMessage::SessionToolSelectorSearchChanged(query) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Search(query),
            false,
        ),
        ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(scope) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Scope(scope),
            false,
        ),
        ChatMessage::SessionToolGroupCollapsedToggled(group) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Collapse(group),
            false,
        ),
        ChatMessage::SessionToolGroupToolsToggled(group) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Group(group),
            true,
        ),
        ChatMessage::SessionToolSelectorSelectAll => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::All,
            true,
        ),
        ChatMessage::SessionToolSelectorInvert => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Invert,
            true,
        ),
        ChatMessage::SessionToolToggled(tool_id) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Tool(tool_id),
            true,
        ),
        ChatMessage::SessionManualToolSelected(tool_id) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::ManualTool(tool_id),
            false,
        ),
        ChatMessage::SessionSkillSelected(skill_id) => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Skill(skill_id),
            false,
        ),
        ChatMessage::SessionToolSelectorReset => selection_message(
            app,
            desktop_session::message::chat::selection::SelectionMessage::Reset,
            true,
        ),
        message @ (ChatMessage::QueueRemove(_)
        | ChatMessage::QueueEdit(_)
        | ChatMessage::QueueGuide(_)
        | ChatMessage::QueueUp(_)
        | ChatMessage::QueueDown(_)
        | ChatMessage::QueueCopy(_)) => {
            use desktop_session::message::chat::request::{QueueAction, QueueEffect, update_queue};
            let action = match message {
                ChatMessage::QueueRemove(i) => QueueAction::Remove(i),
                ChatMessage::QueueEdit(i) => QueueAction::Edit(i),
                ChatMessage::QueueGuide(i) => QueueAction::Guide(i),
                ChatMessage::QueueUp(i) => QueueAction::Up(i),
                ChatMessage::QueueDown(i) => QueueAction::Down(i),
                ChatMessage::QueueCopy(i) => QueueAction::Copy(i),
                _ => unreachable!(),
            };
            let effect = update_queue(
                &mut app.current_session_runtime_mut().request_lifecycle.queue,
                action,
            );
            match effect {
                QueueEffect::None => Task::none(),
                QueueEffect::Send(item) => dispatch_queue_item(app, item),
                QueueEffect::Copy(query) => Task::done(Message::CopyCode(query)),
                QueueEffect::Edit(query) => {
                    app.current_session_runtime_mut().input_editor =
                        iced::widget::text_editor::Content::with_text(&query);
                    if app.active_session_id.is_none() {
                        app.input_editor = app.current_session_runtime().input_editor;
                    }
                    iced::widget::operation::focus(app.input_editor_id.clone())
                }
            }
        }
        ChatMessage::SubmitTick => {
            let is_requesting = app.current_session_is_requesting();
            if !app.current_session_runtime_mut().request_lifecycle.submit_tick(is_requesting) {
                return Task::none();
            }
            message::after(Duration::from_millis(200), Message::Chat(ChatMessage::SubmitTick))
        }
        ChatMessage::ForkSessionAt { anchor, target } => {
            if !crate::app::message::chat::fork_session_anchor_is_current(&anchor, app) {
                return Task::none();
            }
            let Some(dialog) = app
                .chat_fork_dialog
                .as_mut()
                .filter(|dialog| !dialog.in_progress && dialog.anchor == anchor)
            else {
                return Task::none();
            };
            dialog.in_progress = true;
            let show_reasoning_summary = app.dialogue_flow_show_reasoning_summary;
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        let info = desktop_session::message::chat::fork::fork_session(
                            &client, &anchor, target,
                        )
                        .await?;
                        let project_path =
                            crate::app::message::project::helpers::gateway_session_project_path(
                                &client, &info,
                            )
                            .await?;
                        let prepared =
                            crate::app::message::project::helpers::prepare_gateway_session_open(
                                &client,
                                &info.id,
                                &info.directory,
                                show_reasoning_summary,
                            )
                            .await?;
                        Ok(Box::new(ForkSessionLoaded { info, project_path, prepared }))
                    }
                    .await;
                    ChatMessage::ForkSessionFinished { anchor, target, result }
                },
                Message::Chat,
            )
        }
        ChatMessage::ForkSessionFinished { anchor, target, result } => {
            let Some(dialog) = app
                .chat_fork_dialog
                .take_if(|dialog| dialog.in_progress && dialog.anchor == anchor)
            else {
                return Task::none();
            };
            if !crate::app::message::chat::fork_session_anchor_is_current(&anchor, app) {
                return Task::none();
            }
            use crate::app::state::ToastKind;
            let ForkSessionLoaded { info, project_path, prepared } = match result {
                Ok(loaded) => *loaded,
                Err(error) => {
                    return app.show_localized_toast(
                        ToastKind::Error,
                        "分叉会话失败: %s",
                        vec![error.into()],
                    );
                }
            };
            let id = info.id.clone();
            let project_path = app
                .navigation_tabs
                .project_path
                .clone()
                .filter(|_| app.project_id.as_deref() == Some(info.project_id.as_str()))
                .unwrap_or(project_path);
            let open = crate::app::message::project::open_project_session(
                app,
                project_path,
                id,
                Some(info),
                Some(prepared),
            );
            app.current_session_runtime_mut().input_editor =
                iced::widget::text_editor::Content::with_text(&dialog.prompt);
            app.input_text = dialog.prompt;
            let toast = app.show_localized_toast(
                ToastKind::Success,
                match target {
                    ForkSessionTarget::Local => "已分叉到新会话",
                    ForkSessionTarget::NewWorktree => "已分叉到新工作树会话",
                },
                vec![],
            );
            Task::batch([open, toast, iced::widget::operation::focus(app.input_editor_id.clone())])
        }
        ChatMessage::ResetMessageIdsRecovered { session_id, msg_idx, result } => {
            if app.active_session_id.as_deref() != Some(session_id.as_str()) {
                return Task::none();
            }
            if let Ok(ids) = &result
                && (ids.len() == app.chat.len()
                    || message_id_for_index_in_ids(ids, msg_idx).is_some())
            {
                app.chat_message_ids.clone_from(ids);
                app.store_session_chat_snapshot(
                    session_id,
                    crate::app::session::shared_chat_messages(app.chat.clone()),
                    app.chat_message_ids.clone(),
                );
                return Task::done(Message::Chat(ChatMessage::ResetSessionToMessage { msg_idx }));
            }
            reset_session_locally(app, session_id, msg_idx)
        }
        ChatMessage::ResetSessionToMessage { msg_idx } => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let runtime = app.current_session_runtime();
            if runtime.draft.is_some() {
                return if runtime.request_lifecycle.is_requesting {
                    Task::none()
                } else {
                    reset_session_locally(app, session_id, msg_idx)
                };
            }
            let Some(message_id) = message_id_for_index(app, msg_idx) else {
                let root = current_session_directory(app)
                    .or_else(|| app.navigation_tabs.project_path.clone());
                return recover_reset_message_ids_task(session_id, root, msg_idx);
            };
            let root =
                current_session_directory(app).or_else(|| app.navigation_tabs.project_path.clone());
            let (retained_chat, retained_message_ids) =
                desktop_session::message::chat::lifecycle::retain_history(
                    &app.chat,
                    &app.chat_message_ids,
                    msg_idx,
                );
            let save_task = reset_session_locally(app, session_id.clone(), msg_idx);
            let gateway_task = Task::perform(
                async move {
                    let result = match crate::app::gateway_client() {
                        Ok(client) => {
                            desktop_session::message::chat::session::reset_session(
                                client,
                                &session_id,
                                root.as_deref(),
                                message_id,
                            )
                            .await
                        }
                        Err(err) => Err(err),
                    };
                    ChatMessage::ResetSessionFinished {
                        result,
                        session_id,
                        retained_chat,
                        retained_message_ids,
                        directory: root,
                    }
                },
                Message::Chat,
            );
            Task::batch([save_task, gateway_task])
        }
        ChatMessage::ResetSessionFinished {
            result,
            session_id,
            retained_chat,
            retained_message_ids,
            directory,
        } => match result {
            Ok(info) => {
                if let Some(existing) =
                    app.sessions.iter_mut().find(|session| session.id == info.id)
                {
                    *existing = info.clone();
                }
                for sessions in app.project_sessions.values_mut() {
                    if let Some(existing) =
                        sessions.iter_mut().find(|session| session.id == info.id)
                    {
                        *existing = info.clone();
                    }
                }
                app.mark_session_acp_rebuild_required(&session_id);
                app.active_session_id = Some(session_id.clone());
                app.chat = retained_chat;
                app.chat_message_ids = retained_message_ids;
                app.chat_reset_menu_idx = None;
                app.invalidate_chat_ui_state();
                app.sync_active_session_preferences();
                app.rebuild_active_session_message_meta();
                app.store_session_chat_snapshot(
                    session_id.clone(),
                    crate::app::session::shared_chat_messages(app.chat.clone()),
                    app.chat_message_ids.clone(),
                );
                app.sync_active_session_from_chat();

                let now = crate::app::time::now_ms();
                let session_directory = directory.or_else(|| Some(info.directory.clone()));
                let local = desktop_session::message::chat::lifecycle::session_snapshot(
                    session_id,
                    info.title,
                    app.chat.clone(),
                    app.chat_message_ids.clone(),
                    now,
                );
                save_session_task(local, session_directory)
            }
            Err(error) => {
                app.push_localized_notification(
                    "重置会话失败: %s",
                    vec![format!("{}", error).into()],
                );
                Task::none()
            }
        },
        _ => Task::none(),
    }
}

/// 模块内可见函数，执行 start 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn start(app: &mut App, item: QueueItem, keep: bool) -> Task<Message> {
    use desktop_session::message::chat::lifecycle::{StartGate, gate_start};
    let active = app.active_session_id.is_some();
    let preparing = app.active_session_view_state.ui_preparing;
    let base_ready = app.active_session_view_state.base_ready;
    let item = match gate_start(
        app.current_session_runtime_mut(),
        item,
        keep,
        active,
        preparing,
        base_ready,
    ) {
        Ok(item) => item,
        Err(StartGate::HistoryQueued) => {
            return app.show_error_toast("会话历史正在加载，消息已保留在队列中");
        }
        Err(StartGate::PermissionQueued) => return Task::none(),
    };
    app.agent_stream_id = app.agent_stream_id.wrapping_add(1);
    let stream_id = app.agent_stream_id;
    let mut save_tasks = Vec::new();
    let mut initial_save = Task::none();
    app.cache_active_session_chat();

    if app.active_session_id.is_none() {
        app.invalidate_project_chat_preferences_read();
        let id = id::descending(id::Prefix::Session, None)
            .unwrap_or_else(|_| format!("ses_{}", stream_id));
        let project_path = app.navigation_tabs.project_path.clone().unwrap_or_default();
        let inferred_project_id = desktop_session::message::chat::lifecycle::inferred_project_id(
            app.project_id.as_deref(),
            &project_path,
            app.project_sessions.get(&project_path).into_iter().flatten(),
            app.sessions.iter(),
        );
        if inferred_project_id.is_none() {
            if project_path.trim().is_empty() {
                app.set_localized_input_error(
                    crate::app::i18n::input::ErrorField::General,
                    "未选择项目，无法创建会话",
                    vec![],
                );
                return Task::none();
            }
            // 项目页面已打开时，元数据可能尚未返回；由网关解析真实身份后继续发送。
            let directory = project_path.clone();
            return Task::perform(
                async move {
                    crate::app::message::project::helpers::resolve_gateway_project(&directory)
                        .await
                        .map(|project| project.id.0)
                },
                move |result| {
                    Message::Chat(ChatMessage::NewSessionProjectResolved {
                        project_path: project_path.clone(),
                        stream_id,
                        pending: Box::new(PendingSessionSend { item: item.clone(), keep }),
                        result,
                    })
                },
            );
        }
        let mut runtime = app.current_session_runtime();
        desktop_session::message::chat::lifecycle::initialize_draft(
            &mut runtime,
            project_path,
            app.chat.len(),
        );
        app.session_runtime_states.insert(id.clone(), runtime);
        app.reset_draft_full_access();
        app.active_session_id = Some(id);
        save_tasks.push(persist_session_reasoning_effort(app));
    }

    let request_started_ms = crate::app::time::now_ms();
    let mut chat = std::mem::take(&mut app.chat);
    let mut message_ids = std::mem::take(&mut app.chat_message_ids);
    let prepared = desktop_session::message::chat::lifecycle::prepare_request(
        app.current_session_runtime_mut(),
        item,
        &mut chat,
        &mut message_ids,
        request_started_ms,
    );
    app.chat = chat;
    app.chat_message_ids = message_ids;
    let desktop_session::message::chat::lifecycle::PreparedRequest {
        item,
        is_draft,
        request_history,
        request_query,
        appended_start_idx,
        appended_user_idx,
    } = prepared;
    app.invalidate_chat_ui_for_message_idx(
        appended_start_idx.min(app.chat.len().saturating_sub(1)),
    );
    if let Some(last_idx) = app.chat.len().checked_sub(1) {
        app.invalidate_chat_ui_for_message_idx(last_idx);
    }
    app.sync_chat_message_estimated_heights_len();
    app.refine_chat_message_estimated_heights(appended_start_idx, app.chat.len());
    app.active_session_view_state.updated_ms = item.created_ms;
    app.rebuild_active_session_message_meta();
    if let Some(session_id) = app.active_session_id.clone() {
        app.store_session_chat_snapshot(
            session_id,
            crate::app::session::shared_chat_messages(app.chat.clone()),
            app.chat_message_ids.clone(),
        );
    }
    if !keep {
        let is_empty_session = app.active_session_id.is_none();
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor = iced::widget::text_editor::Content::new();
        if is_empty_session {
            app.input_editor = runtime.input_editor.clone();
        }
    }

    if let Some(session_id) = app.active_session_id.clone() {
        {
            let runtime = app.get_session_runtime_mut(&session_id);
            desktop_session::message::chat::lifecycle::begin_request(
                runtime,
                item,
                stream_id,
                session_id.clone(),
                request_query,
                request_history,
                is_draft,
                request_started_ms,
                keep,
            );
        }
        app.sync_task_pet_from_runtime();

        {
            // Persist accepted input before model execution, including first-response failures.
            let now = crate::app::time::now_ms();
            let title = app
                .sessions
                .iter()
                .find(|s| s.id == session_id)
                .map(|s| s.title.clone())
                .unwrap_or_else(|| "新会话".to_string());
            let local = desktop_session::message::chat::lifecycle::session_snapshot(
                session_id.clone(),
                title,
                app.chat.clone(),
                app.chat_message_ids.clone(),
                now,
            );
            let session_directory =
                current_session_directory(app).or_else(|| app.navigation_tabs.project_path.clone());
            // A bounded history page is not a complete replacement snapshot.
            if app
                .session_runtime_states
                .get(&session_id)
                .is_none_or(|runtime| runtime.stream_save.seq_base == 0)
            {
                // Persist the initial rows of a registered session before starting deltas.
                initial_save = save_session_task(local, session_directory);
            }
            // The snapshot writer can fail or be coalesced independently. Include the
            // user row and assistant placeholder in the acknowledged delta queue too.
            app.get_session_runtime_mut(&session_id).stream_save.failures = 0;
            save_tasks.push(stream::register_stream_session_messages(
                app,
                &session_id,
                appended_start_idx,
                app.chat.len(),
                now,
            ));
        }
    }

    app.sync_active_session_from_chat();
    let submit = message::after(Duration::from_millis(200), Message::Chat(ChatMessage::SubmitTick));
    save_tasks.push(submit);
    save_tasks.push(Task::done(Message::Chat(ChatMessage::LoadInputPanelTodos)));
    if let Some(user_idx) = appended_user_idx {
        if user_idx == 0 {
            // 首轮消息天然位于顶部。此时滚动容器刚从空会话视图挂载，
            // 避免在首个有效视口回执前执行 snap_to 和布局测量。
            super::focus_chat_on_turn(app, user_idx);
        } else {
            save_tasks.push(super::scroll_chat_to_turn_top_task(app, user_idx));
        }
    } else if app.chat_auto_scroll {
        save_tasks.push(super::scroll_chat_to_bottom_task(app));
    }
    initial_save.chain(Task::batch(save_tasks))
}
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
