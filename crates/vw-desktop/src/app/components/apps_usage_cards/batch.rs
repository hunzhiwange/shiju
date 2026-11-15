use super::{Event, linked_scope, message_event, read_linked_providers, scope};
use crate::app::message::settings::apps_usage_events::refresh_batch;
use crate::app::{App, Message, components::system_settings::SystemTab};
use iced::Task;
use {gateway_endpoint::{GatewayEndpoint}, {vw_gateway_client::GatewayClient}};

pub(super) fn update(app: &mut App, event: Event, endpoint: GatewayEndpoint) -> Task<Message> {
    let Some(scope) = scope(app, &endpoint, true) else { return Task::none() };
    let linked = linked_scope(app, &endpoint, &scope, true);
    let auth = &mut app.agent_apps_auth;
    let Some(connection) =
        auth.connection.as_ref().filter(|connection| endpoint.is_loopback() && connection.valid())
    else {
        return Task::none();
    };
    let cards = &mut auth.usage;
    match event {
        Event::RefreshAll => {
            if !app.show_system_settings
                || app.system_settings_tab != SystemTab::Auth
                || auth.binding.is_some()
                || auth.pending.is_some()
                || auth.account_action.is_some()
                || auth.rows.is_none()
                || cards.refreshing(&scope)
                || linked.as_ref().is_some_and(|scope| app.app_providers.usage.refreshing(scope))
            {
                return Task::none();
            }
            let Ok(client) = GatewayClient::new(endpoint) else {
                return app.show_error_toast("无法建立额度查询连接");
            };
            cards.select_scope(&scope);
            let ticket = cards.next_ticket();
            cards.batch_pending = Some(ticket);
            cards.batch_result = None;
            let project = scope.project.clone();
            let host = scope.host.clone();
            let token = connection.token.trim().to_owned();
            Task::perform(
                async move { refresh_batch(&client, &project, &host, &token).await },
                move |result| message_event(Event::BatchFinished(scope.clone(), ticket, result)),
            )
        }
        Event::BatchFinished(expected, ticket, result)
            if scope == expected
                && cards.scope.as_ref() == Some(&scope)
                && cards.batch_pending == Some(ticket) =>
        {
            cards.batch_pending = None;
            if let Ok(items) = &result {
                cards.apply_updates(&scope, items);
                if let Some(linked) = linked {
                    app.app_providers.usage.apply_updates(&linked, items);
                }
            }
            let error = result.as_ref().err().copied();
            cards.batch_result = Some(result);
            if let Some(error) = error {
                return app.show_error_toast(error.label());
            }
            if let Some(provider_scope) = &app.app_providers.scope {
                return read_linked_providers(app, provider_scope.app);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
