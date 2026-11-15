//! 验证 CLI 启动计划、参数边界、标准输入读取与安全默认值。
//!
//! 命令解析、内存输出与失败 fixture 共同固定公开 CLI 的可观察契约。

use std::collections::HashMap;
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::{
    AuthPolicy, EXIT_CODE_ERROR, EXIT_CODE_PERMISSION_DENIED, NonInteractivePermissionPolicy,
    OutputFormat, OutputPolicy, PermissionMode, PermissionStats, QUEUE_OWNER_PROCESS_MARKER,
    ResolvedAcpxConfig, prompt_to_display_text,
};

use super::*;

fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

async fn empty_stdin() -> Result<String, CliCoreError> {
    Ok(String::new())
}

async fn text_stdin() -> Result<String, CliCoreError> {
    Ok("from stdin\n".to_string())
}

async fn failing_stdin() -> Result<String, CliCoreError> {
    Err(CliCoreError::Io("stdin unavailable".to_string()))
}

async fn read_prompt_with_empty_stdin(
    prompt_parts: &[String],
    file_path: Option<&str>,
    cwd: impl AsRef<Path>,
    stdin_is_tty: bool,
) -> Result<PromptInput, CliCoreError> {
    read_prompt_with_stdin_reader(prompt_parts, file_path, cwd, stdin_is_tty, empty_stdin).await
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

fn unique_test_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vw-acp-cli-core-{name}-{}-{stamp}", std::process::id()))
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("reader failed"))
    }
}

// 下列行为用例同时检查格式分支、参数边界和错误传播。
#[test]
fn command_argv_skips_binary_and_script_launcher() {
    let status = argv(&["status"]);

    let binary_args = argv(&["vwacp", "status"]);
    assert_eq!(command_argv(&binary_args), status.as_slice());

    let node_args = argv(&["node", "dist/cli.js", "status"]);
    assert_eq!(command_argv(&node_args), status.as_slice());

    let bun_args = argv(&["bun", "src/main.ts", "status"]);
    assert_eq!(command_argv(&bun_args), status.as_slice());

    let tsx_args = argv(&["tsx", "src/main.ts", "status"]);
    assert_eq!(command_argv(&tsx_args), status.as_slice());

    let deno_args = argv(&["deno", "src/main.ts", "status"]);
    assert_eq!(command_argv(&deno_args), status.as_slice());

    let script_args = argv(&["vwacp", "src/main.ts", "status"]);
    assert_eq!(command_argv(&script_args), status.as_slice());

    for script in ["bin/acp.cjs", "bin/acp.mjs", "bin/acp.cts", "bin/acp.mts"] {
        let script_args = argv(&["vwacp", script, "status"]);
        assert_eq!(command_argv(&script_args), status.as_slice());
    }

    let empty_args = argv(&[]);
    assert_eq!(command_argv(&empty_args), &[] as &[String]);

    let binary_only_args = argv(&["vwacp"]);
    assert!(command_argv(&binary_only_args).is_empty());
}

#[test]
fn bootstrap_plan_detects_queue_owner_cwd_and_flags() {
    let plan = build_cli_bootstrap_plan(
        &argv(&[
            "vwacp",
            "--cwd",
            "sub/../repo",
            "--json-strict",
            "--suppress-reads",
            "--skill=rust",
        ]),
        "/work",
    );

    assert_eq!(plan.perf_capture_role, PerfCaptureRole::Cli);
    assert_eq!(plan.initial_cwd, PathBuf::from("/work/repo"));
    assert!(plan.requested_json_strict);
    assert!(plan.suppress_reads);
    assert!(plan.should_handle_skillflag);

    let queue_plan = build_cli_bootstrap_plan(
        &argv(&["vwacp", QUEUE_OWNER_PROCESS_MARKER, "--version", "--skill"]),
        "/work",
    );

    assert_eq!(queue_plan.cli_args, argv(&[QUEUE_OWNER_PROCESS_MARKER, "--version", "--skill"]));
    assert_eq!(queue_plan.perf_capture_role, PerfCaptureRole::QueueOwner);
    assert_eq!(queue_plan.perf_capture_role.as_str(), "queue_owner");
    assert!(queue_plan.print_version);
    assert!(queue_plan.queue_owner_mode);
    assert!(queue_plan.should_handle_skillflag);
    assert_eq!(PerfCaptureRole::Cli.as_str(), "cli");

    let path_buf_plan = build_cli_bootstrap_plan(
        &argv(&["vwacp", "--cwd=child", "--suppress-reads"]),
        PathBuf::from("/workspace"),
    );
    assert_eq!(path_buf_plan.initial_cwd, PathBuf::from("/workspace/child"));
    assert!(path_buf_plan.suppress_reads);

    let path_buf_queue_plan = build_cli_bootstrap_plan(
        &argv(&["vwacp", QUEUE_OWNER_PROCESS_MARKER]),
        PathBuf::from("/workspace"),
    );
    assert_eq!(path_buf_queue_plan.perf_capture_role, PerfCaptureRole::QueueOwner);
}

