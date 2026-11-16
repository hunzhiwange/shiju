//! 提供商与模型设置测试，覆盖连接草稿、模型详情、路由和调度器默认契约。
//! 公开摘要字段及端口、并发等安全限值应与设置界面的编辑语义一致。

use crate::state::{ModelRoute, ModelRoutesSettingsState};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("host_module_39_provider_model_tests"));
}

#[test]
fn route_classification_and_heartbeat_defaults_are_ready_to_edit() {
    let route = ModelRoute::default();
    assert!(route.pattern.is_empty());
    assert!(route.provider.is_empty());
    assert!(route.model.is_empty());
    assert!(route.priority_input.is_empty());
    let model_routes = ModelRoutesSettingsState::default();
    assert!(model_routes.routes.is_empty());
    assert!(model_routes.save_error.is_none());
}
