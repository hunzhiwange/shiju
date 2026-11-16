//! 命令解析功能测试模块
//!
//! 本模块包含针对 `parse_runtime_command` 函数的单元测试，用于验证：
//! - 非模型通道上的工具授权命令解析
//! - 自然语言形式的授权意图识别
//!
//! # 测试范围
//!
//! 1. **斜杠命令解析**：验证标准斜杠命令（如 `/approve`）的解析正确性
//! 2. **自然语言解析**：验证中英文自然语言表达能否正确识别为授权意图
//!
//! # 相关模块
//!
//! - [`super`]：父模块，包含 `parse_runtime_command` 的实际实现
//! - [`ChannelRuntimeCommand`]：运行时命令枚举类型

use super::*;

/// 测试非模型通道上的工具授权命令解析
///
/// # 功能说明
///
/// 验证 `parse_runtime_command` 函数在非模型通道（如 Slack）上能够正确解析
/// 各种工具授权相关的斜杠命令，包括：
///
/// - 请求授权：`/approve-request`
/// - 一次性授权：`/approve-all-once`
/// - 确认授权：`/approve-confirm`
/// - 允许待审批请求：`/approve-allow`
/// - 拒绝审批：`/approve-deny`
/// - 列出待审批：`/approve-pending`
/// - 批准工具：`/approve`
/// - 撤销授权：`/unapprove`
/// - 查看授权列表：`/approvals`
///
/// # 测试用例
///
/// | 命令 | 预期结果 |
/// |------|---------|
/// | `/approve-request shell` | 请求工具 "shell" 的授权 |
/// | `/approve-all-once` | 一次性允许所有工具 |
/// | `/approve-confirm apr-deadbeef` | 确认授权请求 "apr-deadbeef" |
/// | `/approve-allow apr-deadbeef` | 允许待审批请求 "apr-deadbeef" |
/// | `/approve-deny apr-deadbeef` | 拒绝工具审批 "apr-deadbeef" |
/// | `/approve-pending` | 列出所有待审批项 |
/// | `/approve shell` | 批准工具 "shell" |
/// | `/unapprove shell` | 撤销工具 "shell" 的授权 |
/// | `/approvals` | 列出所有授权 |
/// | `/models` | 无效命令，返回 None |
///
/// # 边界条件
///
/// - 测试验证 `/models` 命令在非模型通道上应返回 `None`，表明该命令不适用于此通道类型
#[test]
fn parse_runtime_command_allows_approval_commands_on_non_model_channels() {
    assert_eq!(
        parse_runtime_command("slack", "/approve-request shell"),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve-all-once"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve-confirm apr-deadbeef"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-deadbeef".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve-allow apr-deadbeef"),
        Some(ChannelRuntimeCommand::ApprovePendingRequest("apr-deadbeef".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve-deny apr-deadbeef"),
        Some(ChannelRuntimeCommand::DenyToolApproval("apr-deadbeef".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve-pending"),
        Some(ChannelRuntimeCommand::ListPendingApprovals)
    );
    assert_eq!(
        parse_runtime_command("slack", "/approve shell"),
        Some(ChannelRuntimeCommand::ApproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/unapprove shell"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("slack", "/approvals"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(parse_runtime_command("slack", "/models"), None);
}

/// 测试自然语言形式的授权意图识别
///
/// # 功能说明
///
/// 验证 `parse_runtime_command` 函数能够识别多种自然语言表达形式，
/// 包括中文和英文的授权相关请求。这提供了更友好的用户体验，
/// 允许用户使用自然对话方式与代理交互。
///
/// # 支持的自然语言模式
///
/// ## 请求授权
/// - 中文：`"授权工具 shell"`、`"请放开 shell"`
/// - 英文：`"approve tool shell"`
///
/// ## 一次性授权
/// - 中文：`"请一次性允许所有工具和命令"`
///
/// ## 确认授权
/// - 中文：`"确认授权 apr-deadbeef"`
/// - 英文：`"confirm apr-deadbeef"`
///
/// ## 撤销授权
/// - 中文：`"撤销工具 shell"`
/// - 英文：`"revoke tool shell"`
///
/// ## 查看授权
/// - 中文：`"查看授权"`
/// - 英文：`"show approvals"`
///
/// ## 查看待审批
/// - 英文：`"show pending approvals"`
///
/// # 测试场景
///
/// 1. **中文表达**：验证中文自然语言命令的正确解析
/// 2. **英文表达**：验证英文自然语言命令的正确解析
/// 3. **无效命令**：验证非授权意图的自然语言应返回 `None`
///
/// # 边界条件
///
/// - 测试验证 `"请帮我执行shell"` 这类执行请求不会触发授权解析，
///   返回 `None`，表明该命令不是授权相关的意图
#[test]
fn parse_runtime_command_supports_natural_language_approval_intents() {
    assert_eq!(
        parse_runtime_command("telegram", "授权工具 shell"),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "请放开 shell"),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "approve tool shell"),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "请一次性允许所有工具和命令"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
    assert_eq!(
        parse_runtime_command("telegram", "确认授权 apr-deadbeef"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-deadbeef".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "confirm apr-deadbeef"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-deadbeef".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "撤销工具 shell"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "revoke tool shell"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "查看授权"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(
        parse_runtime_command("telegram", "show approvals"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(
        parse_runtime_command("telegram", "show pending approvals"),
        Some(ChannelRuntimeCommand::ListPendingApprovals)
    );
    assert_eq!(parse_runtime_command("telegram", "请帮我执行shell"), None);
}