#[test]
fn bootstrap_plan_uses_safe_defaults_for_empty_argv() {
    let plan = build_cli_bootstrap_plan(&[], "/work/./repo/../project");

    assert!(plan.cli_args.is_empty());
    assert_eq!(plan.perf_capture_role, PerfCaptureRole::Cli);
    assert!(!plan.print_version);
    assert!(!plan.queue_owner_mode);
    assert!(!plan.should_handle_skillflag);
    assert_eq!(plan.initial_cwd, PathBuf::from("/work/project"));
    assert!(!plan.requested_json_strict);
    assert!(!plan.suppress_reads);
}

#[test]
fn requested_output_format_stops_at_argument_separator() {
    assert_eq!(
        detect_requested_output_format(
            &argv(&["--format", "json", "--", "--format", "quiet"]),
            OutputFormat::Text
        ),
        OutputFormat::Json
    );
    assert!(!detect_json_strict(&argv(&["--", "--json-strict"])));
    assert!(detect_json_strict(&argv(&["--json-strict=false"])));
    assert_eq!(
        detect_requested_output_format(&argv(&["--format", "unknown"]), OutputFormat::Quiet),
        OutputFormat::Quiet
    );
    assert_eq!(
        detect_requested_output_format(&argv(&["--format=", "--format=quiet"]), OutputFormat::Text),
        OutputFormat::Quiet
    );
    assert_eq!(
        detect_requested_output_format(&argv(&["--json-strict"]), OutputFormat::Text),
        OutputFormat::Json
    );
    assert_eq!(
        detect_requested_output_format(
            &argv(&["--format", "json", "--format=quiet"]),
            OutputFormat::Text
        ),
        OutputFormat::Quiet
    );
}

#[test]
fn requested_output_format_handles_missing_invalid_and_trimmed_inline_values() {
    assert_eq!(
        detect_requested_output_format(&argv(&["--format"]), OutputFormat::Quiet),
        OutputFormat::Quiet
    );
    assert_eq!(
        detect_requested_output_format(&argv(&["--format", "invalid"]), OutputFormat::Json),
        OutputFormat::Json
    );
    assert_eq!(
        detect_requested_output_format(&argv(&["--format= json "]), OutputFormat::Text),
        OutputFormat::Json
    );
    assert_eq!(
        detect_requested_output_format(&argv(&["--format=invalid"]), OutputFormat::Quiet),
        OutputFormat::Quiet
    );
}

#[test]
fn output_policy_applies_json_strict_and_suppress_reads() {
    let policy = resolve_requested_output_policy(OutputFormat::Json, true, true);

    assert_eq!(
        policy,
        OutputPolicy {
            format: OutputFormat::Json,
            json_strict: true,
            suppress_reads: true,
            suppress_non_json_stderr: true,
            queue_error_already_emitted: true,
            suppress_sdk_console_errors: true,
        }
    );
}

