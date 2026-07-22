//! CLI 标志解析与默认值处理的单元测试。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::agent_registry::AgentCommandSpec;
use crate::config::ResolvedAcpxConfig;
use crate::types::{
    AuthPolicy, NonInteractivePermissionPolicy, OutputFormat, OutputPolicy, PermissionMode,
};

use super::{
    GlobalFlagOptions, GlobalFlags, PermissionFlags, StatusFlags,
    has_explicit_permission_mode_flag, parse_allowed_tools, parse_auth_policy, parse_history_limit,
    parse_max_turns, parse_non_empty_value, parse_non_interactive_permission_policy,
    parse_output_format, parse_prompt_retries, parse_session_name, parse_timeout_seconds,
    parse_ttl_seconds, resolve_agent_invocation, resolve_global_flags, resolve_output_policy,
    resolve_permission_mode, resolve_session_name_from_flags,
};

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn make_config(ttl_ms: u64) -> ResolvedAcpxConfig {
    ResolvedAcpxConfig {
        default_agent: "codex".to_string(),
        default_permissions: PermissionMode::ApproveAll,
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        auth_policy: AuthPolicy::Skip,
        ttl_ms,
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

fn make_global_flags(cwd: impl Into<String>) -> GlobalFlags {
    GlobalFlags {
        agent: None,
        cwd: cwd.into(),
        auth_policy: Some(AuthPolicy::Skip),
        non_interactive_permissions: NonInteractivePermissionPolicy::Deny,
        json_strict: false,
        suppress_reads: false,
        timeout: None,
        ttl: 300_000,
        verbose: false,
        format: OutputFormat::Text,
        model: None,
        allowed_tools: None,
        max_turns: None,
        prompt_retries: None,
        approve_all: false,
        approve_reads: false,
        deny_all: false,
    }
}

fn custom_agent(command: &str) -> AgentCommandSpec {
    AgentCommandSpec {
        display_name: "Custom".to_string(),
        command: command.to_string(),
        args: vec!["--stdio".to_string()],
        env: HashMap::from([("CUSTOM_ENV".to_string(), "1".to_string())]),
    }
}

fn assert_error_message<T: std::fmt::Debug>(result: Result<T, super::FlagsError>, expected: &str) {
    let error = result.expect_err("expected flags error");
    assert_eq!(error.to_string(), expected);
}

#[test]
fn resolve_global_flags_falls_back_to_five_minute_queue_owner_ttl() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let flags = resolve_global_flags(&GlobalFlagOptions::default(), &make_config(0))
        .expect("resolve flags");

    assert_eq!(flags.ttl, 300_000);
}

#[test]
fn parse_output_format_accepts_known_values_and_trims_whitespace() {
    assert_eq!(parse_output_format(" text ").expect("text format"), OutputFormat::Text);
    assert_eq!(parse_output_format("json").expect("json format"), OutputFormat::Json);
    assert_eq!(parse_output_format("quiet").expect("quiet format"), OutputFormat::Quiet);
}

#[test]
fn parse_output_format_rejects_unknown_value() {
    assert_error_message(
        parse_output_format("yaml"),
        "Invalid format \"yaml\". Expected one of: text, json, quiet",
    );
}

#[test]
fn parse_auth_policy_accepts_known_values_and_rejects_unknown_value() {
    assert_eq!(parse_auth_policy(" skip ").expect("skip policy"), AuthPolicy::Skip);
    assert_eq!(parse_auth_policy("fail").expect("fail policy"), AuthPolicy::Fail);
    assert_error_message(
        parse_auth_policy("prompt"),
        "Invalid auth policy \"prompt\". Expected one of: skip, fail",
    );
}

#[test]
fn parse_non_interactive_permission_policy_accepts_known_values_and_rejects_unknown_value() {
    assert_eq!(
        parse_non_interactive_permission_policy(" deny ").expect("deny policy"),
        NonInteractivePermissionPolicy::Deny
    );
    assert_eq!(
        parse_non_interactive_permission_policy("fail").expect("fail policy"),
        NonInteractivePermissionPolicy::Fail
    );
    assert_error_message(
        parse_non_interactive_permission_policy("ask"),
        "Invalid non-interactive permission policy \"ask\". Expected one of: deny, fail",
    );
}

#[test]
fn parse_timeout_seconds_rounds_fractional_seconds_to_milliseconds() {
    assert_eq!(parse_timeout_seconds("1.2345").expect("timeout"), 1235);
}

#[test]
fn parse_timeout_seconds_rejects_non_positive_non_finite_and_invalid_numbers() {
    assert_error_message(
        parse_timeout_seconds("0"),
        "Timeout must be a positive number of seconds",
    );
    assert_error_message(
        parse_timeout_seconds("-1"),
        "Timeout must be a positive number of seconds",
    );
    assert_error_message(
        parse_timeout_seconds("inf"),
        "Timeout must be a positive number of seconds",
    );
    assert_error_message(parse_timeout_seconds("soon"), "Invalid number \"soon\"");
}

#[test]
fn parse_ttl_seconds_accepts_zero_and_rounds_fractional_seconds() {
    assert_eq!(parse_ttl_seconds("0").expect("zero ttl"), 0);
    assert_eq!(parse_ttl_seconds("2.555").expect("ttl"), 2555);
}

#[test]
fn parse_ttl_seconds_rejects_negative_non_finite_and_invalid_numbers() {
    assert_error_message(
        parse_ttl_seconds("-0.001"),
        "TTL must be a non-negative number of seconds",
    );
    assert_error_message(parse_ttl_seconds("NaN"), "TTL must be a non-negative number of seconds");
    assert_error_message(parse_ttl_seconds("later"), "Invalid number \"later\"");
}

#[test]
fn parse_session_name_trims_non_empty_names_and_rejects_empty_names() {
    assert_eq!(parse_session_name(" alpha ").expect("session name"), "alpha");
    assert_error_message(parse_session_name(" \t "), "Session name must not be empty");
}

#[test]
fn parse_non_empty_value_trims_values_and_uses_label_in_empty_error() {
    assert_eq!(parse_non_empty_value("Model", " gpt ").expect("model"), "gpt");
    assert_error_message(parse_non_empty_value("Model", " "), "Model must not be empty");
}

#[test]
fn parse_history_limit_accepts_positive_integer_and_rejects_invalid_values() {
    assert_eq!(parse_history_limit("3").expect("history limit"), 3);
    assert_error_message(parse_history_limit("0"), "Limit must be a positive integer");
    assert_error_message(parse_history_limit("2.5"), "Limit must be a positive integer");
    assert_error_message(parse_history_limit("many"), "Invalid number \"many\"");
}

#[test]
fn parse_allowed_tools_accepts_empty_and_trimmed_comma_separated_values() {
    assert!(parse_allowed_tools(" \t ").expect("empty tools").is_empty());
    assert_eq!(
        parse_allowed_tools(" shell , read ,write ").expect("allowed tools"),
        vec!["shell".to_string(), "read".to_string(), "write".to_string()]
    );
}

#[test]
fn parse_allowed_tools_rejects_empty_entries() {
    assert_error_message(
        parse_allowed_tools("shell,,read"),
        "Allowed tools must be a comma-separated list without empty entries",
    );
}

#[test]
fn parse_max_turns_accepts_positive_integer_and_rejects_invalid_values() {
    assert_eq!(parse_max_turns("1").expect("max turns"), 1);
    assert_error_message(parse_max_turns("0"), "Max turns must be a positive integer");
    assert_error_message(parse_max_turns("1.25"), "Max turns must be a positive integer");
}

#[test]
fn parse_prompt_retries_accepts_zero_and_rejects_invalid_values() {
    assert_eq!(parse_prompt_retries("0").expect("zero retries"), 0);
    assert_eq!(parse_prompt_retries("4").expect("prompt retries"), 4);
    assert_error_message(
        parse_prompt_retries("-1"),
        "Prompt retries must be a non-negative integer",
    );
    assert_error_message(
        parse_prompt_retries("1.5"),
        "Prompt retries must be a non-negative integer",
    );
}

#[test]
fn has_explicit_permission_mode_flag_reports_any_selected_mode() {
    assert!(!has_explicit_permission_mode_flag(&PermissionFlags::default()));
    assert!(has_explicit_permission_mode_flag(&PermissionFlags {
        approve_all: true,
        ..PermissionFlags::default()
    }));
    assert!(has_explicit_permission_mode_flag(&PermissionFlags {
        approve_reads: true,
        ..PermissionFlags::default()
    }));
    assert!(has_explicit_permission_mode_flag(&PermissionFlags {
        deny_all: true,
        ..PermissionFlags::default()
    }));
}

#[test]
fn resolve_permission_mode_uses_selected_mode_or_default() {
    assert_eq!(
        resolve_permission_mode(&PermissionFlags::default(), PermissionMode::ApproveReads)
            .expect("default permission"),
        PermissionMode::ApproveReads
    );
    assert_eq!(
        resolve_permission_mode(
            &PermissionFlags { approve_all: true, ..PermissionFlags::default() },
            PermissionMode::DenyAll,
        )
        .expect("approve all"),
        PermissionMode::ApproveAll
    );
    assert_eq!(
        resolve_permission_mode(
            &PermissionFlags { approve_reads: true, ..PermissionFlags::default() },
            PermissionMode::DenyAll,
        )
        .expect("approve reads"),
        PermissionMode::ApproveReads
    );
    assert_eq!(
        resolve_permission_mode(
            &PermissionFlags { deny_all: true, ..PermissionFlags::default() },
            PermissionMode::ApproveAll,
        )
        .expect("deny all"),
        PermissionMode::DenyAll
    );
}

#[test]
fn resolve_permission_mode_rejects_multiple_selected_modes() {
    assert_error_message(
        resolve_permission_mode(
            &PermissionFlags { approve_all: true, approve_reads: true, deny_all: false },
            PermissionMode::DenyAll,
        ),
        "Use only one permission mode: --approve-all, --approve-reads, or --deny-all",
    );
}

#[test]
fn resolve_global_flags_merges_options_over_config_defaults() {
    let mut config = make_config(12_000);
    config.timeout_ms = Some(4_000);
    config.format = OutputFormat::Quiet;
    config.auth_policy = AuthPolicy::Fail;
    config.non_interactive_permissions = NonInteractivePermissionPolicy::Fail;

    let flags = resolve_global_flags(
        &GlobalFlagOptions {
            agent: Some("codex --acp".to_string()),
            cwd: Some("/workspace".to_string()),
            auth_policy: Some(AuthPolicy::Skip),
            non_interactive_permissions: Some(NonInteractivePermissionPolicy::Deny),
            json_strict: true,
            suppress_reads: true,
            timeout: Some(9_000),
            ttl: Some(5_000),
            verbose: false,
            format: Some(OutputFormat::Json),
            model: Some(" gpt-5 ".to_string()),
            allowed_tools: Some(vec!["shell".to_string()]),
            max_turns: Some(3),
            prompt_retries: Some(2),
            approve_all: true,
            approve_reads: false,
            deny_all: false,
        },
        &config,
    )
    .expect("resolve flags");

    assert_eq!(flags.agent.as_deref(), Some("codex --acp"));
    assert_eq!(flags.cwd, "/workspace");
    assert_eq!(flags.auth_policy, Some(AuthPolicy::Skip));
    assert_eq!(flags.non_interactive_permissions, NonInteractivePermissionPolicy::Deny);
    assert!(flags.json_strict);
    assert!(flags.suppress_reads);
    assert_eq!(flags.timeout, Some(9_000));
    assert_eq!(flags.ttl, 5_000);
    assert_eq!(flags.format, OutputFormat::Json);
    assert_eq!(flags.model.as_deref(), Some("gpt-5"));
    assert_eq!(flags.allowed_tools, Some(vec!["shell".to_string()]));
    assert_eq!(flags.max_turns, Some(3));
    assert_eq!(flags.prompt_retries, Some(2));
    assert!(flags.approve_all);
}

#[test]
fn resolve_global_flags_uses_config_ttl_and_current_dir_defaults() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let mut config = make_config(12_345);
    config.auth_policy = AuthPolicy::Fail;
    config.non_interactive_permissions = NonInteractivePermissionPolicy::Fail;
    config.timeout_ms = Some(2_500);
    config.format = OutputFormat::Quiet;

    let flags =
        resolve_global_flags(&GlobalFlagOptions::default(), &config).expect("resolve flags");

    let expected_cwd = std::env::current_dir().expect("current dir").to_string_lossy().into_owned();
    assert_eq!(flags.cwd, expected_cwd);
    assert_eq!(flags.auth_policy, Some(AuthPolicy::Fail));
    assert_eq!(flags.non_interactive_permissions, NonInteractivePermissionPolicy::Fail);
    assert_eq!(flags.timeout, Some(2_500));
    assert_eq!(flags.ttl, 12_345);
    assert_eq!(flags.format, OutputFormat::Quiet);
    assert!(!flags.json_strict);
    assert!(!flags.verbose);
}

