pub use desktop_apps_ui::agent_apps_auth::flow::{Update, run};
use super::{Event, message};
use crate::app::{Message, state::agent_apps_auth::{State, Scope, Connection, LoginTarget}};
use vw_gateway_client::GatewayClient;
use iced::Task;
pub(super) fn start(
    state: &mut State,
    scope: Scope,
    client: GatewayClient,
    connection: Connection,
    target: LoginTarget,
) -> Task<Message> {
    let copy_link = state.copy_login_link;
    let (generation, cancelled, pastes) = state.begin_login(target);
    Task::run(
        iced::stream::channel(4, async move |updates| {
            run(client, connection, target, copy_link, cancelled, pastes, updates).await;
        }),
        move |update| message(Event::LoginUpdated { scope: scope.clone(), generation, update }),
    )
}
