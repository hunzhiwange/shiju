//! 工具权限状态解析。
//!
//! 本模块把运行时权限请求映射到具体消息或工具调用，并生成面向用户的授权状态文案。

use serde_json::Value;

/// 重新导出 use super::canonical_tool_name，让上层模块通过稳定路径访问。
use super::canonical_tool_name;
/// 重新导出 use super::tool_meta::tool_inline_summary，让上层模块通过稳定路径访问。
use super::tool_meta::tool_inline_summary;
/// 重新导出 use super::tool_parse::{tool_call_id_from_raw, tool_error_text, tool_status, tool_summary_text}，让上层模块通过稳定路径访问。
use super::tool_parse::{tool_call_id_from_raw, tool_error_text, tool_status, tool_summary_text};

/// 解析或展示工具权限请求的 pending permission targets message 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_targets_message(
    request: Option<&vw_gateway_client::PendingPermissionRequestDto>,
    // message_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    message_id: Option<&str>,
) -> bool {
    let Some(request) = request else {
        return false;
    };
    let Some(tool_meta) = request.tool.as_ref() else {
        return false;
    };
    let Some(message_id) = message_id else {
        return false;
    };

    tool_meta.message_id == message_id
}

/// 解析或展示工具权限请求的 pending permission targets tool call 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_targets_tool_call(
    request: Option<&vw_gateway_client::PendingPermissionRequestDto>,
    // message_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    message_id: Option<&str>,
    // raw 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    raw: &str,
) -> bool {
    let Some(request) = request else {
        return false;
    };
    let Some(tool_meta) = request.tool.as_ref() else {
        return false;
    };
    let Some(message_id) = message_id else {
        return false;
    };
    if tool_meta.message_id != message_id {
        return false;
    }

    let expected_call_id = tool_meta.call_id.trim();
    if expected_call_id.is_empty() {
        return false;
    }

    tool_call_id_from_raw(raw).as_deref() == Some(expected_call_id)
}

/// 解析或展示工具权限请求的 pending permission request for tool call 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_request_for_tool_call<'a>(
    requests: &'a [vw_gateway_client::PendingPermissionRequestDto],
    // message_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    message_id: Option<&str>,
    // raw 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    raw: &str,
) -> Option<&'a vw_gateway_client::PendingPermissionRequestDto> {
    let Some(message_id) = message_id else {
        return None;
    };

    if let Some(request) = requests
        .iter()
        .find(|request| pending_permission_targets_tool_call(Some(request), Some(message_id), raw))
    {
        return Some(request);
    }

    let mut message_matches = requests
        .iter()
        .filter(|request| pending_permission_targets_message(Some(request), Some(message_id)));
    let first = message_matches.next()?;
    if message_matches.next().is_none() && tool_call_id_from_raw(raw).is_none() {
        return Some(first);
    }

    None
}

/// 解析或展示工具权限请求的 pending permission request for message 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_request_for_message<'a>(
    requests: &'a [vw_gateway_client::PendingPermissionRequestDto],
    // message_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    message_id: Option<&str>,
) -> Option<&'a vw_gateway_client::PendingPermissionRequestDto> {
    let Some(message_id) = message_id else {
        return None;
    };

    let mut matches = requests
        .iter()
        .filter(|request| pending_permission_targets_message(Some(request), Some(message_id)));
    let first = matches.next()?;
    if matches.next().is_none() { Some(first) } else { None }
}

/// 解析或展示工具权限请求的 pending permission badge label 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_badge_label(
    requests: &[vw_gateway_client::PendingPermissionRequestDto],
    // current_request_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    current_request_id: Option<&str>,
) -> String {
    if requests.len() <= 1 {
        return "当前审批".to_string();
    }

    let current_idx = current_request_id
        .and_then(|request_id| requests.iter().position(|request| request.id == request_id))
        .map(|idx| idx + 1)
        .unwrap_or(1);
    format!("当前审批 {current_idx}/{}", requests.len())
}

/// 解析或展示工具权限请求的 pending permission request badge label 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn pending_permission_request_badge_label(
    requests: &[vw_gateway_client::PendingPermissionRequestDto],
    // request_id 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    request_id: &str,
) -> String {
    if requests.len() <= 1 {
        return "当前审批".to_string();
    }

    let request_idx = requests
        .iter()
        .position(|request| request.id == request_id)
        .map(|idx| idx + 1)
        .unwrap_or(1);
    format!("待审批 {request_idx}/{}", requests.len())
}

