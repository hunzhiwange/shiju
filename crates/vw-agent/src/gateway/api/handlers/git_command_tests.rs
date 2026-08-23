// 覆盖受限 Git 命令的请求校验、执行及输出规范化。

use super::*;
use vw_api_types::git::GitCommandRequest;

struct TestRepo {
    temp: tempfile::TempDir,
}

impl TestRepo {
    fn path_str(&self) -> &str {
        self.temp.path().to_str().expect("utf8 path")
    }
}

fn run_git(directory: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n").replace('\r', "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n").replace('\r', "\n");
    if output.status.success() {
        return Ok(stdout);
    }
    let detail = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    };
    if detail.is_empty() {
        return Err(format!("git command failed: {}", args.join(" ")));
    }
    Err(detail)
}

fn init_repo() -> TestRepo {
    let temp = tempfile::tempdir().expect("repo dir");
    let repo = TestRepo { temp };
    run_git(repo.path_str(), &["init"]).expect("git init");
    run_git(repo.path_str(), &["config", "user.email", "test@example.com"]).expect("email");
    run_git(repo.path_str(), &["config", "user.name", "Test User"]).expect("name");
    repo
}

// 命令参数先经白名单校验，随后才在临时仓库执行，避免测试放宽生产安全边界。
#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

#[tokio::test]
async fn git_command_v1_rejects_invalid_request_before_spawning_command() {
    let request = GitCommandRequest {
        directory: " ".to_string(),
        args: vec!["status".to_string()],
        timeout_secs: None,
    };

    let error = git_command_v1(Json(request)).await.expect_err("invalid command request");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "git directory is required");
}

#[tokio::test]
async fn git_command_v1_runs_valid_git_command() {
    let repo = init_repo();
    let request = GitCommandRequest {
        directory: repo.path_str().to_string(),
        args: vec!["status".to_string(), "--short".to_string()],
        timeout_secs: Some(5),
    };

    let Json(response) = git_command_v1(Json(request)).await.expect("valid git command");

    assert!(response.success);
    assert_eq!(response.code, Some(0));
    assert_eq!(response.stdout, "");
}

#[test]
fn validate_git_command_request_checks_required_fields() {
    let temp = tempfile::tempdir().expect("temp dir");
    let valid = GitCommandRequest {
        directory: temp.path().to_string_lossy().to_string(),
        args: vec!["status".to_string()],
        timeout_secs: Some(1),
    };
    assert!(validate_git_command_request(&valid).is_ok());

    assert!(
        validate_git_command_request(&GitCommandRequest {
            directory: String::new(),
            ..valid.clone()
        })
        .is_err()
    );
    assert!(
        validate_git_command_request(&GitCommandRequest {
            directory: "/definitely/missing/vw-git".to_string(),
            ..valid.clone()
        })
        .is_err()
    );
    assert!(
        validate_git_command_request(&GitCommandRequest { args: Vec::new(), ..valid.clone() })
            .is_err()
    );
    assert!(
        validate_git_command_request(&GitCommandRequest {
            args: vec!["status\0bad".to_string()],
            ..valid.clone()
        })
        .is_err()
    );
    assert!(
        validate_git_command_request(&GitCommandRequest {
            directory: format!("{}\0bad", temp.path().to_string_lossy()),
            ..valid.clone()
        })
        .is_err()
    );
    assert!(
        validate_git_command_request(&GitCommandRequest { timeout_secs: Some(0), ..valid })
            .is_err()
    );
}

#[test]
fn normalize_git_output_bytes_normalizes_crlf_and_cr() {
    assert_eq!(normalize_git_output_bytes(b"a\r\nb\rc"), "a\nb\nc");
}

#[test]
fn normalize_git_output_bytes_replaces_invalid_utf8_lossily() {
    assert_eq!(normalize_git_output_bytes(b"ok\xff\r\n"), "ok\u{fffd}\n");
}

#[test]
fn run_git_command_blocking_returns_stdout_and_status() {
    let repo = init_repo();

    let response = run_git_command_blocking(&GitCommandRequest {
        directory: repo.path_str().to_string(),
        args: vec!["status".to_string(), "--short".to_string()],
        timeout_secs: None,
    })
    .expect("git status");

    assert!(response.success);
    assert_eq!(response.code, Some(0));
    assert_eq!(response.stdout, "");
}

#[test]
fn run_git_command_blocking_returns_nonzero_status_and_stderr() {
    let repo = init_repo();

    let response = run_git_command_blocking(&GitCommandRequest {
        directory: repo.path_str().to_string(),
        args: vec![
            "rev-parse".to_string(),
            "--verify".to_string(),
            "refs/heads/missing".to_string(),
        ],
        timeout_secs: None,
    })
    .expect("git failure response");

    assert!(!response.success);
    assert_ne!(response.code, Some(0));
    assert!(!response.stderr.trim().is_empty());
}
