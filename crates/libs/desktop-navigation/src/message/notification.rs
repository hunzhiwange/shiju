//! 通知列表、只读编辑器和反馈的局部更新。
use crate::components::notification::Notification;
use desktop_ui_controls::toast::Toast;
use std::collections::HashMap;
use iced::widget::text_editor;

pub struct State<'a> {
    pub notifications: &'a mut Vec<Notification>,
    pub notification_editors: &'a mut HashMap<usize, text_editor::Content>,
    pub next_notification_id: &'a mut usize,
    pub copied_notification_id: &'a mut Option<usize>,
    pub notifications_expanded: &'a mut bool,
    pub active_toast: &'a mut Option<Toast>,
    pub desktop_locale: config_ui::DesktopLocale,
}
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


/// 返回需要由宿主复制并安排反馈复位的通知。
pub fn update(app: State<'_>, message: NotificationMessage) -> Option<(usize, String)> {
    match message {
        // 处理添加通知消息
        NotificationMessage::Add(msg) => {
            // 获取当前通知 ID 并递增计数器
            let id = *app.next_notification_id;
            *app.next_notification_id += 1;

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
            None
        }
        NotificationMessage::Copy(id) => {
            let notification = app.notifications.iter().find(|notification| notification.id == id)?;
            *app.copied_notification_id = Some(id);
            Some((id, notification.render(desktop_i18n::I18n::new(app.desktop_locale))))
        }

        // 处理移除通知消息
        NotificationMessage::Remove(id) => {
            // 从列表中移除指定 ID 的通知
            app.notifications.retain(|n| n.id != id);
            // 移除对应的编辑器内容
            app.notification_editors.remove(&id);
            if *app.copied_notification_id == Some(id) {
                *app.copied_notification_id = None;
            }

            // 如果通知列表为空，自动折叠面板
            if app.notifications.is_empty() {
                *app.notifications_expanded = false;
            }
            None
        }
        // 处理切换展开状态消息
        NotificationMessage::ToggleExpanded => {
            // 切换通知面板的展开/折叠状态
            *app.notifications_expanded = !*app.notifications_expanded;
            None
        }
        // 处理清除所有通知消息
        NotificationMessage::ClearAll => {
            // 清空通知列表并重置展开状态
            app.notifications.clear();
            app.notification_editors.clear();
            *app.copied_notification_id = None;
            *app.notifications_expanded = false;
            None
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
            None
        }
        NotificationMessage::ResetCopied(id) => {
            if *app.copied_notification_id == Some(id) {
                *app.copied_notification_id = None;
            }
            None
        }
        NotificationMessage::HideToast(id) => {
            if app.active_toast.as_ref().map(|toast| toast.id) == Some(id) {
                *app.active_toast = None;
            }
            None
        }
    }
}

#[cfg(test)]
#[path = "notification_tests.rs"]
mod tests;
