use serde_json::Value;

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

use crate::{context::ToolUseContext, host::ContextHost, ToolCallError};
use tool_types::{Tool, ToolSpec};
use crate::approval::{ApprovalRequest, ApprovalResponse, NonCliApprovalPrompt};
use vw_api_tools::PermissionRequestDto;

/// 执行统一权限流水线，并返回最终可执行输入。
pub async fn finalize_tool_input<H: ContextHost>(
    tool: &dyn Tool,
    input: Value,
    context: &ToolUseContext<H>,
) -> Result<Value, ToolCallError> {
    let decision = crate::hooks::PermissionHook::adapt(resolve_permission_decision::<H>(tool, input, context).await);
    finalize_permission_decision(tool, decision).await
}

pub async fn finalize_permission_decision(
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

pub async fn resolve_permission_decision<H: ContextHost>(
    tool: &dyn Tool,
    input: Value,
    context: &ToolUseContext<H>,
) -> PermissionDecision {
    let spec = tool.spec();
    use permission_types::Action;
    // Agent preferences can restrict the existing pipeline, never bypass it.
    let pattern = input
        .get("file_path")
        .or_else(|| input.get("path"))
        .or_else(|| input.get("command"))
        .or_else(|| input.get("skill"))
        .or_else(|| input.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("*");
    let agent_action = context.native_permission_action(&spec.id, &spec.aliases, pattern);
    if agent_action == Action::Deny {
        return PermissionDecision::deny("Tool denied by native agent permission policy");
    }
    let mut initial = security_decision(&spec, input, context, false);
    if agent_action == Action::Ask
        && let PermissionDecision::Allow { updated_input, warning, .. } = initial
    {
        initial = PermissionDecision::ask("Native agent requires tool approval", updated_input)
            .with_warning(warning);
    }
    let needs_interactive_approval = matches!(initial, PermissionDecision::Ask { .. })
        || H::needs_approval(context, spec.id.as_str());

    if matches!(initial, PermissionDecision::Deny { .. }) || !needs_interactive_approval {
        return initial;
    }

    let display = (spec.id.as_str() == "computer_control")
        .then(|| tool.to_audit_input(initial.updated_input().unwrap_or(&Value::Null)));
    if agent_action == Action::Ask {
        let restricted = context.clone().with_bypass_non_cli_approval_for_turn(false);
        request_interactive_approval_with_display(spec.id.as_str(), initial, &restricted, display)
            .await
    } else {
        request_interactive_approval_with_display(spec.id.as_str(), initial, context, display).await
    }
}

pub fn render_reason(reason: String, warning: Option<&str>) -> String {
    match warning {
        Some(warning) if !warning.trim().is_empty() => format!("{reason}. {warning}"),
        _ => reason,
    }
}

pub fn permission_request_from_decision(
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

pub fn classify_message(error: anyhow::Error) -> ToolCallError {
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

#[cfg(any(unix, test))]
pub async fn request_interactive_approval<H: ContextHost>(
    tool_name: &str,
    pending: PermissionDecision,
    context: &ToolUseContext<H>,
) -> PermissionDecision {
    request_interactive_approval_with_display(tool_name, pending, context, None).await
}

pub async fn request_interactive_approval_with_display<H: ContextHost>(
    tool_name: &str,
    pending: PermissionDecision,
    context: &ToolUseContext<H>,
    display: Option<Value>,
) -> PermissionDecision {
    if let Some(runner) = context.hook_runner() {
        let input = pending.updated_input().cloned().unwrap_or(Value::Null);
        let execution = H::permission_hook(runner, tool_name, &input);
        let verdict = if let Some(token) = context.abort_token() {
            tokio::select! { biased; _ = token.cancelled() => Err(anyhow::anyhow!("permission request cancelled")), value = execution => value }
        } else {
            execution.await
        };
        match verdict {
            Ok(Some(true)) => return approve_after_decision(tool_name, input, pending, context),
            Ok(None) => {}
            Ok(Some(false)) | Err(_) => {
                return PermissionDecision::deny("Permission request denied by hook.")
                    .with_updated_input(input);
            }
        }
    }
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
    // Host-controlled display metadata prevents inline file bytes or page text
    // entering generic prompts/audit logs. Execution still uses the exact input.
    let display = display.unwrap_or_else(|| updated_input.clone());
    let channel_name = context.channel().unwrap_or("cli");

    if context.bypass_non_cli_approval_for_turn() && channel_name != "cli" {
        H::record_approval(manager, tool_name, &display, ApprovalResponse::Yes, channel_name);
        return approve_after_decision(tool_name, updated_input, pending, context);
    }

    let request = ApprovalRequest { tool_name: tool_name.to_string(), arguments: display.clone() };

    let decision = if channel_name == "cli" {
        H::prompt_cli(manager, &request)
    } else if let Some(non_cli) = context.non_cli_approval_context() {
        let pending_request = H::create_pending(manager,
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
            display.clone(),
            context.message_id().map(ToOwned::to_owned),
            context.tool_call_id().map(ToOwned::to_owned),
        );
        let sent = non_cli.prompt_tx.send(NonCliApprovalPrompt {
            request_id: pending_request.clone(),
            tool_name: tool_name.to_string(),
            arguments: display.clone(),
            call_id: context.tool_call_id().map(ToOwned::to_owned),
            sender: non_cli.sender.clone(),
            channel: channel_name.to_string(),
            reply_target: non_cli.reply_target.clone(),
        });
        if sent.is_err() {
            let _ = H::reject_pending(manager,
                &pending_request,
                &non_cli.sender,
                channel_name,
                &non_cli.reply_target,
            );
            return PermissionDecision::deny("Approval request delivery failed.")
                .with_warning(pending.warning().map(ToOwned::to_owned))
                .with_updated_input(updated_input);
        }

        H::await_approval(
            manager,
            &pending_request,
            &non_cli.sender,
            channel_name,
            &non_cli.reply_target,
            context.abort_token(),
        )
        .await
    } else {
        ApprovalResponse::No
    };

    H::record_approval(manager, tool_name, &display, decision, channel_name);
    if decision == ApprovalResponse::No {
        return PermissionDecision::deny("Denied by user.")
            .with_warning(pending.warning().map(ToOwned::to_owned))
            .with_updated_input(updated_input);
    }

    approve_after_decision(tool_name, updated_input, pending, context)
}

pub fn approve_after_decision<H: ContextHost>(
    tool_name: &str,
    updated_input: Value,
    pending: PermissionDecision,
    context: &ToolUseContext<H>,
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

pub fn security_decision<H: ContextHost>(
    spec: &ToolSpec,
    input: Value,
    context: &ToolUseContext<H>,
    approved: bool,
) -> PermissionDecision {
    let Some(security) = context.security() else {
        return PermissionDecision::allow(input);
    };

    if spec.source == vw_api_tools::ToolSource::Builtin
        && let Some(command) = command_input(spec.id.as_str(), &input)
    {
        return H::shell_security_decision(spec.id.as_str(), input, security, &command, approved);
    }

    // 客户端声明的只读属性仅供展示，不授予绕过宿主安全策略的能力。
    let read_only = spec.read_only && spec.source != vw_api_tools::ToolSource::Client;
    if !read_only && !H::can_act(security) {
        return PermissionDecision::deny(format!(
            "Security policy: read-only mode, cannot perform '{}'",
            spec.id
        ))
        .with_updated_input(input);
    }

    if !read_only && H::is_rate_limited(security) {
        return PermissionDecision::deny("Rate limit exceeded: action budget exhausted")
            .with_updated_input(input);
    }

    if let Some(path) = blocked_path::<H>(&input, security) {
        return PermissionDecision::deny(format!("Path not allowed by security policy: {path}"))
            .with_updated_input(input);
    }

    PermissionDecision::allow(input)
}

pub fn blocked_path<H: ContextHost>(input: &Value, security: &H::SecurityPolicy) -> Option<String> {
    let Value::Object(map) = input else {
        return None;
    };

    for key in ["path", "dirPath", "dir", "cwd", "oldPath", "newPath"] {
        let Some(value) = map.get(key) else {
            continue;
        };

        if let Some(path) = value.as_str() {
            if !H::is_path_allowed(security, path) {
                return Some(path.to_string());
            }
            continue;
        }

        if let Some(paths) = value.as_array() {
            for path in paths.iter().filter_map(Value::as_str) {
                if !H::is_path_allowed(security, path) {
                    return Some(path.to_string());
                }
            }
        }
    }

    None
}

pub fn command_input(tool_name: &str, input: &Value) -> Option<String> {
    if !matches!(tool_name, "bash" | "shell" | "process") {
        return None;
    }

    input.get("command").and_then(Value::as_str).map(ToOwned::to_owned)
}

pub fn mark_input_as_approved(tool_name: &str, input: Value) -> Value {
    if !matches!(tool_name, "bash" | "shell" | "process") {
        return input;
    }

    let Value::Object(mut map) = input else {
        return input;
    };
    map.insert("approved".to_string(), Value::Bool(true));
    Value::Object(map)
}

/// Always request concrete host scope approval without inheriting turn bypass.
#[cfg(unix)]
pub async fn request_control_scope_approval<H: ContextHost>(name: &str, input: Value, context: &ToolUseContext<H>) -> bool {
    let strict = context.clone().with_bypass_non_cli_approval_for_turn(false);
    matches!(request_interactive_approval(name, PermissionDecision::allow(input), &strict).await, PermissionDecision::Allow { .. })
}
