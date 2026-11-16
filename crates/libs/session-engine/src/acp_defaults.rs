//! Trusted ACP defaults, capability restrictions and automatic skill admission.
use agent_config as config;
use serde_json::Value;
use acp_types::PermissionMode;
use llm_runtime::Error;

pub trait DefaultsHost {
    fn skills(cwd: &std::path::Path, cfg: &config::schema::Config) -> Vec<agent_skills::Skill>;
    fn mcp(cwd: &std::path::Path) -> impl std::future::Future<Output = anyhow::Result<acp_config::ResolvedAcpxConfig>> + Send;
    fn permission_ceiling(key: &str) -> impl std::future::Future<Output = Result<permission_runtime::next::Ruleset, String>> + Send;
    fn preferences() -> impl std::future::Future<Output = anyhow::Result<Value>> + Send;
}

/// Resolve trusted agent defaults at the gateway; raw request options never supply MCP credentials.
pub async fn apply_agent_defaults<H: DefaultsHost, S>(
    cfg: &config::schema::Config,
    options: &Value,
    backend: &str,
    cwd: &std::path::Path,
    parsed: &mut llm_runtime::acp::options::ParsedAcpOptions<S>,
    messages: &Value,
) -> Result<Value, Error> {
    use agent_defaults::{DefaultSelection, DefaultsContext, resolve};
    use config_agent_profile::{AgentDefault, AgentRuntime};
    let Some(key) = options.get("agent").and_then(Value::as_str) else {
        return Ok(messages.clone());
    };
    let catalog = agent_config::catalog::merged_agent_catalog(&cfg.agents);
    let agent = catalog.get(key).ok_or_else(|| acp_option_error("unknown agent"))?;
    match &agent.runtime {
        AgentRuntime::Acp { backend: selected } if selected == backend => {}
        _ => return Err(acp_option_error("agent runtime does not match selected ACP backend")),
    }
    let skills = H::skills(cwd, cfg);
    let mcp = H::mcp(cwd)
        .await
        .map_err(to_api_error)?;
    let ceiling =
        H::permission_ceiling(key).await.map_err(to_api_error)?;
    let context = DefaultsContext {
        skills: skills.iter().map(|skill| (skill.invocation_name().to_string(), true)).collect(),
        mcp: mcp
            .mcp_servers
            .iter()
            .map(|server| {
                (
                    mcp_client::server_name(server).to_string(),
                    mcp_client::management_for_server(&mcp, server).enabled,
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
    .map_err(to_api_error)?;
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
    let preferences = H::preferences().await.map_err(to_api_error)?;
    use permission_runtime::next::{self, Action};
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
            effective.mcp.iter().any(|id| id == mcp_client::server_name(server))
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
        if effective.permission_action("skill", id) == permission_runtime::next::Action::Deny {
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
        let record = agent_skills::invoked::InvokedSkillRecord::new(id, source, None)
            .map_err(to_api_error)?;
        if !record.is_authorized_by(skill) {
            return Err(acp_option_error(
                "default skill is not authorized for automatic invocation",
            ));
        }
        let replay = record.replay().map_err(to_api_error)?;
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

fn acp_option_error(message: impl Into<String>) -> Error {
    Error::Api(message_types::AssistantError::Unknown { message: format!("acp option error: {}", message.into()) })
}
fn to_api_error(error: impl std::fmt::Display) -> Error {
    Error::Api(message_types::AssistantError::Unknown { message: format!("acp: {error}") })
}
