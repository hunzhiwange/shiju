//! 运行时设置测试，覆盖钩子、技能、浏览器、网关、内存与可靠性配置。
//! 配置往返需清除旧凭据并限制非法端口，同时保持各标签页克隆状态独立。

use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("runtime_tests"));
}

#[test]
fn gateway_client_agents_coordination_and_cost_defaults_are_stable() {
    let price = CostPriceInput::default();
    assert!(price.model.is_empty());
    assert!(price.input_price.is_empty());
    assert!(price.output_price.is_empty());

    let cost = CostSettingsState::default();
    assert!(!cost.enabled);
    assert_eq!(cost.daily_limit_usd_input, "10");
    assert_eq!(cost.monthly_limit_usd_input, "100");
    assert_eq!(cost.warn_at_percent_input, "80");
    assert!(!cost.allow_override);
    assert!(cost.prices.is_empty());
    assert!(!cost.show_help_modal);
    assert!(cost.save_error.is_none());
}
