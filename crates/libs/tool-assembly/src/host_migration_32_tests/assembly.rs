//! 验证工具集按运行时能力、配置、安全上下文及动态 MCP 服务完成注册与执行。
//! 覆盖默认、全量与执行环境构造路径，确保能力缺失时不会注册伪可用工具。

use super::*;
use crate::{runtime::*, factory::*, agents::*, toolset::*, ToolSpec};
use crate::tool_search::TOOL_SEARCH_ID;
use super::host;
use host::Host;
use crate::factory::FactoryHost;
type ToolRuntimeContext = crate::runtime::ToolRuntimeContext<Host>;
use agent_config::runtime_schema::{
    BrowserConfig, Config, DelegateAgentConfig, HttpRequestConfig, MemoryConfig, WebFetchConfig,
};
use agent_memory::{self, Memory};
use agent_runtime::NativeRuntime;
use runtime_types::RuntimeAdapter;
use agent_security::SecurityPolicy;
type ToolUseContext = tool_context::context::ToolUseContext<Host>;
use tool_types::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MCP_FIXTURE_TOOL_ID: &str = "mcp__fixture_server__echo_tool_2fb49d0879816c90";

struct EchoTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "echo tool"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: args.to_string(), error: None })
    }
}

struct CapabilityRuntime {
    shell: bool,
    filesystem: bool,
    storage_path: PathBuf,
}

impl CapabilityRuntime {
    fn new(shell: bool, filesystem: bool, storage_path: PathBuf) -> Self {
        Self { shell, filesystem, storage_path }
    }
}

impl RuntimeAdapter for CapabilityRuntime {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "capability-test"
    }

    fn has_shell_access(&self) -> bool {
        self.shell
    }

    fn has_filesystem_access(&self) -> bool {
        self.filesystem
    }

    fn storage_path(&self) -> PathBuf {
        self.storage_path.clone()
    }

    fn supports_long_running(&self) -> bool {
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        anyhow::bail!("capability-test runtime does not execute shell commands")
    }
}

fn test_memory(workspace: &Path) -> Arc<dyn Memory> {
    let mem_cfg = MemoryConfig { backend: "markdown".into(), ..MemoryConfig::default() };
    Arc::from(memory::create_memory(&mem_cfg, workspace, None).unwrap())
}

// 所有工具集用例从同一最小配置出发，只显式开启当前场景所需能力。
fn test_config(workspace: &Path) -> Config {
    Config {
        workspace_dir: workspace.to_path_buf(),
        config_path: workspace.join(".vibewindow.json"),
        ..Config::default()
    }
}

fn security_for_workspace(workspace: &Path) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy { workspace_dir: workspace.to_path_buf(), ..SecurityPolicy::default() })
}

