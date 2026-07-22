//! CLI 核心规划与运行时兼容性测试。
//!
//! 这些用例覆盖顶层动词、启动规划、输出策略、路径检测和权限错误码，
//! 用来保持 Rust CLI 与既有 TypeScript 行为一致。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use vw_acp::{
    AgentCommandSpec, AuthPolicy, CliBootstrapPlan, NonInteractivePermissionPolicy, OutputFormat,
    PermissionMode, PermissionStats, QUEUE_OWNER_PROCESS_MARKER, ResolvedAcpxConfig,
    TOP_LEVEL_VERBS, apply_permission_exit_code, build_cli_bootstrap_plan, build_cli_runtime_plan,
    command_argv, detect_initial_cwd, detect_json_strict, detect_requested_output_format,
    is_queue_owner_mode, is_version_requested, read_prompt, resolve_compatible_config_id,
    resolve_requested_output_policy, should_maybe_handle_skillflag, top_level_verbs,
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

fn temp_file_path(name: &str) -> PathBuf {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("vw-acp-{name}-{suffix}.txt"))
}

/// 验证 top_level_verbs_match_typescript_contract 覆盖的 CLI 行为保持稳定。
#[test]
fn top_level_verbs_match_typescript_contract() {
    let verbs = top_level_verbs();

    assert_eq!(TOP_LEVEL_VERBS.len(), 10);
    assert!(verbs.contains("prompt"));
    assert!(verbs.contains("config"));
    assert!(verbs.contains("help"));
}

/// 验证 bootstrap_plan_detects_version_queue_owner_and_skillflag 覆盖的 CLI 行为保持稳定。
#[test]
fn bootstrap_plan_detects_version_queue_owner_and_skillflag() {
    let argv = vec![
        "node".to_string(),
        "vwacp".to_string(),
        QUEUE_OWNER_PROCESS_MARKER.to_string(),
        "--version".to_string(),
        "--skill=my-skill".to_string(),
    ];

    let plan = build_cli_bootstrap_plan(&argv, "/tmp");

    assert_eq!(
        plan,
        CliBootstrapPlan {
            cli_args: vec![
                QUEUE_OWNER_PROCESS_MARKER.to_string(),
                "--version".to_string(),
                "--skill=my-skill".to_string(),
            ],
            perf_capture_role: vw_acp::PerfCaptureRole::QueueOwner,
            print_version: true,
            queue_owner_mode: true,
            should_handle_skillflag: true,
            initial_cwd: PathBuf::from("/tmp"),
            requested_json_strict: false,
            suppress_reads: false,
        }
    );

    assert!(is_version_requested(&argv));
    assert!(is_queue_owner_mode(&argv));
    assert!(should_maybe_handle_skillflag(&argv));
}

/// 验证 command_argv_supports_native_rust_process_args 覆盖的 CLI 行为保持稳定。
#[test]
fn command_argv_supports_native_rust_process_args() {
    let argv = vec![
        "/tmp/vwacp".to_string(),
        QUEUE_OWNER_PROCESS_MARKER.to_string(),
        "--version".to_string(),
        "--skill=my-skill".to_string(),
    ];

    assert_eq!(
        command_argv(&argv),
        &[
            QUEUE_OWNER_PROCESS_MARKER.to_string(),
            "--version".to_string(),
            "--skill=my-skill".to_string(),
        ]
    );
    assert!(is_queue_owner_mode(&argv));
}

/// 验证 bootstrap_plan_supports_native_rust_process_args 覆盖的 CLI 行为保持稳定。
#[test]
fn bootstrap_plan_supports_native_rust_process_args() {
    let argv = vec![
        "/tmp/vwacp".to_string(),
        "--cwd".to_string(),
        "../other".to_string(),
        "--json-strict".to_string(),
        "--suppress-reads".to_string(),
    ];

    let plan = build_cli_bootstrap_plan(&argv, "/workspace/project");

    assert_eq!(
        plan,
        CliBootstrapPlan {
            cli_args: vec![
                "--cwd".to_string(),
                "../other".to_string(),
                "--json-strict".to_string(),
                "--suppress-reads".to_string(),
            ],
            perf_capture_role: vw_acp::PerfCaptureRole::Cli,
            print_version: false,
            queue_owner_mode: false,
            should_handle_skillflag: false,
            initial_cwd: PathBuf::from("/workspace/other"),
            requested_json_strict: true,
            suppress_reads: true,
        }
    );
}

/// 验证 detect_initial_cwd_prefers_explicit_flag_and_stops_at_terminator 覆盖的 CLI 行为保持稳定。
#[test]
fn detect_initial_cwd_prefers_explicit_flag_and_stops_at_terminator() {
    let current_dir = PathBuf::from("/workspace/project");

    let resolved = detect_initial_cwd(
        &["--cwd".to_string(), "../other".to_string(), "codex".to_string()],
        &current_dir,
    );
    assert_eq!(resolved, PathBuf::from("/workspace/other"));

    let after_terminator = detect_initial_cwd(
        &["--".to_string(), "--cwd".to_string(), "../ignored".to_string()],
        &current_dir,
    );
    assert_eq!(after_terminator, current_dir);
}

