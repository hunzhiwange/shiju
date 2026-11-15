//! 全局菜单消息与导航控件之间的窄适配。
use crate::app::{Message, message};
use crate::app::message::view::MenuType;
use iced::Element;
pub(super) fn menu_btn<'a>(
    label: &'a str,
    menu_type: MenuType,
    active_menu: Option<MenuType>,
) -> Element<'a, Message> {
    desktop_navigation::components::top_bar::widgets::menu_btn_with_actions(
        label, active_menu == Some(menu_type),
        Message::View(message::ViewMessage::ToggleMenu(Some(menu_type))),
        Message::View(message::ViewMessage::MenuHovered(menu_type)),
    )
}


pub(super) fn menu_item_btn<'a>(label: &'a str, shortcut: Option<&'a str>, msg: Option<Message>) -> Element<'a, Message> {
 desktop_navigation::components::top_bar::widgets::menu_item_btn_with_action(label, shortcut, msg, |msg| Message::View(message::ViewMessage::MenuAction(Box::new(msg))))
}
pub(super) fn menu_item_icon_btn<'a>(icon: Element<'a, Message>, label: &'a str, shortcut: Option<&'a str>, msg: Option<Message>) -> Element<'a, Message> {
 desktop_navigation::components::top_bar::widgets::menu_item_icon_btn_with_action(icon, label, shortcut, msg, |msg| Message::View(message::ViewMessage::MenuAction(Box::new(msg))))
}