#[test]
fn resolve_global_flags_rejects_invalid_json_strict_combinations_and_empty_model() {
    assert_error_message(
        resolve_global_flags(
            &GlobalFlagOptions {
                json_strict: true,
                format: Some(OutputFormat::Text),
                ..GlobalFlagOptions::default()
            },
            &make_config(0),
        ),
        "--json-strict requires --format json",
    );
    assert_error_message(
        resolve_global_flags(
            &GlobalFlagOptions {
                json_strict: true,
                verbose: true,
                format: Some(OutputFormat::Json),
                ..GlobalFlagOptions::default()
            },
            &make_config(0),
        ),
        "--json-strict cannot be combined with --verbose",
    );
    assert_error_message(
        resolve_global_flags(
            &GlobalFlagOptions { model: Some(" ".to_string()), ..GlobalFlagOptions::default() },
            &make_config(0),
        ),
        "Model must not be empty",
    );
}

#[test]
fn resolve_output_policy_derives_json_strict_suppression_and_quiet_queue_error_state() {
    assert_eq!(
        resolve_output_policy(OutputFormat::Json, true),
        OutputPolicy {
            format: OutputFormat::Json,
            json_strict: true,
            suppress_reads: false,
            suppress_non_json_stderr: true,
            queue_error_already_emitted: true,
            suppress_sdk_console_errors: true,
        }
    );
    assert!(!resolve_output_policy(OutputFormat::Quiet, false).queue_error_already_emitted);
}

