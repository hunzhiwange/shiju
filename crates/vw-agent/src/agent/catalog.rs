//! Official presets are derived from the ACP registry, independently of CLI availability.
use std::collections::HashMap;
use vw_config_types::agent::{AgentDefinitionConfig, AgentRuntime, merged_agent_configs};

/// Combine native role limits with editable official ACP presets and custom agents.
/// Missing official keys are regenerated; overrides cannot change their source or backend.
pub fn merged_agent_catalog(
    configured: &HashMap<String, AgentDefinitionConfig>,
) -> HashMap<String, AgentDefinitionConfig> {
    let mut catalog = merged_agent_configs(configured);
    for (backend, spec) in vw_acp::built_in_agent_specs() {
        let key = format!("acp-{backend}");
        let mut entry = configured.get(&key).cloned().unwrap_or_else(|| AgentDefinitionConfig {
            enabled: matches!(
                backend.as_str(),
                "claude" | "gemini" | "opencode" | "codex" | "openclaw" | "copilot"
            ),
            recommended_prompts: vec![format!(
                "请使用 {} 帮我分析当前项目并完成任务。",
                spec.display_name
            )],
            ..AgentDefinitionConfig::default()
        });
        if entry.label.is_none() {
            entry.label = Some(spec.display_name.clone());
        }
        if entry.description.is_none() {
            entry.description = Some(format!("通过 ACP 运行 {}。", spec.display_name));
        }
        entry.builtin = true;
        entry.runtime = AgentRuntime::Acp { backend };
        entry.mode = "primary".into();
        entry.allow_as_subagent = false;
        catalog.insert(key, entry);
    }
    catalog
}
