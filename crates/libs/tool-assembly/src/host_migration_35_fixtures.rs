//! Stable agent-definition fixture shared by domain and concrete-host regressions.
use config_agent::DelegateAgentConfig;
use std::collections::HashMap;
pub fn agent(mode: &str, allow_as_subagent: bool) -> DelegateAgentConfig {
DelegateAgentConfig {
            runtime: Default::default(),
            recommended_prompts: Vec::new(),
            defaults: Default::default(),
            label: None,
            description: None,
            builtin: false,
            mode: mode.to_string(),
            enabled: true,
            allow_as_subagent,
            max_parallel_subagents: 4,
            provider: "ollama".to_string(),
            model: "llama3".to_string(),
            force_model_selection: false,
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: 3,
            agentic: false,
            allowed_tools: Vec::new(),
            skill_policy: Default::default(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: serde_json::Value::Null,
            max_iterations: 10,
            compact_context: false,
            max_history_messages: 100,
            parallel_tools: false,
            steps: None,
        }
}
