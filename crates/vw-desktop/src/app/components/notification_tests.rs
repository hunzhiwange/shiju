//! 验证通知面板的空态、编辑器内容与复制反馈渲染。
//! 有无详情编辑器分别构造，确保通知条目不会依赖可选正文。

use crate::app::components::notification::view;
use crate::app::{App, Message};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    let _ = element;
}

#[test]
fn view_builds_empty_notification_panel() {
    let app = test_app();

    keep_element(view(&app));
}

#[test]
fn view_builds_notification_items_with_editors_and_copy_feedback() {
    let mut app = test_app();
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("first".to_string()),
    );
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("second".to_string()),
    );
    app.copied_notification_id = Some(app.notifications[1].id);

    keep_element(view(&app));
}

#[test]
fn view_builds_notification_items_without_editor_content() {
    let mut app = test_app();
    let _ = crate::app::message::notification::update(
        &mut app,
        crate::app::message::NotificationMessage::Add("plain".to_string()),
    );
    app.notification_editors.clear();

    keep_element(view(&app));
}
