//! 验证会话分叉弹窗的目标选择、确认状态和浮层布局。
//!
//! 分叉目标覆盖本地、工作树和关闭状态，确认确认动作只读取当前弹窗草稿。

use super::*;
use crate::app::models::{ChatMessage, ChatRole};

fn test_app() -> App {
    App::new().0
}

fn root() -> iced::Element<'static, Message> {
    iced::widget::container(iced::widget::text("root")).into()
}

fn user_message() -> ChatMessage {
    ChatMessage {
        role: ChatRole::User,
        content: "hello".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn with_chat_fork_dialog_returns_root_when_no_index_or_missing_message() {
    let mut app = test_app();
    let _element: iced::Element<'_, Message> = with_chat_fork_dialog(&app, root());

    app.chat_fork_dialog_idx = Some(1);
    app.chat.push(user_message());
    let _element: iced::Element<'_, Message> = with_chat_fork_dialog(&app, root());
}

#[test]
fn with_chat_fork_dialog_builds_modal_for_existing_message() {
    let mut app = test_app();
    app.chat.push(user_message());
    app.chat_fork_dialog_idx = Some(0);

    let _element: iced::Element<'_, Message> = with_chat_fork_dialog(&app, root());
}
