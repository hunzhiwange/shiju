//! 为委派代理构建受限工具集合。
//!
//! 工具权限由代理配置与受信任技能能力共同收窄，并显式排除再次委派的入口。

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::DelegateAgentConfig;
use crate::session::capability_policy::resolve_skill_capabilities;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;

const BLOCKED_DELEGATE_TOOL_IDS: [&str; 6] =
    ["AgentTool", "batch", "delegate", "subagent_manage", "subagent_spawn", "agent_tool"];

pub(crate) fn build_agentic_tools(
    parent_tools: &[Arc<dyn Tool>],
    agent_config: &DelegateAgentConfig,
    trusted_user_selected_skills: &[String],
) -> Vec<Box<dyn Tool>> {
    let allowed_tool_ids = agent_config
        .allowed_tools
        .iter()
        .map(|tool| tool.trim().to_lowercase())
        .filter(|tool| !tool.is_empty())
        .collect::<BTreeSet<_>>();
    let skill_capabilities = resolve_skill_capabilities(
        agent_config.allowed_skills.iter().chain(trusted_user_selected_skills.iter()),
        &agent_config.allowed_skills,
        trusted_user_selected_skills,
        agent_config.skill_policy,
        std::iter::empty::<&str>(),
    );

    parent_tools
        .iter()
        .filter_map(|tool| {
            let spec = tool.spec();
            let tool_id = spec.id.as_str();
            let blocked = std::iter::once(tool_id)
                .chain(spec.aliases.iter().map(String::as_str))
                .any(|id| BLOCKED_DELEGATE_TOOL_IDS.iter().any(|blocked| id.eq_ignore_ascii_case(blocked)));
            let selected = vw_config_types::tools::is_builtin_required_tool_id(tool_id)
                || allowed_tool_ids.contains(&tool_id.to_lowercase())
                || spec.aliases.iter().any(|alias| allowed_tool_ids.contains(&alias.to_lowercase()));
            if blocked || !selected {
                return None;
            }
            if tool_id == "skill" {
                if agent_config.skill_policy
                    == vw_config_types::agent::DelegateCapabilityPolicy::Unrestricted
                {
                    return Some(Box::new(DelegatedToolRef::new(tool.clone())) as Box<dyn Tool>);
                }
                if skill_capabilities.visible_ids.is_empty() {
                    return None;
                }
                return Some(Box::new(CapabilityFilteredSkillTool::new(
                    tool.clone(),
                    skill_capabilities.visible_ids.clone(),
                )) as Box<dyn Tool>);
            }
            Some(Box::new(DelegatedToolRef::new(tool.clone())) as Box<dyn Tool>)
        })
        .collect()
}

struct DelegatedToolRef {
    inner: Arc<dyn Tool>,
}

impl DelegatedToolRef {
    fn new(inner: Arc<dyn Tool>) -> Self {
        Self { inner }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DelegatedToolRef {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }
}

struct CapabilityFilteredSkillTool {
    inner: Arc<dyn Tool>,
    allowed_skills: BTreeSet<String>,
    description: String,
}

impl CapabilityFilteredSkillTool {
    fn new(inner: Arc<dyn Tool>, allowed_skills: BTreeSet<String>) -> Self {
        let allowed = allowed_skills.iter().cloned().collect::<Vec<_>>().join(", ");
        Self {
            inner,
            allowed_skills,
            description: format!("Load a skill allowed for the delegated agent. Available skills: {allowed}"),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CapabilityFilteredSkillTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        let names = self.allowed_skills.iter().cloned().map(Value::String).collect::<Vec<_>>();
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "enum": names
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let requested =
            args.get("name").and_then(Value::as_str).map(str::trim).filter(|name| !name.is_empty());

        let Some(name) = requested else {
            return self.inner.execute(args).await;
        };

        if !self.allowed_skills.contains(name) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Skill '{name}' is not allowed for this delegate agent")),
            });
        }

        self.inner.execute(args).await
    }
}

#[cfg(test)]
#[path = "delegated_tools_tests.rs"]
mod tests;