fn agent_config(mode: &str, enabled: bool, allowed_skills: Vec<String>) -> DelegateAgentConfig {
    DelegateAgentConfig {
        runtime: Default::default(),
        recommended_prompts: Vec::new(),
        defaults: Default::default(),
        label: None,
        description: None,
        builtin: false,
        mode: mode.to_string(),
        enabled,
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
        allowed_tools: Vec::new(),
        skill_policy: Default::default(),
        allowed_skills,
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

fn tool_names(tools: &[Box<dyn Tool>]) -> Vec<&str> {
    tools.iter().map(|tool| tool.name()).collect()
}

// 本地 stdio 服务用于检查动态工具注册和执行，不依赖用户的 MCP 配置。
fn write_mcp_fixture_server(dir: &Path) -> PathBuf {
    let script = dir.join("fixture_mcp_server.py");
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
        "properties": {"message": {"type": "string"}},
        "required": ["message"]
    },
    "annotations": {"readOnlyHint": True, "destructiveHint": False, "idempotentHint": True}
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
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": "echo:" + str(arguments.get("message", ""))}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .unwrap();
    script
}

fn write_mcp_fixture_workspace(dir: &Path) -> PathBuf {
    let workspace = dir.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let script = write_mcp_fixture_server(dir);
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [{
                "name": "Fixture Server",
                "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
                "args": [script.to_string_lossy()]
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    workspace
}

#[allow(clippy::too_many_arguments)]
fn build_all_tools(
    config: Config,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    browser: &BrowserConfig,
    http: &HttpRequestConfig,
    web_fetch: &WebFetchConfig,
    workspace: &Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &Config,
    session_id: Option<&str>,
) -> Vec<Box<dyn Tool>> {
    let security = security_for_workspace(workspace);
    all_tools_with_runtime::<Host>(
        Arc::new(config),
        &security,
        runtime,
        memory,
        browser,
        http,
        web_fetch,
        workspace,
        agents,
        fallback_api_key,
        root_config,
        session_id,
    )
}

#[test]
fn runtime_context_and_scalar_parsing_cover_core_branches() {
    let ctx = ToolRuntimeContext::new("s", Some("/tmp/root".to_string()))
        .with_tool_use_context(ToolUseContext::new("inner", None));
    assert_eq!(ctx.session, "s");
    assert_eq!(ctx.tool_use_context().session(), "inner");
    assert_eq!(ctx.tool_use_context().root(), Some("/tmp/root"));

    assert_eq!(parse_tool_input("", "").unwrap(), json!({}));
    assert_eq!(parse_tool_input("file_read", "a.txt").unwrap(), json!({"path": "a.txt"}));
    assert_eq!(
        parse_tool_input("web_fetch", "https://e.test").unwrap(),
        json!({"url": "https://e.test"})
    );
    assert_eq!(
        parse_tool_input("webfetch", "https://alias.test").unwrap(),
        json!({"url": "https://alias.test"})
    );
    assert!(parse_tool_input("unknown", "plain").is_err());
    assert!(parse_tool_input("unknown", "{bad").unwrap_err().to_string().contains("invalid JSON"));
}



#[test]
fn default_context_constructors_use_current_directory_roots() {
    let default_ctx = ToolRuntimeContext::default();
    assert_eq!(default_ctx.session, "default");
    assert!(default_ctx.root.is_some());

    let spec_ctx = ToolRuntimeContext::for_specs();
    assert_eq!(spec_ctx.session, "specs");
    assert!(spec_ctx.root.is_some());
}

#[test]
fn default_tools_wrapper_uses_native_runtime() {
    let tools = default_tools::<Host>(Arc::new(SecurityPolicy::default()));
    let names = tool_names(&tools);

    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"batch"));
}

#[test]
fn default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let runtime: Arc<dyn RuntimeAdapter> =
        Arc::new(CapabilityRuntime::new(false, false, dir.path().join("storage")));
    let tools = default_tools_with_runtime::<Host>(Arc::new(SecurityPolicy::default()), runtime);

    assert!(tools.is_empty());
}

#[test]
fn all_tools_with_runtime_respects_optional_tool_configurations() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.web_search.enabled = true;
    root_config.web_search.provider = "exa".to_string();
    root_config.web_search.api_key = Some("search-key".to_string());

    let browser = BrowserConfig {
        enabled: true,
        allowed_domains: vec!["example.com".into()],
        browser_open: "disable".into(),
        ..BrowserConfig::default()
    };
    let http = HttpRequestConfig {
        enabled: true,
        allowed_domains: vec!["api.example.com".into()],
        ..HttpRequestConfig::default()
    };
    let web_fetch = WebFetchConfig {
        enabled: true,
        allowed_domains: vec!["docs.example.com".into()],
        ..WebFetchConfig::default()
    };
    let tools = build_all_tools(
        root_config.clone(),
        Arc::new(NativeRuntime::new()),
        memory,
        &browser,
        &http,
        &web_fetch,
        &workspace,
        &HashMap::new(),
        None,
        &root_config,
        Some("explicit-session"),
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"bash_wait"));
    assert!(names.contains(&"bash_cancel"));
    assert!(!names.contains(&"browser_open"));
    assert!(names.contains(&"browser"));
    assert!(names.contains(&"http_request"));
    assert!(names.contains(&"web_fetch"));
    assert!(names.contains(&"web_search_tool"));
    assert!(names.contains(&"batch"));
}

