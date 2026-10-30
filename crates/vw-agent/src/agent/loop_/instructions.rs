//! 代理循环指令构建模块
//!
//! 本模块负责为系统提示构建工具使用协议和 Bash 策略的指令文本。
//! 这些指令会被注入到 LLM 的系统提示中，指导模型如何正确调用工具以及遵守执行约束。

use crate::app::agent::tools::Tool;
use std::collections::BTreeSet;
use std::fmt::Write;

#[cfg(test)]
#[path = "instructions_tests.rs"]
mod instructions_tests;

/// 从工具注册表构建工具指令块
///
/// 该函数接受一个工具 trait 对象的切片，提取每个工具的规格定义，
/// 然后生成包含工具使用协议和可用工具列表的指令文本。
/// 生成的指令将被包含在系统提示中，以便 LLM 了解如何调用工具。
///
/// # 参数
///
/// * `tools_registry` - 工具注册表，包含所有可用工具的 trait 对象引用
///
/// # 返回值
///
/// 返回格式化的工具使用协议字符串，包含：
/// - 工具调用语法说明（使用  tum 标签包装 JSON）
/// - 工具调用规则和示例
/// - 所有可用工具的名称、描述和参数规格
///
/// # 示例
///
/// ```ignore
/// let tools: Vec<Box<dyn Tool>> = vec![
///     // ... 工具列表
/// ];
/// let instructions = build_tool_instructions(&tools);
/// // instructions 包含完整的工具使用协议文本
/// ```
pub fn build_tool_instructions(tools_registry: &[Box<dyn Tool>]) -> String {
    // 从每个工具 trait 对象中提取规格定义
    let specs: Vec<crate::app::agent::tools::ToolSpec> =
        tools_registry.iter().map(|tool| tool.spec()).collect();
    // 基于规格列表构建指令
    build_tool_instructions_from_specs(&specs)
}

/// 从授权注册规格构建初始可见工具的指令块。
///
/// 该函数接受一个工具规格的切片，生成包含工具使用协议和可用工具列表的指令文本。
/// 与 `build_tool_instructions` 不同，该函数直接操作规格结构体，适用于规格已经预先生成的场景。
///
/// # 参数
///
/// * `tool_specs` - 工具规格列表，每个规格包含工具的名称、描述和参数定义
///
/// # 返回值
///
/// 返回格式化的工具使用协议字符串，包含：
/// - 工具调用语法说明（使用  tum 标签包装 JSON 对象）
/// - 工具调用的关键规则（必须输出实际标签，不能是描述或示例）
/// - 工具调用示例
/// - 多工具调用和结果处理流程说明
/// - 所有可用工具的详细列表（名称、描述、参数 schema）
///
/// # 示例
///
/// ```ignore
/// let specs = vec![ToolSpec {
///     name: "bash".to_string(),
///     description: "执行终端命令".to_string(),
///     parameters: json!({"type": "object"}),
/// }];
/// let instructions = build_tool_instructions_from_specs(&specs);
/// ```
pub(crate) fn build_tool_instructions_from_specs(
    tool_specs: &[crate::app::agent::tools::ToolSpec],
) -> String {
    let discovery = crate::tools::tool_search::ToolDiscoverySession::new(tool_specs.to_vec());
    build_exposed_tool_instructions(&discovery.exposed_specs())
}

/// Render a request's already-authorized exposure snapshot without resetting it.
pub(crate) fn build_exposed_tool_instructions(
    tool_specs: &[crate::app::agent::tools::ToolSpec],
) -> String {
    let mut instructions = String::new();

    // 添加工具使用协议的标题
    instructions.push_str("\n## Tool Use Protocol\n\n");

    // 说明工具调用的基本语法：使用  tum 标签包装 JSON 对象
    instructions.push_str("To use a tool, wrap a JSON object in  tum tags:\n\n");
    instructions.push_str("```\n<tool_call>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool_call>\n```\n\n");

    // 强调关键规则：必须输出实际的工具调用，不能是描述或示例
    instructions.push_str(
        "CRITICAL: Output actual <tool_call> tags—never describe steps or give examples.\n\n",
    );

    // 提供具体的工具调用示例
    instructions.push_str(
        "When a tool is needed, emit a real call (not prose), for example:\n\
<tool_call>\n\
{\"name\":\"tool_name\",\"arguments\":{}}\n\
</tool_call>\n\n",
    );

    // 说明多工具调用和结果处理流程
    instructions.push_str("You may use multiple tool calls in a single response. ");
    instructions.push_str(
        "After tool execution, the runtime appends tool result messages to the conversation history. ",
    );
    instructions
        .push_str("Continue reasoning with those results until you can give a final answer.\n\n");

    // 添加可用工具列表的标题
    instructions.push_str("### Available Tools\n\n");

    // 遍历所有工具规格，生成每个工具的说明
    for tool in tool_specs {
        let _ = writeln!(
            instructions,
            "**{}**: {}\nParameters: `{}`\n",
            tool.id, tool.description, tool.input_schema
        );
    }

    instructions
}

