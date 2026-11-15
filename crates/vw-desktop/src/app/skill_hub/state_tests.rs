//! 跨域账号清理保留在桌面装配层。
use crate::app::{App, shiju_apps::{ShijuAppsState, ShijuAppsLoadState}};

#[test]
fn expert_hub_account_clear_resets_shiju_apps() {
    let (mut app, _) = App::new();
    assert_eq!(app.shiju_apps, ShijuAppsState::default());
    app.clear_expert_hub_account(Some("new-account".to_string()));
    assert_eq!(app.shiju_apps.account_id.as_deref(), Some("new-account"));
    assert!(matches!(app.shiju_apps.store, ShijuAppsLoadState::Idle));
}
