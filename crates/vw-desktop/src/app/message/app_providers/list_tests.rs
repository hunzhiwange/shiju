use super::*;
use crate::app::state::{AgentAppsContext, app_providers::list::ListState};
use {gateway_endpoint::{GatewayEndpoint}, {vw_api_app_identity::AppId}};
fn scope() -> Scope {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture".into(),
        app: AppId::Claude,
    }
}
#[test]
fn cc04_ui_list_stale_response_is_ignored() {
    let mut state = ListState::default();
    let old = scope();
    let generation = state.begin(old.clone());
    let mut new = old.clone();
    new.app = AppId::Pi;
    state.begin(new.clone());
    assert!(!state.finish(&old, generation, Err("过期错误".into())));
    assert_eq!(state.scope, Some(new));
    assert!(state.loading);
    assert!(state.error.is_none());
}
