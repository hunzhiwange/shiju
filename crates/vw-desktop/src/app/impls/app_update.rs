//! 应用程序消息更新模块
//!
//! 本模块实现了应用程序的核心消息处理逻辑，基于 Elm 架构的更新模式。
//! 所有用户交互、系统事件和异步操作结果都通过统一的消息机制路由到对应的处理器。
//!
//! # 主要功能
//!
//! - 消息路由：将不同类型的消息分发到对应的功能模块处理器
//! - 屏幕上下文感知：根据当前屏幕状态执行不同的逻辑分支
//! - 快捷键处理：处理复制、粘贴等全局快捷键操作
//! - 批量消息处理：支持批量执行多个消息以优化性能
//!
//! # 架构设计
//!
//! 采用 Elm 架构的 Update 模式，所有状态变更都通过消息驱动：
//! - 每个功能模块有独立的消息类型和更新函数
//! - 通过模式匹配实现消息分发
//! - 返回 Task 用于执行异步操作（如剪贴板写入）

use iced::Task;

use super::message;
use super::{App, Message, Screen};
use crate::app::ui_types::ClipboardShortcutKind;

#[cfg(test)]
use desktop_settings_network::bootstrap::selected_gateway_became_healthy;

fn route_clipboard_shortcut(app: &App, kind: ClipboardShortcutKind) -> Task<Message> {
    iced::widget::operation::is_focused(app.session_state.input_editor_id.clone())
        .map(move |input_focused| Message::ClipboardShortcutRouted { kind, input_focused })
}

fn copy_shortcut_without_input(app: &mut App) -> Task<Message> {
    if app.navigation_tabs.screen == Screen::Project && app.git.show_git_copy_modal {
        if app.git.git_copy_modal_use_color {
            message::git::update(
                app,
                message::GitMessage::CopyModalCodeEditorEvent(iced_code_editor::Message::Copy),
            )
        } else {
            Task::done(Message::CopyCode(app.git.git_copy_modal_editor.text()))
        }
    } else if app.navigation_tabs.screen == Screen::Preview {
        message::editor::update(app, message::EditorMessage::Copy)
    } else if app.navigation_tabs.screen == Screen::Project
        && app.navigation_panels.show_diff
        && !app.git.git_diff_selected_lines.is_empty()
    {
        message::git::update(app, message::GitMessage::CopyDiffSelection)
    } else if app.navigation_tabs.screen == Screen::Project
        && let Some(selection) = &app.chat_interaction.chat_markdown_selection
        && app.session_state.active_session_id.as_deref() == Some(selection.session_id.as_str())
    {
        Task::done(Message::CopyCode(selection.text.clone()))
    } else {
        message::editor::update(app, message::EditorMessage::Copy)
    }
}

pub(crate) fn copy_chat_message(
    app: &mut App,
    session_id: String,
    message_idx: usize,
) -> Task<Message> {
    if app.session_state.active_session_id.as_deref() != Some(session_id.as_str()) {
        return Task::none();
    }
    let Some(text) = crate::app::components::chat_panel::message_view::chat_message_action_text(
        app,
        message_idx,
        false,
    ) else {
        return Task::none();
    };
    copy_code(app, text)
}

fn copy_code(app: &mut App, text: String) -> Task<Message> {
    use std::time::Duration;
    let hash = app.clipboard_state.copied(&text);

    Task::batch(vec![
        copy_text_task(text, hash),
        message::after(Duration::from_secs(2), Message::CopyFeedbackExpired(hash)),
    ])
}

// Only called by explicit copy actions; never poll or read clipboard contents here.
fn copy_text_task(text: String, hash: u64) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            async move {
                arboard::Clipboard::new()
                    .and_then(|mut clipboard| clipboard.set_text(text))
                    .map_err(|_| "系统剪贴板不可用或访问被拒绝".to_string())
            },
            move |result| Message::ClipboardCopyFinished { hash, result },
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = hash;
        iced::clipboard::write(text).map(|_: ()| Message::CopyDone)
    }
}

