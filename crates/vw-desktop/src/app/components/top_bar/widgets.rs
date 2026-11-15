//! 全局菜单消息与导航控件之间的窄适配。
use crate::app::{Message, message};
use crate::app::message::view::MenuType;
use iced::widget::{button, text, mouse_area};
use iced::{Element, Length, Theme};
pub(super) use desktop_navigation::components::top_bar::widgets::{icon_svg, icon_tooltip_style, menu_button_style, workspace_mode_button_style, menu_container_style, menu_container, menu_separator};
#[cfg(test)]
pub(super) use desktop_navigation::components::top_bar::widgets::{color_with_alpha, icon_svg_style, icon_toggle_button_style, icon_button, icon_toggle_button, icon_toggle_button_opt, menu_item_button_style, menu_separator_style};
pub(super) fn menu_btn<'a>(
    label: &'a str,
    menu_type: MenuType,
    active_menu: Option<MenuType>,
) -> Element<'a, Message> {
    let is_active = active_menu == Some(menu_type);

    let btn = button(
        text(label).size(13).height(Length::Fill).align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::View(message::ViewMessage::ToggleMenu(Some(menu_type))))
    .height(Length::Fill)
    .style(move |theme: &Theme, status| menu_button_style(theme, status, is_active))
    .padding([0, 10]);

    mouse_area(btn).on_enter(Message::View(message::ViewMessage::MenuHovered(menu_type))).into()
}


pub(super) fn menu_item_btn<'a>(label: &'a str, shortcut: Option<&'a str>, msg: Option<Message>) -> Element<'a, Message> {
 desktop_navigation::components::top_bar::widgets::menu_item_btn_with_action(label, shortcut, msg, |msg| Message::View(message::ViewMessage::MenuAction(Box::new(msg))))
}
pub(super) fn menu_item_icon_btn<'a>(icon: Element<'a, Message>, label: &'a str, shortcut: Option<&'a str>, msg: Option<Message>) -> Element<'a, Message> {
 desktop_navigation::components::top_bar::widgets::menu_item_icon_btn_with_action(icon, label, shortcut, msg, |msg| Message::View(message::ViewMessage::MenuAction(Box::new(msg))))
}
