use crate::provider_view_test_support::{Fixture, style};
use crate::view_message::Message;
use desktop_ui_controls::system_settings_common::settings_modal_overlay;
use iced::Element;

pub fn model_overlay<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    crate::custom_model_modal::view_overlays(
        app.provider_settings.custom_model_modal.as_ref(),
        app.provider_settings.save_error.as_ref(),
        style(app),
        base,
        |message| message,
    )
}

pub mod connected {
    use super::*;
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        crate::connected::view(&app.provider_settings, style(app))
    }
}

pub mod catalog {
    use super::*;
    pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
        let Some((close, card)) = crate::catalog::modal(&app.provider_settings, style(app)) else {
            return base;
        };
        settings_modal_overlay(Some(base), close, card)
    }
}

pub mod connect {
    use super::*;
    pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
        let Some((close, card)) = crate::connect::modal(&app.provider_settings, style(app)) else {
            return base;
        };
        settings_modal_overlay(Some(base), close, card)
    }
}

pub mod custom_provider {
    use super::*;
    pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
        let Some((close, card)) = crate::custom_provider::modal(&app.provider_settings, style(app))
        else {
            return base;
        };
        settings_modal_overlay(Some(base), close, card)
    }
}