#[test]
fn permission_exit_code_only_changes_when_all_requests_denied_or_cancelled() {
    assert_eq!(
        apply_permission_exit_code(
            EXIT_CODE_ERROR,
            &PermissionStats { requested: 1, approved: 0, denied: 1, cancelled: 0 }
        ),
        EXIT_CODE_PERMISSION_DENIED
    );
    assert_eq!(
        apply_permission_exit_code(
            EXIT_CODE_ERROR,
            &PermissionStats { requested: 1, approved: 1, denied: 1, cancelled: 0 }
        ),
        EXIT_CODE_ERROR
    );
    assert_eq!(
        apply_permission_exit_code(
            EXIT_CODE_ERROR,
            &PermissionStats { requested: 1, approved: 0, denied: 0, cancelled: 1 }
        ),
        EXIT_CODE_PERMISSION_DENIED
    );
    assert_eq!(
        apply_permission_exit_code(
            EXIT_CODE_ERROR,
            &PermissionStats { requested: 0, approved: 0, denied: 1, cancelled: 0 }
        ),
        EXIT_CODE_ERROR
    );
    assert_eq!(
        apply_permission_exit_code(
            0,
            &PermissionStats { requested: 2, approved: 0, denied: 1, cancelled: 1 }
        ),
        EXIT_CODE_PERMISSION_DENIED
    );
    assert_eq!(
        apply_permission_exit_code(
            0,
            &PermissionStats { requested: 2, approved: 0, denied: 0, cancelled: 0 }
        ),
        0
    );
}

#[test]
fn runtime_plan_respects_cli_format_over_config() {
    let plan = build_cli_runtime_plan(&argv(&["--format=json", "status"]), &config());

    assert_eq!(plan.requested_output_format, OutputFormat::Json);
    assert!(plan.public_cli_plan.dynamic_agent_command.is_none());
}

#[test]
fn runtime_plan_applies_json_strict_and_suppress_reads_flags() {
    let plan =
        build_cli_runtime_plan(&argv(&["--json-strict", "--suppress-reads", "status"]), &config());

    assert_eq!(plan.requested_output_format, OutputFormat::Json);
    assert_eq!(
        plan.requested_output_policy,
        OutputPolicy {
            format: OutputFormat::Json,
            json_strict: true,
            suppress_reads: true,
            suppress_non_json_stderr: true,
            queue_error_already_emitted: true,
            suppress_sdk_console_errors: true,
        }
    );
}

#[test]
fn top_level_verbs_are_exposed_as_owned_set() {
    let verbs = top_level_verbs();

    assert!(verbs.contains("prompt"));
    assert!(verbs.contains("status"));
    assert_eq!(verbs.len(), TOP_LEVEL_VERBS.len());
}

#[test]
fn version_and_queue_owner_detection_use_command_argv() {
    assert!(is_version_requested(&argv(&["vwacp", "-V"])));
    assert!(is_version_requested(&argv(&["vwacp", "--version"])));
    assert!(!is_version_requested(&argv(&["vwacp", "status"])));
    assert!(is_queue_owner_mode(&argv(&["node", "bin/acp.mjs", QUEUE_OWNER_PROCESS_MARKER])));
    assert!(!is_queue_owner_mode(&argv(&["vwacp", "status", QUEUE_OWNER_PROCESS_MARKER])));
    assert!(should_maybe_handle_skillflag(&argv(&["vwacp", "--skill"])));
    assert!(should_maybe_handle_skillflag(&argv(&["vwacp", "--skill=rust"])));
    assert!(!should_maybe_handle_skillflag(&argv(&["vwacp", "--skills"])));
}