impl App {
    /// 处理应用程序消息并更新状态
    ///
    /// 这是应用程序的核心更新函数，实现了 Elm 架构中的 update 模式。
    /// 所有状态变更都通过此函数进行，确保状态变更的可追踪性和可预测性。
    ///
    /// # 参数
    ///
    /// - `message`: 要处理的消息枚举，包含各种用户交互、系统事件或异步操作结果
    ///
    /// # 返回值
    ///
    /// 返回 `Task<Message>`，表示可能需要执行的异步操作。返回的 Task 可能是：
    /// - `Task::none()`: 无需执行任何操作
    /// - `Task::done()`: 立即产生一个新消息
    /// - `Task::batch()`: 批量执行多个任务
    /// - 其他异步任务（如剪贴板操作）
    ///
    /// # 消息处理流程
    ///
    /// 1. 根据消息类型进行模式匹配
    /// 2. 检查当前屏幕上下文（某些操作仅在特定屏幕下有效）
    /// 3. 委托给对应功能模块的更新函数
    /// 4. 返回可能需要的后续任务
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 处理项目消息
    /// let task = app.update(Message::Project(ProjectMessage::OpenFile(path)));
    ///
    /// // 批量处理多个消息
    /// let task = app.update(Message::Batch(vec![
    ///     Message::CopyCode(content),
    ///     Message::Notification(NotificationMessage::Show("已复制".into())),
    /// ]));
    /// ```
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if crate::app::message::app_providers::leave::intercept(self, &message) {
            return Task::none();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            message::voice::invalidate(self);
            if matches!(
                &message,
                Message::View(message::ViewMessage::CloseRequested(_))
                    | Message::View(message::ViewMessage::WindowClosed(_))
                    | Message::View(message::ViewMessage::GlobalKeyPressed(
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        _
                    ))
            ) {
                message::voice::cancel(self);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(
            message,
            Message::Settings(message::SettingsMessage::AgentAppsAuth(
                message::agent_apps_auth::Event::LoginTick
            ))
        ) {
            return Task::none();
        }
        let refresh_pins = self.prepare_session_ui_projection(&message);
        let task = match message {
            Message::SessionDetailsHovered { session_id, project_id, directory } => {
                self.load_session_hover_details(session_id, project_id, directory)
            }
            Message::SessionDetailsLoaded(id, details) => {
                self.session_ui_projection.hover_details_loading.remove(&id);
                self.session_ui_projection.hover_details.insert(id, details);
                Task::none()
            }
            Message::SessionPinsLoaded(generation, result) => {
                self.apply_session_pins(generation, result);
                Task::none()
            }
            #[cfg(not(target_arch = "wasm32"))]
            message @ (Message::StartupCliServiceBootstrapped(_)
            | Message::StartupGatewaySnapshotLoaded(..)
            | Message::StartupSessionsLoaded(..)) => {
                match super::app_init::dispatch::update(self, message) {
                    super::app_init::dispatch::Update::Continue(task) => task,
                    super::app_init::dispatch::Update::Return(task) => return task,
                }
            }
            message @ (Message::StartupAppConfigLoaded(_)
            | Message::BootstrapAppConfig(_)
            | Message::StartupSystemSettingsLoaded(_)
            | Message::BootstrapSystemSettings(_)
            | Message::StartupAgentConfigLoaded(_)
            | Message::StartupBrowserConfigLoaded(_)
            | Message::BootstrapBrowserConfig(_)
            | Message::BootstrapAcpAgentsLoaded(_)
            | Message::BootstrapArchivedSessions(_)
            | Message::ExternalAppsLoaded(_)) => {
                match super::app_init::dispatch::update(self, message) {
                    super::app_init::dispatch::Update::Continue(task) => task,
                    super::app_init::dispatch::Update::Return(task) => return task,
                }
            }
            Message::SessionPreviewsLoaded(result) => {
                match result {
                    Ok(previews) => {
                        self.session_state.session_previews =
                            crate::app::session::build_session_previews(previews);
                    }
                    Err(error) => {
                        self.session_state.session_previews.clear();
                        self.push_notification(format!(
                            "Failed to load session previews: {}",
                            error
                        ));
                    }
                }
                Task::none()
            }
            Message::ProjectChatPreferencesLoaded(scope, revision, result) => {
                self.project_chat_preferences_loaded(scope, revision, result)
            }
            Message::ProjectChatPreferencesSaved(scope, result) => {
                self.project_chat_preferences_saved(scope, result)
            }
            Message::DesktopPreferencesSaved(scope, result) => {
                self.desktop_preferences_saved(scope, result)
            }
            Message::SystemSettingsSaved(scope, section, result) => {
                self.system_settings_saved(scope, section, result)
            }
            message @ (Message::GatewayHealthTick | Message::GatewayHealthChecked(_)) => {
                match super::app_init::dispatch::update(self, message) {
                    super::app_init::dispatch::Update::Continue(task) => task,
                    super::app_init::dispatch::Update::Return(task) => return task,
                }
            }
            Message::Account(msg) => message::account::update(self, msg),
            #[cfg(not(target_arch = "wasm32"))]
            Message::ShijuApps(msg) => message::shiju_apps::update(self, msg),
            #[cfg(not(target_arch = "wasm32"))]
            Message::SkillHub(msg) => message::skill_hub::update(self, msg),
            // 项目管理相关消息，委托给 project 模块处理
            Message::Project(msg) => message::project::update(self, msg),

            // 视图相关消息，委托给 view 模块处理
            Message::View(msg) => message::view::update(self, msg),

            // Git 版本控制相关消息
            Message::Git(msg) => message::git::update(self, msg),

            // 聊天功能相关消息
            #[cfg(not(target_arch = "wasm32"))]
            Message::Voice(event) => message::voice::update(self, event),
            Message::Chat(msg) => message::chat::update(self, msg),

            // 搜索功能相关消息
            Message::Search(msg) => message::search::update(self, msg),

            // 设置相关消息
            Message::Settings(msg) => message::settings::update(self, msg),

            // 终端相关消息
            Message::Terminal(msg) => message::terminal::update(self, msg),

            // 预览相关消息
            Message::Preview(msg) => message::preview::update(self, msg),

            // 编辑器相关消息
            Message::Editor(msg) => message::editor::update(self, msg),

            // 知识库工作台相关消息
            Message::Knowledge(msg) => message::knowledge_tool::update(self, msg),

            // Dify 工作流工具相关消息
            Message::WorkflowTool(msg) => crate::apps::workflow::update(self, msg),

            Message::WorkflowManager(msg) => {
                crate::app::views::workflow_manager::update_app(self, msg)
            }

            // 通知相关消息
            Message::Notification(msg) => message::notification::update(self, msg),

            // 任务看板相关消息
            Message::TaskBoard(msg) => message::task_board::update(self, msg),

            // LSP 心跳消息（仅非 WASM 目标）
            // 用于定期检查语言服务器协议的状态
            #[cfg(not(target_arch = "wasm32"))]
            Message::PreviewLspTick => message::preview::tick_lsp(self),

            // 复制/剪切/粘贴：先判断聊天输入框是否聚焦，再路由到输入区或代码编辑器上下文
            Message::CopyShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Copy),
            Message::CutShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Cut),
            Message::PasteShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Paste),
            Message::ClipboardShortcutRouted { kind, input_focused } => match (kind, input_focused)
            {
                (ClipboardShortcutKind::Copy, true) => {
                    self.update(Message::Chat(message::ChatMessage::CopyInputSelection))
                }
                (ClipboardShortcutKind::Cut, true) => {
                    self.update(Message::Chat(message::ChatMessage::CutInputSelection))
                }
                (ClipboardShortcutKind::Paste, true) => {
                    self.update(Message::Chat(message::ChatMessage::PasteIntoInput))
                }
                (ClipboardShortcutKind::Copy, false) => copy_shortcut_without_input(self),
                (ClipboardShortcutKind::Cut, false) => {
                    message::editor::update(self, message::EditorMessage::Cut)
                }
                (ClipboardShortcutKind::Paste, false) => {
                    message::editor::update(self, message::EditorMessage::Paste)
                }
            },

