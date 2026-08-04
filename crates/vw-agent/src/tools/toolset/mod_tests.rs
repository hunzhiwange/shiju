use super::*;
use crate::app::agent::config::{
    BrowserConfig, Config, DelegateAgentConfig, HttpRequestConfig, MemoryConfig, WasmRuntimeConfig,
    WebFetchConfig,
};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::runtime::{NativeRuntime, RuntimeAdapter, WasmRuntime};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::context::ToolUseContext;
use crate::app::agent::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MCP_FIXTURE_TOOL_ID: &str = "mcp__fixture_server__echo_tool";

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

fn test_config(workspace: &Path) -> Config {
    Config {
        workspace_dir: workspace.to_path_buf(),
        config_path: workspace.join(".vibewindow.json"),
        ..Config::default()
    }
}

fn agent_config(mode: &str, enabled: bool, allowed_skills: Vec<String>) -> DelegateAgentConfig {
    DelegateAgentConfig {
        label: None,
        description: None,
        builtin: false,
        mode: mode.to_string(),
        enabled,
        allow_as_subagent: true,
        max_parallel_subagents: 4,
        provider: "ollama".to_string(),
        model: "llama3".to_string(),
        system_prompt: None,
        api_key: None,
        temperature: None,
        top_p: None,
        identity_format: None,
        hidden: false,
        max_depth: 3,
        agentic: false,
        allowed_tools: Vec::new(),
        allowed_skills,
        enabled_identity_files: Vec::new(),
        options: HashMap::new(),
        permission: serde_json::Value::Null,
        max_iterations: 10,
        steps: None,
    }
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

    let mut root_config = test_config(&workspace);
    root_config.skills.open_skills_enabled = false;
    let agents = HashMap::from([(
        "worker".to_string(),
        agent_config("all", true, vec!["explicit-skill".to_string()]),
    )]);

    let effective =
        enabled_delegate_agents_with_project_skills(&agents, &workspace, &root_config);

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
    root_config.skills.open_skills_enabled = false;
    let agents = HashMap::from([(
        "worker".to_string(),
        agent_config("all", true, vec!["explicit-skill".to_string()]),
    )]);

    let effective =
        enabled_delegate_agents_with_project_skills(&agents, &workspace, &root_config);

    assert_eq!(effective["worker"].allowed_skills, vec!["explicit-skill".to_string()]);
}

#[test]
fn configured_subagent_opt_out_overrides_builtin_default() {
    let mut agents = HashMap::new();

    let mut blocked = agent_config("subagent", true, Vec::new());
    blocked.allow_as_subagent = false;
    agents.insert("researcher".to_string(), blocked);

    let mut disabled = agent_config("subagent", false, Vec::new());
    disabled.allow_as_subagent = true;
    agents.insert("coder".to_string(), disabled);

    let eligible = enabled_delegate_agents(&agents);

    assert!(eligible.contains_key("main"));
    assert!(!eligible.contains_key("researcher"));
    assert!(!eligible.contains_key("coder"));
}

#[test]
fn agent_tool_parallel_limit_uses_current_agent_and_clamps_bounds() {
    let mut agents = HashMap::new();
    let mut main = agent_config("primary", true, Vec::new());
    main.max_parallel_subagents = 3;
    agents.insert("main".to_string(), main);

    let mut worker = agent_config("subagent", true, Vec::new());
    worker.max_parallel_subagents = 99;
    agents.insert("worker".to_string(), worker);

    assert_eq!(agent_tool_parallel_limit(&agents, "main"), 3);
    assert_eq!(agent_tool_parallel_limit(&agents, "worker"), 32);
    assert_eq!(agent_tool_parallel_limit(&agents, "missing"), 4);
}

