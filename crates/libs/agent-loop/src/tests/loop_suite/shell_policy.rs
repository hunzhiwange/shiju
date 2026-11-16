//! Bash 策略指令构建测试模块
//!
//! 本模块包含针对 `build_shell_policy_instructions` 函数的单元测试，
//! 验证不同自主性级别和命令白名单配置下的 Bash 策略指令生成行为。
//!
//! # 测试覆盖范围
//!
//! - 命令白名单的格式化与去重
//! - 通配符权限的处理
//! - 只读模式下 Bash 执行的禁用状态
//!
//! # 相关模块
//!
//! - [`super`] 父模块（agent 循环核心逻辑）
//! - [`config_security::AutonomyConfig`] 自主性配置定义
//! - [`agent_security::AutonomyLevel`] 自主性级别枚举

use crate::instructions::build_shell_policy_instructions;

/// 测试命令白名单的格式化输出
///
/// 验证在 `Supervised`（监督）级别下，允许的命令列表能够正确格式化
/// 到 Bash 策略指令中，并且重复的命令应该被正确处理（显示）。
///
/// # 测试场景
///
/// - 自主性级别：`Supervised`（监督模式）
/// - 允许的命令：`grep`、`cat`、`grep`（包含重复项）
///
/// # 验证点
///
/// 1. 指令中包含 `## Bash Policy` 标题
/// 2. 显示正确的自主性级别 `supervised`
/// 3. 包含 `cat` 命令
/// 4. 包含 `grep` 命令
#[test]
fn build_shell_policy_instructions_lists_allowlist() {
    // 创建默认的自主性配置
    let mut autonomy = config_security::AutonomyConfig::default();
    // 设置为监督级别，需要人工审批执行
    autonomy.level = agent_security::AutonomyLevel::Supervised;
    // 配置允许的命令列表（包含重复的 grep 用于测试去重或显示逻辑）
    autonomy.allowed_commands = vec!["grep".into(), "cat".into(), "grep".into()];

    // 构建完整的 Bash 策略指令字符串
    let instructions = build_shell_policy_instructions(&autonomy);

    // 验证指令包含预期的内容
    assert!(instructions.contains("## Bash Policy"));
    assert!(instructions.contains("Autonomy level: `supervised`"));
    assert!(instructions.contains("`cat`"));
    assert!(instructions.contains("`grep`"));
}

/// 测试通配符权限的处理
///
/// 验证在 `Full`（完全自主）级别下，当允许的命令为通配符 `*` 时，
/// 策略指令能够正确标识为通配符权限，而非列出具体命令。
///
/// # 测试场景
///
/// - 自主性级别：`Full`（完全自主模式）
/// - 允许的命令：`*`（通配符，表示允许所有命令）
///
/// # 验证点
///
/// 1. 显示正确的自主性级别 `full`
/// 2. 包含通配符标识 `wildcard '*'`
#[test]
fn build_shell_policy_instructions_handles_wildcard() {
    // 创建默认的自主性配置
    let mut autonomy = config_security::AutonomyConfig::default();
    // 设置为完全自主级别，代理可自主执行
    autonomy.level = agent_security::AutonomyLevel::Full;
    // 使用通配符表示允许所有命令
    autonomy.allowed_commands = vec!["*".into()];

    // 构建完整的 Bash 策略指令字符串
    let instructions = build_shell_policy_instructions(&autonomy);

    // 验证指令包含完全自主级别和通配符标识
    assert!(instructions.contains("Autonomy level: `full`"));
    assert!(instructions.contains("wildcard `*`"));
}

/// 测试只读模式下的 Bash 执行禁用
///
/// 验证在 `ReadOnly`（只读）级别下，Bash 执行功能应该被明确禁用，
/// 策略指令中应反映这一限制状态。
///
/// # 测试场景
///
/// - 自主性级别：`ReadOnly`（只读模式）
/// - 允许的命令：（默认，不应有任何 Bash 执行权限）
///
/// # 验证点
///
/// 1. 显示正确的自主性级别 `read_only`
/// 2. 包含 Bash 执行禁用的明确说明
#[test]
fn build_shell_policy_instructions_read_only_disables_shell() {
    // 创建默认的自主性配置
    let mut autonomy = config_security::AutonomyConfig::default();
    // 设置为只读级别，禁止所有修改性操作
    autonomy.level = agent_security::AutonomyLevel::ReadOnly;

    // 构建完整的 Bash 策略指令字符串
    let instructions = build_shell_policy_instructions(&autonomy);

    // 验证指令包含只读级别和 Bash 禁用说明
    assert!(instructions.contains("Autonomy level: `read_only`"));
    assert!(instructions.contains("Bash execution is disabled"));
}
