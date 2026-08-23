//! 验证委派工具筛选、技能能力约束与调用转发行为。
//! 递归批处理始终禁用；技能工具按已解析能力裁剪并保持稳定名称、参数与执行转发。

use super::*;
use crate::app::agent::config::{DelegateAgentConfig, DelegateCapabilityPolicy};

fn delegated_config(
    allowed_tools: &[&str],
    skill_policy: DelegateCapabilityPolicy,
    allowed_skills: &[&str],
) -> DelegateAgentConfig {
    DelegateAgentConfig {
        allowed_tools: allowed_tools.iter().map(ToString::to_string).collect(),
        skill_policy,
        allowed_skills: allowed_skills.iter().map(ToString::to_string).collect(),
        ..DelegateAgentConfig::default()
    }
}

struct NamedTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.name.to_string(), error: None })
    }
}

#[test]
// 批处理工具即使被显式选择也保持禁用，防止委派代理递归扩散任务。
fn build_agentic_tools_blocks_batch_even_when_selected() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "batch" })];
    let config = delegated_config(
        &["batch"],
        DelegateCapabilityPolicy::Unrestricted,
        &[],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert!(tools.is_empty());
}

#[test]
fn build_agentic_tools_empty_selection_keeps_builtin_and_blocks_recursive_tool() {
    let parent_tools: Vec<Arc<dyn Tool>> =
        vec![Arc::new(NamedTool { name: "file_read" }), Arc::new(NamedTool { name: "AgentTool" })];

    let tools = build_agentic_tools(&parent_tools, &DelegateAgentConfig::default(), &[]);

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "file_read");
}

#[test]
fn build_agentic_tools_matches_tool_aliases() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "file_write" })];
    let config = delegated_config(
        &["write"],
        DelegateCapabilityPolicy::Unrestricted,
        &[],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "file_write");
}

#[tokio::test]
async fn delegated_skill_tool_enforces_resolved_skill_capabilities() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "skill" })];
    let config = delegated_config(
        &["skill"],
        DelegateCapabilityPolicy::Strict,
        &["translate"],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert_eq!(tools.len(), 1);

    let denied = tools[0].execute(json!({ "name": "other" })).await.unwrap();
    assert!(!denied.success);
    assert!(denied.error.as_deref().unwrap_or("").contains("not allowed"));

    let allowed = tools[0].execute(json!({ "name": "translate" })).await.unwrap();
    assert!(allowed.success);
}

#[test]
fn strict_skill_policy_without_defaults_does_not_expose_skill() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "skill" })];
    let config = delegated_config(
        &["skill"],
        DelegateCapabilityPolicy::Strict,
        &[],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert!(tools.is_empty());
}

#[tokio::test]
async fn delegated_tool_ref_forwards_contract_and_execution() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "file_write" })];
    let config = delegated_config(
        &[" ", " write "],
        DelegateCapabilityPolicy::Unrestricted,
        &[],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "file_write");
    assert_eq!(tools[0].description(), "test tool");
    assert_eq!(tools[0].parameters_schema()["properties"]["name"]["type"], json!("string"));

    let result = tools[0].execute(json!({"ignored": true})).await.unwrap();
    assert!(result.success);
    assert_eq!(result.output, "file_write");
}

#[test]
fn filtered_skill_tool_exposes_sorted_trimmed_skill_contract() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "skill" })];
    let config = delegated_config(
        &["skill"],
        DelegateCapabilityPolicy::Strict,
        &[" zeta ", "alpha", "alpha"],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "skill");
    assert_eq!(
        tools[0].description(),
        "Load a skill allowed for the delegated agent. Available skills: alpha, zeta"
    );
    assert_eq!(
        tools[0].parameters_schema()["properties"]["name"]["enum"],
        json!(["alpha", "zeta"])
    );
}

#[tokio::test]
async fn filtered_skill_tool_delegates_missing_and_blank_names() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![Arc::new(NamedTool { name: "skill" })];
    let config = delegated_config(
        &["skill"],
        DelegateCapabilityPolicy::Strict,
        &["translate"],
    );
    let tools = build_agentic_tools(&parent_tools, &config, &[]);

    let missing = tools[0].execute(json!({})).await.unwrap();
    let blank = tools[0].execute(json!({"name": " \n "})).await.unwrap();

    assert!(missing.success);
    assert_eq!(missing.output, "skill");
    assert!(blank.success);
    assert_eq!(blank.output, "skill");
}
