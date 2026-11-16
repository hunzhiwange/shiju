//! Request authority and capability propagation for native turns.
use crate::{runner::Request, runner_host::RunnerHost, runner_types::ResolvedRequestModel, processor_events::StreamEvent};
use serde_json::Value;
use std::sync::Arc;
use tool_context::{context::ToolUseContext, host::ContextHost};

pub fn request_context<H: RunnerHost>(
    req: &Request<H>,
    native_defaults: Option<crate::capability_policy::NativeAgentDefaults>,
    selected_model: &ResolvedRequestModel,
    model_selected_by_auto: bool,
    snapshot: Arc<<H::CtxHost as ContextHost>::PluginSessionSnapshot>,
    hooks: Option<Arc<<H::CtxHost as ContextHost>::HookRunner>>,
    sink: &mut impl FnMut(StreamEvent) -> bool,
) -> Option<ToolUseContext<H::CtxHost>>
where H::CtxHost: ContextHost<EffectiveDefaults = agent_defaults::EffectiveDefaults>,
{
    let agent_key = req.options.get("agent").and_then(Value::as_str).map(ToOwned::to_owned);
    let mut tool_use_context = crate::runner::with_request_model_context::<H>(
        ToolUseContext::<H::CtxHost>::new(req.session.clone(), req.root.clone()).inherit_delegated_authority(),
        selected_model.step_event_model(),
    )
    .with_session_skills(req.session_skills.clone())
    .with_auto_model_selection(model_selected_by_auto)
    .with_turn_id(
        req.assistant_message_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
    );
    if let Some(name) = agent_key.as_deref() {
        tool_use_context = tool_use_context.with_agent_name(name);
    }
    if let Some(defaults) = native_defaults {
        if defaults.mcp_fixed {
            tool_use_context = tool_use_context
                .with_native_mcp_selection(defaults.effective.mcp.iter().cloned().collect());
        }
        if defaults.permission_fixed {
            tool_use_context = tool_use_context.with_native_permission(defaults.effective);
        }
    }
    tool_use_context = tool_use_context.with_plugin_snapshot(snapshot);
    let delegated_skill_policy = match req.options.get("delegate_skill_policy") {
        Some(value) => {
            match serde_json::from_value::<config_agent::DelegateCapabilityPolicy>(value.clone()) {
                Ok(policy) => Some(policy),
                Err(error) => {
                    sink(StreamEvent::Error(format!("无效的技能能力策略：{error}")));
                    return None;
                }
            }
        }
        None => None,
    };
    if let Some(policy) = delegated_skill_policy {
        tool_use_context = tool_use_context.with_delegate_skill_policy(
            policy,
            if req.options.get("delegate_allowed_skills").is_some() {
                crate::runner_protocol::option_string_values(
                    &req.options,
                    "delegate_allowed_skills",
                )
            } else {
                crate::runner_protocol::option_string_values(
                    &req.options,
                    "delegate_default_allowed_skills",
                )
            },
        );
    }
    if let Some(runner) = hooks {
        tool_use_context = tool_use_context.with_hook_runner(runner);
    }
    let trusted_user_selected_skills = crate::runner_protocol::option_string_values(
        &req.options,
        "delegate_trusted_user_selected_skills",
    );
    if !trusted_user_selected_skills.is_empty() {
        tool_use_context =
            tool_use_context.with_trusted_user_selected_skills(trusted_user_selected_skills);
    }
    if let Some(channel_name) = req.channel_name.clone() {
        tool_use_context = tool_use_context.with_channel(channel_name);
    }
    if let Some(approval) = req.approval.clone() {
        tool_use_context = tool_use_context.with_approval(approval);
    }
    if let Some(non_cli_approval_context) = req.non_cli_approval_context.clone() {
        tool_use_context = tool_use_context.with_non_cli_approval_context(non_cli_approval_context);
    }
    if let Some(message_id) = req.assistant_message_id.clone() {
        tool_use_context = tool_use_context.with_message_id(message_id);
    }
    if let Some(abort_token) = req.abort_token.clone() {
        tool_use_context = tool_use_context.with_abort_token(abort_token);
    }
    let full_access_enabled = crate::runner::request_full_access_enabled(&req.options);
    tool_use_context = tool_use_context.with_full_access_enabled(full_access_enabled);
    if full_access_enabled {
        tool_use_context = tool_use_context.with_bypass_non_cli_approval_for_turn(true);
    }
    Some(tool_use_context)
}
