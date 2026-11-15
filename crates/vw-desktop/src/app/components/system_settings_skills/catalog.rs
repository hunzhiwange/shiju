use crate::app::Message;
pub(super) use desktop_settings_skills::catalog::{
    catalog_item_button_style, icon_svg, section_card_style,
};
use iced::Element;
pub(super) fn skill_badge(label: impl Into<String>, emphasized: bool) -> Element<'static, Message> {
    desktop_settings_skills::catalog::skill_badge(label, emphasized).map(super::browser::message)
}
