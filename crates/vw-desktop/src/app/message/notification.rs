//! 通知消息处理模块
//!
//! 该模块负责处理应用程序中的通知系统相关的消息和状态更新。
//! 提供通知的添加、移除、展开/折叠切换以及批量清除等功能。
//!
//! # 核心组件
//!
//! - [`NotificationMessage`]: 定义通知操作的枚举类型
//! - [`update`]: 处理通知消息并更新应用状态的核心函数
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::message::notification::{NotificationMessage, update};
//!
//! // 添加新通知
//! let msg = NotificationMessage::Add("操作成功".to_string());
//! let task = update(&mut app, msg);
//!
//! // 移除指定通知
//! let msg = NotificationMessage::Remove(1);
//! let task = update(&mut app, msg);
//! ```

use crate::app::{App, Message, state::Notification};
use iced::Task;

/// 通知操作消息枚举
///
/// 定义了应用程序通知系统支持的所有操作类型。
/// 每个变体对应一种特定的通知管理行为。
///
/// # 变体说明
///
/// - `Add(String)`: 添加一条新通知，携带通知消息内容
/// - `Remove(usize)`: 移除指定 ID 的通知
/// - `ToggleExpanded`: 切换通知面板的展开/折叠状态
/// - `ClearAll`: 清除所有通知
///
/// # 示例
///
/// ```ignore
/// // 创建添加通知的消息
/// let add_msg = NotificationMessage::Add("任务完成".to_string());
///
/// // 创建移除通知的消息
/// let remove_msg = NotificationMessage::Remove(42);
///
/// // 切换展开状态
/// let toggle_msg = NotificationMessage::ToggleExpanded;
///
/// // 清除所有通知
/// let clear_msg = NotificationMessage::ClearAll;
/// ```
#[derive(Debug, Clone)]
pub enum NotificationMessage {
    /// 添加新通知
    ///
    /// 参数为通知的文本内容字符串
    Add(String),

    /// 复制指定通知内容
    Copy(usize),

    /// 移除指定通知
    ///
    /// 参数为要移除的通知的唯一标识符
    Remove(usize),

    /// 切换通知面板的展开/折叠状态
    ToggleExpanded,

    /// 清除所有通知
    ClearAll,

    /// 通知编辑器操作（支持选择复制，但禁止编辑）
    EditorAction(usize, iced::widget::text_editor::Action),

    /// 重置通知复制反馈
    ResetCopied(usize),

    /// 隐藏指定轻量提示
    HideToast(usize),
}

/// 处理通知消息并更新应用状态
///
/// 该函数是通知模块的核心处理器，根据接收到的消息类型
/// 执行相应的状态更新操作，并返回可能的后续任务。
///
/// # 参数
///
/// - `app`: 可变引用的应用状态实例，包含通知列表和相关状态
/// - `message`: 要处理的通知消息枚举值
///
/// # 返回值
///
/// 返回 `Task<Message>`，目前所有操作都返回 `Task::none()`，
/// 因为通知操作不需要触发额外的异步任务。
///
/// # 行为说明
///
/// ## Add 添加通知
///
/// 1. 获取当前的通知 ID 计数器值作为新通知的 ID
/// 2. 递增 ID 计数器，为下一条通知做准备
/// 3. 创建新的通知对象，包含 ID、消息内容和创建时间
/// 4. 将新通知添加到通知列表末尾
///
/// ## Remove 移除通知
///
/// 1. 从通知列表中移除指定 ID 的通知
/// 2. 如果移除后列表为空，自动将展开状态重置为 false
///
/// ## ToggleExpanded 切换展开状态
///
/// 切换通知面板的展开/折叠状态，用于 UI 交互控制
///
/// ## ClearAll 清除所有通知
///
/// 1. 清空整个通知列表
/// 2. 将展开状态重置为 false
///
/// # 示例
///
/// ```ignore
/// use crate::app::message::notification::{NotificationMessage, update};
///
/// // 添加通知
/// update(&mut app, NotificationMessage::Add("新消息".to_string()));
///
/// // 移除 ID 为 5 的通知
/// update(&mut app, NotificationMessage::Remove(5));
///
/// // 切换展开状态
/// update(&mut app, NotificationMessage::ToggleExpanded);
///
/// // 清除所有通知
/// update(&mut app, NotificationMessage::ClearAll);
/// ```
pub fn update(app: &mut App, message: NotificationMessage) -> Task<Message> {
    match message {
        // 处理添加通知消息
        NotificationMessage::Add(msg) => {
            // 获取当前通知 ID 并递增计数器
            let id = app.next_notification_id;
            app.next_notification_id += 1;

            // 创建并添加新通知到列表
            app.notifications.push(Notification {
                localized_message: None,
                id,
                message: msg.clone(),
                // 根据目标平台获取当前系统时间
                // 非 wasm32 平台使用标准库的 SystemTime
                // wasm32 平台使用 web_time crate 以支持 Web 环境
                created_at: {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        std::time::SystemTime::now()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_time::SystemTime::now()
                    }
                },
            });
            // 创建对应的编辑器内容，支持文本选择和复制
            app.notification_editors
                .insert(id, iced::widget::text_editor::Content::with_text(&msg));
            Task::none()
        }
        NotificationMessage::Copy(id) => {
            use std::time::Duration;

            let Some(notification) =
                app.notifications.iter().find(|notification| notification.id == id)
            else {
                return Task::none();
            };

            app.copied_notification_id = Some(id);

            Task::batch(vec![
                iced::clipboard::write(
                    notification.render(crate::app::i18n::I18n::new(app.desktop_locale)),
                )
                .map(|_: ()| Message::CopyDone),
                crate::app::message::after(
                    Duration::from_secs(2),
                    Message::Notification(NotificationMessage::ResetCopied(id)),
                ),
            ])
        }
        // 处理移除通知消息
        NotificationMessage::Remove(id) => {
            // 从列表中移除指定 ID 的通知
            app.notifications.retain(|n| n.id != id);
            // 移除对应的编辑器内容
            app.notification_editors.remove(&id);
            if app.copied_notification_id == Some(id) {
                app.copied_notification_id = None;
            }

            // 如果通知列表为空，自动折叠面板
            if app.notifications.is_empty() {
                app.notifications_expanded = false;
            }
            Task::none()
        }
        // 处理切换展开状态消息
        NotificationMessage::ToggleExpanded => {
            // 切换通知面板的展开/折叠状态
            app.notifications_expanded = !app.notifications_expanded;
            Task::none()
        }
        // 处理清除所有通知消息
        NotificationMessage::ClearAll => {
            // 清空通知列表并重置展开状态
            app.notifications.clear();
            app.notification_editors.clear();
            app.copied_notification_id = None;
            app.notifications_expanded = false;
            Task::none()
        }
        // 处理通知编辑器操作（只读模式：允许选择复制，禁止编辑）
        NotificationMessage::EditorAction(id, action) => {
            if let Some(content) = app.notification_editors.get_mut(&id) {
                match action {
                    // 忽略编辑操作，防止修改已发送的通知内容
                    iced::widget::text_editor::Action::Edit(_) => {}
                    // 允许其他操作如选择、复制等
                    other => {
                        content.perform(other);
                    }
                }
            }
            Task::none()
        }
        NotificationMessage::ResetCopied(id) => {
            if app.copied_notification_id == Some(id) {
                app.copied_notification_id = None;
            }
            Task::none()
        }
        NotificationMessage::HideToast(id) => {
            if app.active_toast.as_ref().map(|toast| toast.id) == Some(id) {
                app.active_toast = None;
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod notification_tests;
