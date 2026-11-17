//! Legacy and modern capability policy share the real domain implementations.
use config_agent::{DelegateAgentConfig,DelegateCapabilityPolicy};
use tool_types::{Tool,ToolResult};
use crate::capability_policy::resolve_skill_capabilities;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
struct DelegateParityTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DelegateParityTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "delegate parity fixture"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.name.to_string(), error: None })
    }
}

#[test]
// 委派策略在新旧入口保持一致，兼容层不能绕过能力限制。
fn delegate_capability_policy_is_consistent_across_agenttool_gateway_and_legacy_adapter() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(DelegateParityTool { name: "file_read" }),
        Arc::new(DelegateParityTool { name: "shell" }),
        Arc::new(DelegateParityTool { name: "memory_store" }),
        Arc::new(DelegateParityTool { name: "skill" }),
        Arc::new(DelegateParityTool { name: "AgentTool" }),
    ];
    let defaults = vec!["file_read".to_string(), "skill".to_string()];
    let trusted_skills = vec!["extra".to_string()];

    for (policy, expected_skill_enum) in [
        (DelegateCapabilityPolicy::Strict, Some(json!(["review"]))),
        (DelegateCapabilityPolicy::Unrestricted, None),
    ] {
        let config = DelegateAgentConfig {
            allowed_tools: defaults.clone(),
            skill_policy: policy,
            allowed_skills: vec!["review".to_string()],
            ..DelegateAgentConfig::default()
        };
        let legacy = crate::delegated_tools::build_agentic_tools(
            &parent_tools,
            &config,
            &trusted_skills,
        );
        let legacy_names = legacy.iter().map(|tool| tool.name()).collect::<Vec<_>>();
        // 必需命令能力保留兼容入口；未选择的可选能力与递归委派入口仍不可见。
        assert_eq!(legacy_names, vec!["file_read", "shell", "skill"]);

        let skill = legacy.iter().find(|tool| tool.name() == "skill").expect("skill tool");
        match expected_skill_enum {
            Some(expected) => {
                assert_eq!(skill.parameters_schema()["properties"]["name"]["enum"], expected)
            }
            None => assert!(skill.parameters_schema()["properties"].get("name").is_none()),
        }
        let modern_skills = resolve_skill_capabilities(
            ["review", "extra", "model-generated"],
            ["review"],
            &trusted_skills,
            policy,
            std::iter::empty::<&str>(),
        );
        assert_eq!(
            modern_skills.visible_ids.contains("model-generated"),
            policy == DelegateCapabilityPolicy::Unrestricted,
        );
    }
}
