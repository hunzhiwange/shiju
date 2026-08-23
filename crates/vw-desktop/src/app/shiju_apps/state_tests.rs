//! 时聚应用状态测试，验证账户切换清理、默认标签和异步启动响应防陈旧机制。
//! 启动结果仅在代次、账户及安装标识全部匹配时才能写回当前状态。

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
        launch: ShijuAppLaunchState::Error {
            generation: 8,
            installation_id: "01K1APP0000000000000000001".to_string(),
            message: "private runtime error".to_string(),
        },
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

#[test]
fn launch_response_requires_generation_account_and_installation_identity() {
    let state = ShijuAppsState {
        account_id: Some("company-1".to_string()),
        generation: 8,
        launch: ShijuAppLaunchState::CheckingBinding {
            generation: 8,
            installation_id: "01K1APP0000000000000000001".to_string(),
        },
        ..ShijuAppsState::default()
    };

    assert!(state.launch_response_is_current(
        8,
        "company-1",
        "01K1APP0000000000000000001"
    ));
    assert!(!state.launch_response_is_current(
        7,
        "company-1",
        "01K1APP0000000000000000001"
    ));
    assert!(!state.launch_response_is_current(
        8,
        "company-2",
        "01K1APP0000000000000000001"
    ));
    assert!(!state.launch_response_is_current(
        8,
        "company-1",
        "01K1APP0000000000000000002"
    ));
}