#[test]
fn all_tools_with_runtime_registers_brave_search_without_ipc_tools() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.web_search.enabled = true;
    root_config.web_search.provider = "brave".to_string();
    root_config.web_search.brave_api_key = Some("brave-key".to_string());

    let security = Arc::new(SecurityPolicy::default());
    let tools = all_tools_with_runtime::<Host>(
        Arc::new(root_config.clone()),
        &security,
        Arc::new(NativeRuntime::new()),
        memory,
        &BrowserConfig::default(),
        &HttpRequestConfig::default(),
        &WebFetchConfig::default(),
        &workspace,
        &HashMap::new(),
        None,
        &root_config,
        None,
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"web_search_tool"));
    assert!(!names.contains(&"state_get"));
    assert!(!names.contains(&"state_set"));
}

#[test]
fn all_tools_with_runtime_filters_delegate_agents() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let root_config = test_config(&workspace);

    let mut agents = HashMap::new();
    agents.insert("disabled".to_string(), agent_config("all", false, Vec::new()));
    agents.insert("primary".to_string(), agent_config("primary", true, Vec::new()));
    agents.insert(
        "researcher".to_string(),
        agent_config("all", true, vec!["missing-skill".to_string(), " ".to_string()]),
    );

    let tools = build_all_tools(
        root_config.clone(),
        Arc::new(NativeRuntime::new()),
        memory,
        &BrowserConfig::default(),
        &HttpRequestConfig::default(),
        &WebFetchConfig::default(),
        &workspace,
        &agents,
        Some("  delegate-key  "),
        &root_config,
        None,
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"AgentTool"));
}

#[test]
fn canonical_agent_tool_registers_when_delegate_agent_exists() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    config_agent::ensure_main_agent_config(&mut root_config.agents).compact_context =
        true;

    let mut agents = HashMap::new();
    agents.insert("researcher".to_string(), agent_config("all", true, Vec::new()));

    let tools = build_all_tools(
        root_config.clone(),
        Arc::new(NativeRuntime::new()),
        memory,
        &BrowserConfig::default(),
        &HttpRequestConfig::default(),
        &WebFetchConfig::default(),
        &workspace,
        &agents,
        Some("delegate-key"),
        &root_config,
        None,
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"AgentTool"));
}

#[test]
fn tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("mcp-specs", Some(workspace.to_string_lossy().to_string()));
    let specs = tool_specs_for_context::<Host>(&ctx);

    let spec = specs.iter().find(|spec| spec.id == MCP_FIXTURE_TOOL_ID).unwrap();
    assert_eq!(spec.display_name, "Fixture Server: Echo Tool");
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
}

#[test]
fn codex_discovery_tool_factory_registers_search_for_deferred_tools() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("discovery-specs", Some(workspace.display().to_string()))
        .with_test_config(test_config(&workspace));
    let specs = tool_specs_for_context::<Host>(&ctx);
    let search = specs.iter().find(|spec| spec.id == "tool_search");
    assert!(search.is_some(), "deferred MCP tools require a registered search entry");
    let search = search.unwrap();
    assert!(search.read_only && search.concurrency_safe);
    assert!(search.description.contains("Fixture Server"));
    assert!(!search.description.contains("inputSchema"));
}

#[test]
fn codex_discovery_tool_factory_omits_search_without_deferred_tools() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ToolRuntimeContext::new("discovery-empty", Some(dir.path().display().to_string()))
        .with_test_config(test_config(dir.path()));
    assert!(tool_specs_for_context::<Host>(&ctx).iter().all(|spec| spec.id != "tool_search"));
}

