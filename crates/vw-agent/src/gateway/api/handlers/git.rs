//! Git 提交网关路由模块

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path};
use std::process::Stdio;

use axum::Json;
use axum::Router;
use axum::routing::post;
use base64::Engine;
use similar::TextDiff;
use tempfile::NamedTempFile;
use vw_api_types::git::{
    GitCommandRequest, GitCommandResponse, GitCommitDto, GitCommitRequest, GitCommitResponse,
    GitMergeRequest, GitMergeResponse,
};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::with_instance;
use crate::app::agent::project;
use crate::app::agent::shell::git_std_command;
use crate::worktree;

const DIFF_CONTEXT: usize = 3;

#[derive(Default)]
struct GitFileSelection {
    stage_file: bool,
    hunks: BTreeSet<usize>,
    new_lines: BTreeSet<usize>,
    old_lines: BTreeSet<usize>,
}

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/git/commit", post(git_commit_v1))
        .route("/git/command", post(git_command_v1))
        .route("/git/merge", post(git_merge_v1))
}

async fn git_commit_v1(
    Json(body): Json<GitCommitRequest>,
) -> Result<Json<GitCommitResponse>, ApiError> {
    let message = body.message.trim().to_string();
    if message.is_empty() {
        return Err(ApiError::bad_request("commit message is required"));
    }

    let directory = resolve_git_directory(&body.project_id.0, body.worktree_id.as_ref()).await?;
    let request = GitCommitRequest { message, ..body };
    let response =
        tokio::task::spawn_blocking(move || commit_selected_blocking(&directory, &request))
            .await
            .map_err(|err| ApiError::internal(format!("git commit task failed: {err}")))?
            .map_err(ApiError::bad_request)?;

    Ok(Json(response))
}

async fn git_command_v1(
    Json(body): Json<GitCommandRequest>,
) -> Result<Json<GitCommandResponse>, ApiError> {
    validate_git_command_request(&body)?;

    let response = tokio::task::spawn_blocking(move || run_git_command_blocking(&body))
        .await
        .map_err(|err| ApiError::internal(format!("git command task failed: {err}")))?
        .map_err(ApiError::bad_request)?;

    Ok(Json(response))
}

async fn git_merge_v1(
    Json(body): Json<GitMergeRequest>,
) -> Result<Json<GitMergeResponse>, ApiError> {
    let source_branch = body.source_branch.trim().to_string();
    let target_branch = body.target_branch.trim().to_string();
    validate_merge_branch_pair(&source_branch, &target_branch).map_err(ApiError::bad_request)?;

    let directory = resolve_git_directory(&body.project_id.0, None).await?;
    let response = tokio::task::spawn_blocking(move || {
        merge_branch_blocking(&directory, &source_branch, &target_branch)
    })
    .await
    .map_err(|err| ApiError::internal(format!("git merge task failed: {err}")))?
    .map_err(ApiError::bad_request)?;

    Ok(Json(response))
}

fn validate_git_command_request(request: &GitCommandRequest) -> Result<(), ApiError> {
    if request.directory.trim().is_empty() {
        return Err(ApiError::bad_request("git directory is required"));
    }
    if !Path::new(&request.directory).is_dir() {
        return Err(ApiError::bad_request("git directory does not exist"));
    }
    if request.args.is_empty() {
        return Err(ApiError::bad_request("git args are required"));
    }
    if request.args.iter().any(|arg| arg.contains('\0')) {
        return Err(ApiError::bad_request("git args contain invalid null byte"));
    }
    if request.directory.contains('\0') {
        return Err(ApiError::bad_request("git directory contains invalid null byte"));
    }
    if matches!(request.timeout_secs, Some(0)) {
        return Err(ApiError::bad_request("git timeout must be positive"));
    }
    Ok(())
}

fn run_git_command_blocking(request: &GitCommandRequest) -> Result<GitCommandResponse, String> {
    let output = git_std_command()
        .current_dir(&request.directory)
        .args(&request.args)
        .output()
        .map_err(|err| err.to_string())?;

    Ok(GitCommandResponse {
        success: output.status.success(),
        code: output.status.code(),
        stdout: normalize_git_output_bytes(&output.stdout),
        stderr: normalize_git_output_bytes(&output.stderr),
    })
}

