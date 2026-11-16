//! 节点 ID 允许列表功能测试模块
//!
//! 本模块提供对节点 ID 访问控制机制的单元测试，验证允许列表（allowlist）的正确性。
//! 主要测试场景包括：
//! - 空允许列表时接受任何节点访问
//! - 配置允许列表时按规则进行访问控制

use super::node_id_allowed;

/// 测试空允许列表时接受任何节点
///
/// 当允许列表为空时，访问控制系统应该接受所有节点的访问请求。
/// 这是一个合理的默认行为，避免在没有明确配置的情况下意外阻止合法访问。
///
/// # 测试场景
/// - 允许列表：空数组 `&[]`
/// - 请求节点：`"node-a"`（任意节点 ID）
/// - 期望结果：允许访问（返回 `true`）
#[test]
fn node_id_allowed_with_empty_allowlist_accepts_any() {
    // 空允许列表应该接受任何节点的访问
    assert!(node_id_allowed("node-a", &[]));
}

/// 测试允许列表的访问控制功能
///
/// 当配置了非空允许列表时，只有列表中的节点才能访问，其他节点应该被拒绝。
/// 这是访问控制的核心功能，确保只有授权节点可以访问系统。
///
/// # 测试场景
/// - 允许列表：`["node-1", "node-2"]`
/// - 测试用例 1：`"node-1"` 在列表中 → 允许访问
/// - 测试用例 2：`"node-9"` 不在列表中 → 拒绝访问
#[test]
fn node_id_allowed_respects_allowlist() {
    // 配置允许列表，只包含 node-1 和 node-2
    let allow = vec!["node-1".to_string(), "node-2".to_string()];

    // 在允许列表中的节点应该被允许访问
    assert!(node_id_allowed("node-1", &allow));

    // 不在允许列表中的节点应该被拒绝访问
    assert!(!node_id_allowed("node-9", &allow));
}
