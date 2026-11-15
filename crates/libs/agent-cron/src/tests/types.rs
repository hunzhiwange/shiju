//! 计划任务类型相关测试模块
//!
//! 本模块包含对 `JobType` 枚举类型的转换和验证逻辑的单元测试，
//! 主要验证 `TryFrom<&str>` trait 实现的正确性，包括：
//! - 已知值的接受（大小写不敏感）
//! - 无效值的拒绝

use crate::JobType;

/// 测试 `JobType::try_from` 接受已知值（大小写不敏感）
///
/// 验证以下行为：
/// - 小写形式的 "shell" 和 "agent" 应被接受
/// - 大写形式的 "SHELL" 应被接受
/// - 混合大小写形式的 "AgEnT" 应被接受
///
/// 所有有效输入都应成功转换为对应的 `JobType` 枚举变体。
#[test]
fn job_type_try_from_accepts_known_values_case_insensitive() {
    // 小写输入应被正确识别
    assert_eq!(JobType::try_from("shell").unwrap(), JobType::Shell);
    // 大写输入同样应被识别（大小写不敏感）
    assert_eq!(JobType::try_from("SHELL").unwrap(), JobType::Shell);
    // Agent 类型：小写
    assert_eq!(JobType::try_from("agent").unwrap(), JobType::Agent);
    // Agent 类型：混合大小写，验证大小写不敏感性
    assert_eq!(JobType::try_from("AgEnT").unwrap(), JobType::Agent);
}

/// 测试 `JobType::try_from` 拒绝无效值
///
/// 验证以下无效输入应被拒绝并返回错误：
/// - 空字符串
/// - 未知的任务类型名称
///
/// 这些无效输入应导致 `try_from` 返回 `Err`，而非 panic 或静默接受。
#[test]
fn job_type_try_from_rejects_invalid_values() {
    // 空字符串不是有效的任务类型
    assert!(JobType::try_from("").is_err());
    // 未知的类型名称应被拒绝
    assert!(JobType::try_from("unknown").is_err());
}
