pub mod drop_area;
#[cfg(test)]
mod drop_area_tests;
pub mod attachment;
pub mod model_popover;
pub mod acp_options;
pub mod styles;

pub fn session_lock_tooltip<'a, Message: Clone + 'a>(content: impl Into<iced::Element<'a, Message>>, i18n: desktop_i18n::I18n, locked: bool) -> iced::Element<'a, Message> {
    if locked {
        let tip = iced::widget::container(iced::widget::text(i18n.__("会话开始后无法更改。如需更换，请新建会话。").to_owned()).size(12).color(iced::Color::WHITE))
            .style(desktop_ui_controls::tooltip::tooltip_dark_style).padding([6, 8]);
        iced::widget::tooltip(content, tip, iced::widget::tooltip::Position::Top).gap(6).into()
    } else { content.into() }
}
pub mod model_views;
pub mod icons;
pub mod agent_children;
pub mod agent_sessions;
