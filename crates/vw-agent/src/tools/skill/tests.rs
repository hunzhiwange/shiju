//! `SkillTool` 的聚焦单元测试。
//!
//! 这里覆盖参数解析、工具元数据、安全策略拒绝和同步/异步桥接等无需真实技能目录、
//! 用户交互或外部进程的确定性行为。

use super::{Args, SkillTool, block_on};
use crate::app::agent::config;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

fn tool_with_security(security: SecurityPolicy) -> SkillTool {
    SkillTool {
        security: Arc::new(security),
        session_id: "skill-test-session".to_string(),
        force_prompt_on_allow: false,
        workspace_dir: PathBuf::from("unused-skill-test-workspace"),
        root_config: Arc::new(config::Config::default()),
        description_text: Arc::from("cached skill description"),
    }
}

#[test]
fn args_deserialize_name_and_ignore_unknown_fields() {
    let args: Args = serde_json::from_value(json!({
        "name": "rustcodegraph",
        "ignored": true,
    }))
    .expect("valid skill arguments should deserialize");

    assert_eq!(args.name, "rustcodegraph");
}

#[test]
fn args_reject_missing_or_non_string_name() {
    let missing = serde_json::from_value::<Args>(json!({}));
    let non_string = serde_json::from_value::<Args>(json!({ "name": 239 }));

    assert!(missing.is_err());
    assert!(non_string.is_err());
}

#[test]
fn tool_metadata_uses_stable_name_schema_and_cached_description() {
    let tool = tool_with_security(SecurityPolicy::default());

    assert_eq!(tool.name(), "skill");
    assert_eq!(tool.description(), "cached skill description");
    assert_eq!(tool.parameters_schema(), SkillTool::schema());
}

#[tokio::test]
async fn execute_rejects_action_in_read_only_mode_before_parsing_arguments() {
    let security =
        SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() };
    let tool = tool_with_security(security);

    let result = tool.execute(json!({})).await.expect("tool execution should return a result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(
        result.error.as_deref(),
        Some("Security policy: read-only mode, cannot perform 'skill'")
    );
}

#[tokio::test]
async fn execute_rejects_action_when_budget_is_exhausted() {
    let security = SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() };
    let tool = tool_with_security(security);

    let result = tool
        .execute(json!({ "name": "unused" }))
        .await
        .expect("tool execution should return a result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Rate limit exceeded: action budget exhausted"));
}

#[tokio::test]
async fn execute_reports_invalid_parameters_without_loading_skills() {
    let tool = tool_with_security(SecurityPolicy::default());

    let result = tool
        .execute(json!({ "name": false }))
        .await
        .expect("tool execution should return a result");

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(
        result
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("Missing or invalid parameters:"))
    );
}

#[test]
fn block_on_completes_future_without_existing_runtime() {
    assert_eq!(block_on(async { 239_u16 }), 239);
}

#[tokio::test]
async fn block_on_completes_future_inside_existing_runtime() {
    assert_eq!(block_on(async { "bridged" }), "bridged");
}
