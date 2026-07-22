//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::process_utils::{
    build_command_failure_detail, emit_stderr_log, emit_stdout_log, tail_chars, to_shell_command,
};
use super::state::{GIT_MAINTENANCE_COMMAND_TIMEOUT_SECS, TaskLogStream};
use super::*;
use vw_gateway_client::vw_api_types::git::{GitCommandRequest, GitCommandResponse};

#[derive(Debug, Clone)]
pub(super) struct GitCommandStatus {
    success: bool,
    code: Option<i32>,
}

impl GitCommandStatus {
    pub(super) fn success(&self) -> bool {
        self.success
    }

    pub(super) fn code(&self) -> Option<i32> {
        self.code
    }
}

#[derive(Debug, Clone)]
pub(super) struct GitCommandOutput {
    pub(super) status: GitCommandStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl From<GitCommandResponse> for GitCommandOutput {
    fn from(response: GitCommandResponse) -> Self {
        Self {
            status: GitCommandStatus { success: response.success, code: response.code },
            stdout: response.stdout.into_bytes(),
            stderr: response.stderr.into_bytes(),
        }
    }
}

/// 执行 abort_git_in_progress_states 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn abort_git_in_progress_states(cwd: &str, sender: Option<&Sender<TaskLogStream>>) {
    for args in [["merge", "--abort"], ["rebase", "--abort"], ["cherry-pick", "--abort"]] {
        let args_vec = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        match run_git(cwd, &args) {
            Ok(output) if output.status.success() => {
                emit_stdout_log(
                    sender,
                    format!(
                        "[GATEWAY GIT] cwd={} cmd={} result=aborted",
                        cwd,
                        to_shell_command("git", &args_vec)
                    ),
                );
            }
            Ok(output) => {
                if is_benign_abort_failure(&args, &output) {
                    continue;
                }
                let stdout = String::from_utf8_lossy(&output.stdout)
                    .replace("\r\n", "\n")
                    .replace('\r', "\n");
                let stderr = String::from_utf8_lossy(&output.stderr)
                    .replace("\r\n", "\n")
                    .replace('\r', "\n");
                emit_stderr_log(
                    sender,
                    format!(
                        "[GATEWAY GIT] cwd={} cmd={} result=abort_failed code={:?}",
                        cwd,
                        to_shell_command("git", &args_vec),
                        output.status.code()
                    ),
                );
                if !stdout.trim().is_empty() {
                    emit_stdout_log(
                        sender,
                        format!("[GATEWAY GIT STDOUT] {}", tail_chars(&stdout, 2000)),
                    );
                }
                if !stderr.trim().is_empty() {
                    emit_stderr_log(
                        sender,
                        format!("[GATEWAY GIT STDERR] {}", tail_chars(&stderr, 2000)),
                    );
                }
            }
            Err(error) => {
                emit_stderr_log(
                    sender,
                    format!(
                        "[GATEWAY GIT] cwd={} cmd={} result=abort_error err={}",
                        cwd,
                        to_shell_command("git", &args_vec),
                        error
                    ),
                );
            }
        }
    }
}

fn is_benign_abort_failure(args: &[&str], output: &GitCommandOutput) -> bool {
    let code = output.status.code();
    if !matches!(code, Some(1) | Some(128)) {
        return false;
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    match args {
        ["merge", "--abort"] => {
            stderr.contains("merge_head")
                || stderr.contains("no merge to abort")
                || stderr.contains("there is no merge to abort")
                || stderr.contains("没有要终止的合并")
        }
        ["rebase", "--abort"] => {
            stderr.contains("no rebase in progress") || stderr.contains("没有正在进行的变基")
        }
        ["cherry-pick", "--abort"] => {
            stderr.contains("no cherry-pick or revert in progress")
                || stderr.contains("cherry-pick is not in progress")
                || stderr.contains("拣选或还原操作并未进行")
        }
        _ => false,
    }
}

fn git_maintenance_timeout() -> Duration {
    Duration::from_secs(GIT_MAINTENANCE_COMMAND_TIMEOUT_SECS)
}

/// 执行 run_git_maintenance_logged 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn run_git_maintenance_logged(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
) -> Result<GitCommandOutput, String> {
    let (output, _) = run_git_logged_with_timeout(sender, cwd, args, git_maintenance_timeout())?;
    Ok(output)
}

/// 执行 run_git 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn run_git(cwd: &str, args: &[&str]) -> Result<GitCommandOutput, String> {
    run_git_gateway(cwd, args, None)
}

/// 执行 run_git_with_timeout 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn run_git_with_timeout(
    cwd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(GitCommandOutput, Duration), String> {
    let started_at = std::time::Instant::now();
    let output = run_git_gateway(cwd, args, Some(timeout.as_secs()))?;
    Ok((output, started_at.elapsed()))
}

fn run_git_gateway(
    cwd: &str,
    args: &[&str],
    timeout_secs: Option<u64>,
) -> Result<GitCommandOutput, String> {
    #[cfg(test)]
    if let Ok(output) = run_git_local(cwd, args) {
        return Ok(output);
    }

    let client = crate::app::config::gateway_client()?;
    let request = GitCommandRequest {
        directory: cwd.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        timeout_secs,
    };
    let response = super::block_on_gateway(client.git_command(&request))
        .map_err(|e| format!("网关 git 执行失败 cwd={} args={:?} err={}", cwd, args, e))?;
    Ok(response.into())
}

