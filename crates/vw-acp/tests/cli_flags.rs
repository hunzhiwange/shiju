//! CLI 标志解析与约束测试。
//!
//! 用例覆盖输出格式、工具白名单、权限模式、JSON strict 约束和会话命名，
//! 确保用户输入在进入运行时前被显式校验。

use std::collections::HashMap;

use vw_acp::cli::flags::{
    GlobalFlagOptions, PermissionFlags, StatusFlags, has_explicit_permission_mode_flag,
    parse_allowed_tools, parse_output_format, parse_prompt_retries, resolve_agent_invocation,
    resolve_global_flags, resolve_permission_mode, resolve_session_name_from_flags,
};
use vw_acp::{
    AgentCommandSpec, AuthPolicy, NonInteractivePermissionPolicy, OutputFormat, PermissionMode,
    ResolvedAcpxConfig,
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

/// 验证 parse_output_format_supports_known_values 覆盖的 CLI 标志规则保持稳定。
#[test]
fn parse_output_format_supports_known_values() {
    assert_eq!(parse_output_format("text").unwrap(), OutputFormat::Text);
    assert_eq!(parse_output_format("json").unwrap(), OutputFormat::Json);
    assert_eq!(parse_output_format("quiet").unwrap(), OutputFormat::Quiet);
    assert!(parse_output_format("yaml").is_err());
}

/// 验证 parse_allowed_tools_handles_empty_and_invalid_entries 覆盖的 CLI 标志规则保持稳定。
#[test]
fn parse_allowed_tools_handles_empty_and_invalid_entries() {
    assert_eq!(parse_allowed_tools("").unwrap(), Vec::<String>::new());
    assert_eq!(
        parse_allowed_tools("read_file, search ,run").unwrap(),
        vec!["read_file".to_string(), "search".to_string(), "run".to_string()]
    );
    assert_eq!(
        parse_allowed_tools("read_file, apply_patch, glob").unwrap(),
        vec!["read_file".to_string(), "apply_patch".to_string(), "glob".to_string()]
    );
    assert!(parse_allowed_tools("read,,write").is_err());
}

/// 验证 parse_prompt_retries_accepts_non_negative_integer_values 覆盖的 CLI 标志规则保持稳定。
#[test]
fn parse_prompt_retries_accepts_non_negative_integer_values() {
    assert_eq!(parse_prompt_retries("0").unwrap(), 0);
    assert_eq!(parse_prompt_retries("2.0").unwrap(), 2);
    assert!(parse_prompt_retries("-1").is_err());
    assert!(parse_prompt_retries("1.2").is_err());
}

/// 验证 resolve_permission_mode_rejects_conflicting_flags 覆盖的 CLI 标志规则保持稳定。
#[test]
fn resolve_permission_mode_rejects_conflicting_flags() {
    let flags = PermissionFlags { approve_all: true, approve_reads: true, deny_all: false };
    assert!(resolve_permission_mode(&flags, PermissionMode::ApproveReads).is_err());
    assert!(has_explicit_permission_mode_flag(&flags));
}

/// 验证 resolve_global_flags_enforces_json_strict_constraints 覆盖的 CLI 标志规则保持稳定。
#[test]
fn resolve_global_flags_enforces_json_strict_constraints() {
    let config = test_config();
    let options = GlobalFlagOptions {
        json_strict: true,
        format: Some(OutputFormat::Text),
        ..GlobalFlagOptions::default()
    };
    assert!(resolve_global_flags(&options, &config).is_err());
}

/// 验证 resolve_agent_invocation_applies_registry_and_absolute_cwd 覆盖的 CLI 标志规则保持稳定。
#[test]
fn resolve_agent_invocation_applies_registry_and_absolute_cwd() {
    let config = test_config();
    let flags = resolve_global_flags(&GlobalFlagOptions::default(), &config).unwrap();
    let resolved = resolve_agent_invocation(None, &flags, &config).unwrap();
    assert_eq!(resolved.agent_name, "codex");
    assert!(resolved.agent_command.contains("codex-acp"));
    assert!(std::path::Path::new(&resolved.cwd).is_absolute());
}

/// 验证 resolve_session_name_prefers_local_flag_and_validates_global 覆盖的 CLI 标志规则保持稳定。
#[test]
fn resolve_session_name_prefers_local_flag_and_validates_global() {
    let flags = StatusFlags { session: Some(" local ".to_string()) };
    assert_eq!(
        resolve_session_name_from_flags(&flags, Some("global")).unwrap(),
        Some("local".to_string())
    );
    let empty = StatusFlags::default();
    assert!(resolve_session_name_from_flags(&empty, Some(" ")).is_err());
}
