//! Local page inputs and explicit widget event mapping; no desktop App fixture.
use crate::system_settings_apps as domain;
use desktop_i18n::I18n;
pub use domain::lifecycle_update::LifecycleMessage;
use domain::{Action, catalog::CatalogRow};
use iced::{
    Element, Theme,
    widget::{column, text},
};
type Context = desktop_apps_ui::AgentAppsContext;
#[derive(Clone, Debug)]
pub enum Message {
    Settings(SettingsMessage),
    CopyCode(String),
    View(ViewMessage),
    None,
}
#[derive(Clone, Debug)]
pub enum ViewMessage {
    OpenUrlExternal(String),
}
#[derive(Clone, Debug)]
pub enum SettingsMessage {
    AppsCatalog(AppsCatalogMessage),
}
#[derive(Clone)]
pub enum AppsCatalogMessage {
    EnsureLoaded,
    LocalReady {
        context: Context,
        generation: u64,
        result: Result<local_app_auth::Credentials, String>,
    },
    Refresh,
    QueryInput(String),
    ConnectionOpen,
    ConnectionClose,
    HostInput(String),
    TokenInput(String),
    ToggleDetails(vw_api_app_identity::AppId),
    Drag(domain::drag::DragMessage),
    Launch(domain::launch::LaunchMessage<Context>),
    Preferences(domain::preferences::PreferencesMessage<Context>),
    Lifecycle(LifecycleMessage),
    Other,
}
pub fn message(event: AppsCatalogMessage) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(event))
}
pub use crate::system_settings_apps::catalog_update::CatalogState as AppsCatalogState;
fn view_state(state: &AppsCatalogState) -> domain::ViewState<'_> {
    domain::ViewState {
        rows: &state.rows,
        preferences: &state.preferences,
        lifecycle: &state.lifecycle,
        drag: &state.drag,
        query: &state.query,
        host: &state.host,
        loading: state.loading,
        error: &state.error,
        expanded: state.expanded,
        hovered: state.hovered,
    }
}
fn action(action: Action) -> Message {
    if matches!(action, Action::Download) {
        return Message::View(ViewMessage::OpenUrlExternal(
            domain::uninstall::MANUAL_DOWNLOAD_URL.into(),
        ));
    }
    use AppsCatalogMessage as M;
    message(match action {
        Action::Refresh => M::Refresh,
        Action::QueryInput(v) => M::QueryInput(v),
        Action::ConnectionOpen => M::ConnectionOpen,
        Action::ToggleDetails(id) => M::ToggleDetails(id),
        Action::Drag(v) => M::Drag(v),
        Action::LaunchOpen(id, installation) => {
            M::Launch(domain::launch::LaunchMessage::Open(id, installation))
        }
        Action::PreferencesOpen => M::Preferences(domain::preferences::PreferencesMessage::Open),
        Action::Preview(v) => M::Lifecycle(LifecycleMessage::Preview(v)),
        Action::LifecycleClose => M::Lifecycle(LifecycleMessage::Close),
        Action::LifecycleConfirm => M::Lifecycle(LifecycleMessage::Confirm),
        Action::LifecycleRefresh => M::Lifecycle(LifecycleMessage::Refresh),
        Action::LifecycleCancel => M::Lifecycle(LifecycleMessage::Cancel),
        _ => M::Other,
    })
}
impl desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn heading<'a>(
    i18n: I18n,
    theme: Option<&Theme>,
    title: &'a str,
    description: &'a str,
    size: u16,
) -> Element<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        text(i18n.__(title)).size(size as f32),
        desktop_ui_controls::selectable_text::markdown_selection::selectable_titled_text(
            title,
            i18n.__(description),
            theme.map(|t| t.palette().text),
            i18n,
        ),
    )
    .wrap()
    .into()
}
pub fn localized_view(state: &AppsCatalogState, i18n: I18n) -> Element<'_, Message> {
    view(state, i18n)
}
pub fn view(state: &AppsCatalogState, i18n: I18n) -> Element<'_, Message> {
    let input = view_state(state);
    let details = state.rows.iter().find(|r| Some(r.entry.id) == state.expanded).map(|row| {
        domain::details::view(
            &input,
            row,
            column![].into(),
            domain::uninstall::MANUAL_GUIDANCE,
            action,
        )
    });
    domain::list::view(
        &input,
        |title, description| heading(i18n, None, title, description, 17),
        true,
        domain::domains::view(column![].into(), |_| message(AppsCatalogMessage::Other)),
        column![].into(),
        details,
        action,
    )
}
pub fn view_overlays<'a>(
    state: &'a AppsCatalogState,
    i18n: I18n,
    theme: &Theme,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if state.connection_open {
        return domain::connection::overlay(&state.host, &state.token, base, |event| {
            use domain::connection::ConnectionMessage as E;
            message(match event {
                E::Close => AppsCatalogMessage::ConnectionClose,
                E::HostInput(v) => AppsCatalogMessage::HostInput(v),
                E::TokenInput(v) => AppsCatalogMessage::TokenInput(v),
                E::Refresh => AppsCatalogMessage::Refresh,
            })
        });
    }
    launch::overlay(
        state,
        i18n,
        theme,
        preferences::overlay(
            state,
            i18n,
            theme,
            lifecycle::overlay(&state.lifecycle, &state.host, base),
        ),
    )
}
pub mod launch {
    use super::*;
    pub use domain::launch::{LaunchChoice, LaunchState};
    pub type LaunchMessage = domain::launch::LaunchMessage<Context>;
    pub fn overlay<'a>(
        state: &'a AppsCatalogState,
        i18n: I18n,
        theme: &Theme,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        domain::launch_view::overlay(
            &state.launch,
            &state.host,
            base,
            |title, description, size| heading(i18n, Some(theme), title, description, size),
            |event| message(AppsCatalogMessage::Launch(event)),
        )
    }
}
pub mod preferences {
    use super::*;
    pub use domain::preferences::PreferencesState;
    pub fn visible_rows(state: &AppsCatalogState) -> Vec<&CatalogRow> {
        domain::preferences::visible_rows(&view_state(state))
    }
    pub type PreferencesMessage = domain::preferences::PreferencesMessage<Context>;
    pub fn overlay<'a>(
        state: &'a AppsCatalogState,
        i18n: I18n,
        theme: &Theme,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        domain::preferences_view::overlay(
            &state.preferences,
            &state.host,
            &state.rows,
            base,
            |title, description, size| heading(i18n, Some(theme), title, description, size),
            |event| message(AppsCatalogMessage::Preferences(event)),
        )
    }
}
pub mod lifecycle {
    use super::*;
    pub use domain::lifecycle::*;
    pub fn overlay<'a>(
        state: &'a LifecycleState,
        host: &'a str,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let guidance = state
            .preview
            .as_ref()
            .map(|p| domain::uninstall::guidance(p.request.app_id))
            .unwrap_or("");
        domain::lifecycle_view::overlay(state, host, base, guidance, action)
    }
}
pub mod drag {
    pub use crate::system_settings_apps::drag::*;
}
pub fn preferences_route(
    state: &mut AppsCatalogState,
    event: preferences::PreferencesMessage,
) -> iced::Task<preferences::PreferencesMessage> {
    let context = state.context.clone();
    let gateway = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "127.0.0.1",
        std::env::var("SHIJU_APPS_DRAG_FIXTURE_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(1),
    ))
    .map_err(|e| e.to_string());
    domain::preferences::update(
        &mut domain::preferences::Input {
            context: &state.context,
            host: &state.host,
            token: &state.token,
            generation: state.generation,
            preferences: &mut state.preferences,
        },
        context,
        gateway,
        event,
    )
}