async fn resolve_git_directory(
    project_id: &str,
    worktree_id: Option<&vw_api_types::id::WorktreeId>,
) -> Result<String, ApiError> {
    let info = load_project(project_id).await?;
    let project_directory = project_context_directory(&info)?;

    let Some(worktree_id) = worktree_id else {
        return Ok(project_directory);
    };

    let requested_directory = directory_from_worktree_id(&worktree_id.0)?;
    let allowed_directories = worktree_list_for_project(&info).await?;
    let requested_norm = normalize_path(&requested_directory);
    if allowed_directories.iter().any(|path| normalize_path(path) == requested_norm) {
        return Ok(requested_directory);
    }

    Err(ApiError::not_found("worktree not found"))
}

async fn load_project(project_id: &str) -> Result<project::Info, ApiError> {
    crate::app::agent::gateway::instance::load_local_project(project_id).await
}

fn project_context_directory(info: &project::Info) -> Result<String, ApiError> {
    if !info.worktree.trim().is_empty() {
        return Ok(info.worktree.clone());
    }
    info.sandboxes
        .iter()
        .find(|path| !path.trim().is_empty())
        .cloned()
        .ok_or_else(|| ApiError::bad_request("project directory missing"))
}

async fn worktree_list_for_project(info: &project::Info) -> Result<Vec<String>, ApiError> {
    if !matches!(info.vcs, Some(project::Vcs::Git)) {
        return Ok(Vec::new());
    }

    let context_directory = project_context_directory(info)?;
    let primary = normalize_path(&context_directory);
    let mut directories: Vec<String> = with_instance(context_directory, move || {
        Box::pin(async move {
            let result: Result<Vec<String>, worktree::Error> = worktree::list_directories().await;
            result.map_err(|e: worktree::Error| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;
    directories.sort();
    directories.dedup_by(|left, right| normalize_path(left) == normalize_path(right));
    if !directories.iter().any(|path| normalize_path(path) == primary) {
        directories.insert(0, info.worktree.clone());
    }
    Ok(directories)
}

fn directory_from_worktree_id(worktree_id: &str) -> Result<String, ApiError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(worktree_id)
        .map_err(|_| ApiError::not_found("worktree not found"))?;
    String::from_utf8(bytes).map_err(|_| ApiError::not_found("worktree not found"))
}

fn normalize_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    normalized
        .trim_end_matches('/')
        .trim_start_matches('/')
        .strip_prefix("./")
        .unwrap_or_else(|| normalized.trim_end_matches('/').trim_start_matches('/'))
        .to_string()
}

fn commit_selected_blocking(
    directory: &str,
    request: &GitCommitRequest,
) -> Result<GitCommitResponse, String> {
    if request.stage_all {
        git_stage_all(directory)?;
    } else {
        stage_selected_changes(directory, request)?;
    }

    if !git_has_staged_changes(directory)? {
        return Ok(GitCommitResponse {
            ok: true,
            created: false,
            commit: GitCommitDto {
                sha: git_head_sha(directory)?,
                message: request.message.clone(),
            },
        });
    }

    git_commit_with_message_file(directory, &request.message)?;
    let sha = git_head_sha(directory)?;

    Ok(GitCommitResponse {
        ok: true,
        created: true,
        commit: GitCommitDto { sha, message: request.message.clone() },
    })
}

fn stage_selected_changes(directory: &str, request: &GitCommitRequest) -> Result<(), String> {
    let selections = collect_file_selections(request)?;
    if selections.is_empty() {
        return Err("no changes selected".to_string());
    }

    git_reset_index(directory)?;
    for (path, selection) in selections {
        if selection.stage_file {
            git_stage_file(directory, &path)?;
        } else {
            git_stage_partial_file(directory, &path, &selection)?;
        }
    }
    Ok(())
}

fn collect_file_selections(
    request: &GitCommitRequest,
) -> Result<BTreeMap<String, GitFileSelection>, String> {
    let mut selections = BTreeMap::<String, GitFileSelection>::new();

    for path in &request.selected_files {
        validate_repo_relative_path(path)?;
        selections.entry(path.clone()).or_default().stage_file = true;
    }
    for selection in &request.selected_hunks {
        validate_repo_relative_path(&selection.path)?;
        selections.entry(selection.path.clone()).or_default().hunks.insert(selection.index);
    }
    for selection in &request.selected_lines {
        validate_repo_relative_path(&selection.path)?;
        selections.entry(selection.path.clone()).or_default().new_lines.insert(selection.line);
    }
    for selection in &request.selected_old_lines {
        validate_repo_relative_path(&selection.path)?;
        selections.entry(selection.path.clone()).or_default().old_lines.insert(selection.line);
    }

    Ok(selections)
}

fn validate_merge_branch_pair(source_branch: &str, target_branch: &str) -> Result<(), String> {
    if source_branch.is_empty() || source_branch == "HEAD" {
        return Err("source branch is required".to_string());
    }
    if target_branch.is_empty() || target_branch == "HEAD" {
        return Err("target branch is required".to_string());
    }
    if source_branch == target_branch {
        return Err("source branch and target branch must differ".to_string());
    }
    Ok(())
}

pub(crate) fn merge_branch_blocking(
    directory: &str,
    source_branch: &str,
    target_branch: &str,
) -> Result<GitMergeResponse, String> {
    verify_local_branch(directory, source_branch)?;
    verify_local_branch(directory, target_branch)?;
    let workspace = git_worktree_path_for_branch(directory, target_branch)?
        .unwrap_or_else(|| directory.to_string());

    abort_git_in_progress_states(&workspace);
    run_git(&workspace, &["checkout", target_branch])?;
    let committed_target_changes =
        commit_target_branch_changes_if_needed(&workspace, source_branch, target_branch)?;

    if git_command_success(
        &workspace,
        &["merge-base", "--is-ancestor", source_branch, target_branch],
    )? {
        return Ok(GitMergeResponse {
            ok: true,
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            workspace,
            already_merged: true,
            message: if committed_target_changes {
                format!(
                    "saved target branch changes; {source_branch} already merged into {target_branch}"
                )
            } else {
                format!("{source_branch} already merged into {target_branch}")
            },
        });
    }

    let merge_message = format!("chore(task): merge {source_branch} into {target_branch}");
    let output = run_git_owned(
        &workspace,
        vec![
            "merge".to_string(),
            "--no-verify".to_string(),
            "--no-edit".to_string(),
            "--no-stat".to_string(),
            "-m".to_string(),
            merge_message,
            source_branch.to_string(),
        ],
    );
    match output {
        Ok(_) => Ok(GitMergeResponse {
            ok: true,
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            workspace,
            already_merged: false,
            message: if committed_target_changes {
                format!(
                    "saved target branch changes and merged {source_branch} into {target_branch}"
                )
            } else {
                format!("merged {source_branch} into {target_branch}")
            },
        }),
        Err(error) => {
            abort_git_in_progress_states(&workspace);
            Err(error)
        }
    }
}

fn commit_target_branch_changes_if_needed(
    directory: &str,
    source_branch: &str,
    target_branch: &str,
) -> Result<bool, String> {
    if git_is_clean(directory)? {
        return Ok(false);
    }

    git_stage_all(directory)?;
    if !git_has_staged_changes(directory)? {
        return Err(format!(
            "target branch {target_branch} has uncommitted changes but no staged changes after git add"
        ));
    }

    let message =
        format!("chore(task): save {target_branch} changes before merging {source_branch}");
    git_commit_no_verify_with_message_file(directory, &message)?;
    if git_is_clean(directory)? {
        Ok(true)
    } else {
        Err(format!(
            "target branch {target_branch} still has uncommitted changes after auto commit"
        ))
    }
}

fn validate_repo_relative_path(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("git selection path is empty".to_string());
    }

    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        return Err(format!("git selection path must be relative: {trimmed}"));
    }
    if candidate.components().any(|component| {
        matches!(component, Component::ParentDir | Component::Prefix(_) | Component::RootDir)
    }) {
        return Err(format!("git selection path escapes repository: {trimmed}"));
    }

    Ok(())
}

fn git_stage_file(directory: &str, file: &str) -> Result<(), String> {
    run_git(directory, &["add", "-A", "--", file]).map(|_| ())
}

fn git_stage_all(directory: &str) -> Result<(), String> {
    run_git(directory, &["add", "-A"]).map(|_| ())
}

fn git_reset_index(directory: &str) -> Result<(), String> {
    run_git(directory, &["reset", "-q"]).map(|_| ())
}

fn git_commit_with_message_file(directory: &str, message: &str) -> Result<(), String> {
    let mut temp = NamedTempFile::new().map_err(|err| err.to_string())?;
    temp.write_all(message.as_bytes()).map_err(|err| err.to_string())?;
    temp.flush().map_err(|err| err.to_string())?;
    let path = temp.path().to_string_lossy().to_string();
    run_git_owned(directory, vec!["commit".to_string(), "-F".to_string(), path]).map(|_| ())
}

fn git_commit_no_verify_with_message_file(directory: &str, message: &str) -> Result<(), String> {
    let mut temp = NamedTempFile::new().map_err(|err| err.to_string())?;
    temp.write_all(message.as_bytes()).map_err(|err| err.to_string())?;
    temp.flush().map_err(|err| err.to_string())?;
    let path = temp.path().to_string_lossy().to_string();
    run_git_owned(
        directory,
        vec!["commit".to_string(), "--no-verify".to_string(), "-F".to_string(), path],
    )
    .map(|_| ())
}

fn git_stage_partial_file(
    directory: &str,
    file_path: &str,
    selection: &GitFileSelection,
) -> Result<(), String> {
    let (old_content, new_content) = get_file_content_pair(directory, file_path)
        .ok_or_else(|| "Failed to read file content".to_string())?;
    if is_final_line_ending_only_change(&old_content, &new_content) {
        return stage_final_line_ending_only_change(directory, file_path);
    }

    let selected_content = build_selected_file_content(&old_content, &new_content, selection)?;
    if selected_content == old_content {
        return Err(format!("selected changes do not change file: {file_path}"));
    }

    write_index_content(directory, file_path, &selected_content)
}

fn build_selected_file_content(
    old_content: &str,
    new_content: &str,
    selection: &GitFileSelection,
) -> Result<String, String> {
    let old_lines = split_lines_preserve_ending(old_content);
    let new_lines = split_lines_preserve_ending(new_content);
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut selected_old_lines = selection.old_lines.clone();
    let mut selected_new_lines = selection.new_lines.clone();

    let grouped_ops = diff.grouped_ops(DIFF_CONTEXT);
    for hunk_index in &selection.hunks {
        let group = grouped_ops.get(*hunk_index).ok_or_else(|| "bad hunk index".to_string())?;
        for op in group {
            match *op {
                similar::DiffOp::Equal { .. } => {}
                similar::DiffOp::Delete { old_index, old_len, .. } => {
                    for line in old_index..old_index + old_len {
                        selected_old_lines.insert(line);
                    }
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    for line in new_index..new_index + new_len {
                        selected_new_lines.insert(line);
                    }
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    for line in old_index..old_index + old_len {
                        selected_old_lines.insert(line);
                    }
                    for line in new_index..new_index + new_len {
                        selected_new_lines.insert(line);
                    }
                }
            }
        }
    }

    let mut selected_content = String::new();
    for op in diff.ops() {
        match *op {
            similar::DiffOp::Equal { old_index, len, .. } => {
                push_line_range(&mut selected_content, &old_lines, old_index, len);
            }
            similar::DiffOp::Delete { old_index, old_len, .. } => {
                for line in old_index..old_index + old_len {
                    if !selected_old_lines.contains(&line)
                        && let Some(content) = old_lines.get(line)
                    {
                        selected_content.push_str(content);
                    }
                }
            }
            similar::DiffOp::Insert { new_index, new_len, .. } => {
                for line in new_index..new_index + new_len {
                    if selected_new_lines.contains(&line)
                        && let Some(content) = new_lines.get(line)
                    {
                        selected_content.push_str(content);
                    }
                }
            }
            similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                for line in new_index..new_index + new_len {
                    if selected_new_lines.contains(&line)
                        && let Some(content) = new_lines.get(line)
                    {
                        selected_content.push_str(content);
                    }
                }
                for line in old_index..old_index + old_len {
                    if !selected_old_lines.contains(&line)
                        && let Some(content) = old_lines.get(line)
                    {
                        selected_content.push_str(content);
                    }
                }
            }
        }
    }

    Ok(selected_content)
}

