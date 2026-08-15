//! tools 模块注册与序列化回归测试。
//!
//! 覆盖默认工具集合、运行时差异、注册表暴露面和基础 serde 行为，防止工具能力在
//! 不同配置下被意外扩大或遗漏。

use super::*;
use crate::app::agent::config::{BrowserConfig, Config, MemoryConfig, WasmRuntimeConfig};
use crate::app::agent::memory;
use crate::app::agent::runtime::WasmRuntime;
use serde_json::json;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MCP_TOOLSET_TOOL_ID: &str = "mcp__fixture_server__echo_tool";

fn test_config(tmp: &TempDir) -> Config {
    Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    }
}

fn write_mcp_toolset_server(tmp: &TempDir) -> PathBuf {
    let script = tmp.path().join("fixture_mcp_server.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message from the MCP fixture.",
    "inputSchema": {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    },
    "annotations": {
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True
    }
}

def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        params = message.get("params") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"},
                "instructions": "Fixture instructions for dynamic MCP tool tests."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        text = "echo:" + str(arguments.get("message", ""))
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": text}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("fixture MCP server script should be written");
    script
}

fn write_mcp_toolset_workspace(tmp: &TempDir) -> PathBuf {
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    let script = write_mcp_toolset_server(tmp);
    let config = json!({
        "mcpServers": [{
            "name": "Fixture Server",
            "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
            "args": [script.to_string_lossy()]
        }]
    });
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&config).expect("MCP config should serialize"),
    )
    .expect("MCP config should be written");
    workspace
}

fn mcp_toolset_context(workspace: &Path) -> ToolRuntimeContext {
    ToolRuntimeContext::new("mcp-toolset-test", Some(workspace.to_string_lossy().to_string()))
}

#[test]
fn default_tools_include_apply_patch_once() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = default_tools(security);
    assert_eq!(tools.iter().filter(|tool| tool.name() == "apply_patch").count(), 1);
}

#[test]
fn default_tools_with_runtime_includes_wasm_module_for_wasm_runtime() {
    let security = Arc::new(SecurityPolicy::default());
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));
    let tools = default_tools_with_runtime(security, runtime);
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"wasm_module"));
}

#[test]
fn default_tools_with_runtime_excludes_shell_and_fs_for_wasm_runtime() {
    let security = Arc::new(SecurityPolicy::default());
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));
    let tools = default_tools_with_runtime(security, runtime);
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(!names.contains(&"shell"));
    assert!(!names.contains(&"file_read"));
    assert!(!names.contains(&"notebook_edit"));
    assert!(!names.contains(&"file_write"));
    assert!(!names.contains(&"file_edit"));
    assert!(!names.contains(&"apply_patch"));
    assert!(!names.contains(&"ls"));
    assert!(!names.contains(&"glob"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"content_search"));
    assert!(!names.contains(&"grep"));
}

#[test]
fn all_tools_excludes_browser_when_disabled() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig {
        enabled: false,
        allowed_domains: vec!["example.com".into()],
        session_name: None,
        ..BrowserConfig::default()
    };
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(!names.contains(&"browser_open"));
    assert!(names.contains(&"schedule"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"Brief"));
    assert!(names.contains(&"Sleep"));
    assert!(names.contains(&"model_routing_config"));
    assert!(names.contains(&"pushover"));
    assert!(names.contains(&"proxy_config"));
    assert!(names.contains(&"SendUserFile"));
}

#[test]
fn all_tools_includes_browser_when_enabled() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig {
        enabled: true,
        allowed_domains: vec!["example.com".into()],
        session_name: None,
        ..BrowserConfig::default()
    };
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"browser_open"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"grep"));
    assert!(!names.contains(&"process"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"content_search"));
    assert!(names.contains(&"model_routing_config"));
    assert!(names.contains(&"pushover"));
    assert!(names.contains(&"proxy_config"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"Brief"));
    assert!(names.contains(&"Sleep"));
    assert!(names.contains(&"SendUserFile"));
}