/// ToolPermissionState 描述 tool_permission 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolPermissionState {
    ApprovalRequired,
    Rejected,
}

/// 解析或展示工具权限请求的 tool permission state 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_state(
    tool_name: &str,
    // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    value: &Value,
) -> Option<ToolPermissionState> {
    if !matches!(tool_status(value), "error" | "denied") {
        return None;
    }

    let error_text = tool_error_text(value).unwrap_or_default();
    let summary_text = tool_summary_text(value).unwrap_or_default();
    let permission_reason = value
        .get("permission_request")
        .and_then(|item| item.get("reason"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or_default();
    let permission_warning = value
        .get("permission_request")
        .and_then(|item| item.get("warning"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or_default();
    let combined =
        [error_text.as_str(), summary_text.as_str(), permission_reason, permission_warning]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    let lower = combined.to_ascii_lowercase();

    if lower.contains("approval required")
        || lower.contains("requires user approval")
        || lower.contains("requires approval from supervisor")
        || lower.contains("interactive approval required")
    {
        return Some(ToolPermissionState::ApprovalRequired);
    }

    if lower.contains("denied by user")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("not allowed")
        || lower.contains("forbidden")
        || lower.contains("权限不足")
        || lower.contains("用户拒绝授权")
    {
        return Some(ToolPermissionState::Rejected);
    }

    match canonical_tool_name(tool_name) {
        "write" | "file_write" | "file_edit" | "notebook_edit" | "apply_patch" | "skill"
            if tool_status(value) == "denied" =>
        {
            Some(ToolPermissionState::Rejected)
        }
        _ => None,
    }
}

/// 解析或展示工具权限请求的 tool permission summary 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_summary(
    tool_name: &str,
    // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    value: &Value,
) -> Option<&'static str> {
    match tool_permission_state(tool_name, value)? {
        // ToolPermissionState 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        ToolPermissionState::ApprovalRequired => Some("需要权限批准"),
        // ToolPermissionState 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        ToolPermissionState::Rejected => Some("权限已拒绝"),
    }
}

/// 解析或展示工具权限请求的 tool permission detail text 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_detail_text(
    tool_name: &str,
    // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    value: &Value,
) -> Option<String> {
    let request = value.get("permission_request")?;
    let reason = request
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let warning = request
        .get("warning")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let target = request
        .get("updated_input")
        .and_then(|input| permission_request_input_summary(tool_name, input));

    let mut lines = Vec::new();
    if let Some(reason) = reason {
        lines.push(format!("原因：{reason}"));
    }
    if let Some(warning) = warning {
        lines.push(format!("提示：{warning}"));
    }
    if let Some(target) = target {
        lines.push(format!("目标：{target}"));
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// 解析或展示工具权限请求的 tool permission error text 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_error_text(
    tool_name: &str,
    // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    value: &Value,
) -> Option<String> {
    let detail = tool_permission_detail_text(tool_name, value).unwrap_or_default();
    let error = tool_error_text(value).unwrap_or_default();
    let detail = detail.trim();
    let error = error.trim();

    match (detail.is_empty(), error.is_empty()) {
        (true, true) => None,
        (false, true) => Some(detail.to_string()),
        (true, false) => Some(error.to_string()),
        (false, false) => Some(format!("{detail}\n{error}")),
    }
}

/// 解析或展示工具权限请求的 tool permission target summary 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_target_summary(
    tool_name: &str,
    // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    value: &Value,
) -> Option<String> {
    value
        .get("permission_request")
        .and_then(|request| request.get("updated_input"))
        .and_then(|input| permission_request_input_summary(tool_name, input))
}

/// 解析或展示工具权限请求的 tool permission title 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn tool_permission_title(
    base_title: &str,
    // state 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    state: ToolPermissionState,
) -> String {
    match state {
        // ToolPermissionState 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        ToolPermissionState::ApprovalRequired => format!("{base_title}待批准"),
        // ToolPermissionState 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        ToolPermissionState::Rejected => format!("{base_title}已拒绝"),
    }
}

/// 解析或展示工具权限请求的 permission request input summary 状态。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn permission_request_input_summary(
    tool_name: &str,
    // input 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    input: &Value,
) -> Option<String> {
    let raw_input = match input {
        // Value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Value::Null => return None,
        // Value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).ok()?,
    };

    if let Some(summary) = tool_inline_summary(tool_name, &raw_input)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
    {
        return Some(summary.chars().take(80).collect());
    }

    match input {
        // Value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.chars().take(80).collect()) }
        }
        _ => None,
    }
}
