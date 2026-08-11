use super::ModelRoutingConfigTool;
use super::{DEFAULT_AGENT_MAX_DEPTH, DEFAULT_AGENT_MAX_ITERATIONS};
use crate::app::agent::config::{self, DelegateAgentConfig};
use crate::app::agent::util::MaybeSet;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::super::traits::ToolResult;

impl ModelRoutingConfigTool {
    /// 处理创建或更新委托代理请求。
    pub(super) async fn handle_upsert_agent(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let name = Self::parse_non_empty_string(args, "name")?;
        let provider = Self::parse_non_empty_string(args, "provider")?;
        let model = Self::parse_non_empty_string(args, "model")?;

        let system_prompt_update = Self::parse_optional_string_update(args, "system_prompt")?;
        let api_key_update = Self::parse_optional_string_update(args, "api_key")?;
        let temperature_update = Self::parse_optional_f64_update(args, "temperature")?;
        let max_depth_update = Self::parse_optional_u32_update(args, "max_depth")?;
        let max_iterations_update = Self::parse_optional_usize_update(args, "max_iterations")?;
        let agentic_update = Self::parse_optional_bool(args, "agentic")?;

        let allowed_tools_update = if let Some(raw) = args.get("allowed_tools") {
            Some(Self::parse_string_list(raw, "allowed_tools")?)
        } else {
            None
        };
        let allowed_skills_update = if let Some(raw) = args.get("allowed_skills") {
            Some(Self::parse_string_list(raw, "allowed_skills")?)
        } else {
            None
        };

        let mut cfg = config::get_project_blocking(&self.config.workspace_dir)?;
        let before = cfg.clone();

        let mut next_agent = cfg.agents.get(&name).cloned().unwrap_or(DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            allow_as_subagent: true,
            max_parallel_subagents: 4,
            provider: provider.clone(),
            model: model.clone(),
            force_model_selection: false,
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: DEFAULT_AGENT_MAX_DEPTH,
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: serde_json::Value::Null,
            max_iterations: DEFAULT_AGENT_MAX_ITERATIONS,
            steps: None,
        });

        next_agent.provider = provider;
        next_agent.model = model;

        match system_prompt_update {
            MaybeSet::Set(value) => next_agent.system_prompt = Some(value),
            MaybeSet::Null => next_agent.system_prompt = None,
            MaybeSet::Unset => {}
        }

        match api_key_update {
            MaybeSet::Set(value) => next_agent.api_key = Some(value),
            MaybeSet::Null => next_agent.api_key = None,
            MaybeSet::Unset => {}
        }

        match temperature_update {
            MaybeSet::Set(value) => {
                if !(0.0..=2.0).contains(&value) {
                    anyhow::bail!("'temperature' must be between 0.0 and 2.0");
                }
                next_agent.temperature = Some(value);
            }
            MaybeSet::Null => next_agent.temperature = None,
            MaybeSet::Unset => {}
        }

        match max_depth_update {
            MaybeSet::Set(value) => next_agent.max_depth = value,
            MaybeSet::Null => next_agent.max_depth = DEFAULT_AGENT_MAX_DEPTH,
            MaybeSet::Unset => {}
        }

        match max_iterations_update {
            MaybeSet::Set(value) => next_agent.max_iterations = value,
            MaybeSet::Null => next_agent.max_iterations = DEFAULT_AGENT_MAX_ITERATIONS,
            MaybeSet::Unset => {}
        }

        if let Some(agentic) = agentic_update {
            next_agent.agentic = agentic;
        }

        if let Some(allowed_tools) = allowed_tools_update {
            next_agent.allowed_tools = allowed_tools;
        }
        if let Some(allowed_skills) = allowed_skills_update {
            next_agent.allowed_skills = allowed_skills;
        }

        if next_agent.max_depth == 0 {
            anyhow::bail!("'max_depth' must be greater than 0");
        }

        if next_agent.max_iterations == 0 {
            anyhow::bail!("'max_iterations' must be greater than 0");
        }

        if next_agent.agentic && next_agent.allowed_tools.is_empty() {
            anyhow::bail!(
                "Agent '{name}' has agentic=true but allowed_tools is empty. Set allowed_tools or disable agentic mode."
            );
        }

        cfg.agents.insert(name.clone(), next_agent);
        cfg =
            config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Delegate agent upserted",
                "name": name,
                "config": Self::snapshot(&cfg),
            }))?,
            error: None,
        })
    }

    /// 处理删除委托代理请求。
    pub(super) async fn handle_remove_agent(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let name = Self::parse_non_empty_string(args, "name")?;

        let mut cfg = config::get_project_blocking(&self.config.workspace_dir)?;
        let before = cfg.clone();

        if cfg.agents.remove(&name).is_none() {
            anyhow::bail!("No delegate agent found with name '{name}'");
        }

        cfg =
            config::update_project_from_configs(&self.config.workspace_dir, &before, &cfg).await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "Delegate agent removed",
                "name": name,
                "config": Self::snapshot(&cfg),
            }))?,
            error: None,
        })
    }
}
#[cfg(test)]
#[path = "agents_tests.rs"]
mod agents_tests;