#[test]
fn all_tools_with_runtime_includes_wasm_module_for_wasm_runtime() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));

    let browser = BrowserConfig::default();
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = all_tools_with_runtime(
        Arc::new(Config::default()),
        &security,
        runtime,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"wasm_module"));
    assert!(!names.contains(&"shell"));
    assert!(!names.contains(&"process"));
    assert!(!names.contains(&"git_operations"));
    assert!(!names.contains(&"file_read"));
    assert!(!names.contains(&"notebook_edit"));
    assert!(!names.contains(&"file_write"));
    assert!(!names.contains(&"file_edit"));
    assert!(!names.contains(&"ls"));
    assert!(!names.contains(&"grep"));
    assert!(!names.contains(&"SendUserFile"));
}

#[test]
fn default_tools_names() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = default_tools(security);
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"shell"));
    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"notebook_edit"));
    assert!(names.contains(&"file_edit"));
    assert!(names.contains(&"file_write"));
    assert!(names.contains(&"apply_patch"));
    assert!(names.contains(&"ls"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"grep"));
    assert!(!names.contains(&"glob_search"));
    assert!(!names.contains(&"content_search"));
}

#[test]
fn default_tools_all_have_descriptions() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = default_tools(security);
    for tool in &tools {
        assert!(!tool.description().is_empty(), "Tool {} has empty description", tool.name());
    }
}

#[test]
fn default_tools_all_have_schemas() {
    let security = Arc::new(SecurityPolicy::default());
    let tools = default_tools(security);
    for tool in &tools {
        let schema = tool.parameters_schema();
        assert!(schema.is_object(), "Tool {} schema is not an object", tool.name());
        assert!(schema["properties"].is_object(), "Tool {} schema has no properties", tool.name());
    }
}

#[test]
fn apply_patch_spec_uses_v2_aliases() {
    let security = Arc::new(SecurityPolicy::default());
    let spec = default_tools(security)
        .into_iter()
        .find(|tool| tool.name() == "apply_patch")
        .expect("apply_patch tool should exist")
        .spec();

    assert_eq!(spec.id, "apply_patch");
    assert!(!spec.aliases.iter().any(|alias| alias == "edit"));
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
    let security = Arc::new(SecurityPolicy::default());
    let tools = default_tools(security);
    for tool in &tools {
        let spec = tool.spec();
        let expected_id = if tool.name() == "shell" { "bash" } else { tool.name() };
        assert_eq!(spec.id, expected_id);
        assert_eq!(spec.description, tool.description());
        assert!(spec.input_schema.is_object());
    }
}

#[test]
fn default_tools_specs_expose_bash_alias() {
    let security = Arc::new(SecurityPolicy::default());
    let spec = default_tools(security)
        .into_iter()
        .find(|tool| tool.name() == "shell")
        .expect("shell tool should exist")
        .spec();

    assert_eq!(spec.id, "bash");
    assert!(spec.aliases.iter().any(|alias| alias == "shell"));
}

#[test]
fn registry_specs_expose_bash_and_compact_search_surface() {
    let names: Vec<String> = registry::specs(None).into_iter().map(|spec| spec.id).collect();

    assert!(names.iter().any(|name| name == "bash"));
    assert!(!names.iter().any(|name| name == "shell"));
    assert!(!names.iter().any(|name| name == "process"));
    assert!(names.iter().any(|name| name == "glob"));
    assert!(names.iter().any(|name| name == "grep"));
    assert!(names.iter().any(|name| name == "tool_search"));
    assert!(names.iter().any(|name| name == "Config"));
    assert!(names.iter().any(|name| name == "Brief"));
    assert!(names.iter().any(|name| name == "Sleep"));
    assert!(names.iter().any(|name| name == "SendUserFile"));
    assert!(!names.iter().any(|name| name == "RemoteTrigger"));
    assert!(!names.iter().any(|name| name == "glob_search"));
    assert!(!names.iter().any(|name| name == "content_search"));
    assert!(!names.iter().any(|name| name == "codesearch"));
}