#[test]
fn resolve_agent_invocation_rejects_positional_agent_with_override_command() {
    let mut flags = make_global_flags(".");
    flags.agent = Some("custom-acp".to_string());

    assert_error_message(
        resolve_agent_invocation(Some("codex"), &flags, &make_config(0)),
        "Do not combine positional agent with --agent override",
    );
}

#[test]
fn resolve_agent_invocation_uses_override_command_without_agent_config() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let mut flags = make_global_flags(".");
    flags.agent = Some(" custom-acp --stdio ".to_string());

    let invocation =
        resolve_agent_invocation(None, &flags, &make_config(0)).expect("agent invocation");

    assert_eq!(invocation.agent_name, "codex");
    assert_eq!(invocation.agent_command, "custom-acp --stdio");
    assert!(invocation.agent_config.is_none());
    let expected_cwd = std::env::current_dir().expect("current dir");
    assert_eq!(invocation.cwd, expected_cwd.to_string_lossy());
}

#[test]
fn resolve_agent_invocation_uses_config_command_but_no_config_for_blank_override() {
    let mut config = make_config(0);
    config.default_agent = "custom".to_string();
    config.agents.insert("custom".to_string(), custom_agent("custom-acp"));
    let mut flags = make_global_flags("/abs/workspace");
    flags.agent = Some(" \t ".to_string());

    let invocation = resolve_agent_invocation(None, &flags, &config).expect("agent invocation");

    assert_eq!(invocation.agent_name, "custom");
    assert_eq!(invocation.agent_command, "custom-acp --stdio");
    assert!(invocation.agent_config.is_none());
    assert_eq!(invocation.cwd, "/abs/workspace");
}

