//! Account-domain composition and cross-domain usage-message mapping.
use crate::app::{App, Message};
use super::apps_usage_cards;
use desktop_settings_account::{self as account, Event};
use iced::Element;
#[cfg(test)]
mod binding_modal_tests;
#[cfg(test)]
pub(in crate::app) mod login_modal_tests;
fn state(app: &App) -> account::State<'_> { project(&app.agent_apps_auth) }
fn project(s: &crate::app::state::agent_apps_auth::State) -> account::State<'_> {
    account::State {
        scope: &s.scope,
        rows: &s.rows,
        form: &s.form,
        connection: &s.connection,
        connection_error: &s.connection_error,
        login: &s.login,
        binding: &s.binding,
        account_action: &s.account_action,
        pending: s.pending,
        loopback: s.loopback,
        focused_service: s.focused_service,
        copy_login_link: s.copy_login_link,
        rows_generation: s.rows_generation(),
    }
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let s = &app.agent_apps_auth;
    let scope = apps_usage_cards::current_scope(app, true);
    let mut accounts = std::collections::HashMap::new();
    if s.connection.is_some() {
        for row in s.rows.iter().flatten() {
            if let Ok(entries) = &row.accounts {
                for entry in entries {
                    if entry.credential_state == vw_api_app_auth::AuthCredentialState::Ready
                        && apps_usage_cards::binding_app(row.service).is_some() {
                        accounts.insert((row.service, entry.account_id.clone()),
                            apps_usage_cards::account_view(scope.as_ref(), &s.usage, row.service, &entry.account_id)
                                .map(Event::Usage));
                    }
                }
            }
        }
    }
    account::system_settings_auth::view(&state(app), account::Usage {
        available: scope.is_some(),
        batch_pending: scope.as_ref().is_some_and(|scope| s.usage.batch_pending(scope)),
        lines: scope.as_ref().map_or_else(Vec::new, |scope| s.usage.batch_lines(scope)),
        accounts,
    }).map(map)
}
pub(crate) fn overlay<'a>(s: &'a crate::app::state::agent_apps_auth::State, base: Element<'a, Message>) -> Element<'a, Message> {
    account::system_settings_auth::overlay(&project(s), base.map(Event::Usage)).map(map)
}
fn map(event: Event<Message>) -> Message {
    use crate::app::message::agent_apps_auth::{self as auth, Event as E, actions::ActionEvent as A, browser::LinkAction as L};
    let event = match event {
        Event::Usage(message) => return message,
        Event::RefreshUsage => return apps_usage_cards::message_event(apps_usage_cards::Event::RefreshAll),
        Event::Open => E::Open,
        Event::Close => E::Close,
        Event::Connect => E::Connect,
        Event::Refresh => E::Refresh,
        Event::RetryLogin => E::RetryLogin,
        Event::CloseLogin => E::CloseLogin,
        Event::Host(value) => E::Host(value),
        Event::Username(value) => E::Username(value),
        Event::Password(value) => E::Password(value),
        Event::Token(value) => E::Token(value),
        Event::RetryService(value) => E::RetryService(value),
        Event::OpenBrowserPreference(value) => E::OpenBrowserPreference(value),
        Event::StartLogin(value) => E::StartLogin(value),
        Event::PasteSubmit(value) => E::PasteSubmit(value),
        Event::CloseBinding(value) => E::CloseBinding(value),
        Event::LoadBinding(value) => E::LoadBinding(value),
        Event::SaveBinding(value) => E::SaveBinding(value),
        Event::PasteInput(generation, value) => E::PasteInput(generation, value),
        Event::SelectBinding(generation, value) => E::SelectBinding(generation, value),
        Event::LoginLink(generation, action) => E::LoginLink(generation, match action {
            account::LinkAction::Open => L::Open, account::LinkAction::Copy => L::Copy,
        }),
        Event::AccountAction(action) => E::AccountAction(match action {
            account::ActionEvent::Open {rows_generation, operation, service, account_id} => A::Open {rows_generation, operation, service, account_id},
            account::ActionEvent::Review(g) => A::Review(g),
            account::ActionEvent::Confirm(g) => A::Confirm(g),
            account::ActionEvent::Close(g) => A::Close(g),
        }),
    };
    auth::message(event)
}
