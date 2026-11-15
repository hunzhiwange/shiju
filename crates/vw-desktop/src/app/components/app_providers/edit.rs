use super::list::Help;
use crate::app::{Message,state::app_providers::edit::EditForm};
use iced::Element;
use super::super::system_settings_common::*;
pub(super) fn map(event: desktop_provider_apps::Message) -> Message {
    use desktop_provider_apps::Message as E;
    use crate::app::message::app_providers as events;
    match event {
E::Mode(value) => { use desktop_provider_apps::components::mode::Event as V; events::mode::message(match value { V::Close => events::mode::Event::Close,V::Refresh => events::mode::Event::Refresh,V::Cancel => events::mode::Event::Cancel,V::Confirm => events::mode::Event::Confirm,V::ReviewDirect => events::mode::Event::ReviewDirect, }) },
E::Paths(value) => { use desktop_provider_apps::components::paths::Event as V; events::paths::message(match value { V::Close => events::paths::Event::Close,V::Refresh => events::paths::Event::Refresh,V::Draft(value) => events::paths::Event::Draft(value),V::PickFolder => events::paths::Event::PickFolder,V::OpenFolder => events::paths::Event::OpenFolder,V::Save => events::paths::Event::Save, }) },
E::Preview(value) => { use desktop_provider_apps::components::preview::Event as V; events::preview::message(match value { V::Close => events::preview::Event::Close,V::Refresh => events::preview::Event::Refresh,V::Edit(value) => events::preview::Event::Edit(value),V::Save(value) => events::preview::Event::Save(value),V::CancelSave => events::preview::Event::CancelSave,V::Check => events::preview::Event::Check, }) },
E::Snippets(value) => { use desktop_provider_apps::components::snippets::Event as V; events::snippets::message(match value { V::Close => events::snippets::Event::Close,V::SelectProvider(value) => events::snippets::Event::SelectProvider(value),V::Extract => events::snippets::Event::Extract,V::Edit(value) => events::snippets::Event::Edit(value),V::RetryRead => events::snippets::Event::RetryRead,V::AdoptExtracted => events::snippets::Event::AdoptExtracted,V::Preview => events::snippets::Event::Preview,V::Save => events::snippets::Event::Save, }) },
E::Universal(value) => { use desktop_provider_apps::components::universal::Event as V; events::universal::message(match value { V::Close => events::universal::Event::Close,V::Select(value) => events::universal::Event::Select(value),V::Toggle(value, checked) => events::universal::Event::Toggle(value, checked),V::RetryList => events::universal::Event::RetryList,V::Submit => events::universal::Event::Submit, }) },
        E::ListEditor(value) => { use desktop_provider_apps::components::list::EditorMessage as V; use events::list::ListMessage as L; events::list::message(match value { V::Sort(v)=>L::Sort(v),V::Category(v)=>L::Category(v),V::Notes(v)=>L::Notes(v),V::Cancel=>L::Cancel,V::Save=>L::Save }) },
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