/// 构建 Bash 策略指令块
///
/// 该函数根据自治配置生成 Bash 工具的执行约束指令。
/// 这些指令让模型了解当前环境下的命令执行权限和限制，
/// 避免生成不符合安全策略的 Bash 调用。
///
/// # 参数
///
/// * `autonomy` - 自治配置，包含自治级别、允许的命令列表和风险控制设置
///
/// # 返回值
///
/// 返回格式化的 Shell 策略指令字符串，包含：
/// - 当前自治级别（read_only/supervised/full）
/// - 允许执行的命令列表或通配符说明
/// - 命令批准和阻断规则
/// - 越界处理建议
///
/// # 自治级别说明
///
/// - `ReadOnly`: 禁止所有 Bash 命令执行
/// - `Supervised`: 中风险命令需要显式批准
/// - `Full`: 允许执行所有允许列表中的命令
///
/// # 示例
///
/// ```ignore
/// let autonomy = AutonomyConfig {
///     level: AutonomyLevel::Supervised,
///     allowed_commands: vec!["ls".to_string(), "cat".to_string()],
///     require_approval_for_medium_risk: true,
///     block_high_risk_commands: true,
/// };
/// let instructions = build_shell_policy_instructions(&autonomy);
/// ```
pub fn build_shell_policy_instructions(
    autonomy: &crate::app::agent::config::AutonomyConfig,
) -> String {
    let mut instructions = String::new();

    // 添加 Bash 策略标题和说明
    instructions.push_str("\n## Bash Policy\n\n");
    instructions
        .push_str("When using the `bash` tool, follow these runtime constraints exactly.\n\n");

    // 将自治级别枚举转换为用户可读的标签
    let autonomy_label = match autonomy.level {
        crate::app::agent::security::AutonomyLevel::ReadOnly => "read_only",
        crate::app::agent::security::AutonomyLevel::Supervised => "supervised",
        crate::app::agent::security::AutonomyLevel::Full => "full",
    };
    let _ = writeln!(instructions, "- Autonomy level: `{autonomy_label}`");

    // 如果是只读模式，直接返回禁用说明，不需要继续处理命令列表
    if autonomy.level == crate::app::agent::security::AutonomyLevel::ReadOnly {
        instructions.push_str(
            "- Bash execution is disabled in `read_only` mode. Do not emit bash tool calls.\n",
        );
        return instructions;
    }

    // 规范化允许命令列表：
    // 1. 去除每条命令的首尾空白
    // 2. 过滤掉空字符串
    // 3. 去重并排序（使用 BTreeSet 保证确定性）
    let normalized: BTreeSet<String> = autonomy
        .allowed_commands
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    // 根据规范化后的命令列表生成说明
    if normalized.contains("*") {
        // 通配符模式：允许任意命令
        instructions.push_str(
            "- Allowed commands: wildcard `*` is configured (any command name/path may be allowlisted).\n",
        );
    } else if normalized.is_empty() {
        // 空列表模式：拒绝所有命令
        instructions
            .push_str("- Allowed commands: none configured. Any shell command will be rejected.\n");
    } else {
        // 具体命令列表模式：显示允许的命令（限制最大显示数量以避免提示过长）
        const MAX_DISPLAY_COMMANDS: usize = 64;
        let shown: Vec<String> =
            normalized.iter().take(MAX_DISPLAY_COMMANDS).map(|cmd| format!("`{cmd}`")).collect();
        let hidden = normalized.len().saturating_sub(MAX_DISPLAY_COMMANDS);
        let _ = write!(instructions, "- Allowed commands: {}", shown.join(", "));
        // 如果命令数量超过显示上限，说明还有更多命令未显示
        if hidden > 0 {
            let _ = write!(instructions, " (+{hidden} more)");
        }
        instructions.push('\n');
    }

    // 添加监督模式下的中等风险命令批准要求
    if autonomy.level == crate::app::agent::security::AutonomyLevel::Supervised
        && autonomy.require_approval_for_medium_risk
    {
        instructions.push_str(
            "- Medium-risk bash commands require explicit approval in `supervised` mode.\n",
        );
    }

    // 添加高风险命令阻断规则
    if autonomy.block_high_risk_commands {
        instructions.push_str(
            "- High-risk bash commands are blocked even when command names are allowed.\n",
        );
    }

    // 添加越界处理建议：引导模型选择合法替代方案
    instructions.push_str(
        "- If a requested command is outside policy, choose allowed alternatives and explain the limitation.\n",
    );

    instructions
}
