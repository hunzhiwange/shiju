//! 网关 Git 提交 / 合并路由（git2）。
//!
//! `/git/commit` 与 `/git/merge` 仅使用 libgit2，不启动系统 `git`。
//! 仍依赖 Git CLI、不在本计划范围的路径：`/v1/git/command`、普通 worktree 的
//! fetch/submodule、skill clone/pull、snapshot 等。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use axum::Json;
use axum::Router;
use axum::routing::post;
use base64::Engine;
use git2::{
    BranchType, DiffOptions, IndexAddOption, IndexEntry, IndexTime, Oid, Repository, ResetType,
    StatusOptions,
};
use similar::TextDiff;
use vw_api_types::git::{
    GitCommitDto, GitCommitRequest, GitCommitResponse, GitMergeRequest, GitMergeResponse,
};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::with_instance;
use crate::app::agent::project;
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
        .route("/git/merge", post(git_merge_v1))
        .merge(super::git_command::router())
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
    let workspace = worktree_path_for_branch(directory, target_branch)?
        .unwrap_or_else(|| directory.to_string());

    let repo = open_repo(&workspace)?;
    clear_in_progress_git_state(&repo)?;
    checkout_local_branch(&repo, target_branch)?;
    drop(repo);

    let committed_target_changes =
        commit_target_branch_changes_if_needed(&workspace, source_branch, target_branch)?;

    let repo = open_repo(&workspace)?;
    let target_oid = branch_commit_oid(&repo, target_branch)?;
    let source_commit = {
        let branch = repo
            .find_branch(source_branch, BranchType::Local)
            .map_err(|err| err.to_string())?;
        branch.into_reference().peel_to_commit().map_err(|err| err.to_string())?
    };
    let annotated = repo
        .find_annotated_commit(source_commit.id())
        .map_err(|err| err.to_string())?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).map_err(|err| err.to_string())?;

    if analysis.is_up_to_date() {
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
    let merge_result = if analysis.is_fast_forward() {
        fast_forward_branch(&repo, target_branch, &annotated)
    } else {
        create_merge_commit(&repo, &annotated, &source_commit, &merge_message)
    };

    match merge_result {
        Ok(()) => Ok(GitMergeResponse {
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
            restore_target_after_failed_merge(&repo, target_branch, target_oid)?;
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
    git_commit_with_message(directory, &message)?;
    if git_is_clean(directory)? {
        Ok(true)
    } else {
        Err(format!(
            "target branch {target_branch} still has uncommitted changes after auto commit"
        ))
    }
}

fn worktree_path_for_branch(directory: &str, branch: &str) -> Result<Option<String>, String> {
    let repo = open_repo(directory)?;
    let branch_ref = format!("refs/heads/{branch}");

    if worktree_has_branch_head(Path::new(directory), &branch_ref) {
        return Ok(Some(workdir_string(&repo, directory)));
    }

    if let Some(primary) = repo.workdir() {
        if primary != Path::new(directory) && worktree_has_branch_head(primary, &branch_ref) {
            return Ok(Some(path_to_string(primary)));
        }
    }

    let names = repo.worktrees().map_err(|err| err.to_string())?;
    for name in names.iter() {
        let Ok(Some(name)) = name else {
            continue;
        };
        let worktree = match repo.find_worktree(name) {
            Ok(worktree) => worktree,
            Err(_) => continue,
        };
        let path = worktree.path().to_path_buf();
        if worktree_has_branch_head(&path, &branch_ref) {
            return Ok(Some(path_to_string(&path)));
        }
    }
    Ok(None)
}

fn worktree_has_branch_head(path: &Path, branch_ref: &str) -> bool {
    let Ok(repo) = Repository::open(path) else {
        return false;
    };
    repo.head()
        .ok()
        .and_then(|head| head.name().ok().map(|name| name == branch_ref))
        .unwrap_or(false)
}

fn workdir_string(repo: &Repository, fallback: &str) -> String {
    repo.workdir().map(path_to_string).unwrap_or_else(|| fallback.to_string())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

fn clear_in_progress_git_state(repo: &Repository) -> Result<(), String> {
    if matches!(repo.state(), git2::RepositoryState::Clean) {
        return Ok(());
    }
    if let Ok(head) = repo.head().and_then(|head| head.peel_to_commit()) {
        let object = head.into_object();
        let _ = repo.reset(&object, ResetType::Hard, None);
    }
    repo.cleanup_state().map_err(|err| err.to_string())?;
    Ok(())
}

fn checkout_local_branch(repo: &Repository, branch: &str) -> Result<(), String> {
    let reference = format!("refs/heads/{branch}");
    let object = repo.revparse_single(&reference).map_err(|err| err.to_string())?;
    repo.checkout_tree(&object, None).map_err(|err| err.to_string())?;
    repo.set_head(&reference).map_err(|err| err.to_string())?;
    Ok(())
}

fn branch_commit_oid(repo: &Repository, branch: &str) -> Result<Oid, String> {
    repo.find_branch(branch, BranchType::Local)
        .map_err(|err| err.to_string())?
        .get()
        .peel_to_commit()
        .map_err(|err| err.to_string())
        .map(|commit| commit.id())
}

fn fast_forward_branch(
    repo: &Repository,
    target_branch: &str,
    annotated: &git2::AnnotatedCommit<'_>,
) -> Result<(), String> {
    let reference_name = format!("refs/heads/{target_branch}");
    let msg = format!("Fast-Forward: Setting {target_branch} to id: {}", annotated.id());
    match repo.find_reference(&reference_name) {
        Ok(mut reference) => {
            reference.set_target(annotated.id(), &msg).map_err(|err| err.to_string())?;
        }
        Err(err) => return Err(err.to_string()),
    }
    repo.set_head(&reference_name).map_err(|err| err.to_string())?;
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repo.checkout_head(Some(&mut checkout)).map_err(|err| err.to_string())?;
    Ok(())
}

fn create_merge_commit(
    repo: &Repository,
    annotated: &git2::AnnotatedCommit<'_>,
    source_commit: &git2::Commit<'_>,
    message: &str,
) -> Result<(), String> {
    let mut merge_opts = git2::MergeOptions::new();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repo.merge(&[annotated], Some(&mut merge_opts), Some(&mut checkout))
        .map_err(|err| err.to_string())?;

    let mut index = repo.index().map_err(|err| err.to_string())?;
    if index.has_conflicts() {
        return Err(format!(
            "merge conflict while merging {}; repository restored to the pre-merge target state",
            annotated.id()
        ));
    }

    let tree_id = index.write_tree().map_err(|err| err.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|err| err.to_string())?;
    let signature = repo.signature().map_err(|err| {
        format!("git user.name and user.email must be configured in the repository: {err}")
    })?;
    let head_commit = repo
        .head()
        .map_err(|err| err.to_string())?
        .peel_to_commit()
        .map_err(|err| err.to_string())?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&head_commit, source_commit],
    )
    .map_err(|err| err.to_string())?;
    repo.cleanup_state().map_err(|err| err.to_string())?;
    Ok(())
}

fn restore_target_after_failed_merge(
    repo: &Repository,
    target_branch: &str,
    target_oid: Oid,
) -> Result<(), String> {
    let object = repo.find_object(target_oid, None).map_err(|err| err.to_string())?;
    repo.reset(&object, ResetType::Hard, None).map_err(|err| err.to_string())?;
    let reference = format!("refs/heads/{target_branch}");
    repo.set_head(&reference).map_err(|err| err.to_string())?;
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repo.checkout_head(Some(&mut checkout)).map_err(|err| err.to_string())?;
    repo.cleanup_state().map_err(|err| err.to_string())?;
    Ok(())
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
    let repo = open_repo(directory)?;
    let mut index = repo.index().map_err(|err| err.to_string())?;
    let relative = Path::new(file);
    let absolute = Path::new(directory).join(relative);
    if absolute.exists() {
        index.add_path(relative).map_err(|err| err.to_string())?;
    } else {
        // `git add -A -- path` stages deletions when the worktree file is gone.
        let _ = index.remove_path(relative);
    }
    index.write().map_err(|err| err.to_string())?;
    Ok(())
}

fn git_stage_all(directory: &str) -> Result<(), String> {
    let repo = open_repo(directory)?;
    let mut index = repo.index().map_err(|err| err.to_string())?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|err| err.to_string())?;
    index.update_all(["*"].iter(), None).map_err(|err| err.to_string())?;
    index.write().map_err(|err| err.to_string())?;
    Ok(())
}

fn git_reset_index(directory: &str) -> Result<(), String> {
    let repo = open_repo(directory)?;
    match repo.head() {
        Ok(head) => {
            let object = head.peel_to_commit().map_err(|err| err.to_string())?.into_object();
            repo.reset(&object, ResetType::Mixed, None).map_err(|err| err.to_string())?;
        }
        Err(_) => {
            let mut index = repo.index().map_err(|err| err.to_string())?;
            index.clear().map_err(|err| err.to_string())?;
            index.write().map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn git_commit_with_message_file(directory: &str, message: &str) -> Result<(), String> {
    git_commit_with_message(directory, message)
}

fn git_commit_with_message(directory: &str, message: &str) -> Result<(), String> {
    let repo = open_repo(directory)?;
    let signature = repo.signature().map_err(|err| {
        format!("git user.name and user.email must be configured in the repository: {err}")
    })?;
    let mut index = repo.index().map_err(|err| err.to_string())?;
    let tree_id = index.write_tree().map_err(|err| err.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|err| err.to_string())?;
    let parent = match repo.head() {
        Ok(head) => Some(head.peel_to_commit().map_err(|err| err.to_string())?),
        Err(err) if err.code() == git2::ErrorCode::UnbornBranch => None,
        Err(err) => return Err(err.to_string()),
    };
    let parents: Vec<&git2::Commit<'_>> = parent.as_ref().into_iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
        .map_err(|err| err.to_string())?;
    Ok(())
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
    let repo = open_repo(directory)?;
    let mut index = repo.index().map_err(|err| err.to_string())?;
    if content.is_empty() {
        if index.get_path(Path::new(file_path), 0).is_some() {
            index.remove_path(Path::new(file_path)).map_err(|err| err.to_string())?;
            index.write().map_err(|err| err.to_string())?;
        }
        return Ok(());
    }

    let mode = index.get_path(Path::new(file_path), 0).map(|entry| entry.mode).unwrap_or(0o100644);
    let entry = IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: Oid::ZERO_SHA1,
        flags: 0,
        flags_extended: 0,
        path: file_path.as_bytes().to_vec(),
    };
    index.add_frombuffer(&entry, content.as_bytes()).map_err(|err| err.to_string())?;
    index.write().map_err(|err| err.to_string())?;
    Ok(())
}

fn git_file_mode(directory: &str, file_path: &str) -> Result<String, String> {
    let repo = open_repo(directory)?;
    let index = repo.index().map_err(|err| err.to_string())?;
    Ok(index
        .get_path(Path::new(file_path), 0)
        .map(|entry| format!("{:o}", entry.mode))
        .filter(|mode| !mode.is_empty())
        .unwrap_or_else(|| "100644".to_string()))
}

fn git_hash_object_stdin(directory: &str, content: &str) -> Result<String, String> {
    let repo = open_repo(directory)?;
    let oid = repo.blob(content.as_bytes()).map_err(|err| err.to_string())?;
    Ok(oid.to_string())
}

fn get_file_content_pair(directory: &str, file_path: &str) -> Option<(String, String)> {
    let repo = Repository::open(directory).ok()?;
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

fn open_repo(directory: &str) -> Result<Repository, String> {
    Repository::open(directory).map_err(|err| err.to_string())
}

fn git_head_sha(directory: &str) -> Result<String, String> {
    let repo = open_repo(directory)?;
    let oid = repo
        .head()
        .map_err(|err| err.to_string())?
        .peel_to_commit()
        .map_err(|err| err.to_string())?
        .id();
    Ok(oid.to_string())
}

fn verify_local_branch(directory: &str, branch: &str) -> Result<(), String> {
    let repo = open_repo(directory)?;
    repo.find_branch(branch, BranchType::Local).map(|_| ()).map_err(|err| err.to_string())
}

fn git_has_staged_changes(directory: &str) -> Result<bool, String> {
    let repo = open_repo(directory)?;
    let head_tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());
    let mut opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))
        .map_err(|err| err.to_string())?;
    Ok(diff.deltas().len() > 0)
}

fn git_is_clean(directory: &str) -> Result<bool, String> {
    let repo = open_repo(directory)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false);
    let statuses = repo.statuses(Some(&mut opts)).map_err(|err| err.to_string())?;
    Ok(statuses.is_empty())
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod git_tests;