#[test]
fn codex_discovery_tool_factory_searches_authorized_catalog_without_remote_calls() {
    use crate::tool_search::ToolDiscoverySession;

    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let marker = dir.path().join("remote_calls");
    let script = dir.path().join("fixture_mcp_server.py");
    let source = std::fs::read_to_string(&script).unwrap().replace(
        "    elif method == \"tools/call\":\n",
        &format!(
            "    elif method == \"tools/call\":\n        with open({}, 'a') as calls:\n            calls.write('called\\n')\n",
            serde_json::to_string(&marker.display().to_string()).unwrap(),
        ),
    );
    std::fs::write(&script, source).unwrap();
    let ctx = ToolRuntimeContext::new("discovery-call", Some(workspace.display().to_string()))
        .with_test_config(test_config(&workspace));
    let authorized = tool_specs_for_context::<Host>(&ctx)
        .into_iter()
        .filter(|spec| spec.id == MCP_FIXTURE_TOOL_ID || spec.id == TOOL_SEARCH_ID)
        .collect();
    let discovery = Arc::new(ToolDiscoverySession::new(authorized));
    let ctx = ctx.with_tool_use_context(
        ToolUseContext::new("discovery-call", Some(workspace.display().to_string()))
            .with_tool_discovery(discovery.clone()),
    );
    assert!(tool_specs_for_context::<Host>(&ctx).iter().all(|spec| spec.id != MCP_FIXTURE_TOOL_ID));
    let result = execute_tool_call::<Host>(TOOL_SEARCH_ID, r#"{"query":"echo"}"#, &ctx).unwrap();
    let output: Value = serde_json::from_str(&result.model_text()).unwrap();
    assert_eq!(output["tools"][0]["id"], MCP_FIXTURE_TOOL_ID);
    assert_eq!(output["tools"][0]["input_schema"]["required"], json!(["message"]));
    assert!(!marker.exists(), "catalog search must not execute the remote MCP tool");
    assert!(tool_specs_for_context::<Host>(&ctx).iter().any(|spec| spec.id == MCP_FIXTURE_TOOL_ID));

    let result =
        execute_tool_call::<Host>(MCP_FIXTURE_TOOL_ID, r#"{"message":"after-search"}"#, &ctx).unwrap();
    assert!(result.model_text().contains("after-search"));
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "called\n");
}

#[test]
fn codex_discovery_tool_factory_respects_disabled_and_denied_search() {
    use crate::tool_search::ToolDiscoverySession;

    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("discovery-denied", Some(workspace.display().to_string()))
        .with_test_config(test_config(&workspace));
    let mcp = tool_specs_for_context::<Host>(&ctx)
        .into_iter()
        .find(|spec| spec.id == MCP_FIXTURE_TOOL_ID)
        .unwrap();
    for authorized in [Vec::new(), vec![mcp]] {
        let discovery = Arc::new(ToolDiscoverySession::new(authorized));
        let restricted = ctx.clone().with_tool_use_context(
            ToolUseContext::new("discovery-denied", Some(workspace.display().to_string()))
                .with_tool_discovery(discovery.clone()),
        );
        assert!(execute_tool_call::<Host>(TOOL_SEARCH_ID, r#"{"query":"echo"}"#, &restricted).is_err());
        let specs = tool_specs_for_context::<Host>(&restricted);
        assert_eq!(specs.len(), usize::from(discovery.authorizes(MCP_FIXTURE_TOOL_ID)));
        assert!(specs.iter().all(|spec| spec.id == MCP_FIXTURE_TOOL_ID));
        assert!(specs.iter().all(|spec| spec.id != TOOL_SEARCH_ID));
    }
}

#[test]
fn execution_environment_registers_agent_tool_from_loaded_config() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let mut config = test_config(workspace);
    config.agents.insert("worker".to_string(), agent_config("subagent", true, Vec::new()));
    let ctx =
        ToolRuntimeContext::new("parent-session", Some(workspace.to_string_lossy().to_string()));

    let (tools, _) = execution_environment_for_config::<Host>(&ctx, Arc::new(config));

    assert!(tool_names(&tools).contains(&"AgentTool"));
}

#[test]
fn default_config_registers_agent_tool_with_builtin_subagents() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let config = test_config(workspace);
    assert!(config.agents.is_empty());
    let ctx =
        ToolRuntimeContext::new("parent-session", Some(workspace.to_string_lossy().to_string()));

    let (tools, _) = execution_environment_for_config::<Host>(&ctx, Arc::new(config));

    assert!(tool_names(&tools).contains(&"AgentTool"));
}

#[test]
fn default_toolset_registers_thread_tools_without_delegate_agents() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let config = test_config(workspace);
    let ctx =
        ToolRuntimeContext::new("parent-session", Some(workspace.to_string_lossy().to_string()));

    let (tools, _) = execution_environment_for_config::<Host>(&ctx, Arc::new(config));

    assert!(!tool_names(&tools).contains(&"SessionTool"));
    assert!(tool_names(&tools).contains(&"create_thread"));
    for name in ["list_threads", "read_thread", "wait_threads"] {
        assert!(tool_names(&tools).contains(&name), "missing {name}");
    }
}

