//! # 搜索消息模块
//!
//! 本模块提供应用内搜索功能的 UI 消息定义与状态更新逻辑。
//! 支持的搜索目标包括：项目路径、文件路径、以及历史会话。
//!
//! ## 主要功能
//!
//! - **输入变更处理**：响应搜索框输入变化，动态显示/隐藏搜索遮罩层
//! - **搜索遮罩切换**：控制搜索遮罩层的显示状态
//! - **项目选择**：根据搜索结果打开并索引指定项目
//! - **文件选择**：将选中的文件路径填入输入框
//! - **会话选择**：加载历史会话并恢复相关状态

use crate::app::{App, Message};
use iced::Task;

/// 搜索相关的 UI 消息枚举
///
/// 定义了搜索功能中所有可能的用户交互消息，
/// 包括输入变更、遮罩层切换、以及各类搜索结果的选择操作。
#[derive(Debug, Clone)]
pub enum SearchMessage {
    /// 打开搜索面板
    ///
    /// 会清空上一次查询，让 Command+P 或顶部搜索按钮始终从空输入开始。
    Open,

    /// 搜索输入框内容变更
    ///
    /// 当用户在搜索框中输入或修改文本时触发。
    /// 会自动根据输入内容是否为空来控制搜索遮罩层的显示。
    InputChanged(String),

    /// 切换搜索遮罩层的显示状态
    ///
    /// 用于手动控制搜索遮罩层的显示或隐藏。
    Toggle(bool),

    /// 选择项目
    ///
    /// 当用户从搜索结果中选择一个项目时触发。
    /// 会打开该项目并建立索引。
    SelectProject(String),

    /// 选择文件
    ///
    /// 当用户从搜索结果中选择一个文件时触发。
    /// 会将文件路径填入文件输入框。
    SelectFile(String),

    /// 选择会话
    ///
    /// 当用户从搜索结果中选择一个历史会话时触发。
    /// 会优先复用项目会话打开链路，否则直接按会话 ID 拉取完整消息。
    SelectSession(String),
}

#[cfg(test)]
mod tests;

/// 处理搜索相关的消息并更新应用状态
///
/// 根据传入的 [`SearchMessage`] 消息类型，执行相应的状态更新操作，
/// 并返回可能需要执行的后续任务。
///
/// # 参数
///
/// - `app`: 可变引用的应用状态，包含所有需要更新的字段
/// - `message`: 搜索消息枚举，指示要执行的具体操作
///
/// # 返回值
///
/// 返回一个 [`Task<Message>`]，可能包含后续需要执行的异步任务。
/// 大多数情况下返回 `Task::none()` 表示无需额外任务。
///
/// # 示例
///
/// ```ignore
/// let task = update(&mut app, SearchMessage::InputChanged("test".to_string()));
/// ```
pub fn update(app: &mut App, message: SearchMessage) -> Task<Message> {
    match message {
        SearchMessage::Open => {
            app.search_text.clear();
            app.show_search_overlay = true;
            app.refresh_search_panel_file_cache();
            Task::none()
        }

        // 处理搜索输入变更
        // 更新搜索文本并根据内容是否为空决定是否显示搜索遮罩层
        SearchMessage::InputChanged(v) => {
            app.search_text = v;
            // 仅当输入内容非空时显示搜索遮罩层
            app.show_search_overlay = !app.search_text.trim().is_empty();
            app.refresh_search_panel_file_cache();
            Task::none()
        }

        // 处理搜索遮罩层的显示/隐藏切换
        SearchMessage::Toggle(b) => {
            app.show_search_overlay = b;
            Task::none()
        }

        // 处理项目选择
        // 调用应用方法打开项目并建立索引
        SearchMessage::SelectProject(path) => {
            app.show_search_overlay = false;
            app.open_project_and_index(path)
        }

        // 处理文件选择
        // 将选中的文件路径填入文件 URL 输入框
        SearchMessage::SelectFile(path) => {
            app.file_url_input = path;
            app.show_search_overlay = false;
            Task::none()
        }

        // 处理会话选择
        // 加载历史会话数据，汇总 Token 使用量，并恢复会话状态
        SearchMessage::SelectSession(id) => {
            app.show_search_overlay = false;
            let project_path =
                app.known_session_directory(&id).filter(|directory| !directory.trim().is_empty());

            if let Some(project_path) = project_path {
                Task::done(Message::Project(
                    crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
                        project_path,
                        id,
                    ),
                ))
            } else {
                app.cache_active_session_chat();
                app.active_session_id = Some(id.clone());
                app.mark_active_session_viewed();
                app.restore_chat_for_session(&id);
                app.usage = crate::app::models::TokenUsage::default();
                app.active_session_view_state.updated_ms = 0;
                app.clear_active_session_steps();
                app.active_session_view_state.ui_preparing = true;
                app.active_session_view_state.base_ready = false;
                app.invalidate_chat_ui_state();
                app.sync_active_session_preferences();

                let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                let initial_prewarm_task = if app.chat.is_empty() {
                    Task::none()
                } else {
                    app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                    app.pin_chat_ui_chunk(Some(base_chunk_start));
                    crate::app::message::project::prepare_session_ui_task(
                        id.clone(),
                        app.active_shared_chat_messages(),
                        base_chunk_start,
                        true,
                        app.dialogue_flow_show_reasoning_summary,
                    )
                };

                Task::batch([
                    initial_prewarm_task,
                    crate::app::message::project::helpers::load_session_messages_task_scoped(
                        None, id,
                    ),
                    Task::done(Message::Chat(
                        crate::app::message::ChatMessage::LoadInputPanelTodos,
                    )),
                ])
            }
        }
    }
}