fn split_lines_preserve_ending(content: &str) -> Vec<&str> {
    if content.is_empty() { Vec::new() } else { content.split_inclusive('\n').collect() }
}

fn push_line_range(output: &mut String, lines: &[&str], start: usize, len: usize) {
    for line in start..start + len {
        if let Some(content) = lines.get(line) {
            output.push_str(content);
        }
    }
}

fn strip_final_line_ending(value: &str) -> Option<&str> {
    value.strip_suffix("\r\n").or_else(|| value.strip_suffix('\n'))
}

fn is_final_line_ending_only_change(old_content: &str, new_content: &str) -> bool {
    if old_content == new_content {
        return false;
    }

    match (strip_final_line_ending(old_content), strip_final_line_ending(new_content)) {
        (Some(old_without_newline), None) => old_without_newline == new_content,
        (None, Some(new_without_newline)) => old_content == new_without_newline,
        _ => false,
    }
}

fn stage_final_line_ending_only_change(directory: &str, file_path: &str) -> Result<(), String> {
    tracing::debug!(
        path = %file_path,
        "staging final line ending only git selection as whole file"
    );
    git_stage_file(directory, file_path)
}

fn write_index_content(directory: &str, file_path: &str, content: &str) -> Result<(), String> {
    if content.is_empty() {
        return run_git(directory, &["update-index", "--force-remove", "--", file_path])
            .map(|_| ());
    }

    let mode = git_file_mode(directory, file_path)?;
    let oid = git_hash_object_stdin(directory, content)?;
    run_git_owned(
        directory,
        vec![
            "update-index".to_string(),
            "--add".to_string(),
            "--cacheinfo".to_string(),
            mode,
            oid,
            file_path.to_string(),
        ],
    )
    .map(|_| ())
}

