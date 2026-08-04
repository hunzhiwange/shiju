//! 工具权限决策流水线。
//!
//! 该模块负责把下列原本分散在不同入口里的判断串成一条统一流水线：
//!
//! 1. 工具输入校验后的自身权限检查
//! 2. 运行时安全策略（SecurityPolicy）
//! 3. 交互式审批（ApprovalManager）
//! 4. 非 CLI 审批等待
//!
//! 流水线的结果统一表达为 `PermissionDecision`，从而让调用方不再自己拼接
//! `allow / ask / deny` 分支。

use super::context::ToolUseContext;
use super::hooks::PermissionHook;
use super::toolset::ToolCallError;
use super::{Tool, ToolSpec};
use crate::agent::loop_::approval::NonCliApprovalPrompt;
use crate::agent::loop_::approval::await_non_cli_approval_decision;
use crate::app::agent::approval::{ApprovalRequest, ApprovalResponse};
use crate::app::agent::security::SecurityPolicy;
use crate::tools::shell::permissions::{Permission, PermissionContext, PermissionMode};
use serde_json::Value;
use vw_api_types::tools::PermissionRequestDto;

/// 工具权限决策结果。
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    /// 允许执行，并可能附带归一化后的输入。
    Allow { reason: Option<String>, updated_input: Value, warning: Option<String> },
    /// 需要继续走审批确认。
    Ask { reason: String, updated_input: Value, warning: Option<String> },
    /// 拒绝执行。
    Deny { reason: String, updated_input: Option<Value>, warning: Option<String> },
}

impl PermissionDecision {
    /// 构造允许决策。
    pub fn allow(updated_input: Value) -> Self {
        Self::Allow { reason: None, updated_input, warning: None }
    }

    /// 构造需要审批的决策。
    pub fn ask(reason: impl Into<String>, updated_input: Value) -> Self {
        Self::Ask { reason: reason.into(), updated_input, warning: None }
    }

    /// 构造拒绝决策。
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny { reason: reason.into(), updated_input: None, warning: None }
    }

    /// 为决策补充 warning。
    pub fn with_warning(mut self, warning: Option<String>) -> Self {
        match &mut self {
            Self::Allow { warning: slot, .. }
            | Self::Ask { warning: slot, .. }
            | Self::Deny { warning: slot, .. } => *slot = warning,
        }
        self
    }

    /// 为拒绝决策补充输入快照。
    pub fn with_updated_input(mut self, updated_input: Value) -> Self {
        if let Self::Deny { updated_input: slot, .. } = &mut self {
            *slot = Some(updated_input);
        }
        self
    }

    /// 读取决策原因。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Allow { reason, .. } => reason.as_deref(),
            Self::Ask { reason, .. } | Self::Deny { reason, .. } => Some(reason.as_str()),
        }
    }

    /// 读取 warning。
    pub fn warning(&self) -> Option<&str> {
        match self {
            Self::Allow { warning, .. }
            | Self::Ask { warning, .. }
            | Self::Deny { warning, .. } => warning.as_deref(),
        }
    }

    /// 读取更新后的输入。
    pub fn updated_input(&self) -> Option<&Value> {
        match self {
            Self::Allow { updated_input, .. } | Self::Ask { updated_input, .. } => {
                Some(updated_input)
            }
            Self::Deny { updated_input, .. } => updated_input.as_ref(),
        }
    }
}

/// 执行统一权限流水线，并返回最终可执行输入。
pub(crate) async fn finalize_tool_input(
    tool: &dyn Tool,
    input: Value,
    context: &ToolUseContext,
) -> Result<Value, ToolCallError> {
    let decision = PermissionHook::adapt(resolve_permission_decision(tool, input, context).await);
    finalize_permission_decision(tool, decision).await
}

async fn finalize_permission_decision(
    tool: &dyn Tool,
    decision: PermissionDecision,
) -> Result<Value, ToolCallError> {
    match decision {
        PermissionDecision::Allow { updated_input, .. } => {
            tool.check_permissions(&updated_input).await.map_err(classify_message)?;
            Ok(updated_input)
        }
        PermissionDecision::Ask { reason, warning, updated_input } => {
            let message = render_reason(reason.clone(), warning.as_deref());
            Err(ToolCallError::denied_with_permission_request(
                message,
                PermissionRequestDto { reason, warning, updated_input: Some(updated_input) },
            ))
        }
        PermissionDecision::Deny { reason, warning, updated_input } => {
            let message = render_reason(reason.clone(), warning.as_deref());
            match permission_request_from_decision(
                &reason,
                warning.as_deref(),
                updated_input.as_ref(),
            ) {
                Some(permission_request) => {
                    Err(ToolCallError::denied_with_permission_request(message, permission_request))
                }
                None => Err(ToolCallError::denied(message)),
            }
        }
    }
}

