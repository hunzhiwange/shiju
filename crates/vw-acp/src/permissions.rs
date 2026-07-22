//! 权限请求的决策分类与模式匹配逻辑。
//!
//! 本模块集中处理工具调用时的权限决策，负责根据当前权限模式、
//! 非交互策略和请求内容判断是自动放行、拒绝，还是进入确认流程。
//!
//! 该层的目标是让权限语义保持可预测，避免运行时各处重复实现分散的判断分支。

use agent_client_protocol::{
    PermissionOption, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, ToolKind,
};

use crate::errors::PermissionPromptUnavailableError;
use crate::permission_prompt::{
    PermissionPromptOptions, can_prompt_for_permission, prompt_for_permission,
};
use crate::types::{NonInteractivePermissionPolicy, PermissionMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Approved,
    Denied,
    Cancelled,
}

fn permission_mode_rank(mode: PermissionMode) -> u8 {
    match mode {
        PermissionMode::DenyAll => 0,
        PermissionMode::ApproveReads => 1,
        PermissionMode::ApproveAll => 2,
    }
}

fn selected(option_id: String) -> RequestPermissionResponse {
    RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option_id),
    ))
}

fn cancelled() -> RequestPermissionResponse {
    RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
}

fn pick_option<'a>(
    options: &'a [PermissionOption],
    kinds: &[PermissionOptionKind],
) -> Option<&'a PermissionOption> {
    for kind in kinds {
        if let Some(option) = options.iter().find(|option| option.kind == *kind) {
            return Some(option);
        }
    }
    None
}

fn infer_tool_kind_from_title(title: &str) -> Option<&'static str> {
    let head = title
        .trim()
        .to_ascii_lowercase()
        .split(':')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if head.is_empty() {
        return None;
    }
    if head.contains("read") || head.contains("cat") {
        return Some("read");
    }
    if head.contains("search") || head.contains("find") || head.contains("grep") {
        return Some("search");
    }
    if head.contains("write") || head.contains("edit") || head.contains("patch") {
        return Some("edit");
    }
    if head.contains("delete") || head.contains("remove") {
        return Some("delete");
    }
    if head.contains("move") || head.contains("rename") {
        return Some("move");
    }
    if head.contains("run") || head.contains("execute") || head.contains("bash") {
        return Some("execute");
    }
    if head.contains("fetch") || head.contains("http") || head.contains("url") {
        return Some("fetch");
    }
    if head.contains("think") {
        return Some("think");
    }
    Some("other")
}

fn tool_kind_name(kind: ToolKind) -> &'static str {
    let mut name = "other";
    if kind == ToolKind::Read {
        name = "read";
    }
    if kind == ToolKind::Edit {
        name = "edit";
    }
    if kind == ToolKind::Delete {
        name = "delete";
    }
    if kind == ToolKind::Move {
        name = "move";
    }
    if kind == ToolKind::Search {
        name = "search";
    }
    if kind == ToolKind::Execute {
        name = "execute";
    }
    if kind == ToolKind::Think {
        name = "think";
    }
    if kind == ToolKind::Fetch {
        name = "fetch";
    }
    if kind == ToolKind::SwitchMode {
        name = "switch_mode";
    }
    name
}

fn tool_call_title_field(params: &RequestPermissionRequest) -> Option<String> {
    params.tool_call.fields.title.clone()
}

fn tool_call_kind_field(params: &RequestPermissionRequest) -> Option<String> {
    params.tool_call.fields.kind.map(tool_kind_name).map(str::to_string)
}

fn infer_tool_kind(params: &RequestPermissionRequest) -> Option<String> {
    tool_call_kind_field(params).or_else(|| {
        tool_call_title_field(params)
            .and_then(|value| infer_tool_kind_from_title(&value).map(str::to_string))
    })
}

fn is_auto_approved_read_kind(kind: Option<&str>) -> bool {
    matches!(kind, Some("read" | "search"))
}

fn prompt_for_tool_permission(params: &RequestPermissionRequest) -> Result<bool, std::io::Error> {
    let tool_name = tool_call_title_field(params).unwrap_or_else(|| "tool".to_string());
    let tool_kind = infer_tool_kind(params).unwrap_or_else(|| "other".to_string());
    prompt_for_permission(&PermissionPromptOptions {
        prompt: format!("\n[permission] Allow {tool_name} [{tool_kind}]? (y/N) "),
        header: None,
        details: None,
    })
}