#[test]
fn execute_tool_call_runs_dynamic_mcp_tool_through_context_registry() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("mcp-execute", Some(workspace.to_string_lossy().to_string()));

    let result = execute_tool_call::<Host>(MCP_FIXTURE_TOOL_ID, r#"{"message":"hello"}"#, &ctx).unwrap();

    assert!(result.is_success());
    assert_eq!(
        result.model_result,
        json!({
            "content":[{"type":"text","text":"echo:hello"}],
            "structuredContent":{"echoed":"hello"}
        })
    );
    assert_eq!(result.data["server"], json!("Fixture Server"));
}

#[tokio::test]
async fn execute_tool_from_registry_uses_supplied_tool_list() {
    let tools = vec![boxed_registry_from_arcs(vec![Arc::new(EchoTool)]).remove(0)];
    let context = Arc::new(ToolUseContext::new("registry", None));

    let executed = execute_tool_from_registry::<Host>(&tools, "echo", json!({"message": "hi"}), context)
        .await
        .unwrap();

    assert!(executed.result.is_success());
    assert_eq!(executed.result.model_text(), "{\"message\":\"hi\"}");
}

#[test]
fn register_dynamic_mcp_tools_skips_disabled_servers_and_keeps_existing_tools() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [{
                "name": "disabled-fixture",
                "command": "missing-fixture-server",
                "enabled": false
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let mut tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];
    host::Host::block_on(crate::mcp::register_dynamic_mcp_tools::<Host>(&mut tools, host::Host::load_runtime_mcp_config(&workspace), std::sync::Arc::new(<Host as tool_mcp::McpToolHost>::new_connections())));

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "echo");
}

#[test]
fn delegate_agents_auto_load_project_skills_by_default_and_preserve_explicit_skills() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".git")).unwrap();
    let skill_dir = workspace.join(".agents/skills/project-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: project-skill\ndescription: Project skill\n---\n",
    )
    .unwrap();

    let root_config = test_config(&workspace);
    let agents = HashMap::from([(
        "worker".to_string(),
        agent_config("all", true, vec!["explicit-skill".to_string()]),
    )]);

    let effective = enabled_delegate_agents_with_project_skills::<Host>(&agents, &workspace, &root_config);

    assert_eq!(
        effective["worker"].allowed_skills,
        vec!["explicit-skill".to_string(), "project-skill".to_string()]
    );
}
#[test]
fn delegate_agents_require_explicit_skills_when_project_auto_load_is_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".git")).unwrap();
    let skill_dir = workspace.join("skills/project-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Project Skill\n").unwrap();

    let mut root_config = test_config(&workspace);
    root_config.skills.auto_load_project_skills = false;
    let agents = HashMap::from([(
        "worker".to_string(),
        agent_config("all", true, vec!["explicit-skill".to_string()]),
    )]);

    let effective = enabled_delegate_agents_with_project_skills::<Host>(&agents, &workspace, &root_config);

    assert_eq!(effective["worker"].allowed_skills, vec!["explicit-skill".to_string()]);
}
#[test]
fn agent_config_cleanup_delegate_fixed_selection_preserves_strict_boundary() {
    use config_agent_profile::AgentDefault;
    use config_agent::DelegateCapabilityPolicy;
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    std::fs::create_dir_all(workspace.join(".git")).unwrap();
    let skill_dir = workspace.join("skills/project-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: project-skill\ndescription: fixture\n---\n",
    )
    .unwrap();
    let root = test_config(workspace);
    for selection in [
        AgentDefault::Inherit,
        AgentDefault::Fixed(vec![]),
        AgentDefault::Fixed(vec!["allowed".into()]),
    ] {
        let mut config = agent_config("all", true, vec!["allowed".into()]);
        config.skill_policy = DelegateCapabilityPolicy::Strict;
        config.defaults.skills = selection.clone();
        let agents = HashMap::from([("worker".into(), config)]);
        let result = enabled_delegate_agents_with_project_skills::<Host>(&agents, workspace, &root);
        assert_eq!(result["worker"].allowed_skills, vec!["allowed"]);
        assert_eq!(result["worker"].defaults.skills, selection);
    }
}