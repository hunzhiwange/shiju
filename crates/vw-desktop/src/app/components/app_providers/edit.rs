use super::list::Help;
use crate::app::{Message,state::app_providers::edit::EditForm};
use iced::Element;
use super::super::system_settings_common::*;
pub(super) fn map(event: desktop_provider_apps::Message) -> Message {
    use desktop_provider_apps::Message as E;
    use crate::app::message::app_providers as events;
    match event {
        E::None => Message::None,
        E::CopyCode(value) => Message::CopyCode(value),
        E::Edit(value) => events::edit::message(value),
        E::Icons(value) => events::icons::message(value),
        E::Endpoints(value) => events::endpoints::message(value),
        E::Env(value) => events::env::message(value),
        E::Desktop(value) => events::desktop::message(value),
        E::EndpointDiagnostics => crate::app::message::agent_apps_diagnostics::endpoints::message(crate::app::message::agent_apps_diagnostics::endpoints::Event::Open),
        E::Binding => Message::None,
    }
}
pub(super) fn render_help<'a>(title: &'a str, body: std::borrow::Cow<'a,str>, color: Option<iced::Color>, i18n: crate::app::i18n::I18n) -> Element<'a,desktop_provider_apps::Message> {
    crate::app::components::chat_panel::message_view::selectable_titled_text(title,body,color,i18n).map(|message| match message {
        Message::CopyCode(value) => desktop_provider_apps::Message::CopyCode(value),
        _ => desktop_provider_apps::Message::None,
    })
}
#[cfg(test)]
pub(crate) fn overlay<'a>(form: &'a EditForm, help: Help<'_>,base:Element<'a,Message>)->Element<'a,Message>{overlay_with_binding(form,help,base,None)}
pub(super) fn overlay_with_binding<'a>(form:&'a EditForm,help:Help<'_>,base:Element<'a,Message>,binding:Option<Message>)->Element<'a,Message>{
    let diagnostics=crate::app::message::agent_apps_diagnostics::endpoint_state::editable(form);
    let card=desktop_provider_apps::components::edit::card(form,help.provider_help(),binding.is_some(),diagnostics).map(move |event| match event {
        desktop_provider_apps::Message::Binding=>binding.clone().unwrap_or(Message::None),
        event=>map(event),
    });
    settings_modal_overlay(Some(base),crate::app::message::app_providers::edit::message(crate::app::message::app_providers::edit::EditMessage::Cancel),card)
}
