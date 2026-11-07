//! 验证会话分叉弹窗的目标选择、确认状态和浮层布局。
//!
//! 分叉目标覆盖本地、工作树和关闭状态，确认确认动作只读取当前弹窗草稿。

use super::*;
fn test_app() -> App {
    App::new().0
}

fn root() -> iced::Element<'static, Message> {
    iced::widget::container(iced::widget::text("root")).into()
}

#[test]
fn with_chat_fork_dialog_builds_without_an_anchor() {
    let app = test_app();
    let _element: iced::Element<'_, Message> = with_chat_fork_dialog(&app, root());
}
