use crate::app::message::agent_apps_health::{Context, State};
use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
use vw_gateway_client::vw_api_types::{agent_apps::AppId, agent_apps_high_availability::*, id::ProjectScope};

#[test]
fn ha_desktop_health_ignores_stale_route_events() {
    let tenant = Default::default();
    let context = Context {
        scope: Scope { context: AgentAppsContext { gateway: "http://localhost".into(), tenant, project: None }, host: "host-a".into(), app: AppId::Claude },
        query: FailoverQueueQuery { project_id: "project".into(), target_host_id: "host-a".into(), app_id: AppId::Claude },
        generation: 3, access: 4, provider: "provider-a".into(),
    };
    let scope = FailoverQueueScope { project: ProjectScope::new(context.scope.context.tenant.clone(), context.query.project_id.clone()).unwrap(), target_host_id: "host-a".into(), app_id: AppId::Claude };
    let health = FailoverHealthResponse { scope: scope.clone(), provider_id: context.provider.clone(), state: FailoverHealthState::HalfOpen, retry_after_ms: None };
    let route = FailoverRouteResponse { scope, configured_provider_id: Some("provider-a".into()), actual_provider_id: Some("provider-b".into()), failure_reason: Some(FailoverFailureReason::Connection) };
    let mut state = State { context: Some(context.clone()), serial: 7, ..Default::default() };
    assert!(state.accept(&context, 7, health.clone(), route.clone()));
    assert!(!state.accept(&context, 6, health.clone(), route.clone()));
    let mut previous_host = context.clone();
    previous_host.query.target_host_id = "host-b".into();
    assert!(!state.accept(&previous_host, 7, health.clone(), route.clone()));
    let mut previous_version = context.clone();
    previous_version.generation -= 1;
    assert!(!state.accept(&previous_version, 7, health.clone(), route.clone()));
    let mut previous_access = context.clone();
    previous_access.access -= 1;
    assert!(!state.accept(&previous_access, 7, health.clone(), route.clone()));
    let mut foreign = route.clone();
    foreign.scope.target_host_id = "host-b".into();
    assert!(!state.accept(&context, 7, health, foreign));
    assert_eq!(state.route, Some(route));
}
