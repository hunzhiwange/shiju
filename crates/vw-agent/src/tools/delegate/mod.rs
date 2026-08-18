//! 任务委托工具模块
//!
//! 本模块提供将子任务委托给具有不同提供方/模型配置的命名代理的能力。
//! 支持单次提示模式和带工具调用循环的智能体模式。

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::DelegateAgentConfig;
use crate::app::agent::providers::{Provider, ProviderRuntimeOptions};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

mod agentic;
mod execution;
mod support;

/// 子智能体提供方调用的默认超时时间（秒）
const DELEGATE_TIMEOUT_SECS: u64 = 120;

/// 智能体模式子智能体运行的默认超时时间（秒）
const DELEGATE_AGENTIC_TIMEOUT_SECS: u64 = 300;

/// 任务委托工具
pub struct DelegateTool {
    agents: Arc<HashMap<String, DelegateAgentConfig>>,
    security: Arc<SecurityPolicy>,
    fallback_credential: Option<String>,
    provider_runtime_options: ProviderRuntimeOptions,
    depth: u32,
    parent_tools: Arc<Vec<Arc<dyn Tool>>>,
    multimodal_config: crate::app::agent::config::MultimodalConfig,
    workspace_identity_context: String,
    skill_contexts: HashMap<String, String>,
}

impl DelegateTool {
    pub fn new(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
    ) -> Self {
        Self::new_with_options(
            agents,
            fallback_credential,
            security,
            ProviderRuntimeOptions::default(),
        )
    }

    pub fn new_with_options(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        provider_runtime_options: ProviderRuntimeOptions,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            provider_runtime_options,
            depth: 0,
            parent_tools: Arc::new(Vec::new()),
            multimodal_config: crate::app::agent::config::MultimodalConfig::default(),
            workspace_identity_context: String::new(),
            skill_contexts: HashMap::new(),
        }
    }

    pub fn with_depth(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        depth: u32,
    ) -> Self {
        Self::with_depth_and_options(
            agents,
            fallback_credential,
            security,
            depth,
            ProviderRuntimeOptions::default(),
        )
    }

    pub fn with_depth_and_options(
        agents: HashMap<String, DelegateAgentConfig>,
        fallback_credential: Option<String>,
        security: Arc<SecurityPolicy>,
        depth: u32,
        provider_runtime_options: ProviderRuntimeOptions,
    ) -> Self {
        Self {
            agents: Arc::new(agents),
            security,
            fallback_credential,
            provider_runtime_options,
            depth,
            parent_tools: Arc::new(Vec::new()),
            multimodal_config: crate::app::agent::config::MultimodalConfig::default(),
            workspace_identity_context: String::new(),
            skill_contexts: HashMap::new(),
        }
    }

    pub fn with_parent_tools(mut self, parent_tools: Arc<Vec<Arc<dyn Tool>>>) -> Self {
        self.parent_tools = parent_tools;
        self
    }

    pub fn with_multimodal_config(
        mut self,
        config: crate::app::agent::config::MultimodalConfig,
    ) -> Self {
        self.multimodal_config = config;
        self
    }

    pub fn with_workspace_identity_context(mut self, workspace_identity_context: String) -> Self {
        self.workspace_identity_context = workspace_identity_context;
        self
    }

    pub fn with_skill_contexts(mut self, skill_contexts: HashMap<String, String>) -> Self {
        self.skill_contexts = skill_contexts;
        self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "Delegate a subtask to a specialized agent. Use when a task benefits from a different model, such as fast summarization, deep reasoning, or code generation.\
         The subagent runs a single prompt by default; set agentic=true to enable an iterative loop with filtered tool access."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        let agent_names: Vec<&str> = self.agents.keys().map(|name| name.as_str()).collect();
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "agent": {
                    "type": "string",
                    "minLength": 1,
                    "description": format!(
                        "Name of the agent to delegate to. Available agents: {}",
                        if agent_names.is_empty() {
                            "none configured".to_string()
                        } else {
                            agent_names.join(", ")
                        }
                    )
                },
                "prompt": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Task or prompt to send to the subagent"
                },
                "context": {
                    "type": "string",
                    "description": "Optional prefixed context, such as relevant code or prior findings"
                }
            },
            "required": ["agent", "prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        execution::execute(self, args).await
    }
}

impl DelegateTool {
    fn merged_system_prompt(
        &self,
        agent_name: &str,
        agent_system_prompt: Option<&str>,
    ) -> Option<String> {
        let mut sections = Vec::new();
        if !self.workspace_identity_context.trim().is_empty() {
            sections.push(self.workspace_identity_context.clone());
        }
        if let Some(skill_context) =
            self.skill_contexts.get(agent_name).filter(|context| !context.trim().is_empty())
        {
            sections.push(skill_context.clone());
        }
        if let Some(agent_prompt) =
            agent_system_prompt.map(str::trim).filter(|prompt| !prompt.is_empty())
        {
            sections.push(agent_prompt.to_string());
        }

        (!sections.is_empty()).then(|| sections.join("\n\n"))
    }

    async fn execute_agentic(
        &self,
        agent_name: &str,
        agent_config: &DelegateAgentConfig,
        provider: &dyn Provider,
        system_prompt: Option<&str>,
        full_prompt: &str,
        temperature: f64,
    ) -> anyhow::Result<ToolResult> {
        agentic::execute_agentic(
            self,
            agent_name,
            agent_config,
            provider,
            system_prompt,
            full_prompt,
            temperature,
        )
        .await
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
