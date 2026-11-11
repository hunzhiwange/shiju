use super::failover_guard::{FailoverError, admit};
use crate::providers::{
    live::commit::{AppState, Mode, ModeState},
    mode_state::StackState,
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[test]
fn aggregation_t12_failover_guard_preserves_contract() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut state = AppState {
        mode: Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some("primary".into()),
            contract: None,
        }),
        mode_tenant: Some(tenant.clone()),
        stack: StackState {
            enabled: true,
            members: vec!["primary".into()],
            keys: Default::default(),
        },
        stack_tenant: Some(tenant.clone()),
        ..Default::default()
    };
    assert_eq!(admit(AppId::Codex, &tenant, &state), Err(FailoverError::AggregationConflict));
    state.stack.enabled = false;
    assert_eq!(admit(AppId::Codex, &tenant, &state), Ok(()));
    assert_eq!(state.mode.as_ref().unwrap().proxy_route.as_deref(), Some("primary"));
    let foreign = TenantScope::new("platform", "other").unwrap();
    assert_eq!(admit(AppId::Codex, &foreign, &state), Err(FailoverError::Forbidden));
    state.mode.as_mut().unwrap().attached = false;
    assert_eq!(admit(AppId::Codex, &tenant, &state), Err(FailoverError::Inactive));
    assert_eq!(admit(AppId::Pi, &tenant, &state), Err(FailoverError::UnsupportedApp));
}
