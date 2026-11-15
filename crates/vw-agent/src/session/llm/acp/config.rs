//! Host resolution of trusted ACP agent defaults.
use serde_json::Value;
use acp_types::PermissionMode;
use crate::config;
use super::{Error, ParsedAcpOptions, acp_option_error};
pub(crate) use llm_runtime::acp::config::{normalize_acp_agent_config, build_acp_command_line, lookup_acp_command};

/// Resolve trusted agent defaults at the gateway; raw request options never supply MCP credentials.
pub(super) async fn apply_agent_defaults(
    cfg: &config::schema::Config,
    options: &Value,
    backend: &str,
    cwd: &std::path::Path,
    parsed: &mut ParsedAcpOptions,
    messages: &Value,
) -> Result<Value, Error> {
    use crate::agent::defaults::{DefaultSelection, DefaultsContext, resolve};
    use config_agent_profile::{AgentDefault, AgentRuntime};
    let Some(key) = options.get("agent").and_then(Value::as_str) else {
        return Ok(messages.clone());
    };
    let catalog = crate::agent::catalog::merged_agent_catalog(&cfg.agents);
    let agent = catalog.get(key).ok_or_else(|| acp_option_error("unknown agent"))?;
    match &agent.runtime {
        AgentRuntime::Acp { backend: selected } if selected == backend => {}
        _ => return Err(acp_option_error("agent runtime does not match selected ACP backend")),
    }
    let skills = crate::skills::load_skills_with_config(cwd, cfg);
    let mcp = crate::tools::mcp_common::load_resolved_acp_config_from(cwd)
        .await
        .map_err(super::to_api_error)?;
    let ceiling =
        crate::agent::native_permission_ceiling(key).await.map_err(super::to_api_error)?;
    let context = DefaultsContext {
        skills: skills.iter().map(|skill| (skill.invocation_name().to_string(), true)).collect(),
        mcp: mcp
            .mcp_servers
            .iter()
            .map(|server| {
                (
                    crate::tools::mcp_common::server_name(server).to_string(),
                    crate::tools::mcp_common::management_for_server(&mcp, server).enabled,
                )
            })
            .collect(),
        permission_ceiling: ceiling,
    };
    let effective = resolve(
        agent,
        &DefaultSelection::default(),
        &DefaultSelection {
            skills: Some(agent.allowed_skills.clone()),
            mcp: Some(
                context
                    .mcp
                    .iter()
                    .filter(|(_, enabled)| **enabled)
                    .map(|(id, _)| id.clone())
                    .collect(),
            ),
            ..Default::default()
        },
        &context,
    )
    .map_err(super::to_api_error)?;
    parsed.full_access = match options.get("full_access") {
        Some(value) => {
            value.as_bool().ok_or_else(|| acp_option_error("full_access must be a boolean"))?
        }
        None => agent.defaults.full_access,
    };
    // ACP has no portable per-tool/path permission policy. Only an unconditional deny
    // can be represented without weakening the host policy; reject other fixed policies.
    if let AgentDefault::Fixed(permission) = &agent.defaults.permission {
        if permission != &serde_json::json!({"*":"deny"}) {
            return Err(acp_option_error(
                "ACP does not support this fixed permission policy; only unconditional deny is supported",
            ));
        }
        parsed.permission_mode = Some(PermissionMode::DenyAll);
        parsed.full_access = false;
    }
    // Host tool defaults also include local-only controls (question/doom-loop).
    // Check configured live denials separately: an external ACP process cannot
    // represent selective per-tool/path rules without weakening those boundaries.
    let tenant = crate::session::ui_store::current_tenant_scope();
    let preferences = tokio::task::spawn_blocking(move || {
        crate::session::ui_config::load_app_config_result(&tenant)
    })
    .await
    .map_err(super::to_api_error)?
    .map_err(super::to_api_error)?;
    use crate::permission::next::{self, Action};
    let live_rules = next::merge(&[
        next::from_config(&agent.permission),
        next::from_config(preferences.get("permission").unwrap_or(&Value::Null)),
    ]);
    if live_rules.iter().any(|rule| rule.action == Action::Deny) {
        if live_rules.last().is_some_and(|rule| {
            rule.permission == "*" && rule.pattern == "*" && rule.action == Action::Deny
        }) {
            parsed.permission_mode = Some(PermissionMode::DenyAll);
            parsed.full_access = false;
        } else {
            return Err(acp_option_error("ACP cannot enforce selective host permission denials"));
        }
    }
    if parsed.permission_mode == Some(PermissionMode::DenyAll) && !effective.skills.is_empty() {
        return Err(acp_option_error("default skill is denied by permission policy"));
    }
    parsed.mcp_servers = mcp
        .mcp_servers
        .into_iter()
        .filter(|server| {
            effective.mcp.iter().any(|id| id == crate::tools::mcp_common::server_name(server))
        })
        .collect();
    let mut messages = messages
        .as_array()
        .cloned()
        .ok_or_else(|| acp_option_error("ACP messages must be an array"))?;
    let mut append_system = |content: String| {
        if !messages
            .iter()
            .any(|message| message["role"] == "system" && message["content"] == content)
        {
            messages.push(serde_json::json!({"role":"system","content":content}));
        }
    };
    if let Some(rule) = effective.system_prompt.as_ref().filter(|rule| !rule.trim().is_empty()) {
        append_system(rule.clone());
    }
    parsed.skill_mounts.clear();
    for id in &effective.skills {
        if effective.permission_action("skill", id) == crate::permission::next::Action::Deny {
            return Err(acp_option_error("default skill is denied by permission policy"));
        }
        let skill = skills
            .iter()
            .find(|skill| skill.invocation_name() == *id)
            .ok_or_else(|| acp_option_error("default skill is unavailable"))?;
        let source = skill
            .location
            .as_ref()
            .ok_or_else(|| acp_option_error("default skill has no local source"))?;
        let record = crate::skills::InvokedSkillRecord::new(id, source, None)
            .map_err(super::to_api_error)?;
        if !record.is_authorized_by(skill) {
            return Err(acp_option_error(
                "default skill is not authorized for automatic invocation",
            ));
        }
        let replay = record.replay().map_err(super::to_api_error)?;
        if !replay.allowed_tools.is_empty() || replay.model.is_some() || replay.effort.is_some() {
            return Err(acp_option_error(
                "ACP default skill requires unsupported invocation options",
            ));
        }
        let directory = std::path::Path::new(source)
            .parent()
            .ok_or_else(|| acp_option_error("default skill has no directory"))?;
        parsed.skill_mounts.push(acp_types::SessionSkillMount {
            name: id.clone(),
            path: directory.to_string_lossy().into_owned(),
        });
    }
    Ok(Value::Array(messages))
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

pub(crate) fn parse_acp_options(options: &Value, agent: &str, config: &config::schema::AcpAgentConfig) -> Result<ParsedAcpOptions, Error> {
    llm_runtime::acp::config::parse_acp_options(options, agent, config)
}