#[test]
fn detect_initial_cwd_handles_missing_values_separator_and_absolute_paths() {
    assert_eq!(detect_initial_cwd(&argv(&["--cwd"]), "/work/repo"), PathBuf::from("/work/repo"));
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd", "--"]), "/work/repo"),
        PathBuf::from("/work/repo")
    );
    assert_eq!(detect_initial_cwd(&argv(&["--cwd="]), "/work/repo"), PathBuf::from("/work/repo"));
    assert_eq!(
        detect_initial_cwd(&argv(&["--", "--cwd", "other"]), "/work/./repo"),
        PathBuf::from("/work/repo")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd=/tmp/../var/app"]), "/work/repo"),
        PathBuf::from("/var/app")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd=child"]), PathBuf::from("/work/repo")),
        PathBuf::from("/work/repo/child")
    );
    assert_eq!(normalize_path_like_node(PathBuf::from("./repo")), PathBuf::from("repo"));
    assert_eq!(normalize_path_like_node(PathBuf::from("..")), PathBuf::from(Path::new("/")));
    assert_eq!(normalize_path_like_node(PathBuf::new()), PathBuf::from(Path::new("/")));

    let current_dir = PathBuf::from("/work/repo");
    assert_eq!(detect_initial_cwd(&argv(&["--cwd"]), current_dir.clone()), current_dir);
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd=child"]), PathBuf::from("/work/repo")),
        PathBuf::from("/work/repo/child")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd", "child"]), PathBuf::from("/work/repo")),
        PathBuf::from("/work/repo/child")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd="]), PathBuf::from("/work/repo")),
        PathBuf::from("/work/repo")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--"]), PathBuf::from("/work/repo")),
        PathBuf::from("/work/repo")
    );
}

#[test]
fn detect_initial_cwd_scans_options_and_trims_inline_value() {
    assert_eq!(
        detect_initial_cwd(
            &argv(&["--format", "json", "--cwd", "./child/../repo/./nested"]),
            "/work/base"
        ),
        PathBuf::from("/work/base/repo/nested")
    );
    assert_eq!(
        detect_initial_cwd(&argv(&["--cwd= ./child/../next "]), "/work/base"),
        PathBuf::from("/work/base/next")
    );
}

#[test]
fn compatible_config_id_only_rewrites_codex_thought_level() {
    assert_eq!(
        resolve_compatible_config_id("codex", "codex acp", "thought_level"),
        "reasoning_effort"
    );
    assert_eq!(
        resolve_compatible_config_id("other", "codex acp", "thought_level"),
        "thought_level"
    );
    assert_eq!(resolve_compatible_config_id("codex", "codex acp", "model"), "model");
}

#[tokio::test]
async fn read_prompt_prefers_argument_text_and_reports_tty_empty() {
    let prompt = read_prompt(&argv(&["  hello", "world  "]), None, "/work", true)
        .await
        .expect("argument prompt should parse");

    assert_eq!(prompt_to_display_text(&prompt), "hello world");
    assert_eq!(read_prompt(&[], None, "/work", true).await, Err(CliCoreError::PromptRequired));

    let prompt = read_prompt(&argv(&["from", "string cwd"]), None, "/work".to_string(), true)
        .await
        .expect("argument prompt should parse with owned cwd");
    assert_eq!(prompt_to_display_text(&prompt), "from string cwd");

    let prompt = read_prompt(&argv(&["from", "pathbuf cwd"]), None, PathBuf::from("/work"), true)
        .await
        .expect("argument prompt should parse with owned path cwd");
    assert_eq!(prompt_to_display_text(&prompt), "from pathbuf cwd");
}

#[tokio::test]
async fn read_prompt_reads_relative_file_and_appends_argument_text() {
    let dir = unique_test_dir("file");
    std::fs::create_dir_all(dir.join("prompts")).expect("test dir should be created");
    std::fs::write(dir.join("prompts/source.txt"), "from file\n")
        .expect("prompt file should be written");

    let prompt = read_prompt(&argv(&["suffix"]), Some("prompts/../prompts/source.txt"), &dir, true)
        .await
        .expect("file prompt should parse");

    assert_eq!(prompt_to_display_text(&prompt), "from file\n\nsuffix");
    assert_eq!(
        read_prompt_file_source("prompts/source.txt", &dir).await.expect("file source should read"),
        "from file\n"
    );
    std::fs::remove_dir_all(dir).expect("test dir should be removed");
}

#[tokio::test]
async fn read_prompt_reads_absolute_file_path_with_parent_segments() {
    let dir = unique_test_dir("absolute-file");
    std::fs::create_dir_all(dir.join("prompts")).expect("test dir should be created");
    let source_path = dir.join("prompts").join("source.txt");
    std::fs::write(&source_path, "from absolute file").expect("prompt file should be written");
    let absolute_path = dir.join("prompts").join("..").join("prompts").join("source.txt");
    let absolute_path =
        absolute_path.to_str().expect("temp path should be valid utf-8 for this test");

    let prompt = read_prompt(&argv(&["suffix"]), Some(absolute_path), "/ignored", true)
        .await
        .expect("absolute file prompt should parse");

    assert_eq!(prompt_to_display_text(&prompt), "from absolute file\n\nsuffix");
    std::fs::remove_dir_all(dir).expect("test dir should be removed");
}