            // 复制代码到剪贴板
            Message::CopyCode(s) => copy_code(self, s),
            Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text } => {
                desktop_chat_view::chat_ui::update_markdown_selection(
                    &mut self.chat_interaction.chat_markdown_selection,
                    self.session_state.active_session_id.as_deref(),
                    session_id,
                    message_idx,
                    text_idx,
                    text,
                );
                Task::none()
            }

            Message::CopyChatMessage { session_id, message_idx } => {
                copy_chat_message(self, session_id, message_idx)
            }

            // 复制文件内容到剪贴板
            Message::CopyFile(path) => match std::fs::read_to_string(path) {
                Ok(content) => copy_code(self, content),
                Err(_) => self.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "读取文件失败，未修改剪贴板",
                    vec![],
                ),
            },

            Message::ClipboardCopyFinished { hash, result } => {
                if result.is_err() {
                    self.clipboard_state.clear(hash);
                    self.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "复制失败：系统剪贴板不可用或访问被拒绝",
                        vec![],
                    )
                } else {
                    Task::none()
                }
            }

            // 复制完成消息，无需执行任何操作
            Message::CopyDone => Task::none(),

            Message::CopyFeedbackExpired(hash) => {
                self.clipboard_state.clear(hash);
                Task::none()
            }

            // 关闭错误提示
            Message::CloseError => {
                self.input_feedback.error_message = None;
                Task::none()
            }

            // 空消息，无需执行任何操作
            Message::None => Task::none(),

            // 批量消息处理
            Message::Batch(messages) => {
                // 对每个消息递归调用 update，收集所有返回的 Task
                let tasks: Vec<Task<Message>> =
                    messages.into_iter().map(|msg| self.update(msg)).collect();
                // 将多个 Task 合并为一个批量 Task
                Task::batch(tasks)
            }

            #[cfg(target_os = "macos")]
            Message::NativeMenu(action) => {
                match crate::app::native_menu::resolve_native_menu_action(self, action) {
                    Some(resolved) => self.update(resolved),
                    None => Task::none(),
                }
            }
        };

        #[cfg(target_os = "macos")]
        {
            crate::app::native_menu::install_or_sync(
                crate::app::native_menu::NativeMenuSnapshot::from_app(self),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        if !self.navigation_panels.show_system_settings
            || self.navigation_panels.system_settings_tab
                != crate::app::components::system_settings::SystemTab::Auth
        {
            self.agent_apps_auth.login = None;
            self.agent_apps_auth.binding = None;
            if self.agent_apps_auth.account_action.as_ref().is_some_and(|action| {
                !matches!(action.phase, crate::app::state::agent_apps_auth::ActionPhase::Saving)
            }) {
                self.agent_apps_auth.account_action = None;
            }
            self.agent_apps_auth.focused_service = None;
        }
        message::agent_apps_diagnostics::model::invalidate(self);
        #[cfg(not(target_arch = "wasm32"))]
        message::voice::invalidate(self);
        let cleanup = message::settings::mcp_management_probe::invalidate(self);
        let tools_cleanup = message::settings::mcp_management_tools::invalidate(self);
        let cron_history = message::settings::cron_history::reconcile(self);
        let leave = crate::app::message::app_providers::leave::resume(self);
        let task = Task::batch([task, cleanup, tools_cleanup, cron_history, leave]);
        if refresh_pins { Task::batch([task, self.refresh_session_pins_task()]) } else { task }
    }
}
#[cfg(test)]
#[path = "app_update_tests.rs"]
mod app_update_tests;