async fn resolve_permission_decision(
    tool: &dyn Tool,
    input: Value,
    context: &ToolUseContext,
) -> PermissionDecision {
    let spec = tool.spec();
    let initial = security_decision(&spec, input, context, false);
    let needs_interactive_approval = matches!(initial, PermissionDecision::Ask { .. })
        || context
            .approval_manager()
            .is_some_and(|manager| manager.needs_approval(spec.id.as_str()));

    if matches!(initial, PermissionDecision::Deny { .. }) || !needs_interactive_approval {
        return initial;
    }

    request_interactive_approval(spec.id.as_str(), initial, context).await
}

async fn request_interactive_approval(
    tool_name: &str,
    pending: PermissionDecision,
    context: &ToolUseContext,
) -> PermissionDecision {
    let Some(manager) = context.approval_manager() else {
        return PermissionDecision::deny(match pending.reason() {
            Some(reason) => format!("Approval required but unavailable: {reason}"),
            None => "Approval required but unavailable".to_string(),
        })
        .with_warning(pending.warning().map(ToOwned::to_owned))
        .with_updated_input(
            pending.updated_input().cloned().unwrap_or_else(|| Value::Object(Default::default())),
        );
    };

    let updated_input =
        pending.updated_input().cloned().unwrap_or_else(|| Value::Object(Default::default()));
    let channel_name = context.channel().unwrap_or("cli");

    if context.bypass_non_cli_approval_for_turn() && channel_name != "cli" {
        manager.record_decision(tool_name, &updated_input, ApprovalResponse::Yes, channel_name);
        return approve_after_decision(tool_name, updated_input, pending, context);
    }

    let request =
        ApprovalRequest { tool_name: tool_name.to_string(), arguments: updated_input.clone() };

    let decision = if channel_name == "cli" {
        manager.prompt_cli(&request)
    } else if let Some(non_cli) = context.non_cli_approval_context() {
        let pending_request = manager.create_non_cli_pending_request(
            tool_name,
            &non_cli.sender,
            channel_name,
            &non_cli.reply_target,
            Some(
                pending
                    .reason()
                    .unwrap_or("interactive approval required for tool execution")
                    .to_string(),
            ),
            updated_input.clone(),
            context.message_id().map(ToOwned::to_owned),
            context.tool_call_id().map(ToOwned::to_owned),
        );
        let sent = non_cli.prompt_tx.send(NonCliApprovalPrompt {
            request_id: pending_request.request_id.clone(),
            tool_name: tool_name.to_string(),
            arguments: updated_input.clone(),
            call_id: context.tool_call_id().map(ToOwned::to_owned),
            sender: non_cli.sender.clone(),
            channel: channel_name.to_string(),
            reply_target: non_cli.reply_target.clone(),
        });
        if sent.is_err() {
            let _ = manager.reject_non_cli_pending_request(
                &pending_request.request_id,
                &non_cli.sender,
                channel_name,
                &non_cli.reply_target,
            );
            return PermissionDecision::deny("Approval request delivery failed.")
                .with_warning(pending.warning().map(ToOwned::to_owned))
                .with_updated_input(updated_input);
        }

        await_non_cli_approval_decision(
            manager,
            &pending_request.request_id,
            &non_cli.sender,
            channel_name,
            &non_cli.reply_target,
            context.abort_token(),
        )
        .await
    } else {
        ApprovalResponse::No
    };

    manager.record_decision(tool_name, &updated_input, decision, channel_name);
    if decision == ApprovalResponse::No {
        return PermissionDecision::deny("Denied by user.")
            .with_warning(pending.warning().map(ToOwned::to_owned))
            .with_updated_input(updated_input);
    }

    approve_after_decision(tool_name, updated_input, pending, context)
}

fn approve_after_decision(
    tool_name: &str,
    updated_input: Value,
    pending: PermissionDecision,
    context: &ToolUseContext,
) -> PermissionDecision {
    let approved_input = mark_input_as_approved(tool_name, updated_input);

    if matches!(pending, PermissionDecision::Ask { .. }) {
        let spec = ToolSpec::new(tool_name, tool_name, Value::Null);
        let rechecked = security_decision(&spec, approved_input, context, true);
        return match rechecked {
            PermissionDecision::Ask { reason, warning, updated_input } => PermissionDecision::deny(
                format!("Approval granted but security policy still requires approval: {reason}"),
            )
            .with_warning(warning)
            .with_updated_input(updated_input),
            other => other,
        };
    }

    PermissionDecision::Allow {
        reason: pending.reason().map(ToOwned::to_owned),
        updated_input: approved_input,
        warning: pending.warning().map(ToOwned::to_owned),
    }
}