#[tokio::test]
async fn read_prompt_covers_owned_cwd_variants() {
    let dir = unique_test_dir("owned-cwd");
    std::fs::create_dir_all(&dir).expect("test dir should be created");
    std::fs::write(dir.join("source.txt"), "from owned cwd")
        .expect("prompt file should be written");
    std::fs::write(dir.join("empty.txt"), " \n").expect("empty prompt file should be written");

    let string_cwd = dir.to_string_lossy().to_string();
    let prompt = read_prompt(&argv(&["suffix"]), Some("source.txt"), string_cwd.clone(), true)
        .await
        .expect("string cwd file prompt should parse");
    assert_eq!(prompt_to_display_text(&prompt), "from owned cwd\n\nsuffix");
    assert_eq!(
        read_prompt(&[], Some("empty.txt"), string_cwd.clone(), true).await,
        Err(CliCoreError::PromptFileEmpty)
    );
    assert_eq!(
        read_prompt(&[], None, string_cwd.clone(), true).await,
        Err(CliCoreError::PromptRequired)
    );
    assert_eq!(
        read_prompt_with_empty_stdin(&[], None, string_cwd, false).await,
        Err(CliCoreError::PromptStdinEmpty)
    );

    let path_buf_cwd = dir.clone();
    let prompt = read_prompt(&argv(&["suffix"]), Some("source.txt"), path_buf_cwd.clone(), true)
        .await
        .expect("path cwd file prompt should parse");
    assert_eq!(prompt_to_display_text(&prompt), "from owned cwd\n\nsuffix");
    assert_eq!(
        read_prompt(&[], Some("empty.txt"), path_buf_cwd.clone(), true).await,
        Err(CliCoreError::PromptFileEmpty)
    );
    assert_eq!(
        read_prompt(&[], None, path_buf_cwd.clone(), true).await,
        Err(CliCoreError::PromptRequired)
    );
    assert_eq!(
        read_prompt_with_empty_stdin(&[], None, path_buf_cwd, false).await,
        Err(CliCoreError::PromptStdinEmpty)
    );

    let prompt = read_prompt(&argv(&["from", "ref cwd"]), None, &dir, true)
        .await
        .expect("referenced cwd prompt should parse");
    assert_eq!(prompt_to_display_text(&prompt), "from ref cwd");

    std::fs::remove_dir_all(dir).expect("test dir should be removed");
}

#[tokio::test]
async fn read_prompt_reports_file_errors_and_empty_file() {
    let dir = unique_test_dir("errors");
    std::fs::create_dir_all(&dir).expect("test dir should be created");
    std::fs::write(dir.join("empty.txt"), "   \n").expect("empty prompt file should be written");
    std::fs::write(
        dir.join("invalid.json"),
        r#"[{"type":"image","mimeType":"text/plain","data":"AA=="}]"#,
    )
    .expect("invalid prompt file should be written");

    assert_eq!(
        read_prompt(&[], Some("empty.txt"), &dir, true).await,
        Err(CliCoreError::PromptFileEmpty)
    );
    assert!(matches!(
        read_prompt(&[], Some("missing.txt"), &dir, true).await,
        Err(CliCoreError::Io(message)) if !message.is_empty()
    ));
    assert!(matches!(
        read_prompt(&[], Some("invalid.json"), &dir, true).await,
        Err(CliCoreError::PromptInputValidation(message))
            if message.contains("mimeType must start with image/")
    ));

    std::fs::remove_dir_all(dir).expect("test dir should be removed");
}

