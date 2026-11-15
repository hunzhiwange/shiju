//! 验证顶部栏通用部件的布局、样式和交互状态。
//!
//! 通用部件通过无头渲染与样式取样覆盖可用、悬停和禁用状态。

use iced::widget::{button, text};
use iced::{Element, Length};
use super::widgets::{menu_btn, menu_item_btn, menu_item_icon_btn};
use crate::app::Message;
use crate::app::message::view::MenuType;

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn menu_btn_fills_top_bar_height_for_inactive_menu() {
    let element = menu_btn("File", MenuType::File, None);

    assert_size(&element, Length::Shrink, Length::Fill);
}

#[test]
fn menu_btn_fills_top_bar_height_for_active_menu() {
    let element = menu_btn("View", MenuType::View, Some(MenuType::View));

    assert_size(&element, Length::Shrink, Length::Fill);
}

#[test]
fn menu_item_btn_with_message_fills_available_width() {
    let element = menu_item_btn("New session", Some("Cmd+N"), Some(Message::None));

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_btn_without_message_fills_available_width() {
    let element = menu_item_btn("Unavailable", None, None);

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_icon_btn_with_message_fills_available_width() {
    let icon: Element<'_, Message> = text("i").into();
    let element = menu_item_icon_btn(icon, "Open", Some("Cmd+O"), Some(Message::None));

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_icon_btn_without_message_fills_available_width() {
    let icon: Element<'_, Message> = button(text("i")).into();
    let element = menu_item_icon_btn(icon, "Open", None, None);

    assert_size(&element, Length::Fill, Length::Shrink);
}
