//! 验证安全策略轻量类型的显示、复制与比较契约。

use super::*;

#[test]
fn display_formats_command_risk_levels() {
    assert_eq!(CommandRiskLevel::Low.to_string(), "low");
    assert_eq!(CommandRiskLevel::Medium.to_string(), "medium");
    assert_eq!(CommandRiskLevel::High.to_string(), "high");
}

#[test]
// 这些枚举会频繁参与策略分支，复制和比较契约避免不必要所有权转移。
fn lightweight_policy_enums_are_copy_and_comparable() {
    let read = ToolOperation::Read;
    let act = ToolOperation::Act;
    assert_eq!(read, ToolOperation::Read);
    assert_ne!(read, act);

    let quote = QuoteState::Double;
    assert_eq!(quote, QuoteState::Double);
    assert_ne!(quote, QuoteState::Single);

    let risk = CommandRiskLevel::Medium;
    assert_eq!(risk, CommandRiskLevel::Medium);
    assert_ne!(risk, CommandRiskLevel::High);
}
