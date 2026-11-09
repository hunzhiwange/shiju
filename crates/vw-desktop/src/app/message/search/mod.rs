//! # 搜索消息模块
//!
//! 本模块提供应用内搜索功能的 UI 消息定义与状态更新逻辑。
//! 支持的搜索目标包括：项目路径、文件路径、以及历史会话。
//!
//! ## 主要功能
//!
//! - **输入变更处理**：响应搜索框输入变化并刷新搜索结果
//! - **搜索遮罩切换**：控制搜索遮罩层的显示状态
//! - **项目选择**：根据搜索结果打开并索引指定项目
//! - **文件选择**：将选中的文件路径填入输入框
//! - **会话选择**：加载历史会话并恢复相关状态

use crate::app::{App, Message, components::search_panel};
use iced::{Task, widget::operation};

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
    /// 输入内容不控制搜索遮罩层，清空查询时弹窗仍保持打开。
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
    /// 通过项目导航入口验证来源后再打开会话。
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
            operation::focus(search_panel::input_id())
        }

        // 处理搜索输入变更；弹窗仅由打开、关闭或结果选择操作控制。
        SearchMessage::InputChanged(v) => {
            app.search_text = v;
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
            let path = app.known_session_directory(&id).unwrap_or_default();
            Task::done(Message::Project(
                crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(path, id),
            ))
        }
    }
}