pub mod details {
    use super::*;
use iced::widget::column;
    pub fn view<'a>(state: &'a AppsCatalogState, row: &'a CatalogRow) -> Element<'a, Message> {
        domain::details::view(
            &view_state(state),
            row,
            column![].into(),
            domain::uninstall::MANUAL_GUIDANCE,
            action,
        )
    }
}

pub fn drag_update(
    state: &mut AppsCatalogState,
    value: domain::drag::DragMessage,
) -> iced::Task<preferences::PreferencesMessage> {
    let result = domain::drag::update(&view_state(state), value);
    state.drag = result.drag;
    state.expanded = result.expanded;
    state.hovered = result.hovered;
    match result.order {
        Some(order) => preferences_route(state, preferences::PreferencesMessage::Reorder(order)),
        None => iced::Task::none(),
    }
}

pub fn catalog_route(state: &mut AppsCatalogState, event: AppsCatalogMessage) {
    use domain::catalog_update::RefreshDecision;
    match event {
        AppsCatalogMessage::HostInput(value) => state.input().credentials(Some(value), None),
        AppsCatalogMessage::TokenInput(value) => state.input().credentials(None, Some(value)),
        AppsCatalogMessage::Refresh | AppsCatalogMessage::EnsureLoaded => {
            let cached = matches!(event, AppsCatalogMessage::EnsureLoaded);
            if state.input().refresh_decision(cached, false) == RefreshDecision::Load {
                state.begin(state.context.clone().unwrap());
            }
        }
        AppsCatalogMessage::LocalReady { context, generation, result } => {
            let current = state.context.clone();
            if state.input().local_ready(current.as_ref(), &context, generation, result) == Ok(true)
            {
                catalog_route(state, AppsCatalogMessage::Refresh);
            }
        }
        _ => {}
    }
}

impl std::fmt::Debug for AppsCatalogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AppsCatalogMessage")
    }
}
