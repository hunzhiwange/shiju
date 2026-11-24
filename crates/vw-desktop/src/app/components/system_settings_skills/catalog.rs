#[cfg(test)]
pub(super) fn skill_badge(label: impl Into<String>, emphasized: bool) -> Element<'static, Message> {
    desktop_settings_skills::catalog::skill_badge(label, emphasized).map(super::browser::message)
}

#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use iced::Element;