/// 验证 detect_output_format_and_json_strict_follow_typescript_rules 覆盖的 CLI 行为保持稳定。
#[test]
fn detect_output_format_and_json_strict_follow_typescript_rules() {
    assert_eq!(
        detect_requested_output_format(
            &["--format".to_string(), "quiet".to_string()],
            OutputFormat::Text,
        ),
        OutputFormat::Quiet
    );
    assert_eq!(
        detect_requested_output_format(&["--json-strict".to_string()], OutputFormat::Text),
        OutputFormat::Json
    );
    assert!(detect_json_strict(&["--json-strict=true".to_string()]));
}

/// 验证 runtime_plan_combines_output_policy_and_public_cli_registration_plan 覆盖的 CLI 行为保持稳定。
#[test]
fn runtime_plan_combines_output_policy_and_public_cli_registration_plan() {
    let config = test_config();
    let plan = build_cli_runtime_plan(
        &[
            "--format".to_string(),
            "quiet".to_string(),
            "--suppress-reads".to_string(),
            "custom-agent".to_string(),
        ],
        &config,
    );

    assert_eq!(plan.requested_output_format, OutputFormat::Quiet);
    assert!(plan.requested_output_policy.suppress_reads);
    assert_eq!(plan.public_cli_plan.dynamic_agent_command, Some("custom-agent".to_string()));
}

/// 验证 bootstrap_cli_args_preserve_agent_session_subcommands_for_runtime_plan 覆盖的 CLI 行为保持稳定。
#[test]
fn bootstrap_cli_args_preserve_agent_session_subcommands_for_runtime_plan() {
    let config = test_config();
    let bootstrap_plan = build_cli_bootstrap_plan(
        &[
            "/tmp/target/debug/acp".to_string(),
            "claude".to_string(),
            "sessions".to_string(),
            "new".to_string(),
        ],
        "/tmp/workspace",
    );

    let runtime_plan = build_cli_runtime_plan(&bootstrap_plan.cli_args, &config);

    assert_eq!(bootstrap_plan.cli_args, ["claude", "sessions", "new"]);
    assert_eq!(runtime_plan.public_cli_plan.dynamic_agent_command, Some("claude".to_string()));
}

/// 验证 resolve_requested_output_policy_preserves_base_behavior_and_overrides_suppress_reads 覆盖的 CLI 行为保持稳定。
#[test]
fn resolve_requested_output_policy_preserves_base_behavior_and_overrides_suppress_reads() {
    let output_policy = resolve_requested_output_policy(OutputFormat::Json, true, true);

    assert_eq!(output_policy.format, OutputFormat::Json);
    assert!(output_policy.json_strict);
    assert!(output_policy.suppress_reads);
    assert!(output_policy.suppress_non_json_stderr);
}

/// 验证 apply_permission_exit_code_matches_permission_denied_behavior 覆盖的 CLI 行为保持稳定。
#[test]
fn apply_permission_exit_code_matches_permission_denied_behavior() {
    let stats = PermissionStats { requested: 2, approved: 0, denied: 1, cancelled: 1 };

    assert_eq!(apply_permission_exit_code(0, &stats), 5);
}

/// 验证 resolve_compatible_config_id_maps_codex_thought_level 覆盖的 CLI 行为保持稳定。
#[test]
fn resolve_compatible_config_id_maps_codex_thought_level() {
    assert_eq!(
        resolve_compatible_config_id(
            "codex",
            "npx -y @agentclientprotocol/codex-acp@latest",
            "thought_level",
        ),
        "reasoning_effort".to_string()
    );
    assert_eq!(
        resolve_compatible_config_id("claude", "npx claude", "thought_level"),
        "thought_level".to_string()
    );
}

/// 验证异步 CLI 路径在运行时组合下保持稳定。
#[tokio::test]
async fn read_prompt_reads_text_file_and_merges_suffix_prompt() {
    let path = temp_file_path("prompt");
    fs::write(&path, "Base prompt").unwrap();

    let prompt = read_prompt(
        &["follow-up".to_string()],
        Some(path.to_string_lossy().as_ref()),
        "/tmp",
        true,
    )
    .await
    .unwrap();

    fs::remove_file(&path).ok();

    assert_eq!(vw_acp::prompt_to_display_text(&prompt), "Base prompt\n\nfollow-up");
}

/// 验证异步 CLI 路径在运行时组合下保持稳定。
#[tokio::test]
async fn read_prompt_rejects_empty_prompt_when_interactive_without_input() {
    let error = read_prompt(&[], None, "/tmp", true).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Prompt is required (pass as argument, --file, or pipe via stdin)"
    );
}