#[test]
fn resolve_agent_invocation_uses_positional_agent_and_relative_cwd() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let mut config = make_config(0);
    config.agents.insert("custom".to_string(), custom_agent("custom-acp"));

    let invocation =
        resolve_agent_invocation(Some("custom"), &make_global_flags("workspace"), &config)
            .expect("agent invocation");

    let expected_cwd =
        std::env::current_dir().expect("current dir").join(PathBuf::from("workspace"));
    assert_eq!(invocation.agent_name, "custom");
    assert_eq!(invocation.agent_command, "custom-acp --stdio");
    let agent_config = invocation.agent_config.expect("agent config");
    assert_eq!(agent_config.command, "custom-acp");
    assert_eq!(agent_config.args, vec!["--stdio".to_string()]);
    assert_eq!(agent_config.env.get("CUSTOM_ENV").map(String::as_str), Some("1"));
    assert_eq!(invocation.cwd, expected_cwd.to_string_lossy());
}

#[test]
fn resolve_agent_invocation_normalizes_current_and_parent_dirs_in_absolute_cwd() {
    let invocation = resolve_agent_invocation(
        Some("local-acp"),
        &make_global_flags("/abs/./workspace/../other"),
        &make_config(0),
    )
    .expect("agent invocation");

    assert_eq!(invocation.cwd, "/abs/other");
}

