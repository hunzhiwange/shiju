//! 会话处理器辅助函数模块。
//!
//! 本模块仅承接无状态的纯辅助逻辑，避免主循环入口承担工具权限解析、
//! 预览裁剪与 ACP 请求识别等细节职责。

use tool_types as tools;
use serde_json::Value;
use std::collections::HashSet;

/// Actual registry snapshot and existing input redaction boundary.
pub trait HelperHost {
    fn specs(model: Option<&str>) -> Vec<tools::ToolSpec>;
    fn sanitize_tool_input(name: &str, arguments: &str) -> String;
}

/// 获取允许使用的工具 ID 集合。
///
/// 根据指定的模型名称，从工具注册表中提取当前模型可见的工具 ID，
/// 供会话处理主循环和工具解析逻辑复用。
pub fn allowed_tool_ids<H: HelperHost>(model: Option<&str>) -> HashSet<String> {
    allowed_tool_ids_from_specs(&H::specs(model))
}

pub fn allowed_tool_ids_from_specs(specs: &[tools::ToolSpec]) -> HashSet<String> {
    specs
        .iter()
        .flat_map(|spec| std::iter::once(spec.id.clone()).chain(spec.aliases.iter().cloned()))
        .collect()
}

pub fn allowed_tool_ids_for_request<H: HelperHost>(
    model: Option<&str>,
    options: &Value,
) -> HashSet<String> {
    let specs = H::specs(model);
    allowed_tool_ids_for_request_from_specs(&specs, options)
}

pub fn allowed_tool_ids_for_request_from_specs(
    specs: &[tools::ToolSpec],
    options: &Value,
) -> HashSet<String> {
    if options.get("disable_tools").and_then(Value::as_bool).unwrap_or(false) {
        return HashSet::new();
    }

    let base = allowed_tool_ids_from_specs(specs);
    let Some(raw_tools) = options.get("allowed_tools").and_then(Value::as_array) else {
        return base;
    };

    let requested = raw_tools
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|tool| !tool.is_empty())
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();

    // 内置必需工具始终注册；其余工具必须同时存在于请求清单和当前注册表中。
    base.iter()
        .filter(|tool| {
            requested.contains(*tool) || config_tools::is_builtin_required_tool_id(tool)
        })
        .cloned()
        .collect()
}

/// 构建用于日志输出的响应预览。
pub fn response_preview(text: &str) -> String {
    agent_util::truncate_with_ellipsis(
        &agent_util::credentials::scrub_credentials(text.trim()),
        240,
    )
}

/// 构建用于日志输出的工具调用预览。
pub fn tool_call_preview<H: HelperHost>(tool_name: &str, arguments: &str) -> String {
    let sanitized = H::sanitize_tool_input(tool_name, arguments);
    format!("{}({})", tool_name, agent_util::truncate_with_ellipsis(&sanitized, 120))
}

/// 判断当前请求是否走 ACP 代理路径。
pub fn is_acp_request(options: &Value) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

/// 判断结构化工具调用是否允许在本地直接执行。
pub fn should_execute_structured_tool_calls_locally(options: &Value) -> bool {
    !is_acp_request(options)
}

#[cfg(test)]
#[path = "helpers_visibility_tests.rs"]
mod visibility_tests;
