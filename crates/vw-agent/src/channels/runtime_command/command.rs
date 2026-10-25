//! 非 CLI 通道运行时命令的解析与分类。
//!
//! 本模块把聊天消息中的斜杠命令和受限自然语言意图转换为明确枚举，供处理层
//! 统一执行。授权相关命令在这里保持可识别但不直接放权，实际权限检查位于 handler。

use super::super::*;

/// 通道运行时可识别的控制命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChannelRuntimeCommand {
    ListProjects,
    ShowProject,
    SelectProject(String),
    /// 展示可用 provider。
    ShowProviders,
    /// 切换当前会话 provider。
    SetProvider(String),
    /// 展示当前模型。
    ShowModel,
    /// 切换当前模型。
    SetModel(String),
    /// 开启新会话并清理当前上下文。
    NewSession,
    /// 进入或处理任务模式。
    TaskMode,
    /// 请求一次性允许所有工具/命令。
    RequestAllToolsOnce,
    /// 为单个工具创建待确认授权请求。
    RequestToolApproval(String),
    /// 确认待授权请求并可能持久化。
    ConfirmToolApproval(String),
    /// 允许当前待审批请求，仅作用于本次调用。
    ApprovePendingRequest(String),
    /// 拒绝当前待审批请求。
    DenyToolApproval(String),
    /// 列出当前作用域内待处理授权。
    ListPendingApprovals,
    /// 直接批准单个工具。
    ApproveTool(String),
    /// 撤销单个工具的批准。
    UnapproveTool(String),
    /// 列出现有授权配置与运行时状态。
    ListApprovals,
}

/// 判断指定通道是否支持运行时模型/provider 切换。
///
/// 参数：`channel_name` 是通道注册名。
///
/// 返回值：支持 `/model` 和 `/models` 时返回 `true`。
///
/// 错误处理：该函数不产生错误，未知通道按不支持处理。
pub(crate) fn supports_runtime_model_switch(channel_name: &str) -> bool {
    matches!(channel_name, "telegram" | "discord")
}

/// 把用户消息解析为运行时命令。
///
/// 参数：
/// - `channel_name`：消息来源通道，用于判断通道能力。
/// - `content`：原始消息正文。
///
/// 返回值：识别成功返回命令枚举，否则返回 `None`。
///
/// 错误处理：解析失败不会报错，避免普通聊天消息被当作异常路径。
pub(crate) fn parse_runtime_command(
    channel_name: &str,
    content: &str,
) -> Option<ChannelRuntimeCommand> {
    let trimmed = content.trim();
    if !trimmed.starts_with('/') {
        return parse_natural_language_runtime_command(trimmed);
    }

    let mut parts = trimmed.split_whitespace();
    let command_token = parts.next()?;
    let base_command =
        command_token.split('@').next().unwrap_or(command_token).to_ascii_lowercase();
    let args: Vec<&str> = parts.collect();
    let tail = args.join(" ").trim().to_string();

    // Telegram bot 命令可能带有 `@bot_name` 后缀，解析时只比较基础命令名。
    match base_command.as_str() {
        "/projects" => Some(ChannelRuntimeCommand::ListProjects),
        "/project" if tail.is_empty() => Some(ChannelRuntimeCommand::ShowProject),
        "/project" => Some(ChannelRuntimeCommand::SelectProject(tail)),
        "/new" | "/clear" | "/session" => Some(ChannelRuntimeCommand::NewSession),
        "/task" => Some(ChannelRuntimeCommand::TaskMode),
        "/approve-all-once" => Some(ChannelRuntimeCommand::RequestAllToolsOnce),
        "/approve-request" => Some(ChannelRuntimeCommand::RequestToolApproval(tail)),
        "/approve-confirm" => Some(ChannelRuntimeCommand::ConfirmToolApproval(tail)),
        "/approve-allow" => Some(ChannelRuntimeCommand::ApprovePendingRequest(tail)),
        "/approve-deny" => Some(ChannelRuntimeCommand::DenyToolApproval(tail)),
        "/approve-pending" => Some(ChannelRuntimeCommand::ListPendingApprovals),
        "/approve" => Some(ChannelRuntimeCommand::ApproveTool(tail)),
        "/unapprove" => Some(ChannelRuntimeCommand::UnapproveTool(tail)),
        "/approvals" => Some(ChannelRuntimeCommand::ListApprovals),
        "/models" if supports_runtime_model_switch(channel_name) => {
            if let Some(provider) = args.first() {
                Some(ChannelRuntimeCommand::SetProvider(provider.trim().to_string()))
            } else {
                Some(ChannelRuntimeCommand::ShowProviders)
            }
        }
        "/model" if supports_runtime_model_switch(channel_name) => {
            let model = tail;
            if model.is_empty() {
                Some(ChannelRuntimeCommand::ShowModel)
            } else {
                Some(ChannelRuntimeCommand::SetModel(model))
            }
        }
        _ => None,
    }
}

/// 判断字符串是否可作为运行时命令尾部 token。
///
/// 参数：`value` 是候选 token。
///
/// 返回值：非空且只包含受限 ASCII 字符时返回 `true`。
///
/// 错误处理：该函数不产生错误。
pub(crate) fn is_runtime_token(value: &str) -> bool {
    let token = value.trim();
    // 授权请求 ID 和工具名只允许保守字符集，避免自然语言误解析成危险控制输入。
    !token.is_empty()
        && token.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
}