fn security_decision(
    spec: &ToolSpec,
    input: Value,
    context: &ToolUseContext,
    approved: bool,
) -> PermissionDecision {
    let Some(security) = context.security() else {
        return PermissionDecision::allow(input);
    };

    if let Some(command) = command_input(spec.id.as_str(), &input) {
        return shell_security_decision(spec.id.as_str(), input, security, &command, approved);
    }

    if !spec.read_only && !security.can_act() {
        return PermissionDecision::deny(format!(
            "Security policy: read-only mode, cannot perform '{}'",
            spec.id
        ))
        .with_updated_input(input);
    }

    if !spec.read_only && security.is_rate_limited() {
        return PermissionDecision::deny("Rate limit exceeded: action budget exhausted")
            .with_updated_input(input);
    }

    if let Some(path) = blocked_path(&input, security) {
        return PermissionDecision::deny(format!("Path not allowed by security policy: {path}"))
            .with_updated_input(input);
    }

    PermissionDecision::allow(input)
}

fn shell_security_decision(
    tool_name: &str,
    input: Value,
    security: &SecurityPolicy,
    command: &str,
    approved: bool,
) -> PermissionDecision {
    let context = PermissionContext {
        autonomy: security.autonomy,
        in_sandbox: false,
        mode: PermissionMode::Normal,
        approved,
        workspace_dir: security.workspace_dir.clone(),
        allowed_roots: security.allowed_roots.clone(),
    };

    let updated_input = if approved { mark_input_as_approved(tool_name, input) } else { input };

    match security.check_shell_permission(command, &context).permission {
        Some(Permission::Allow) | None => PermissionDecision::allow(updated_input),
        Some(Permission::Ask { reason, warning }) => {
            PermissionDecision::ask(reason, updated_input).with_warning(warning)
        }
        Some(Permission::Deny { reason }) => {
            PermissionDecision::deny(reason).with_updated_input(updated_input)
        }
    }
}

fn blocked_path(input: &Value, security: &SecurityPolicy) -> Option<String> {
    let Value::Object(map) = input else {
        return None;
    };

    for key in ["path", "filePath", "file_path", "dirPath", "dir", "cwd", "oldPath", "newPath"] {
        let Some(value) = map.get(key) else {
            continue;
        };

        if let Some(path) = value.as_str() {
            if !security.is_path_allowed(path) {
                return Some(path.to_string());
            }
            continue;
        }

        if let Some(paths) = value.as_array() {
            for path in paths.iter().filter_map(Value::as_str) {
                if !security.is_path_allowed(path) {
                    return Some(path.to_string());
                }
            }
        }
    }

    None
}

fn command_input(tool_name: &str, input: &Value) -> Option<String> {
    if !matches!(tool_name, "bash" | "shell" | "process") {
        return None;
    }

    input.get("command").and_then(Value::as_str).map(ToOwned::to_owned)
}

fn mark_input_as_approved(tool_name: &str, input: Value) -> Value {
    if !matches!(tool_name, "bash" | "shell" | "process") {
        return input;
    }

    let Value::Object(mut map) = input else {
        return input;
    };
    map.insert("approved".to_string(), Value::Bool(true));
    Value::Object(map)
}

fn render_reason(reason: String, warning: Option<&str>) -> String {
    match warning {
        Some(warning) if !warning.trim().is_empty() => format!("{reason}. {warning}"),
        _ => reason,
    }
}

fn permission_request_from_decision(
    reason: &str,
    warning: Option<&str>,
    updated_input: Option<&Value>,
) -> Option<PermissionRequestDto> {
    let lower = reason.to_ascii_lowercase();
    let is_permission_related = lower.contains("approval")
        || lower.contains("denied by user")
        || lower.contains("supervisor")
        || lower.contains("user approval");

    is_permission_related.then(|| PermissionRequestDto {
        reason: reason.to_string(),
        warning: warning.map(ToOwned::to_owned),
        updated_input: updated_input.cloned(),
    })
}

fn classify_message(error: anyhow::Error) -> ToolCallError {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("denied")
        || lower.contains("not allowed")
        || lower.contains("forbidden")
        || lower.contains("blocked")
    {
        ToolCallError::denied(message)
    } else {
        ToolCallError::Failed(message)
    }
}
#[cfg(test)]
mod tests;
