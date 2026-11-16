//! 最大键数量规范化的宿主工具测试。

use super::normalize_max_keys;

/// 测试 `normalize_max_keys` 函数对零值的处理
///
/// ## 测试目的
/// 验证当输入值为零时，函数应使用提供的回退值作为结果。
/// 这允许配置中使用零作为"使用默认值"的标记。
///
/// ## 测试用例
/// 1. `normalize_max_keys(0, 10_000)` → 返回 `10_000`（使用回退值）
/// 2. `normalize_max_keys(0, 0)` → 返回 `1`（最小值保护）
///
/// ## 设计考量
/// - 零值通常表示"未配置"或"使用默认"
/// - 当回退值也为零时，返回 `1` 作为绝对最小值，防止无意义的零限制
#[test]
fn normalize_max_keys_uses_fallback_for_zero() {
    // 零值应使用回退值 10_000
    assert_eq!(normalize_max_keys(0, 10_000), 10_000);

    // 即使回退值也是零，也应返回最小值 1
    assert_eq!(normalize_max_keys(0, 0), 1);
}

/// 测试 `normalize_max_keys` 函数对非零值的保留行为
///
/// ## 测试目的
/// 验证当输入值为有效的非零值时，函数应原样返回该值，
/// 不进行任何修改或截断。
///
/// ## 测试用例
/// 1. `normalize_max_keys(2_048, 10_000)` → 返回 `2_048`（保留用户配置）
/// 2. `normalize_max_keys(1, 10_000)` → 返回 `1`（最小有效值保留）
///
/// ## 设计考量
/// 非零值代表用户明确的配置意图，应被尊重。
/// 即使配置值小于回退值，也应按用户配置执行。
#[test]
fn normalize_max_keys_preserves_nonzero_values() {
    // 非零值应原样保留，即使小于回退值
    assert_eq!(normalize_max_keys(2_048, 10_000), 2_048);

    // 最小有效值 1 也应保留
    assert_eq!(normalize_max_keys(1, 10_000), 1);
}
