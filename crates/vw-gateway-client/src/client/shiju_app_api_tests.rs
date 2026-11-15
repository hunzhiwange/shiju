//! 验证兼容门面继续导出领域 DTO。
use super::{ShijuAppCatalogDto, ShijuAppLaunchDto};

#[test]
fn shiju_app_facade_preserves_domain_types() {
    let catalog: gateway_shiju_app::ShijuAppCatalogDto = ShijuAppCatalogDto { apps: vec![] };
    assert!(catalog.apps.is_empty());
    let launch = ShijuAppLaunchDto {
        installation_id: "fixture".into(),
        name: "fixture".into(),
        runtime_url: "private-url".into(),
        runtime_set_cookie: "private-cookie".into(),
    };
    let launch: gateway_shiju_app::ShijuAppLaunchDto = launch;
    assert!(!format!("{launch:?}").contains("private-"));
}
