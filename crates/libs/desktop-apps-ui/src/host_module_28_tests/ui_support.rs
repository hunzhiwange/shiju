//! Local routing adapters for real provider views and reducers; no desktop App fixture.
pub use config_ui::DesktopLocale;
pub use desktop_i18n::I18n;
use desktop_provider_apps as domain;
pub use domain::message::desktop::Event as DesktopEvent;
pub use domain::message::edit::EditMessage;
pub use domain::{
    AgentAppsContext, Scope,
    state::{
        desktop::Form,
        edit::{EditForm, Mode},
        list::{Editor, Snapshot},
    },
};
use iced::{Element, Theme};
pub type ListState = domain::state::list::ListState<(), (), (), (), (), (), (), (), ()>;
#[derive(Clone)]
pub enum Message {
    None,
    Settings(SettingsMessage),
    CopyCode(String),
    Chat(ChatMessage),
    View(ViewMessage),
}
#[derive(Clone)]
pub enum ChatMessage {
    AppendText(String),
}
#[derive(Clone)]
pub enum ViewMessage {
    OpenUrlExternal(String),
}
#[derive(Clone)]
pub enum SettingsMessage {
    AppProviderEdit(EditMessage),
    AppProviders(ListMessage),
}
#[derive(Clone)]
pub enum ListMessage {
    Desktop(DesktopEvent),
    Refresh,
    Cancel,
    Close,
    ModelCatalog(desktop_settings_providers::catalog_api::Event),
}
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderViewEvent")
    }
}
#[derive(Default)]
pub struct Fixture {
    pub app_providers: domain::message::provider_update::State,
}
pub fn with_editor(f: impl FnOnce(Fixture, std::net::TcpListener, ())) {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    f(Fixture::default(), listener, ())
}
pub mod events {
    pub use desktop_provider_apps::message::*;
    pub mod edit {
        pub use desktop_provider_apps::message::edit::*;
        pub fn update(
            fixture: &mut super::super::Fixture,
            event: EditMessage,
        ) -> iced::Task<desktop_provider_apps::message::provider_update::Effect> {
            if change(&mut fixture.app_providers, &event) {
                return iced::Task::none();
            }
            update_domain(&mut fixture.app_providers, None, "", None, event)
        }
        use desktop_provider_apps::message::edit::update as update_domain;
    }
    pub mod desktop {
        pub use desktop_provider_apps::message::desktop::Event;
        pub fn update(
            fixture: &mut super::super::Fixture,
            event: Event,
        ) -> iced::Task<desktop_provider_apps::message::provider_update::Effect> {
            desktop_provider_apps::message::desktop::update(
                &mut fixture.app_providers,
                None,
                "",
                false,
                event,
            )
        }
    }
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect::*;
        match effect {
            Copy(text) => Self::CopyCode(text),
            Append(text) => Self::Chat(ChatMessage::AppendText(text)),
            OpenUrl(url) => Self::View(ViewMessage::OpenUrlExternal(url)),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
#[derive(Clone, Copy)]
pub struct Help<'a> {
    pub i18n: I18n,
    pub theme: &'a Theme,
}
impl<'h> Help<'h> {
    pub fn new(i18n: I18n, theme: &Theme) -> Help<'_> {
        Help { i18n, theme }
    }
    pub fn test() -> Help<'static> {
        Help::new(I18n::new(DesktopLocale::SimplifiedChinese), &Theme::Light)
    }
    pub fn provider(self) -> domain::components::help::Help<'h> {
        domain::components::help::Help { i18n: self.i18n, theme: self.theme, render: provider_help }
    }
}
fn provider_help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: I18n,
) -> Element<'a, domain::Message> {
    // Provider cards expose CopyCode; full selection actions are exercised through the view adapter below.
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<HelpEvent>(
        title, body, color, i18n,
    )
    .map(|e| match e {
        HelpEvent::Copy(text) => domain::Message::CopyCode(text),
        _ => domain::Message::None,
    })
}
#[derive(Clone)]
enum HelpEvent {
    None,
    Copy(String),
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for HelpEvent {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::Copy(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::Copy(text)
            }
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn map_provider(event: domain::Message) -> Message {
    match event {
        domain::Message::Edit(event) => Message::Settings(SettingsMessage::AppProviderEdit(event)),
        domain::Message::Desktop(event) => {
            Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(event)))
        }
        domain::Message::ListEditor(domain::components::list::EditorMessage::Cancel) => {
            Message::Settings(SettingsMessage::AppProviders(ListMessage::Cancel))
        }
        domain::Message::CopyCode(text) => Message::CopyCode(text),
        _ => Message::None,
    }
}
pub mod edit {
    use super::*;
    pub fn overlay<'a>(
        form: &'a EditForm,
        help: Help<'_>,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(base),
            Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel)),
            domain::components::edit::card(form, help.provider(), false, crate::agent_apps_diagnostics::endpoint_state::editable(form)).map(map_provider),
        )
    }
}
pub mod desktop_view {
    use super::*;
    pub fn overlay<'a>(
        form: &'a Form,
        help: Help<'_>,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(base),
            Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(
                DesktopEvent::Cancel,
            ))),
            domain::components::desktop::card(form, help.provider()).map(map_provider),
        )
    }
}
pub mod list {
    use super::*;
    pub fn overlay<'a>(
        state: &'a ListState,
        help: ui_support::Help<'a>,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let Some(scope) = &state.scope else { return base };
        let map = |event| match event {
            domain::components::list::ListMessage::Create => {
                Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Create))
            }
            domain::components::list::ListMessage::OpenEdit { id, copy, delete } => {
                Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Open {
                    id,
                    copy,
                    delete,
                }))
            }
            domain::components::list::ListMessage::Desktop(id) => Message::Settings(
                SettingsMessage::AppProviders(ListMessage::Desktop(DesktopEvent::Open(id))),
            ),
            domain::components::list::ListMessage::Refresh => {
                Message::Settings(SettingsMessage::AppProviders(ListMessage::Refresh))
            }
            _ => Message::None,
        };
        let card = domain::components::list::card(
            map,
            domain::components::list::View {
                scope: state.scope.as_ref(),
                query: &state.query,
                loading: state.loading,
                error: state.error.as_ref(),
                snapshot: state.snapshot.as_ref(),
                editing: state.form.is_some() || state.editor.is_some(),
            },
            vec![],
        );
        let base = desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(base),
            Message::Settings(SettingsMessage::AppProviders(ListMessage::Close)),
            card,
        );
        if let Some(form) = &state.model_catalog {
            let style = desktop_settings_providers::catalog::CatalogStyle {
                i18n: help.i18n,
                theme: help.theme,
                help_text: catalog_help,
            };
            let (close, card) = desktop_settings_providers::catalog::deployments(
                form,
                state.generation,
                help.i18n,
                style,
            );
            return desktop_ui_controls::system_settings_common::settings_modal_overlay(
                Some(base),
                map_catalog(close),
                card.map(map_catalog),
            );
        }
        if let Some((close, card)) = domain::components::list::local_card(state, help.provider()) {
            return desktop_ui_controls::system_settings_common::settings_modal_overlay(
                Some(base),
                map_provider(close),
                card.map(map_provider),
            );
        }
        if let Some(form) = &state.form {
            return edit::overlay(form, help, base);
        }
        if let Some(form) = &state.editor {
            return desktop_ui_controls::system_settings_common::settings_modal_overlay(
                Some(base),
                Message::Settings(SettingsMessage::AppProviders(ListMessage::Cancel)),
                domain::components::list::editor_card(
                    &form.sort,
                    &form.category,
                    &form.notes,
                    form.saving,
                )
                .map(map_provider),
            );
        }
        base
    }
}
fn map_catalog(event: desktop_settings_providers::view_message::Message) -> Message {
    match event {
        desktop_settings_providers::view_message::Message::Catalog(event) => Message::Settings(
            SettingsMessage::AppProviders(ListMessage::ModelCatalog(catalog_event(event))),
        ),
        desktop_settings_providers::view_message::Message::CopyCode(text) => {
            Message::CopyCode(text)
        }
        _ => Message::None,
    }
}
fn catalog_help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: I18n,
) -> Element<'a, desktop_settings_providers::view_message::Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<CatalogHelp>(
        title, body, color, i18n,
    )
    .map(|event| match event {
        CatalogHelp::Copy(text) => {
            desktop_settings_providers::view_message::Message::CopyCode(text)
        }
        _ => desktop_settings_providers::view_message::Message::None,
    })
}
#[derive(Clone)]
enum CatalogHelp {
    None,
    Copy(String),
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for CatalogHelp {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::Copy(text)
    }
    fn context(e: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match e {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(t) => {
                Self::Copy(t)
            }
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
pub mod agy {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::agy::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::agy::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod dsh {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::dsh::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::dsh::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod hermes {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::hermes::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::hermes::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod kimi {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::kimi::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::kimi::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod mcode {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::mcode::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::mcode::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod omp {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::omp::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::omp::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod openclaw {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::openclaw::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::openclaw::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod pi {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::pi::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::pi::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod claude {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::claude::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::claude::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod codex {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::codex::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::codex::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod gemini {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::gemini::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::gemini::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod grok {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::grok::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::grok::fields(form, draft, help.provider()).map(map_provider)
    }
}
pub mod opencode {
    use super::*;
    pub fn fields<'a>(
        form: &'a EditForm,
        draft: &'a domain::state::opencode::Draft,
        help: Help<'_>,
    ) -> Element<'a, Message> {
        domain::components::opencode::fields(form, draft, help.provider()).map(map_provider)
    }
}

pub use events::desktop;

fn catalog_event(
    event: desktop_settings_providers::catalog_message::Event,
) -> desktop_settings_providers::catalog_api::Event {
    use desktop_settings_providers::catalog_message::{
        DiscoveryEvent as V, Event as D, FillEvent as F,
    };
    use desktop_settings_providers::{catalog_api::Event as H, discovery, metadata_fill};
    match event {
        D::Open(value) => H::Open(value),
        D::Close => H::Close,
        D::Query(value) => H::Query(value),
        D::Search => H::Search,
        D::ChooseEndpoint { generation, preset } => H::ChooseEndpoint { generation, preset },
        D::Toggle { generation, preset, model, checked } => {
            H::Toggle { generation, preset, model, checked }
        }
        D::Endpoint(value) => H::Endpoint(value),
        D::ManualId(value) => H::ManualId(value),
        D::Alias(value) => H::Alias(value),
        D::MakeDefault(value) => H::MakeDefault(value),
        D::EditModel { generation, model_id } => H::EditModel { generation, model_id },
        D::CancelEdit => H::CancelEdit,
        D::LoadCatalog => H::LoadCatalog,
        D::Save => H::Save,
        D::Fill(event) => H::Fill(match event {
            F::Preview => metadata_fill::Event::Preview,
            F::Commit => metadata_fill::Event::Commit,
            F::Silent(value) => metadata_fill::Event::Silent(value),
        }),
        D::Discovery(event) => H::Discovery(match event {
            V::Start(method) => discovery::Event::Start(method),
            V::Cancel => discovery::Event::Cancel,
            V::Toggle { run, index, checked } => discovery::Event::Toggle { run, index, checked },
        }),
    }
}