#[test]
fn mcp_toolset_tools_are_registered_from_config() {
    let tmp = TempDir::new().unwrap();
    let workspace = write_mcp_toolset_workspace(&tmp);
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
    let workspace = write_mcp_toolset_workspace(&tmp);
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
fn tool_result_serde() {
    let result = ToolResult { success: true, output: "hello".into(), error: None };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ToolResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.success);
    assert_eq!(parsed.output, "hello");
    assert!(parsed.error.is_none());
}

#[test]
fn tool_result_with_error_serde() {
    let result = ToolResult { success: false, output: String::new(), error: Some("boom".into()) };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ToolResult = serde_json::from_str(&json).unwrap();
    assert!(!parsed.success);
    assert_eq!(parsed.error.as_deref(), Some("boom"));
}

#[test]
fn tool_spec_serde() {
    let spec = ToolSpec::new("test", "A test tool", serde_json::json!({"type": "object"}));
    let json = serde_json::to_string(&spec).unwrap();
    let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "test");
    assert_eq!(parsed.description, "A test tool");
}

#[test]
fn all_tools_includes_agent_tool_when_agents_configured() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig::default();
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let mut agents = HashMap::new();
    agents.insert(
        "researcher".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            allow_as_subagent: true,
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
            tool_policy: Default::default(),
            allowed_tools: Vec::new(),
            skill_policy: Default::default(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: serde_json::Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &agents,
        Some("delegate-test-credential"),
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"AgentTool"));
    assert!(!names.contains(&"delegate"));
    assert!(names.contains(&"delegate_coordination_status"));
}

#[test]
fn all_tools_excludes_agent_tool_when_no_agents() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig::default();
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &HashMap::new(),
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(!names.contains(&"AgentTool"));
    assert!(!names.contains(&"delegate"));
    assert!(!names.contains(&"delegate_coordination_status"));
}

#[test]
fn all_tools_excludes_agent_tool_when_only_primary_agents() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig::default();
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let cfg = test_config(&tmp);

    let mut agents = HashMap::new();
    agents.insert(
        "main".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "primary".to_string(),
            enabled: true,
            allow_as_subagent: false,
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
            tool_policy: Default::default(),
            allowed_tools: Vec::new(),
            skill_policy: Default::default(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: serde_json::Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &agents,
        None,
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(!names.contains(&"AgentTool"));
    assert!(!names.contains(&"delegate"));
    assert!(!names.contains(&"delegate_coordination_status"));
}

#[test]
fn all_tools_disables_coordination_tool_when_coordination_is_disabled() {
    let tmp = TempDir::new().unwrap();
    let security = Arc::new(SecurityPolicy::default());
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    let mem: Arc<dyn Memory> =
        Arc::from(memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

    let browser = BrowserConfig::default();
    let http = crate::app::agent::config::HttpRequestConfig::default();
    let mut cfg = test_config(&tmp);
    cfg.coordination.enabled = false;

    let mut agents = HashMap::new();
    agents.insert(
        "researcher".to_string(),
        DelegateAgentConfig {
            label: None,
            description: None,
            builtin: false,
            mode: "all".to_string(),
            enabled: true,
            allow_as_subagent: true,
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
            tool_policy: Default::default(),
            allowed_tools: Vec::new(),
            skill_policy: Default::default(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: serde_json::Value::Null,
            max_iterations: 10,
            steps: None,
        },
    );

    let tools = all_tools(
        Arc::new(Config::default()),
        &security,
        mem,
        &browser,
        &http,
        &crate::app::agent::config::WebFetchConfig::default(),
        tmp.path(),
        &agents,
        Some("delegate-test-credential"),
        &cfg,
        None,
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"AgentTool"));
    assert!(!names.contains(&"delegate"));
    assert!(!names.contains(&"delegate_coordination_status"));
}