fn prompt_for_tool_permission_or_false(params: &RequestPermissionRequest) -> bool {
    prompt_for_tool_permission(params).unwrap_or(false)
}

fn response_for_prompt_decision(
    approved: bool,
    allow_option: Option<&PermissionOption>,
    reject_option: Option<&PermissionOption>,
) -> RequestPermissionResponse {
    if approved && let Some(option) = allow_option {
        return selected(option.option_id.to_string());
    }
    if !approved && let Some(option) = reject_option {
        return selected(option.option_id.to_string());
    }
    cancelled()
}

pub fn permission_mode_satisfies(actual: PermissionMode, required: PermissionMode) -> bool {
    permission_mode_rank(actual) >= permission_mode_rank(required)
}

#[allow(clippy::result_large_err)]
fn resolve_permission_request_with_prompt(
    params: &RequestPermissionRequest,
    mode: PermissionMode,
    non_interactive_policy: Option<NonInteractivePermissionPolicy>,
    can_prompt: bool,
    prompt: fn(&RequestPermissionRequest) -> bool,
) -> Result<RequestPermissionResponse, PermissionPromptUnavailableError> {
    let options = &params.options;
    if options.is_empty() {
        return Ok(cancelled());
    }

    let allow_once_option =
        pick_option(options, &[PermissionOptionKind::AllowOnce, PermissionOptionKind::AllowAlways]);
    let allow_always_option =
        pick_option(options, &[PermissionOptionKind::AllowAlways, PermissionOptionKind::AllowOnce]);
    let reject_option = pick_option(
        options,
        &[PermissionOptionKind::RejectOnce, PermissionOptionKind::RejectAlways],
    );

    match mode {
        PermissionMode::ApproveAll => {
            return Ok(selected(allow_always_option.unwrap_or(&options[0]).option_id.to_string()));
        }
        PermissionMode::DenyAll => {
            return Ok(reject_option
                .map(|option| selected(option.option_id.to_string()))
                .unwrap_or_else(cancelled));
        }
        PermissionMode::ApproveReads => {}
    }

    let kind = infer_tool_kind(params);
    if is_auto_approved_read_kind(kind.as_deref())
        && let Some(option) = allow_always_option
    {
        return Ok(selected(option.option_id.to_string()));
    }

    if let Some(policy) = non_interactive_policy {
        return match policy {
            NonInteractivePermissionPolicy::Deny => Ok(reject_option
                .map(|option| selected(option.option_id.to_string()))
                .unwrap_or_else(cancelled)),
            NonInteractivePermissionPolicy::Fail => Err(PermissionPromptUnavailableError::new()),
        };
    }

    if !can_prompt {
        return Ok(reject_option
            .map(|option| selected(option.option_id.to_string()))
            .unwrap_or_else(cancelled));
    }

    Ok(response_for_prompt_decision(prompt(params), allow_once_option, reject_option))
}

#[allow(clippy::result_large_err)]
pub fn resolve_permission_request(
    params: &RequestPermissionRequest,
    mode: PermissionMode,
    non_interactive_policy: Option<NonInteractivePermissionPolicy>,
) -> Result<RequestPermissionResponse, PermissionPromptUnavailableError> {
    resolve_permission_request_with_prompt(
        params,
        mode,
        non_interactive_policy,
        can_prompt_for_permission(),
        prompt_for_tool_permission_or_false,
    )
}

pub fn classify_permission_decision(
    params: &RequestPermissionRequest,
    response: &RequestPermissionResponse,
) -> PermissionDecision {
    let RequestPermissionOutcome::Selected(SelectedPermissionOutcome { option_id, .. }) =
        &response.outcome
    else {
        return PermissionDecision::Cancelled;
    };

    let Some(selected_option) = params.options.iter().find(|option| &option.option_id == option_id)
    else {
        return PermissionDecision::Cancelled;
    };

    let kind = selected_option.kind;
    let mut decision = PermissionDecision::Cancelled;
    if kind == PermissionOptionKind::AllowOnce || kind == PermissionOptionKind::AllowAlways {
        decision = PermissionDecision::Approved;
    }
    if kind == PermissionOptionKind::RejectOnce || kind == PermissionOptionKind::RejectAlways {
        decision = PermissionDecision::Denied;
    }
    decision
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod permissions_tests;
