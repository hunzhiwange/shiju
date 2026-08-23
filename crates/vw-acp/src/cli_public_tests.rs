//! 验证公开 CLI 对代理令牌、全局选项和动态代理命令的解析规则。
//!
//! 命令解析、内存输出与失败 fixture 共同固定公开 CLI 的可观察契约。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::{
    AuthPolicy, NonInteractivePermissionPolicy, OutputFormat, PermissionMode, ResolvedAcpxConfig,
};

use super::*;

fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn config() -> ResolvedAcpxConfig {
    ResolvedAcpxConfig {
        default_agent: "codex".to_string(),
        default_permissions: PermissionMode::ApproveAll,
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        auth_policy: AuthPolicy::Skip,
        ttl_ms: 30_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: OutputFormat::Text,
        agents: HashMap::new(),
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

// 下列行为用例同时检查格式分支、参数边界和错误传播。
#[test]
fn detect_agent_token_skips_global_options() {
    let scan = detect_agent_token(&argv(&[
        "--cwd",
        "/tmp/repo",
        "--format=json",
        "--approve-reads",
        "custom-agent",
        "prompt",
    ]));

    assert_eq!(scan.token, Some("custom-agent".to_string()));
    assert!(!scan.has_agent_override);
}

#[test]
fn detect_agent_token_treats_stdin_marker_as_agent_token() {
    let scan = detect_agent_token(&argv(&["--json-strict", "-", "prompt"]));

    assert_eq!(scan.token, Some("-".to_string()));
    assert!(!scan.has_agent_override);
}

#[test]
fn detect_agent_token_stops_at_argument_separator() {
    let scan = detect_agent_token(&argv(&["--ttl=30", "--", "custom-agent"]));

    assert_eq!(scan.token, None);
    assert!(!scan.has_agent_override);
}

#[test]
fn detect_agent_token_skips_equals_style_agent_override() {
    let scan = detect_agent_token(&argv(&["--agent=./server", "--timeout", "5", "prompt"]));

    assert_eq!(scan.token, Some("prompt".to_string()));
    assert!(scan.has_agent_override);
}

#[test]
fn detect_agent_token_skips_boolean_global_flags() {
    let scan = detect_agent_token(&argv(&[
        "--approve-all",
        "--deny-all",
        "--verbose",
        "--suppress-reads",
        "custom-agent",
    ]));

    assert_eq!(scan.token, Some("custom-agent".to_string()));
    assert!(!scan.has_agent_override);
}

#[test]
fn detect_agent_token_skips_space_separated_value_global_flags() {
    for flag in [
        "--auth-policy",
        "--non-interactive-permissions",
        "--format",
        "--model",
        "--allowed-tools",
        "--max-turns",
        "--timeout",
        "--ttl",
        "--file",
    ] {
        let scan = detect_agent_token(&argv(&[flag, "value", "custom-agent"]));

        assert_eq!(scan.token, Some("custom-agent".to_string()), "{flag}");
        assert!(!scan.has_agent_override, "{flag}");
    }
}

#[test]
fn detect_agent_token_skips_equals_value_global_flags() {
    for flag in [
        "--cwd=/tmp/repo",
        "--auth-policy=skip",
        "--non-interactive-permissions=deny",
        "--format=json",
        "--model=fast",
        "--allowed-tools=read",
        "--max-turns=3",
        "--json-strict=true",
        "--timeout=10",
        "--ttl=30",
        "--file=prompt.txt",
    ] {
        let scan = detect_agent_token(&argv(&[flag, "custom-agent"]));

        assert_eq!(scan.token, Some("custom-agent".to_string()), "{flag}");
        assert!(!scan.has_agent_override, "{flag}");
    }
}

#[test]
fn detect_agent_token_returns_none_for_unknown_flag_before_agent() {
    let scan = detect_agent_token(&argv(&["--unknown", "custom-agent"]));

    assert_eq!(scan.token, None);
    assert!(!scan.has_agent_override);
}

#[test]
fn detect_agent_token_records_agent_override_without_dynamic_command() {
    let scan = detect_agent_token(&argv(&["--agent", "./server", "prompt"]));

    assert_eq!(scan.token, Some("prompt".to_string()));
    assert!(scan.has_agent_override);
    assert!(resolve_dynamic_agent_command(&scan, &[], &HashSet::new()).is_none());
}

#[test]
fn resolve_dynamic_agent_command_ignores_top_level_verbs() {
    let verbs = HashSet::from(["status".to_string()]);
    let agents = vec!["codex".to_string()];

    assert_eq!(
        resolve_dynamic_agent_command(
            &AgentTokenScan { token: Some("my-agent".to_string()), has_agent_override: false },
            &agents,
            &verbs,
        ),
        Some("my-agent".to_string())
    );
    assert_eq!(
        resolve_dynamic_agent_command(
            &AgentTokenScan { token: Some("status".to_string()), has_agent_override: false },
            &[],
            &verbs,
        ),
        None
    );
    assert_eq!(
        resolve_dynamic_agent_command(
            &AgentTokenScan { token: Some("codex".to_string()), has_agent_override: false },
            &agents,
            &verbs,
        ),
        None
    );
}

#[test]
fn root_prompt_action_shows_help_or_errors_for_empty_tty_prompt() {
    assert_eq!(
        resolve_root_prompt_action(&[], true, false).expect("help action"),
        RootPromptAction::ShowHelp
    );
    assert!(resolve_root_prompt_action(&[], true, true).is_err());
    assert_eq!(
        resolve_root_prompt_action(&["hello".to_string()], true, true).expect("prompt action"),
        RootPromptAction::HandlePrompt
    );
}

#[test]
fn root_prompt_action_handles_empty_non_tty_prompt() {
    assert_eq!(
        resolve_root_prompt_action(&[], false, false).expect("prompt action"),
        RootPromptAction::HandlePrompt
    );
    assert_eq!(
        resolve_root_prompt_action(&[], false, true).expect("prompt action"),
        RootPromptAction::HandlePrompt
    );
}

#[test]
fn configure_public_cli_invokes_registration_callbacks_in_order() {
    let calls = RefCell::new(Vec::new());
    let mut program = ();
    let plan = configure_public_cli(ConfigurePublicCliOptions {
        program: &mut program,
        argv: &argv(&["new-agent", "hello"]),
        config: &config(),
        requested_json_strict: true,
        top_level_verbs: &HashSet::from(["status".to_string()]),
        register_agent_command: |_, name, _| calls.borrow_mut().push(format!("agent:{name}")),
        register_default_commands: |_, _| calls.borrow_mut().push("defaults".to_string()),
        register_root_prompt: |_, strict| calls.borrow_mut().push(format!("root:{strict}")),
        add_help_text: |_, text| {
            calls.borrow_mut().push(format!("help:{}", text.contains("Examples:")))
        },
    });
    let calls = calls.into_inner();

    assert_eq!(plan.dynamic_agent_command, Some("new-agent".to_string()));
    assert!(calls.iter().any(|call| call == "agent:new-agent"));
    assert_eq!(calls[calls.len() - 2], "root:true");
    assert_eq!(calls[calls.len() - 1], "help:true");
}

#[test]
fn configure_public_cli_skips_dynamic_agent_for_top_level_verb() {
    let calls = RefCell::new(Vec::new());
    let mut program = ();
    let plan = configure_public_cli(ConfigurePublicCliOptions {
        program: &mut program,
        argv: &argv(&["status"]),
        config: &config(),
        requested_json_strict: false,
        top_level_verbs: &HashSet::from(["status".to_string()]),
        register_agent_command: |_, name, _| calls.borrow_mut().push(format!("agent:{name}")),
        register_default_commands: |_, _| calls.borrow_mut().push("defaults".to_string()),
        register_root_prompt: |_, strict| calls.borrow_mut().push(format!("root:{strict}")),
        add_help_text: |_, text| {
            calls.borrow_mut().push(format!("help:{}", text.contains("Examples:")))
        },
    });
    let calls = calls.into_inner();

    assert_eq!(plan.dynamic_agent_command, None);
    assert!(!calls.iter().any(|call| call == "agent:status"));
    assert_eq!(calls[calls.len() - 2], "root:false");
    assert_eq!(calls[calls.len() - 1], "help:true");
}