#[tokio::test]
async fn read_prompt_reports_empty_stdin_for_file_and_piped_prompt() {
    assert_eq!(
        read_prompt_with_empty_stdin(&[], Some("-"), "/work", true).await,
        Err(CliCoreError::PromptFileEmpty)
    );
    assert_eq!(
        read_prompt_file_source_with_stdin_reader("-", Path::new("/work"), empty_stdin)
            .await
            .expect("stdin source should read"),
        ""
    );
    assert_eq!(
        read_prompt_with_empty_stdin(&[], None, "/work", false).await,
        Err(CliCoreError::PromptStdinEmpty)
    );
}

#[tokio::test]
async fn read_prompt_reads_stdin_for_file_dash_and_piped_prompt() {
    let prompt = read_prompt_with_stdin_reader(&[], Some("-"), "/work", true, text_stdin)
        .await
        .expect("stdin file prompt should parse");
    assert_eq!(prompt_to_display_text(&prompt), "from stdin");

    let prompt =
        read_prompt_with_stdin_reader(&argv(&["suffix"]), Some("-"), "/work", true, text_stdin)
            .await
            .expect("stdin file prompt should merge argument text");
    assert_eq!(prompt_to_display_text(&prompt), "from stdin\n\nsuffix");

    let prompt = read_prompt_with_stdin_reader(&[], None, "/work", false, text_stdin)
        .await
        .expect("piped stdin prompt should parse");
    assert_eq!(prompt_to_display_text(&prompt), "from stdin");
}

#[tokio::test]
async fn read_prompt_preserves_stdin_reader_errors() {
    assert_eq!(
        read_prompt_with_stdin_reader(&[], Some("-"), "/work", true, failing_stdin).await,
        Err(CliCoreError::Io("stdin unavailable".to_string()))
    );
    assert_eq!(
        read_prompt_with_stdin_reader(&[], None, "/work", false, failing_stdin).await,
        Err(CliCoreError::Io("stdin unavailable".to_string()))
    );
}

#[test]
fn prompt_from_stdin_source_accepts_non_empty_text() {
    let prompt = prompt_from_stdin_source(" stdin text \n").expect("stdin text should parse");

    assert_eq!(prompt_to_display_text(&prompt), "stdin text");
}

#[test]
fn prompt_from_stdin_helpers_preserve_errors() {
    assert_eq!(
        prompt_from_stdin_result(Ok(" \n".to_string())),
        Err(CliCoreError::PromptStdinEmpty)
    );
    assert_eq!(
        prompt_from_stdin_result(Err(CliCoreError::Io("stdin failed".to_string()))),
        Err(CliCoreError::Io("stdin failed".to_string()))
    );
    let ok_source: Result<Result<String, std::io::Error>, &str> = Ok(Ok("stdin text".to_string()));
    assert_eq!(flatten_stdin_read_result(ok_source), Ok("stdin text".to_string()));
    let join_error: Result<Result<String, std::io::Error>, &str> = Err("join failed");
    assert_eq!(
        flatten_stdin_read_result(join_error),
        Err(CliCoreError::Io("join failed".to_string()))
    );
    let io_error: Result<Result<String, std::io::Error>, &str> = Ok(Err(Error::other("io failed")));
    assert_eq!(flatten_stdin_read_result(io_error), Err(CliCoreError::Io("io failed".to_string())));
    assert!(matches!(
        read_prompt_input_from_reader(&mut FailingReader),
        Err(error) if error.to_string() == "reader failed"
    ));
    let mut reader = std::io::Cursor::new("reader text");
    assert_eq!(
        read_prompt_input_from_reader(&mut reader).expect("reader should succeed"),
        "reader text"
    );
    assert!(matches!(
        prompt_from_stdin_source(r#"[{"type":"image","mimeType":"text/plain","data":"AA=="}]"#),
        Err(CliCoreError::PromptInputValidation(message))
            if message.contains("mimeType must start with image/")
    ));
}

#[test]
fn command_arg_offset_handles_empty_and_binary_only() {
    assert_eq!(command_arg_offset(&[]), 0);
    assert_eq!(command_arg_offset(&argv(&["vwacp"])), 1);
    assert_eq!(command_arg_offset(&argv(&["vwacp", "prompt"])), 1);
    assert_eq!(command_arg_offset(&argv(&["unknown", "bin/acp.js"])), 2);
    assert_eq!(command_arg_offset(&argv(&["node", "prompt"])), 2);
}
