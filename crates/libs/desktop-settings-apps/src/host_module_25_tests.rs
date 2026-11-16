//! Migrated lifecycle controller assertions use the production domain update.
use crate::system_settings_apps::{catalog, lifecycle, uninstall};
mod lifecycle_tests;
mod uninstall_tests;

struct LifecycleFixture {
    lifecycle: lifecycle::LifecycleState,
    rows: Vec<catalog::CatalogRow>,
    context: desktop_apps_ui::AgentAppsContext,
    notice: Option<crate::aggregation::Notice>,
}
fn lifecycle_app() -> (LifecycleFixture, desktop_apps_ui::AgentAppsContext) {
    let client = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let context = desktop_apps_ui::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: None,
    };
    (
        LifecycleFixture {
            lifecycle: Default::default(),
            rows: vec![],
            context: context.clone(),
            notice: None,
        },
        context,
    )
}
fn route(
    state: &mut LifecycleFixture,
    message: crate::system_settings_apps::lifecycle_update::LifecycleMessage,
) {
    let client = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let _ = crate::system_settings_apps::lifecycle_update::update(
        &mut state.lifecycle,
        &state.rows,
        client,
        state.context.clone(),
        "fixture-host".into(),
        "fixture-grant".into(),
        message,
        &mut state.notice,
        &mut false,
    );
}

mod launch_tests;
type LaunchFixture = crate::system_settings_apps::catalog_update::CatalogState;
fn launch_fixture() -> (LaunchFixture, desktop_apps_ui::AgentAppsContext) {
    let (_, context) = lifecycle_app();
    (
        LaunchFixture {
            context: Some(context.clone()),
            host: String::new(),
            token: String::new(),
            generation: 0,
            launch: Default::default(),
            ..Default::default()
        },
        context,
    )
}
fn launch_route(
    state: &mut LaunchFixture,
    message: crate::system_settings_apps::launch::LaunchMessage<desktop_apps_ui::AgentAppsContext>,
) -> iced::Task<crate::system_settings_apps::launch::LaunchMessage<desktop_apps_ui::AgentAppsContext>>
{
    let context = state.context.clone().unwrap();
    let gateway = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .map_err(|error| error.to_string());
    crate::system_settings_apps::launch::update(
        &mut crate::system_settings_apps::launch::Input {
            context: &state.context,
            host: &state.host,
            token: &state.token,
            generation: state.generation,
            loading: false,
            launch: &mut state.launch,
            rows: &[],
        },
        &None,
        gateway,
        context,
        message,
    )
}

mod catalog_ui_refresh_06_tests;
mod launch_ui_refresh_07_tests;
mod page_fixture;
mod preferences_tests;

mod details_tests;

mod drag_tests;

mod catalog_tests;
