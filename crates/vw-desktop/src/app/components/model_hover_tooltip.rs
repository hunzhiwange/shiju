use crate::app::{App, Message};
use desktop_navigation::components::model_hover_tooltip as shared;
pub use shared::{HoverAnchor, hover_text_trigger, hover_model_trigger};
fn input(app: &App) -> shared::Input<'_> { shared::Input { model_popover_hover: &app.model_popover_hover, providers: &app.model_settings.providers, desktop_locale: app.general_settings.desktop_locale } }
pub fn hover_tooltip_overlay<'a>(app: &'a App, content: impl Into<iced::Element<'a, Message>>, style: fn(&iced::Theme) -> iced::widget::container::Style) -> iced::Element<'a, Message> { shared::hover_tooltip_overlay(input(app), content, style) }
