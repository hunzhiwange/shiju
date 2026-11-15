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

pub mod slice_message;
pub mod file_references;
#[cfg(test)]
mod file_references_tests;
pub mod file_search;
#[cfg(test)]
mod file_search_tests;
pub mod queue_panel;
#[cfg(test)]
mod queue_panel_tests;
pub mod input_editor;
#[cfg(test)]
mod input_editor_tests;
#[cfg(test)]
mod icons_tests;

pub mod quick_messages;
pub mod quick_message;
pub mod quick_styles;
pub mod reasoning_selector;
pub mod reasoning_message;

#[cfg(test)]
mod reasoning_selector_tests;

pub mod send_message;
pub mod send_controls;
pub mod slash_palette;
pub mod surface_styles;
#[cfg(test)]
mod send_controls_tests;
#[cfg(test)]
mod surface_styles_tests;

pub mod task_mode;
#[cfg(test)]
mod task_mode_tests;

pub mod todo_panel;
#[cfg(test)]
mod todo_panel_tests;

pub mod usage;
#[cfg(test)]
mod usage_tests;

pub mod usage_button;
#[cfg(test)]
mod usage_button_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod voice_controls;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod voice_controls_tests;

pub mod panel;
#[cfg(test)]
mod panel_tests;
pub mod layout;
