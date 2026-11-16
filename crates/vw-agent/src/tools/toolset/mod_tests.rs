//! 验证工具集按运行时能力、配置、安全上下文及动态 MCP 服务完成注册与执行。
//! 覆盖默认、全量与执行环境构造路径，确保能力缺失时不会注册伪可用工具。

use super::*;
use crate::app::agent::config::{
    Config, DelegateAgentConfig, MemoryConfig,
};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::runtime::NativeRuntime;
use runtime_types::RuntimeAdapter;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

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

    let effective = enabled_delegate_agents_with_project_skills(&agents, &workspace, &root_config);

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

    let effective = enabled_delegate_agents_with_project_skills(&agents, &workspace, &root_config);

    assert_eq!(effective["worker"].allowed_skills, vec!["explicit-skill".to_string()]);
}



fn tool_names(tools: &[Box<dyn Tool>]) -> Vec<&str> {
    tools.iter().map(|tool| tool.name()).collect()
}

// 本地 stdio 服务用于检查动态工具注册和执行，不依赖用户的 MCP 配置。


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





// 先固定纯辅助逻辑，再由后续用例组合运行时、MCP 与递归代理上下文。



#[tokio::test]
async fn block_on_bridges_when_called_inside_tokio_runtime() {
    let value = block_on(async { 42 });
    assert_eq!(value, 42);
}













#[test]
fn all_tools_wrapper_registers_default_network_tools() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let memory = test_memory(&workspace);
    let root_config = test_config(&workspace);
    let security = Arc::new(SecurityPolicy::default());

    let tools = all_tools(
        Arc::new(root_config.clone()),
        &security,
        memory,
        &root_config.browser,
        &root_config.http_request,
        &root_config.web_fetch,
        &workspace,
        &HashMap::new(),
        None,
        &root_config,
        Some("  "),
    );
    let names = tool_names(&tools);

    assert!(names.contains(&"browser_open"));
    assert!(names.contains(&"browser"));
    assert!(names.contains(&"http_request"));
    assert!(names.contains(&"web_search_tool"));
    assert!(!names.contains(&"web_fetch"));
    assert!(names.contains(&"batch"));
}























#[test]
fn thread_tools_visible_without_tool_search() {
    let dir = tempfile::tempdir().unwrap();
    let ctx =
        ToolRuntimeContext::new("parent-session", Some(dir.path().to_string_lossy().to_string()));
    let specs = tool_specs_for_context(&ctx);
    let expected = [
        "create_thread",
        "fork_thread",
        "list_threads",
        "read_thread",
        "send_message_to_thread",
        "handoff_thread",
        "set_thread_title",
        "set_thread_pinned",
        "set_thread_archived",
        "wait_threads",
    ];
    for name in expected {
        assert!(specs.iter().any(|spec| spec.id == name), "missing {name}");
    }
    assert!(!specs.iter().any(|spec| spec.id == "SessionTool"));

    let disabled = crate::session::processor::allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"disable_tools": true}),
    );
    assert!(disabled.is_empty());
}

#[test]
fn restricted_toolset_can_exclude_thread_tools() {
    let specs = vec![
        ToolSpec::new("create_thread", "Visible tasks", json!({"type": "object"})),
        ToolSpec::new("file_read", "Read files", json!({"type": "object"})),
    ];

    let allowed = crate::session::processor::allowed_tool_ids_for_request_from_specs(
        &specs,
        &json!({"disable_tools": true, "allowed_tools": ["file_read"]}),
    );

    assert!(!allowed.contains("file_read"));
    assert!(!allowed.contains("create_thread"));
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
        let (tools, tool_use_context) = execution_environment_for_config(&ctx, Arc::clone(&config));
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
fn shell_outer_timeout_yield_does_not_cancel_process() {
    let dir = tempfile::tempdir().unwrap();
    let root = Some(dir.path().to_string_lossy().to_string());
    let context = ToolUseContext::new("outer-yield", root.clone()).with_full_access_enabled(true);
    let ctx = ToolRuntimeContext::new("outer-yield", root).with_tool_use_context(context);
    let started = execute_tool_call(
        "bash",
        &json!({
            "command": "printf yield-ready; exec sleep 60",
            "description": "outer timeout regression", "yield_time_ms": 250,
        })
        .to_string(),
        &ctx,
    )
    .unwrap();
    let id = started.data["process_id"].as_str().unwrap();
    let mut ready = started.clone();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.data["output"].as_str().unwrap().contains("yield-ready")
        && std::time::Instant::now() < deadline
    {
        ready = execute_tool_call(
            "bash_wait",
            &json!({"process_id": id, "cursor": 0, "yield_time_ms": 250}).to_string(),
            &ctx,
        )
        .unwrap();
    }
    // Each synchronous call destroys its bridge runtime and rebuilds its registry.
    let waiting = execute_tool_call(
        "bash_wait",
        &json!({"process_id": id, "cursor": ready.data["next_cursor"], "yield_time_ms": 250})
            .to_string(),
        &ctx,
    )
    .unwrap();
    let cancelled =
        execute_tool_call("bash_cancel", &json!({"process_id": id}).to_string(), &ctx).unwrap();
    assert_eq!(ready.data["state"], "running");
    assert!(ready.data["output"].as_str().unwrap().contains("yield-ready"));
    assert_eq!(waiting.data["state"], "running");
    assert!(waiting.data["exit_code"].is_null());
    assert_eq!(cancelled.data["state"], "cancelled");
    eprintln!(
        "process={id} survived two yield deadlines and bridge runtimes; cancelled and reaped"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_plugin_compat_t57_toolset_bridge_preserves_turn_and_tenant() {
    let root = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.workspace_dir = root.path().join("workspace");
    config.config_path = root.path().join("config/shiju.json");
    let tenant =
        vw_api_transport::TenantScope::new("snapshot-platform", "snapshot-company").unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let turn = crate::plugins::session_snapshot::PluginTurnSnapshot::capture(&config).unwrap();
        let context = Arc::new(
            ToolUseContext::new("snapshot-bridge", None).with_plugin_snapshot(turn.data.clone()),
        );
        super::super::scope_tool_use_context_sync(context, || {
            block_on(async {
                assert_eq!(crate::session::ui_store::current_tenant_scope(), tenant);
                assert!(
                    crate::tools::current_tool_use_context().unwrap().plugin_snapshot().is_some()
                );
                let options = crate::plugins::PluginDiscoveryOptions::new(
                    &config.workspace_dir,
                    config.config_path.parent().unwrap(),
                    config.plugins.clone(),
                );
                assert!(crate::plugins::session_snapshot::discover_for_runtime(&options).is_ok());
                let mut foreign = options;
                foreign.workspace_dir = root.path().join("foreign");
                assert!(crate::plugins::session_snapshot::discover_for_runtime(&foreign).is_err());
            })
        });
    })
    .await;
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
        let result = enabled_delegate_agents_with_project_skills(&agents, workspace, &root);
        assert_eq!(result["worker"].allowed_skills, vec!["allowed"]);
        assert_eq!(result["worker"].defaults.skills, selection);
    }
}

#[test]
fn default_tools_include_native_capability_tools() {
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(NativeRuntime::new());
    let tools = default_tools_with_runtime(Arc::new(SecurityPolicy::default()), runtime);
    let names = tools.iter().map(|tool| tool.name()).collect::<Vec<_>>();

    assert!(names.contains(&"bash_wait"));
    assert!(names.contains(&"bash_cancel"));
    assert!(names.contains(&"file_read"));
    assert!(names.contains(&"batch"));
}