#[test]
fn resolve_agent_invocation_normalizes_parent_dir_in_relative_cwd() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let invocation = resolve_agent_invocation(
        Some("local-acp"),
        &make_global_flags("workspace/../other"),
        &make_config(0),
    )
    .expect("agent invocation");

    let expected_cwd = std::env::current_dir().expect("current dir").join(PathBuf::from("other"));
    assert_eq!(invocation.cwd, expected_cwd.to_string_lossy());
}

#[test]
fn resolve_agent_invocation_normalizes_current_dir_in_relative_cwd() {
    let _cwd_guard = CWD_LOCK.lock().expect("cwd lock");
    let invocation = resolve_agent_invocation(
        Some("local-acp"),
        &make_global_flags("./workspace"),
        &make_config(0),
    )
    .expect("agent invocation");

    let expected_cwd =
        std::env::current_dir().expect("current dir").join(PathBuf::from("workspace"));
    assert_eq!(invocation.cwd, expected_cwd.to_string_lossy());
}

#[test]
fn resolve_agent_invocation_uses_trimmed_config_default_or_builtin_default_agent() {
    let mut config = make_config(0);
    config.default_agent = " custom ".to_string();
    config.agents.insert("custom".to_string(), custom_agent("custom-acp"));

    let invocation = resolve_agent_invocation(None, &make_global_flags("/abs/workspace"), &config)
        .expect("agent invocation");

    assert_eq!(invocation.agent_name, "custom");
    assert_eq!(invocation.agent_command, "custom-acp --stdio");
    assert_eq!(invocation.cwd, "/abs/workspace");

    config.default_agent = " ".to_string();
    let default_invocation =
        resolve_agent_invocation(None, &make_global_flags("/abs/workspace"), &config)
            .expect("agent invocation");
    assert_eq!(default_invocation.agent_name, "codex");
}

#[test]
fn resolve_agent_invocation_preserves_unknown_agent_without_agent_config() {
    let invocation = resolve_agent_invocation(
        Some("local-acp"),
        &make_global_flags("/abs/workspace"),
        &make_config(0),
    )
    .expect("agent invocation");

    assert_eq!(invocation.agent_name, "local-acp");
    assert_eq!(invocation.agent_command, "local-acp");
    assert!(invocation.agent_config.is_none());
}

#[test]
fn resolve_session_name_from_flags_prefers_status_flag_over_global_session() {
    assert_eq!(
        resolve_session_name_from_flags(
            &StatusFlags { session: Some(" local ".to_string()) },
            Some("global"),
        )
        .expect("session name"),
        Some("local".to_string())
    );
}

#[test]
fn resolve_session_name_from_flags_uses_global_session_or_none_and_rejects_empty_values() {
    assert_eq!(
        resolve_session_name_from_flags(&StatusFlags::default(), Some(" global "))
            .expect("session name"),
        Some("global".to_string())
    );
    assert_eq!(
        resolve_session_name_from_flags(&StatusFlags::default(), None).expect("no session"),
        None
    );
    assert_error_message(
        resolve_session_name_from_flags(
            &StatusFlags { session: Some(" ".to_string()) },
            Some("global"),
        ),
        "Session name must not be empty",
    );
    assert_error_message(
        resolve_session_name_from_flags(&StatusFlags::default(), Some(" ")),
        "Session name must not be empty",
    );
}