#[cfg(test)]
fn run_git_local(cwd: &str, args: &[&str]) -> Result<GitCommandOutput, String> {
    let output = vw_shared::shell::git_std_command()
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|err| format!("本地 git 执行失败 cwd={} args={:?} err={}", cwd, args, err))?;
    Ok(GitCommandOutput {
        status: GitCommandStatus { success: output.status.success(), code: output.status.code() },
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// 执行 run_git_logged 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn run_git_logged(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
) -> Result<GitCommandOutput, String> {
    let args_vec = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    emit_stdout_log(
        sender,
        format!("[GATEWAY GIT] cwd={} cmd={}", cwd, to_shell_command("git", &args_vec)),
    );

    let output = run_git(cwd, args)?;
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n").replace('\r', "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n").replace('\r', "\n");
    if !stdout.trim().is_empty() {
        emit_stdout_log(sender, format!("[GATEWAY GIT STDOUT] {}", tail_chars(&stdout, 4000)));
    }
    if !stderr.trim().is_empty() {
        emit_stderr_log(sender, format!("[GATEWAY GIT STDERR] {}", tail_chars(&stderr, 4000)));
    }

    emit_stdout_log(
        sender,
        format!(
            "[GATEWAY GIT EXIT] success={} code={:?}",
            output.status.success(),
            output.status.code()
        ),
    );
    Ok(output)
}

/// 执行 run_git_logged_with_timeout 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn run_git_logged_with_timeout(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(GitCommandOutput, Duration), String> {
    let args_vec = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    emit_stdout_log(
        sender,
        format!(
            "[GATEWAY GIT] cwd={} timeout={}s cmd={}",
            cwd,
            timeout.as_secs(),
            to_shell_command("git", &args_vec)
        ),
    );

    let (output, elapsed) = run_git_with_timeout(cwd, args, timeout).map_err(|error| {
        emit_stderr_log(sender, format!("[GATEWAY GIT ERROR] {}", error));
        error
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n").replace('\r', "\n");
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n").replace('\r', "\n");
    if !stdout.trim().is_empty() {
        emit_stdout_log(sender, format!("[GATEWAY GIT STDOUT] {}", tail_chars(&stdout, 4000)));
    }
    if !stderr.trim().is_empty() {
        emit_stderr_log(sender, format!("[GATEWAY GIT STDERR] {}", tail_chars(&stderr, 4000)));
    }

    emit_stdout_log(
        sender,
        format!(
            "[GATEWAY GIT EXIT] success={} code={:?} elapsed_ms={}",
            output.status.success(),
            output.status.code(),
            elapsed.as_millis()
        ),
    );
    Ok((output, elapsed))
}

/// 执行 git_branch_name 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn git_branch_name(cwd: &str) -> Option<String> {
    let output = run_git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// 执行 git_repo_root 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn git_repo_root(cwd: &str) -> Option<String> {
    let output = run_git(cwd, &["rev-parse", "--show-toplevel"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// 执行 git_is_clean 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[cfg(test)]
pub(super) fn git_is_clean(cwd: &str) -> Result<bool, String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    if !output.status.success() {
        return Err("git status 返回失败".to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().is_empty())
}

/// 执行 git_has_staged_changes 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn git_has_staged_changes(cwd: &str) -> Result<bool, String> {
    let output = run_git(cwd, &["diff", "--cached", "--quiet"])?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(format!("git diff --cached --quiet 失败 code={:?}", output.status.code())),
    }
}

/// 执行 git_output_failure_detail 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn git_output_failure_detail(output: &GitCommandOutput) -> String {
    build_command_failure_detail(
        output.status.code(),
        None,
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
        false,
    )
}

/// 执行 build_review_diff_context 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn build_review_diff_context(
    project_path: &str,
    source_branch: Option<&str>,
    target_branch: Option<&str>,
) -> Result<(String, String, String, String, String), String> {
    const EMPTY_TREE_HASH: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    let source_ref =
        source_branch.map(str::trim).filter(|v| !v.is_empty()).unwrap_or("HEAD").to_string();
    let target_ref =
        target_branch.map(str::trim).filter(|v| !v.is_empty()).unwrap_or("HEAD").to_string();

    let source_commit_output = run_git(project_path, &["rev-parse", &source_ref])?;
    if !source_commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&source_commit_output.stderr).trim().to_string();
        return Err(format!("git rev-parse 失败 source={} err={}", source_ref, stderr));
    }
    let commit2 = String::from_utf8_lossy(&source_commit_output.stdout).trim().to_string();
    if commit2.is_empty() {
        return Err(format!("git rev-parse 未返回提交 source={}", source_ref));
    }

    let parent_output = run_git(project_path, &["rev-list", "--parents", "-n", "1", &commit2])?;
    if !parent_output.status.success() {
        let stderr = String::from_utf8_lossy(&parent_output.stderr).trim().to_string();
        return Err(format!("git rev-list 失败 commit={} err={}", commit2, stderr));
    }
    let parent_line = String::from_utf8_lossy(&parent_output.stdout).trim().to_string();
    if parent_line.is_empty() {
        return Err(format!("git rev-list 未返回提交 commit={}", commit2));
    }

    let parents = parent_line.split_whitespace().collect::<Vec<_>>();
    let commit1 =
        if parents.len() >= 2 { parents[1].to_string() } else { EMPTY_TREE_HASH.to_string() };

    let diff_output = run_git(
        project_path,
        &[
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-ext-diff",
            "--unified=3",
            &commit1,
            &commit2,
            "--",
            ".",
        ],
    )?;
    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr).trim().to_string();
        return Err(format!(
            "git diff 失败 commit1={} commit2={} err={}",
            commit1, commit2, stderr
        ));
    }
    let diff =
        String::from_utf8_lossy(&diff_output.stdout).replace("\r\n", "\n").replace('\r', "\n");

    Ok((source_ref, target_ref, commit1, commit2, diff))
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
