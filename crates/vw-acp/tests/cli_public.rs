//! 公共 CLI 入口动态代理解析测试。
//!
//! 这些用例锁定未知位置参数、顶层动词拒绝、根提示和帮助文本注册逻辑，
//! 防止公共 CLI 对用户输入做出不兼容解释。

use std::collections::{HashMap, HashSet};

use vw_acp::{
    AgentCommandSpec, AgentTokenScan, AuthPolicy, ConfigurePublicCliOptions,
    NonInteractivePermissionPolicy, OutputFormat, PUBLIC_CLI_HELP_TEXT, PermissionMode,
    ResolvedAcpxConfig, RootPromptAction, build_public_cli_plan, configure_public_cli,
    detect_agent_token, resolve_dynamic_agent_command, resolve_root_prompt_action,
};

fn test_config() -> ResolvedAcpxConfig {
    ResolvedAcpxConfig {
        default_agent: "codex".to_string(),
        default_permissions: PermissionMode::ApproveReads,
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        auth_policy: AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: Some(30_000),
        queue_max_depth: 16,
        format: OutputFormat::Text,
        agents: HashMap::from([(
            "codex".to_string(),
            AgentCommandSpec {
                display_name: "Codex CLI".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@agentclientprotocol/codex-acp@latest".to_string()],
                env: HashMap::new(),
            },
        )]),
        auth: HashMap::new(),
        disable_exec: false,
        mcp_servers: Vec::new(),
        mcp_server_management: Vec::new(),
        global_path: "/tmp/global.json".to_string(),
        project_path: "/tmp/project.json".to_string(),
        has_global_config: false,
        has_project_config: false,
    }
}

#[derive(Debug, Default)]
struct RecordingProgram {
    registered_agents: Vec<String>,
    default_registration_count: usize,
    root_prompt_flags: Vec<bool>,
    help_texts: Vec<String>,
}

/// 验证 detect_agent_token_skips_known_global_flags 覆盖的公共 CLI 行为保持稳定。
#[test]
fn detect_agent_token_skips_known_global_flags() {
    let argv = vec![
        "--cwd".to_string(),
        "/tmp/workspace".to_string(),
        "--ttl=30".to_string(),
        "--json-strict".to_string(),
        "custom-agent".to_string(),
        "prompt".to_string(),
    ];

    assert_eq!(
        detect_agent_token(&argv),
        AgentTokenScan { token: Some("custom-agent".to_string()), has_agent_override: false }
    );
}

/// 验证 detect_agent_token_marks_agent_override 覆盖的公共 CLI 行为保持稳定。
#[test]
fn detect_agent_token_marks_agent_override() {
    let argv = vec!["--agent=./custom-server".to_string(), "prompt".to_string()];

    assert_eq!(
        detect_agent_token(&argv),
        AgentTokenScan { token: Some("prompt".to_string()), has_agent_override: true }
    );
}

/// 验证 build_public_cli_plan_adds_dynamic_agent_for_unknown_positional_token 覆盖的公共 CLI 行为保持稳定。
#[test]
fn build_public_cli_plan_adds_dynamic_agent_for_unknown_positional_token() {
    let config = test_config();
    let argv = vec!["custom-agent".to_string(), "fix bug".to_string()];
    let top_level_verbs = HashSet::from(["config".to_string(), "sessions".to_string()]);

    let plan = build_public_cli_plan(&argv, &config, &top_level_verbs);

    assert_eq!(plan.dynamic_agent_command, Some("custom-agent".to_string()));
    assert!(plan.agent_commands.iter().any(|agent| agent == "codex"));
}

/// 验证 resolve_dynamic_agent_command_rejects_top_level_verbs 覆盖的公共 CLI 行为保持稳定。
#[test]
fn resolve_dynamic_agent_command_rejects_top_level_verbs() {
    let top_level_verbs = HashSet::from(["config".to_string()]);
    let agent_commands = vec!["codex".to_string(), "claude".to_string()];

    assert_eq!(
        resolve_dynamic_agent_command(
            &AgentTokenScan { token: Some("config".to_string()), has_agent_override: false },
            &agent_commands,
            &top_level_verbs,
        ),
        None
    );

    assert_eq!(
        resolve_dynamic_agent_command(
            &AgentTokenScan { token: Some("claude".to_string()), has_agent_override: false },
            &agent_commands,
            &top_level_verbs,
        ),
        Some("claude".to_string())
    );
}

/// 验证 resolve_root_prompt_action_matches_typescript_behavior 覆盖的公共 CLI 行为保持稳定。
#[test]
fn resolve_root_prompt_action_matches_typescript_behavior() {
    let prompt_parts = Vec::<String>::new();

    assert_eq!(
        resolve_root_prompt_action(&prompt_parts, true, false).unwrap(),
        RootPromptAction::ShowHelp
    );

    let error = resolve_root_prompt_action(&prompt_parts, true, true).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Prompt is required (pass as argument, --file, or pipe via stdin)"
    );

    assert_eq!(
        resolve_root_prompt_action(&["hello".to_string()], true, true).unwrap(),
        RootPromptAction::HandlePrompt
    );
}

/// 验证 configure_public_cli_registers_agent_commands_root_prompt_and_help_text 覆盖的公共 CLI 行为保持稳定。
#[test]
fn configure_public_cli_registers_agent_commands_root_prompt_and_help_text() {
    let config = test_config();
    let argv = vec!["custom-agent".to_string()];
    let top_level_verbs = HashSet::from(["config".to_string(), "sessions".to_string()]);
    let mut program = RecordingProgram::default();

    let plan = configure_public_cli(ConfigurePublicCliOptions {
        program: &mut program,
        argv: &argv,
        config: &config,
        requested_json_strict: true,
        top_level_verbs: &top_level_verbs,
        register_agent_command: |program, agent_name, _| {
            program.registered_agents.push(agent_name.to_string());
        },
        register_default_commands: |program, _| {
            program.default_registration_count += 1;
        },
        register_root_prompt: |program, requested_json_strict| {
            program.root_prompt_flags.push(requested_json_strict);
        },
        add_help_text: |program, help_text| {
            program.help_texts.push(help_text.to_string());
        },
    });

    assert_eq!(plan.dynamic_agent_command, Some("custom-agent".to_string()));
    assert_eq!(program.default_registration_count, 1);
    assert_eq!(program.root_prompt_flags, vec![true]);
    assert_eq!(program.help_texts, vec![PUBLIC_CLI_HELP_TEXT.to_string()]);
    assert!(program.registered_agents.iter().any(|agent| agent == "codex"));
    assert!(program.registered_agents.iter().any(|agent| agent == "custom-agent"));
}
