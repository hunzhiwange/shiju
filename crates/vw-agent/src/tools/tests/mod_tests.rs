//! tools 模块注册与序列化回归测试。
//!
//! 覆盖默认工具集合、运行时差异、注册表暴露面和基础 serde 行为，防止工具能力在
//! 不同配置下被意外扩大或遗漏。

use super::*;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

const MCP_TOOLSET_TOOL_ID: &str = "mcp__fixture_server__echo_tool_2fb49d0879816c90";


use tool_assembly::host_migration_35_mcp_fixture::write_mcp_toolset_workspace;

fn mcp_toolset_context(workspace: &Path) -> ToolRuntimeContext {
    ToolRuntimeContext::new("mcp-toolset-test", Some(workspace.to_string_lossy().to_string()))
}


#[test]
fn all_tools_excludes_browser_when_disabled() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_excludes_browser_when_disabled::<crate::tools::context::AgentContextHost>();
}

#[test]
fn all_tools_includes_browser_when_enabled() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_includes_browser_when_enabled::<crate::tools::context::AgentContextHost>();
}


#[test]
fn default_tools_all_have_descriptions() {
    tool_assembly::host_migration_35_factory_acceptance::default_tools_all_have_descriptions::<crate::tools::context::AgentContextHost>();
}

#[test]
fn default_tools_all_have_schemas() {
    tool_assembly::host_migration_35_factory_acceptance::default_tools_all_have_schemas::<crate::tools::context::AgentContextHost>();
}


#[test]
fn registry_specs_include_apply_patch_by_default() {
    let names: Vec<String> = registry::specs(None).into_iter().map(|spec| spec.id).collect();
    assert!(names.iter().any(|name| name == "apply_patch"));
}

#[test]
fn registry_specs_include_apply_patch_for_explicit_models() {
    let names: Vec<String> =
        registry::specs(Some("vibewindow/claude-3-opus")).into_iter().map(|spec| spec.id).collect();
    assert!(names.iter().any(|name| name == "apply_patch"));
}

#[test]
fn tool_spec_generation() {
    tool_assembly::host_migration_35_factory_acceptance::tool_spec_generation::<crate::tools::context::AgentContextHost>();
}


#[test]
fn registry_specs_expose_bash_and_compact_search_surface() {
    let names: Vec<String> = registry::specs(None).into_iter().map(|spec| spec.id).collect();

    assert!(names.iter().any(|name| name == "bash"));
    assert!(!names.iter().any(|name| name == "shell"));
    assert!(!names.iter().any(|name| name == "process"));
    assert!(names.iter().any(|name| name == "glob"));
    assert!(names.iter().any(|name| name == "grep"));
    assert!(names.iter().any(|name| name == "Config"));
    assert!(names.iter().any(|name| name == "Sleep"));
    assert!(!names.iter().any(|name| name == "RemoteTrigger"));
    assert!(!names.iter().any(|name| name == "glob_search"));
    assert!(!names.iter().any(|name| name == "content_search"));
    assert!(!names.iter().any(|name| name == "codesearch"));
}

#[test]
fn mcp_toolset_tools_are_registered_from_config() {
    let tmp = TempDir::new().unwrap();
    let workspace = write_mcp_toolset_workspace(tmp.path());
    let specs = tool_specs_for_context(&mcp_toolset_context(&workspace));

    let spec = specs
        .iter()
        .find(|spec| spec.id == MCP_TOOLSET_TOOL_ID)
        .expect("configured MCP tool should be registered");

    assert_eq!(spec.display_name, "Fixture Server: Echo Tool");
    assert_eq!(spec.input_schema["required"], json!(["message"]));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
}

#[test]
fn mcp_toolset_tools_can_be_executed_from_registry() {
    let tmp = TempDir::new().unwrap();
    let workspace = write_mcp_toolset_workspace(tmp.path());
    let result = execute_tool_call(
        MCP_TOOLSET_TOOL_ID,
        r#"{"message":"hello"}"#,
        &mcp_toolset_context(&workspace),
    )
    .expect("configured MCP tool should execute through the registry");

    assert!(result.is_success());
    assert_eq!(result.model_text(), "echo:hello");
    assert_eq!(result.data["server"], json!("Fixture Server"));
    assert_eq!(result.data["tool"], json!("Echo Tool"));
}




#[test]
fn all_tools_includes_agent_tool_when_agents_configured() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_includes_agent_tool_when_agents_configured::<crate::tools::context::AgentContextHost>();
}

#[test]
fn all_tools_includes_default_agent_tool_when_no_agents() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_includes_default_agent_tool_when_no_agents::<crate::tools::context::AgentContextHost>();
}

#[test]
fn all_tools_includes_default_agent_tool_when_only_primary_agents() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_includes_default_agent_tool_when_only_primary_agents::<crate::tools::context::AgentContextHost>();
}

#[test]
fn all_tools_registers_agent_tool_for_delegate_agents() {
    tool_assembly::host_migration_35_factory_acceptance::all_tools_registers_agent_tool_for_delegate_agents::<crate::tools::context::AgentContextHost>();
}
