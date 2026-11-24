//! 桌面仅注入应用状态和导航并映射局部消息。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use iced::Element;
pub fn view<'a, M: Clone + 'static>(
    app: &WorkspaceInput<'a>,
    map: fn(Message) -> M,
    layout: impl FnOnce(Element<'a, M>) -> Element<'a, M>,
) -> Element<'a, M> {
    crate::views::shiju_apps::view(
        &input(app),
        super::section_tabs(app, map),
        move |message| map(map_app_message(message)),
        layout,
    )
}
fn input<'a>(app: &WorkspaceInput<'a>) -> crate::views::shiju_apps::ViewInput<'a> {
    crate::views::shiju_apps::ViewInput {
        shiju_apps: app.shiju_apps,
        desktop_locale: app.desktop_locale,
    }
}
fn map_app_message(message: crate::views::shiju_apps::Message) -> Message {
    match message {
        crate::views::shiju_apps::Message::ShijuApps(m) => Message::ShijuApps(m),
        crate::views::shiju_apps::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
    }
}
