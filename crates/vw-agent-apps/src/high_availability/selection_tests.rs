use super::{SelectionError, SelectionSnapshot, select};
use crate::{
    high_availability::circuit_breaker::{AttemptOutcome, CircuitBreaker},
    providers::{
        failover::{ProviderAuth, ProviderSnapshot},
        live::commit::{AppState, Mode, ModeState},
    },
};
use std::time::{Duration, Instant};
use vw_api_types::{
    agent_apps::AppId, agent_apps_high_availability::CircuitBreakerPolicy,
    app_providers::AppProviderSummary, id::TenantScope,
};

#[test]
fn ha_selection_respects_queue_and_open_breakers() {
    let tenant = TenantScope::default();
    let providers = ["alpha", "beta", "gamma"].map(|id| ProviderSnapshot {
        provider: AppProviderSummary {
            id: id.into(),
            app_id: AppId::Codex,
            name: id.into(),
            configured: true,
        },
        auth: ProviderAuth::ThirdParty,
    });
    let state = AppState {
        mode: Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some("alpha".into()),
            contract: None,
        }),
        mode_tenant: Some(tenant.clone()),
        ..Default::default()
    };
    let members = ["beta", "stale", "gamma", "alpha"].map(str::to_owned);
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 10,
        error_rate_threshold: 0.5,
        min_requests: 5,
    };
    let now = Instant::now();
    let beta = CircuitBreaker::default();
    beta.allow_request(&policy, now).unwrap().record(AttemptOutcome::ProviderFailure, now);
    let ids = |entries: Vec<&ProviderSnapshot>| {
        entries.iter().map(|entry| entry.provider.id.to_string()).collect::<Vec<_>>()
    };
    let mut snapshot = SelectionSnapshot {
        app: AppId::Codex,
        tenant: &tenant,
        state: &state,
        enabled: false,
        members: &members,
        providers: &providers,
    };
    assert_eq!(
        ids(select(&snapshot, |_| panic!("disabled consulted breaker")).unwrap()),
        ["alpha"]
    );
    snapshot.enabled = true;
    assert_eq!(
        ids(select(&snapshot, |id| id != "beta" || beta.is_available(&policy, now)).unwrap()),
        ["gamma", "alpha"],
        "selection must preserve saved order while skipping open circuits"
    );
    assert_eq!(select(&snapshot, |_| false), Err(SelectionError::AllProvidersCircuitOpen));
    snapshot.members = &[];
    assert_eq!(select(&snapshot, |_| true), Err(SelectionError::NoProvidersConfigured));
    snapshot.members = &members;
    assert_eq!(ids(select(&snapshot, |_| true).unwrap()), ["beta", "gamma", "alpha"]);
    let probe_at = now + Duration::from_secs(10);
    assert!(beta.allow_request(&policy, now).is_none());
    assert!(beta.is_available(&policy, probe_at));
    assert_eq!(
        ids(select(&snapshot, |id| id != "beta" || beta.is_available(&policy, probe_at)).unwrap()),
        ["beta", "gamma", "alpha"]
    );
    let probe = beta.allow_request(&policy, probe_at).unwrap();
    assert!(!beta.is_available(&policy, probe_at));
    assert_eq!(
        ids(select(&snapshot, |id| id != "beta" || beta.is_available(&policy, probe_at)).unwrap()),
        ["gamma", "alpha"]
    );
    assert!(beta.allow_request(&policy, probe_at).is_none());
    drop(probe);
    assert!(beta.allow_request(&policy, probe_at).is_some());

    let mut aggregation = state.clone();
    aggregation.stack.enabled = true;
    aggregation.stack_tenant = Some(tenant.clone());
    snapshot.state = &aggregation;
    assert_eq!(select(&snapshot, |_| true), Err(SelectionError::UnsupportedMode));
    let foreign = TenantScope::new("other", "tenant").unwrap();
    snapshot.tenant = &foreign;
    assert_eq!(select(&snapshot, |_| true), Err(SelectionError::Forbidden));

    let official = [ProviderSnapshot {
        provider: providers[0].provider.clone(),
        auth: ProviderAuth::OfficialAccount,
    }];
    snapshot.tenant = &tenant;
    snapshot.state = &state;
    snapshot.providers = &official;
    assert_eq!(
        ids(select(&snapshot, |_| panic!("official consulted breaker")).unwrap()),
        ["alpha"]
    );
    snapshot.providers = &providers;
    let stale = ["missing".to_owned()];
    snapshot.members = &stale;
    assert_eq!(select(&snapshot, |_| true), Err(SelectionError::NoProvidersConfigured));
    let mut direct = state.clone();
    direct.mode.as_mut().unwrap().mode = Mode::Direct;
    snapshot.state = &direct;
    assert_eq!(select(&snapshot, |_| true), Err(SelectionError::UnsupportedMode));
    snapshot.state = &state;
    let duplicates = ["gamma", "gamma", "alpha"].map(str::to_owned);
    snapshot.members = &duplicates;
    assert_eq!(ids(select(&snapshot, |_| true).unwrap()), ["gamma", "alpha"]);
}
