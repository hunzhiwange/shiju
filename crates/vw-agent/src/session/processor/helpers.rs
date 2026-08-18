//! 会话处理器辅助函数模块。
//!
//! 本模块仅承接无状态的纯辅助逻辑，避免主循环入口承担工具权限解析、
//! 预览裁剪与 ACP 请求识别等细节职责。

use super::utils;
use crate::session::capability_policy::{CapabilityPolicyResolution, resolve_tool_capabilities};
use crate::tools;
use serde_json::Value;
use std::collections::HashSet;
use vw_config_types::agent::DelegateCapabilityPolicy;

/// 获取允许使用的工具 ID 集合。
///
/// 根据指定的模型名称，从工具注册表中提取当前模型可见的工具 ID，
/// 供会话处理主循环和工具解析逻辑复用。
pub(crate) fn allowed_tool_ids(model: Option<&str>) -> HashSet<String> {
    allowed_tool_ids_from_specs(&tools::registry::specs(model))
}

pub(crate) fn allowed_tool_ids_from_specs(specs: &[tools::ToolSpec]) -> HashSet<String> {
    specs
        .iter()
        .flat_map(|spec| std::iter::once(spec.id.clone()).chain(spec.aliases.iter().cloned()))
        .collect()
}

pub(crate) fn allowed_tool_ids_for_request(
    model: Option<&str>,
    options: &Value,
) -> HashSet<String> {
    let specs = tools::registry::specs(model);
    allowed_tool_ids_for_request_from_specs(&specs, options)
}

pub(crate) fn allowed_tool_ids_for_request_from_specs(
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

    if requested.is_empty() {
        return base;
    }

    // 技能通过轻量目录动态读取，不属于智能体注册工具白名单的限制范围。
    // 其余工具必须同时存在于请求白名单和当前注册表中。
    base.iter()
        .filter(|tool| requested.contains(*tool) || tool.eq_ignore_ascii_case("skill"))
        .cloned()
        .collect()
}

/// 解析委托会话的工具能力策略；缺少策略输入时保持旧请求过滤语义。
pub(crate) fn delegated_tool_policy_resolution_from_specs(
    specs: &[tools::ToolSpec],
    options: &Value,
) -> Option<CapabilityPolicyResolution> {
    let _policy = options
        .get("delegate_tool_policy")
        .cloned()
        .and_then(|value| serde_json::from_value::<DelegateCapabilityPolicy>(value).ok())?;
    let defaults = option_string_values(options, "delegate_default_allowed_tools");
    let trusted_user_selection =
        option_string_values(options, "delegate_trusted_user_selected_tools");
    let hard_disabled = specs
        .iter()
        .filter(|spec| {
            crate::session::processor::tools_exec::is_agent_tool(&spec.id)
                || spec
                    .aliases
                    .iter()
                    .any(|alias| crate::session::processor::tools_exec::is_agent_tool(alias))
        })
        .map(|spec| spec.id.as_str())
        .collect::<Vec<_>>();
    // 智能体声明的工具集合是硬上界；用户选择与历史 unrestricted 策略都不能
    // 把列表外工具重新加入 provider 的工具定义。
    let mut resolution = resolve_tool_capabilities(
        specs,
        defaults,
        trusted_user_selection,
        DelegateCapabilityPolicy::Strict,
        hard_disabled,
    );

    for spec in specs {
        if spec.id.eq_ignore_ascii_case("skill") {
            resolution.visible_ids.insert(spec.id.to_lowercase());
        }
    }
    Some(resolution)
}

/// 返回 Gateway 请求边界中已固化的可信用户工具选择。
pub(crate) fn trusted_user_selected_tool_ids_from_options(options: &Value) -> Vec<String> {
    let delegated = option_string_values(options, "delegate_trusted_user_selected_tools");
    if !delegated.is_empty() {
        return delegated;
    }
    option_string_values(options, "allowed_tools")
}

fn option_string_values(options: &Value, key: &str) -> Vec<String> {
    options
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// 构建用于日志输出的响应预览。
pub(crate) fn response_preview(text: &str) -> String {
    crate::app::agent::util::truncate_with_ellipsis(
        &crate::agent::loop_::scrub_credentials(text.trim()),
        240,
    )
}

/// 构建用于日志输出的工具调用预览。
pub(crate) fn tool_call_preview(tool_name: &str, arguments: &str) -> String {
    let sanitized = utils::sanitize_tool_input(tool_name, arguments);
    format!("{}({})", tool_name, crate::app::agent::util::truncate_with_ellipsis(&sanitized, 120))
}

/// 判断当前请求是否走 ACP 代理路径。
pub(crate) fn is_acp_request(options: &Value) -> bool {
    options.get("acp_test").and_then(Value::as_bool).unwrap_or(false)
        || options
            .get("acp_agent")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

/// 判断结构化工具调用是否允许在本地直接执行。
pub(crate) fn should_execute_structured_tool_calls_locally(options: &Value) -> bool {
    !is_acp_request(options)
}
#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