fn tool_names(tools: &[Box<dyn Tool>]) -> Vec<&str> {
    tools.iter().map(|tool| tool.name()).collect()
}

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
    let security = Arc::new(SecurityPolicy::default());
    all_tools_with_runtime(
        Arc::new(config),
        &security,
        runtime,
        memory,
        None,
        None,
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
fn classify_message_and_binary_detection_cover_errors_and_content() {
    assert!(matches!(classify_message("blocked".to_string()), ToolCallError::Denied { .. }));
    assert!(matches!(
        classify_message("not allowed here".to_string()),
        ToolCallError::Denied { .. }
    ));
    assert!(matches!(classify_message("forbidden path".to_string()), ToolCallError::Denied { .. }));
    assert!(matches!(
        classify_message("permission denied".to_string()),
        ToolCallError::Denied { .. }
    ));
    assert!(matches!(classify_message("failed".to_string()), ToolCallError::Failed(_)));

    let dir = tempfile::tempdir().unwrap();
    let text = dir.path().join("a.txt");
    let bin = dir.path().join("a.bin");
    let nul = dir.path().join("a.data");
    std::fs::write(&text, b"hello\n").unwrap();
    std::fs::write(&bin, b"text").unwrap();
    std::fs::write(&nul, b"a\0b").unwrap();
    assert!(!is_binary(&text));
    assert!(is_binary(&bin));
    assert!(is_binary(&nul));
    assert!(!is_binary(&dir.path().join("missing")));
}

#[tokio::test]
async fn block_on_bridges_when_called_inside_tokio_runtime() {
    let value = block_on(async { 42 });
    assert_eq!(value, 42);
}

#[tokio::test]
async fn arc_delegating_tool_forwards_methods() {
    let boxed = ArcDelegatingTool::boxed(Arc::new(EchoTool));
    assert_eq!(boxed.name(), "echo");
    assert_eq!(boxed.parameters_schema(), json!({"type": "object"}));
    assert_eq!(boxed.validate_input(json!({"x": 1})).unwrap(), json!({"x": 1}));
    assert!(boxed.execute(json!({"x": 1})).await.unwrap().success);
}

#[tokio::test]
async fn arc_delegating_tool_forwards_v2_contract_methods() {
    let boxed = ArcDelegatingTool::boxed(Arc::new(EchoTool));
    let input = json!({"x": 1});

    assert_eq!(boxed.description(), "echo tool");
    boxed.check_permissions(&input).await.unwrap();
    let called = boxed.call(input.clone()).await.unwrap();
    assert!(called.is_success());
    assert_eq!(boxed.map_result_for_model(&called), called.default_model_result());
    assert_eq!(boxed.render_hint(&called).unwrap().title.as_deref(), Some("echo"));
    assert!(!boxed.is_concurrency_safe());
    assert!(!boxed.is_read_only());
    assert_eq!(boxed.to_audit_input(&input), input);
    assert_eq!(boxed.spec().id, "echo");
}

#[test]
fn default_tools_include_native_capability_tools() {
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(NativeRuntime::new());
    let tools = default_tools_with_runtime(Arc::new(SecurityPolicy::default()), runtime);
    let names = tools.iter().map(|tool| tool.name()).collect::<Vec<_>>();

    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"batch"));
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
    let tools = default_tools(Arc::new(SecurityPolicy::default()));
    let names = tool_names(&tools);

    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"batch"));
}

#[test]
fn default_tools_with_runtime_includes_wasm_module_for_wasm_runtime() {
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(WasmRuntime::new(WasmRuntimeConfig::default()));
    let tools = default_tools_with_runtime(Arc::new(SecurityPolicy::default()), runtime);
    let names = tool_names(&tools);

    assert!(names.contains(&"wasm_module"));
    assert!(names.contains(&"batch"));
}

#[test]
fn default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let runtime: Arc<dyn RuntimeAdapter> =
        Arc::new(CapabilityRuntime::new(false, false, dir.path().join("storage")));
    let tools = default_tools_with_runtime(Arc::new(SecurityPolicy::default()), runtime);

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

    assert!(!names.contains(&"browser_open"));
    assert!(names.contains(&"browser"));
    assert!(names.contains(&"http_request"));
    assert!(names.contains(&"web_fetch"));
    assert!(names.contains(&"web_search_tool"));
    assert!(names.contains(&"batch"));
}

#[test]
fn all_tools_wrapper_uses_native_runtime_and_browser_open_default_mode() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let root_config = test_config(&workspace);
    let browser = BrowserConfig {
        enabled: true,
        allowed_domains: vec!["example.com".into()],
        ..BrowserConfig::default()
    };
    let security = Arc::new(SecurityPolicy::default());

    let tools = all_tools(
        Arc::new(root_config.clone()),
        &security,
        memory,
        Some(""),
        None,
        &browser,
        &HttpRequestConfig::default(),
        &WebFetchConfig::default(),
        &workspace,
        &HashMap::new(),
        None,
        &root_config,
        Some("  "),
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"browser_open"));
    assert!(names.contains(&"browser"));
    assert!(names.contains(&"Brief"));
    assert!(names.contains(&"batch"));
}

#[test]
fn all_tools_with_runtime_registers_composio_brave_search_and_ipc_tools() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.web_search.enabled = true;
    root_config.web_search.provider = "brave".to_string();
    root_config.web_search.brave_api_key = Some("brave-key".to_string());
    root_config.agents_ipc.enabled = true;
    root_config.agents_ipc.db_path = workspace.join("agents.sqlite").to_string_lossy().to_string();

    let security = Arc::new(SecurityPolicy::default());
    let tools = all_tools_with_runtime(
        Arc::new(root_config.clone()),
        &security,
        Arc::new(NativeRuntime::new()),
        memory,
        Some("composio-key"),
        Some("entity-id"),
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

    assert!(names.contains(&"composio"));
    assert!(names.contains(&"web_search_tool"));
    assert!(names.contains(&"agents_list"));
    assert!(names.contains(&"agents_send"));
    assert!(names.contains(&"agents_inbox"));
    assert!(names.contains(&"state_get"));
    assert!(names.contains(&"state_set"));
}

