use super::*;
use vw_gateway_client::{ShijuAppCatalogDto, ShijuAppListDto};

#[test]
fn shiju_apps_state_defaults_to_mine_without_reusing_skill_load_state() {
    let state = ShijuAppsState::default();

    assert_eq!(state.tab, ShijuAppsTab::Mine);
    assert_eq!(state.account_id, None);
    assert_eq!(state.generation, 0);
    assert!(matches!(state.store, ShijuAppsLoadState::Idle));
    assert!(matches!(state.mine, ShijuAppsLoadState::Idle));
    assert!(state.detail.is_none());
    assert!(matches!(state.launch, ShijuAppLaunchState::Idle));
}

#[test]
fn account_change_clears_shiju_apps_state() {
    let mut state = ShijuAppsState {
        account_id: Some("old-account".to_string()),
        generation: 8,
        store: ShijuAppsLoadState::Loaded(ShijuAppCatalogDto { apps: Vec::new() }),
        mine: ShijuAppsLoadState::Loaded(ShijuAppListDto { apps: Vec::new() }),
        detail: Some(ShijuAppDetailTarget::Store(0)),
        launch: ShijuAppLaunchState::Error("private runtime error".to_string()),
        ..ShijuAppsState::default()
    };

    state.clear_for_account(Some("new-account".to_string()));

    assert_eq!(state.account_id.as_deref(), Some("new-account"));
    assert!(state.generation > 8);
    assert!(matches!(state.store, ShijuAppsLoadState::Idle));
    assert!(matches!(state.mine, ShijuAppsLoadState::Idle));
    assert!(state.detail.is_none());
    assert!(matches!(state.launch, ShijuAppLaunchState::Idle));
}