/// 从指定前缀后提取一个受限运行时 token。
///
/// 参数：
/// - `text`：待匹配文本。
/// - `prefixes`：允许的命令前缀集合。
///
/// 返回值：匹配到合法 token 时返回其字符串。
///
/// 错误处理：该函数不产生错误，无法匹配时返回 `None`。
pub(crate) fn extract_runtime_tail_token(text: &str, prefixes: &[&str]) -> Option<String> {
    prefixes.iter().find_map(|prefix| {
        text.strip_prefix(prefix).and_then(|rest| {
            let token = rest.trim();
            if is_runtime_token(token) { Some(token.to_string()) } else { None }
        })
    })
}

/// 判断文本中是否包含任一片段。
///
/// 参数：
/// - `haystack`：待搜索文本。
/// - `fragments`：候选片段列表。
///
/// 返回值：存在任一片段时返回 `true`。
///
/// 错误处理：该函数不产生错误。
pub(crate) fn contains_any_fragment(haystack: &str, fragments: &[&str]) -> bool {
    fragments.iter().any(|fragment| haystack.contains(fragment))
}

/// 识别“一次性允许所有工具”的自然语言意图。
///
/// 参数：`content` 是用户消息正文。
///
/// 返回值：同时具备授权动词、全量工具范围和一次性范围时返回 `true`。
///
/// 错误处理：该函数不产生错误；空字符串和不完整表达都按 `false` 处理。
pub(crate) fn is_natural_language_all_tools_once_intent(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    let has_allow_verb = contains_any_fragment(&lower, &["approve", "allow"])
        || contains_any_fragment(trimmed, &["授权", "放开", "允许"]);
    let has_all_tools_scope = contains_any_fragment(&lower, &["all tools", "all commands"])
        || contains_any_fragment(trimmed, &["所有工具", "全部工具", "所有命令", "全部命令"]);
    let has_one_time_scope = contains_any_fragment(&lower, &["once", "one-time", "one time"])
        || contains_any_fragment(trimmed, &["一次", "这次"]);

    has_allow_verb && has_all_tools_scope && has_one_time_scope
}

/// 生成人类可读的授权目标标签。
///
/// 参数：`tool_name` 是工具名或内部一次性全工具 token。
///
/// 返回值：用于回复消息的显示名称。
///
/// 错误处理：该函数不产生错误。
pub(crate) fn approval_target_label(tool_name: &str) -> String {
    if tool_name == APPROVAL_ALL_TOOLS_ONCE_TOKEN {
        "all tools/commands (one-time bypass token)".to_string()
    } else {
        tool_name.to_string()
    }
}

/// 解析受限自然语言形式的运行时授权命令。
///
/// 参数：`content` 是已去除斜杠命令路径后的消息正文。
///
/// 返回值：识别成功返回授权相关命令，否则返回 `None`。
///
/// 错误处理：该函数不产生错误；自然语言授权只识别窄模式，减少误触发。
pub(crate) fn parse_natural_language_runtime_command(
    content: &str,
) -> Option<ChannelRuntimeCommand> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "show pending approvals" | "list pending approvals" | "pending approvals"
    ) {
        return Some(ChannelRuntimeCommand::ListPendingApprovals);
    }
    if trimmed == "查看授权"
        || matches!(lower.as_str(), "show approvals" | "list approvals" | "approvals")
    {
        return Some(ChannelRuntimeCommand::ListApprovals);
    }
    if is_natural_language_all_tools_once_intent(trimmed)
        || matches!(
            lower.as_str(),
            "approve all tools once" | "allow all tools once" | "approve all once"
        )
    {
        return Some(ChannelRuntimeCommand::RequestAllToolsOnce);
    }

    // 自然语言路径只接受单 token 尾部参数，不接受自由文本，降低越权误识别风险。
    if let Some(request_id) = extract_runtime_tail_token(&lower, &["confirm "]) {
        return Some(ChannelRuntimeCommand::ConfirmToolApproval(request_id));
    }
    if let Some(request_id) = extract_runtime_tail_token(trimmed, &["确认授权 "]) {
        return Some(ChannelRuntimeCommand::ConfirmToolApproval(request_id));
    }

    if let Some(tool) =
        extract_runtime_tail_token(&lower, &["revoke tool ", "unapprove ", "revoke "])
    {
        return Some(ChannelRuntimeCommand::UnapproveTool(tool));
    }
    if let Some(tool) = extract_runtime_tail_token(trimmed, &["撤销工具 ", "取消授权 "]) {
        return Some(ChannelRuntimeCommand::UnapproveTool(tool));
    }

    if let Some(tool) = extract_runtime_tail_token(&lower, &["approve tool ", "approve "]) {
        return Some(ChannelRuntimeCommand::RequestToolApproval(tool));
    }
    if let Some(tool) = extract_runtime_tail_token(trimmed, &["授权工具 ", "请放开 ", "放开 "])
    {
        return Some(ChannelRuntimeCommand::RequestToolApproval(tool));
    }

    None
}

/// 判断命令是否属于授权管理范围。
///
/// 参数：`command` 是已解析的运行时命令。
///
/// 返回值：授权创建、确认、拒绝、撤销或列表命令返回 `true`。
///
/// 错误处理：该函数不产生错误。
pub(crate) fn is_approval_management_command(command: &ChannelRuntimeCommand) -> bool {
    matches!(
        command,
        ChannelRuntimeCommand::RequestAllToolsOnce
            | ChannelRuntimeCommand::RequestToolApproval(_)
            | ChannelRuntimeCommand::ConfirmToolApproval(_)
            | ChannelRuntimeCommand::ApprovePendingRequest(_)
            | ChannelRuntimeCommand::DenyToolApproval(_)
            | ChannelRuntimeCommand::ListPendingApprovals
            | ChannelRuntimeCommand::ApproveTool(_)
            | ChannelRuntimeCommand::UnapproveTool(_)
            | ChannelRuntimeCommand::ListApprovals
    )
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod command_tests;