#[test]
fn all_tools_with_runtime_filters_delegate_agents_and_disables_coordination() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.coordination.enabled = false;

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
    assert!(!names.contains(&"delegate_coordination_status"));
}

#[test]
fn all_tools_with_runtime_registers_delegate_coordination_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.coordination.enabled = true;
    root_config.coordination.lead_agent = " ".to_string();
    root_config.agent.compact_context = true;

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
    assert!(names.contains(&"delegate_coordination_status"));
}

#[test]
fn all_tools_with_runtime_continues_when_agents_ipc_database_cannot_open() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let mut root_config = test_config(&workspace);
    root_config.agents_ipc.enabled = true;
    root_config.agents_ipc.db_path = workspace.to_string_lossy().to_string();

    let tools = build_all_tools(
        root_config.clone(),
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

    assert!(names.contains(&"McpStatus"));
    assert!(!names.contains(&"agents_list"));
}

#[test]
fn tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("mcp-specs", Some(workspace.to_string_lossy().to_string()));
    let specs = tool_specs_for_context(&ctx);

    let spec = specs.iter().find(|spec| spec.id == MCP_FIXTURE_TOOL_ID).unwrap();
    assert_eq!(spec.display_name, "Fixture Server: Echo Tool");
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
}

#[test]
fn execution_environment_registers_agent_tool_from_loaded_config() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let mut config = test_config(workspace);
    config.agents.insert("worker".to_string(), agent_config("subagent", true, Vec::new()));
    let ctx =
        ToolRuntimeContext::new("parent-session", Some(workspace.to_string_lossy().to_string()));

    let (tools, _) = execution_environment_for_config(&ctx, Arc::new(config));

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

    let (tools, _) = execution_environment_for_config(&ctx, Arc::new(config));

    assert!(tool_names(&tools).contains(&"AgentTool"));
}

#[tokio::test]
async fn workflow_and_goal_sessions_inherit_recursive_agent_tool_controls() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path();
    let config = Arc::new(test_config(workspace));
    let delegate_agents = enabled_delegate_agents(&config.agents);

    assert!(!delegate_agents.is_empty());
    assert!(
        delegate_agents
            .values()
            .all(|agent| agent.enabled && agent.allow_as_subagent && agent.max_depth > 0)
    );
    assert_eq!(agent_tool_parallel_limit(&config.agents, "main"), 4);

    for session_id in ["workflow-subtask-session", "goal-continuation-session"] {
        let ctx =
            ToolRuntimeContext::new(session_id, Some(workspace.to_string_lossy().to_string()));
        let (tools, tool_use_context) =
            execution_environment_for_config(&ctx, Arc::clone(&config));
        let agent_tool =
            tools.iter().find(|tool| tool.name() == "AgentTool").expect("AgentTool registered");
        let actions = agent_tool.parameters_schema()["properties"]["action"]["enum"]
            .as_array()
            .expect("AgentTool action enum")
            .clone();
        for action in ["launch", "run", "status", "wait", "stop"] {
            assert!(actions.iter().any(|value| value == action), "missing {action}");
        }

        let status = execute_tool_from_registry(
            &tools,
            "AgentTool",
            json!({"action": "status"}),
            tool_use_context,
        )
        .await
        .expect("session-bound AgentTool status");
        assert!(status.result.is_success());
        assert!(status.result.model_text().contains(r#""action":"status""#));
    }
}

#[test]
fn execute_tool_call_runs_dynamic_mcp_tool_through_context_registry() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = write_mcp_fixture_workspace(dir.path());
    let ctx = ToolRuntimeContext::new("mcp-execute", Some(workspace.to_string_lossy().to_string()));

    let result = execute_tool_call(MCP_FIXTURE_TOOL_ID, r#"{"message":"hello"}"#, &ctx).unwrap();

    assert!(result.is_success());
    assert_eq!(result.model_text(), "echo:hello");
    assert_eq!(result.data["server"], json!("Fixture Server"));
}

#[tokio::test]
async fn execute_tool_from_registry_uses_supplied_tool_list() {
    let tools = vec![ArcDelegatingTool::boxed(Arc::new(EchoTool))];
    let context = Arc::new(ToolUseContext::new("registry", None));

    let executed = execute_tool_from_registry(&tools, "echo", json!({"message": "hi"}), context)
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
    register_dynamic_mcp_tools(&mut tools, &workspace);

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "echo");
}