fn git_file_mode(directory: &str, file_path: &str) -> Result<String, String> {
    let output = run_git(directory, &["ls-files", "-s", "--", file_path])?;
    Ok(output
        .split_whitespace()
        .next()
        .filter(|mode| !mode.trim().is_empty())
        .unwrap_or("100644")
        .to_string())
}

fn git_hash_object_stdin(directory: &str, content: &str) -> Result<String, String> {
    let mut child = git_std_command()
        .current_dir(directory)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| err.to_string())?;

    let mut stdin = child.stdin.take().ok_or_else(|| "failed to open git stdin".to_string())?;
    stdin.write_all(content.as_bytes()).map_err(|err| err.to_string())?;
    drop(stdin);

    let output = child.wait_with_output().map_err(|err| err.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() { Err("git hash-object failed".to_string()) } else { Err(stderr) }
}

fn get_file_content_pair(directory: &str, file_path: &str) -> Option<(String, String)> {
    let repo = git2::Repository::open(directory).ok()?;
    let old_content = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok())
        .and_then(|tree| tree.get_path(Path::new(file_path)).ok())
        .and_then(|entry| entry.to_object(&repo).ok())
        .and_then(|obj| {
            obj.as_blob().map(|blob| String::from_utf8_lossy(blob.content()).to_string())
        })
        .unwrap_or_default();

    let new_content =
        std::fs::read_to_string(Path::new(directory).join(file_path)).unwrap_or_default();
    Some((old_content, new_content))
}

