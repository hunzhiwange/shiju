//! Effective tool security preserves delegated restrictions.
use agent_config::runtime_schema::Config;
use agent_security::{SecurityPolicy, AutonomyLevel};
use std::sync::Arc;

pub fn security_for_tool_context<H: tool_context::host::ContextHost<SecurityPolicy = SecurityPolicy>>(
    config: &Config,
    workspace_dir: &std::path::Path,
    base_context: &tool_context::context::ToolUseContext<H>,
) -> Arc<SecurityPolicy> {
    let mut security =
        SecurityPolicy::from_configs(&config.autonomy, &config.security, workspace_dir);
    if base_context.full_access_enabled() {
        security.bash_security_enabled = false;
        security.autonomy = AutonomyLevel::Full;
        security.workspace_only = false;
        security.allowed_commands = vec!["*".to_string()];
        security.forbidden_paths.clear();
        security.max_actions_per_hour = u32::MAX;
        security.require_approval_for_medium_risk = false;
        security.block_high_risk_commands = false;
        security.allow_unsafe_shell_patterns = true;
    }
    if let Some(parent) = base_context.delegated_security() {
        use agent_security::ShellRedirectPolicy;
        security.autonomy = match (parent.autonomy, security.autonomy) {
            (AutonomyLevel::ReadOnly, _) | (_, AutonomyLevel::ReadOnly) => AutonomyLevel::ReadOnly,
            (AutonomyLevel::Supervised, _) | (_, AutonomyLevel::Supervised) => {
                AutonomyLevel::Supervised
            }
            _ => AutonomyLevel::Full,
        };
        security.workspace_only |= parent.workspace_only;
        security
            .allowed_roots
            .retain(|root| parent.allowed_roots.iter().any(|allowed| root.starts_with(allowed)));
        security.forbidden_paths.extend(parent.forbidden_paths.clone());
        if security.allowed_commands.iter().any(|command| command == "*") {
            security.allowed_commands = parent.allowed_commands.clone();
        } else {
            security.allowed_commands.retain(|command| {
                parent.allowed_commands.contains(command)
                    || parent.allowed_commands.iter().any(|value| value == "*")
            });
        }
        security.shell_env_passthrough.retain(|name| parent.shell_env_passthrough.contains(name));
        security.max_actions_per_hour =
            security.max_actions_per_hour.min(parent.max_actions_per_hour);
        security.max_cost_per_day_cents =
            security.max_cost_per_day_cents.min(parent.max_cost_per_day_cents);
        security.require_approval_for_medium_risk |= parent.require_approval_for_medium_risk;
        security.block_high_risk_commands |= parent.block_high_risk_commands;
        security.allow_unsafe_shell_patterns &= parent.allow_unsafe_shell_patterns;
        security.shell_redirect_policy =
            match (parent.shell_redirect_policy, security.shell_redirect_policy) {
                (ShellRedirectPolicy::Block, _) | (_, ShellRedirectPolicy::Block) => {
                    ShellRedirectPolicy::Block
                }
                (ShellRedirectPolicy::Strip, _) | (_, ShellRedirectPolicy::Strip) => {
                    ShellRedirectPolicy::Strip
                }
                _ => ShellRedirectPolicy::Allow,
            };
        // A different shell validation program has no proven intersection. Refuse shell
        // commands rather than silently choosing one validator's behavior.
        if serde_json::to_value(&security.bash_security).ok()
            != serde_json::to_value(&parent.bash_security).ok()
        {
            security.allowed_commands.clear();
            security.bash_security = Default::default();
            security.bash_security.enabled = true;
            security.bash_security.policy.enabled = true;
            security.bash_security.policy.command_rules = true;
            security.bash_security_enabled = true;
        }
        security.bash_security_enabled |= parent.bash_security_enabled;
        security.tracker = parent.tracker.clone();
    }
    Arc::new(security)
}
