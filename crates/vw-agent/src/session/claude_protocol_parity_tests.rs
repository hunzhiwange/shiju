//! 验证 Claude 协议入口在代理、技能、压缩与委派策略上的一致性。

use crate::app::agent::config::{DelegateAgentConfig, DelegateCapabilityPolicy};
use crate::app::agent::skills::{InvokedSkillRecord, Skill, SkillListingSessionState};
use crate::app::agent::tools::{Tool, ToolResult};
use crate::channels::prompt::build_entrypoint_protocol_messages;
use crate::session::capability_policy::resolve_skill_capabilities;
use crate::session::child_agent::stream_request;
use crate::session::instruction_discovery::{self, DiscoveryRoots, SourceKind};
use crate::session::processor::llm_messages::assemble_session_protocol_messages;
use async_trait::async_trait;
use serde_json::json;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{InstructionSource, Resolved, instruction_rules};

fn write(path: impl AsRef<Path>, content: &str) -> PathBuf {
    let path = path.as_ref().to_path_buf();
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn resolved(path: &Path, content: String, conditions: Vec<String>) -> Resolved {
    let filepath = path.to_string_lossy().to_string();
    Resolved {
        content: format!("Instructions from: {filepath}\n{content}"),
        filepath: filepath.clone(),
        sources: vec![InstructionSource { filepath, parent: None, conditions }],
        diagnostics: Vec::new(),
    }
}


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
        let legacy = crate::app::agent::tools::delegated_tools::build_agentic_tools(
            &parent_tools,
            &config,
            &trusted_skills,
        );
        let legacy_names = legacy.iter().map(|tool| tool.name()).collect::<Vec<_>>();
        assert_eq!(legacy_names, vec!["file_read", "skill"]);

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