fn run_git(directory: &str, args: &[&str]) -> Result<String, String> {
    run_git_owned(directory, args.iter().map(|value| (*value).to_string()).collect())
}

fn git_head_sha(directory: &str) -> Result<String, String> {
    Ok(run_git(directory, &["rev-parse", "HEAD"])?.trim().to_string())
}

fn verify_local_branch(directory: &str, branch: &str) -> Result<(), String> {
    let branch_ref = format!("refs/heads/{branch}");
    run_git(directory, &["show-ref", "--verify", "--quiet", &branch_ref]).map(|_| ())
}

fn git_has_staged_changes(directory: &str) -> Result<bool, String> {
    Ok(!git_command_success(directory, &["diff", "--cached", "--quiet"])?)
}

fn git_is_clean(directory: &str) -> Result<bool, String> {
    Ok(run_git(directory, &["status", "--porcelain=v1"])?.trim().is_empty())
}

fn git_command_success(directory: &str, args: &[&str]) -> Result<bool, String> {
    let output = git_std_command()
        .current_dir(directory)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    Ok(output.status.success())
}

fn git_worktree_path_for_branch(directory: &str, branch: &str) -> Result<Option<String>, String> {
    let output = run_git(directory, &["worktree", "list", "--porcelain"])?;
    let mut current_path: Option<String> = None;
    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
        } else if let Some(branch_ref) = line.strip_prefix("branch ")
            && branch_ref == format!("refs/heads/{branch}")
        {
            return Ok(current_path);
        } else if line.trim().is_empty() {
            current_path = None;
        }
    }
    Ok(None)
}

fn abort_git_in_progress_states(directory: &str) {
    let _ = run_git(directory, &["merge", "--abort"]);
    let _ = run_git(directory, &["reset", "--merge"]);
}

fn run_git_owned(directory: &str, args: Vec<String>) -> Result<String, String> {
    let output = git_std_command()
        .current_dir(directory)
        .args(&args)
        .output()
        .map_err(|err| err.to_string())?;

    let stdout = normalize_git_output_bytes(&output.stdout);
    let stderr = normalize_git_output_bytes(&output.stderr);
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

fn normalize_git_output_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
