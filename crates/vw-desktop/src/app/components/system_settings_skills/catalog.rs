#[cfg(test)]
pub(super) fn skill_badge(label: impl Into<String>, emphasized: bool) -> Element<'static, Message> {
    desktop_settings_skills::catalog::skill_badge(label, emphasized).map(super::browser::message)
}

#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use iced::Element;
#[cfg(test)]
pub(super) use desktop_settings_skills::catalog::catalog_item_button_style;
#[cfg(test)]
pub(super) use desktop_settings_skills::catalog::icon_svg;
#[cfg(test)]
pub(super) use desktop_settings_skills::catalog::section_card_style;
