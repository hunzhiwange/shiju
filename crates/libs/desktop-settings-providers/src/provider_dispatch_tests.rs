use crate::provider_state::{ProviderSettingsState, ProviderConnectState};
use crate::state::ModelSettingsState;
use crate::provider_message::SettingsMessage;

fn update(state: &mut ProviderSettingsState, message: SettingsMessage) {
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1);
    let _ = crate::provider_update::update(state, &mut ModelSettingsState::default(), endpoint.clone(), desktop_config::ConfigAccess::new(endpoint, None), false, message);
}

// 验证提供商顶层消息转发及连接状态清理。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn update_forwards_close_and_clears_connect_state() {
    let mut state = ProviderSettingsState::default();
    state.connect_modal = Some(ProviderConnectState::default());
    state.connect_error = Some(("invalid key".to_string()).into());

    update(&mut state, SettingsMessage::ProviderConnectClose);

    assert!(state.connect_modal.is_none());
    assert!(state.connect_error.is_none());
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn update_forwards_disconnect_request_payload_unchanged() {
    let mut state = ProviderSettingsState::default();
    let provider_id = " custom-provider/测试 ".to_string();

    update(
        &mut state,
        SettingsMessage::ProviderDisconnectRequested(provider_id.clone()),
    );

    assert_eq!(
        state.disconnect_confirm_provider_id.as_deref(),
        Some(provider_id.as_str())
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_forwards_disconnect_error_and_clears_confirmation() {
    let mut state = ProviderSettingsState::default();
    state.disconnect_confirm_provider_id = Some("openai".to_string());

    update(
        &mut state,
        SettingsMessage::ProviderDisconnectDone(Err("disconnect denied".to_string().into())),
    );

    assert_eq!(state.save_error.as_deref(), Some("disconnect denied"));
    assert!(state.disconnect_confirm_provider_id.is_none());
}
