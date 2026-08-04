//! 提供 Git 消息处理之间共享的状态更新和任务构造辅助逻辑。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::{GitMessage, SelectedCommitRequest};
use crate::app::message::project::ProjectMessage;
use crate::app::{
    App, Message, set_config_field,
    state::{ConventionalCommitType, GitDiffLineRange, GitDiffSelectedLine, GitWorktreeOption},
};
use iced::Font;
use iced::Task;
use iced_code_editor::theme;
use std::time::Duration;

/// schedule_commit_button_animation_tick 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn schedule_commit_button_animation_tick() -> iced::Task<Message> {
    crate::app::message::after(
        Duration::from_millis(90),
        Message::Project(ProjectMessage::FileManagerRefreshAnimationTick),
    )
}

/// dismiss_preview_transient_ui 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn dismiss_preview_transient_ui(app: &mut App) {
    crate::app::message::preview::dismiss_preview_popup_menus(app);
    app.git_diff_file_menu = None;

    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::app::message::preview::lsp::clear_lsp_hover(app);
        crate::app::message::preview::lsp::clear_lsp_completion(app, false);
    }
}

/// git_context_path_for_app 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn git_context_path_for_app(app: &App) -> Option<String> {
    if let Some(dir) = app.selected_git_worktree_directory.as_ref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Some(dir.to_string());
        }
    }

    if let Some(active_id) = app.active_session_id.as_ref()
        && let Some(info) = app.sessions.iter().find(|session| &session.id == active_id)
    {
        let dir = info.directory.trim();
        if !dir.is_empty() {
            return Some(dir.to_string());
        }
    }

    app.project_path.clone()
}

/// build_selected_commit_request 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
fn build_commit_request(
    app: &App,
    stage_all: bool,
) -> Result<SelectedCommitRequest, String> {
    let summary = app.git_commit_message.trim().to_string();
    let body = app.git_commit_description.trim().to_string();
    let subject = if let Some(commit_type) = app.git_commit_type {
        let scope = app.git_commit_scope.trim();
        if scope.is_empty() {
            format!("{}: {}", commit_type.as_str(), summary)
        } else {
            format!("{}({}): {}", commit_type.as_str(), scope, summary)
        }
    } else {
        summary.clone()
    };

    let message = if body.is_empty() {
        subject.trim().to_string()
    } else {
        format!("{}\n\n{}", subject.trim(), body)
    };
    if message.trim().is_empty() {
        return Err("提交消息不能为空".to_string());
    }

    Ok(SelectedCommitRequest {
        message,
        stage_all,
        selected_files: app.staged_files_selected.clone(),
        selected_hunks: app.staged_hunks_selected.clone(),
        selected_lines: app.staged_lines_selected.clone(),
        selected_old_lines: app.staged_old_lines_selected.clone(),
    })
}

pub(super) fn build_selected_commit_request(app: &App) -> Result<SelectedCommitRequest, String> {
    build_commit_request(app, false)
}

pub(super) fn build_all_commit_request(app: &App) -> Result<SelectedCommitRequest, String> {
    build_commit_request(app, true)
}

/// reset_commit_form_state 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn reset_commit_form_state(app: &mut App) {
    app.staged_files_selected.clear();
    app.staged_hunks_selected.clear();
    app.staged_lines_selected.clear();
    app.staged_old_lines_selected.clear();
    app.clear_expanded_files();
    app.expanded_hunks.clear();
    app.context_expansions.clear();
    app.git_commit_message.clear();
    app.git_commit_type = Some(ConventionalCommitType::Feat);
    app.git_commit_scope.clear();
    app.git_commit_description.clear();
    app.git_commit_description_editor = iced::widget::text_editor::Content::new();
}

/// execute_selected_commit_via_gateway 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) async fn execute_selected_commit_via_gateway(
    project_path: String,
    request: SelectedCommitRequest,
) -> Result<(), String> {
    let context =
        crate::app::message::project::helpers::resolve_gateway_file_context(&project_path).await?;
    let client = crate::app::gateway_client()?;
    let response = client
        .git_commit(&vw_gateway_client::vw_api_types::git::GitCommitRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            message: request.message,
            stage_all: request.stage_all,
            selected_files: request.selected_files,
            selected_hunks: request
                .selected_hunks
                .into_iter()
                .map(|(path, index)| vw_gateway_client::vw_api_types::git::GitHunkSelectionDto {
                    path,
                    index,
                })
                .collect(),
            selected_lines: request
                .selected_lines
                .into_iter()
                .map(|(path, line)| vw_gateway_client::vw_api_types::git::GitLineSelectionDto {
                    path,
                    line,
                })
                .collect(),
            selected_old_lines: request
                .selected_old_lines
                .into_iter()
                .map(|(path, line)| vw_gateway_client::vw_api_types::git::GitLineSelectionDto {
                    path,
                    line,
                })
                .collect(),
        })
        .await?;
    if !response.ok {
        return Err("网关未确认提交成功".to_string());
    }
    if response.commit.sha.trim().is_empty() {
        return Err("网关返回的提交结果缺少 SHA".to_string());
    }
    Ok(())
}

pub(super) async fn execute_stage_all_via_gateway(directory: String) -> Result<(), String> {
    let response = crate::app::gateway_client()?
        .git_command(&vw_gateway_client::vw_api_types::git::GitCommandRequest {
            directory,
            args: vec!["add".to_string(), "--all".to_string()],
            timeout_secs: Some(30),
        })
        .await?;
    if response.success {
        Ok(())
    } else {
        let error = response.stderr.trim();
        Err(if error.is_empty() {
            "网关未确认 Git 暂存成功".to_string()
        } else {
            error.to_string()
        })
    }
}

/// text_too_large_for_code_editor 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn text_too_large_for_code_editor(s: &str) -> bool {
    const MAX_BYTES: usize = 1_000_000;
    const MAX_LINES: usize = 20_000;
    const MAX_LINE_BYTES: usize = 20_000;

    if s.len() > MAX_BYTES {
        return true;
    }

    for (i, line) in s.lines().enumerate() {
        if i > MAX_LINES || line.len() > MAX_LINE_BYTES || line.contains('\0') {
            return true;
        }
    }
    false
}

/// refresh_git_panel_data_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn refresh_git_panel_data_task() -> Task<Message> {
    Task::batch(vec![
        Task::done(Message::Git(GitMessage::RefreshWorktreeOptions)),
        Task::done(Message::Git(GitMessage::RefreshChangedFiles)),
        Task::done(Message::Git(GitMessage::RefreshDiffFileMetas)),
    ])
}

pub(super) fn refresh_git_panel_data_after_repo_mutation_task(app: &mut App) -> Task<Message> {
    app.git_changed_files_loading = false;
    app.git_changed_files_repo_path = None;
    app.git_diff_file_metas_loading = false;
    app.git_diff_file_metas_repo_path = None;
    app.git_diff_contents_loading.clear();
    refresh_git_panel_data_task()
}

fn normalize_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}

fn worktree_label(name: &str, branch: &str, directory: &str) -> String {
    let base = if name.trim().is_empty() {
        std::path::Path::new(directory)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("工作树")
            .to_string()
    } else {
        name.to_string()
    };

    if branch.trim().is_empty() { base } else { format!("{base} · {branch}") }
}

pub(super) async fn load_git_worktree_options(
    project_path: String,
) -> Result<Vec<GitWorktreeOption>, String> {
    let client = crate::app::gateway_client()?;
    let project =
        crate::app::message::project::helpers::resolve_gateway_project(&project_path).await?;
    if !project.git.is_repo {
        return Ok(Vec::new());
    }
    let mut options = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let project_directory = normalize_path(&project.directory);
    let main_branch = project.current_branch.clone().filter(|value| !value.trim().is_empty());
    let main_label = main_branch
        .as_ref()
        .map(|branch| format!("主工作区 · {branch}"))
        .unwrap_or_else(|| "主工作区".to_string());
    seen.insert(project_directory);
    options.push(GitWorktreeOption {
        directory: project.directory.clone(),
        label: main_label,
        branch: main_branch,
    });

    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let mut worktrees = client.project_worktrees(&project_scope).await?.items;
    worktrees.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.branch.cmp(&right.branch))
            .then(left.directory.cmp(&right.directory))
    });
    for worktree in worktrees {
        let key = normalize_path(&worktree.directory);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let branch = (!worktree.branch.trim().is_empty()).then_some(worktree.branch.clone());
        options.push(GitWorktreeOption {
            directory: worktree.directory.clone(),
            label: worktree_label(&worktree.name, &worktree.branch, &worktree.directory),
            branch,
        });
    }

    Ok(options)
}

/// take_diff_content_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn take_diff_content_task(app: &mut App, file: String) -> Option<Task<Message>> {
    if app.git_diff_contents.contains_key(&file) || app.git_diff_contents_loading.contains(&file) {
        return None;
    }

    let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app)?;
    if app.git_diff_file_metas_repo_path.as_deref() != Some(repo_path.as_str()) {
        return Some(Task::done(Message::Git(GitMessage::RefreshDiffFileMetas)));
    }

    let meta = app.git_diff_file_metas.iter().find(|meta| meta.path == file.as_str()).cloned()?;

    app.git_diff_contents_loading.insert(file.clone());

    Some(Task::perform(
        async move {
            let load_repo_path = repo_path.clone();
            let load_meta = meta.clone();
            let (old_content, new_content) = crate::app::message::spawn_blocking_opt(move || {
                Some(crate::app::components::git_panel::load_diff_content_for_repo_path(
                    &load_repo_path,
                    &load_meta,
                ))
            })
            .await
            .unwrap_or_default();
            (Some(repo_path), file, old_content, new_content)
        },
        |(repo_path, file, old_content, new_content)| {
            Message::Git(GitMessage::DiffContentReady { repo_path, file, old_content, new_content })
        },
    ))
}

/// load_missing_diff_contents 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn load_missing_diff_contents<I>(app: &mut App, files: I) -> Task<Message>
where
    I: IntoIterator<Item = String>,
{
    let tasks: Vec<_> =
        files.into_iter().filter_map(|file| take_diff_content_task(app, file)).collect();

    if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
}

/// normalize_range 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn normalize_range(mut range: GitDiffLineRange) -> GitDiffLineRange {
    if range.start > range.end {
        std::mem::swap(&mut range.start, &mut range.end);
    }
    range
}

/// configure_git_code_editor 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn configure_git_code_editor(
    theme: iced::Theme,
    font_size: f32,
    line_height: f32,
    language: iced_code_editor::i18n::Language,
    editor: &mut iced_code_editor::CodeEditor,
) {
    editor.set_font(Font::with_name("JetBrains Mono"));
    editor.set_theme(theme::from_iced_theme(&theme));
    editor.set_font_size(font_size, true);
    editor.set_line_height(line_height);
    editor.set_language(language);
    editor.set_line_numbers_enabled(false);
    editor.set_search_replace_enabled(true);
    editor.set_wrap_enabled(false);
}

#[cfg(not(target_arch = "wasm32"))]
/// git_repo_path_for_app 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn git_repo_path_for_app(app: &App) -> Option<String> {
    if let Some(dir) = app.selected_git_worktree_directory.as_ref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            let dir_path = std::path::Path::new(dir);
            if dir_path.exists() && git2::Repository::open(dir).is_ok() {
                return Some(dir.to_string());
            }
        }
        return None;
    }

    if let Some(active_id) = app.active_session_id.as_ref()
        && let Some(info) = app.sessions.iter().find(|s| &s.id == active_id)
    {
        let dir = info.directory.trim();
        if !dir.is_empty() {
            let dir_path = std::path::Path::new(dir);
            if dir_path.exists() && git2::Repository::open(dir).is_ok() {
                return Some(dir.to_string());
            }
        }
    }
    app.project_path.clone()
}

#[cfg(not(target_arch = "wasm32"))]
fn diff_side_lines_for_selection(app: &App, file: &str, is_old: bool) -> Option<Vec<String>> {
    let (old_content, new_content) = diff_contents_for_file(app, file)?;
    let content = if is_old { old_content } else { new_content };
    Some(content.lines().map(ToString::to_string).collect())
}

#[cfg(not(target_arch = "wasm32"))]
/// diff_contents_for_file 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn diff_contents_for_file(app: &App, file: &str) -> Option<(String, String)> {
    let repo_path = git_repo_path_for_app(app)?;
    let repo = git2::Repository::open(&repo_path).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;

    let old_content = if let Ok(entry) = tree.get_path(std::path::Path::new(file)) {
        let obj = entry.to_object(&repo).ok()?;
        let blob = obj.as_blob()?;
        String::from_utf8_lossy(blob.content()).to_string()
    } else {
        String::new()
    };

    let new_content =
        std::fs::read_to_string(std::path::Path::new(&repo_path).join(file)).unwrap_or_default();

    Some((old_content, new_content))
}

#[cfg(not(target_arch = "wasm32"))]
/// changed_diff_line_sets 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn changed_diff_line_sets(
    app: &App,
    file: &str,
) -> Option<(std::collections::HashSet<usize>, std::collections::HashSet<usize>)> {
    let (old_content, new_content) = diff_contents_for_file(app, file)?;
    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    let mut old_lines = std::collections::HashSet::new();
    let mut new_lines = std::collections::HashSet::new();

    for group in diff.grouped_ops(crate::app::git::DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Delete { old_index, old_len, .. } => {
                    for k in 0..old_len {
                        old_lines.insert(old_index + k);
                    }
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    for k in 0..new_len {
                        new_lines.insert(new_index + k);
                    }
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    for k in 0..old_len {
                        old_lines.insert(old_index + k);
                    }
                    for k in 0..new_len {
                        new_lines.insert(new_index + k);
                    }
                }
                similar::DiffOp::Equal { .. } => {}
            }
        }
    }

    Some((old_lines, new_lines))
}

#[cfg(not(target_arch = "wasm32"))]
/// new_position_for_old_line 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn new_position_for_old_line(app: &App, file: &str, old_idx: usize) -> Option<usize> {
    let (old_content, new_content) = diff_contents_for_file(app, file)?;
    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    let mut last_new_end = 0usize;
    let mut target_new_pos = None::<usize>;

    for group in diff.grouped_ops(crate::app::git::DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Equal { old_index, new_index, len } => {
                    if old_index <= old_idx && old_idx < old_index + len {
                        target_new_pos = Some(new_index + (old_idx - old_index));
                    }
                    last_new_end = last_new_end.max(new_index + len);
                }
                similar::DiffOp::Delete { old_index, old_len, new_index } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index);
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    last_new_end = last_new_end.max(new_index + new_len);
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index + new_len);
                }
            }
        }
    }

    Some(target_new_pos.unwrap_or(last_new_end))
}

/// selected_lines_from_range 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn selected_lines_from_range(
    _app: &App,
    _range: &GitDiffLineRange,
) -> Vec<GitDiffSelectedLine> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(lines) = diff_side_lines_for_selection(_app, &_range.file, _range.is_old) {
            let start = _range.start.min(_range.end);
            let end = _range.start.max(_range.end);
            return (start..=end)
                .filter_map(|line| {
                    lines.get(line).cloned().map(|text| GitDiffSelectedLine {
                        file: _range.file.clone(),
                        line,
                        is_old: _range.is_old,
                        text,
                    })
                })
                .collect();
        }
    }

    Vec::new()
}

fn range_contains_line(range: &GitDiffLineRange, file: &str, line: usize, is_old: bool) -> bool {
    range.file == file && range.is_old == is_old && line >= range.start && line <= range.end
}

/// diff_context_target_range 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn diff_context_target_range(app: &App) -> Option<GitDiffLineRange> {
    let range = app.git_diff_context_menu.as_ref().map(|menu| {
        if let Some(selected_range) = app.git_diff_selected_range.clone().map(normalize_range)
            && range_contains_line(&selected_range, &menu.file, menu.line, menu.is_old)
        {
            selected_range
        } else {
            GitDiffLineRange {
                file: menu.file.clone(),
                start: menu.line,
                end: menu.line,
                is_old: menu.is_old,
            }
        }
    });

    let range = range.or_else(|| app.git_diff_selected_range.clone()).or_else(|| {
        app.git_diff_selected_lines.first().map(|line| GitDiffLineRange {
            file: line.file.clone(),
            start: line.line,
            end: line.line,
            is_old: line.is_old,
        })
    })?;

    Some(normalize_range(range))
}

/// diff_context_target_stage_lines 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn diff_context_target_stage_lines(app: &App) -> Vec<(String, usize, bool)> {
    if let Some(menu) = app.git_diff_context_menu.as_ref() {
        if let Some(selected_range) = app.git_diff_selected_range.clone().map(normalize_range)
            && range_contains_line(&selected_range, &menu.file, menu.line, menu.is_old)
        {
            return (selected_range.start..=selected_range.end)
                .map(|line| (selected_range.file.clone(), line, selected_range.is_old))
                .collect();
        }

        let mut selected_lines = app
            .git_diff_selected_lines
            .iter()
            .filter(|selected| selected.file == menu.file && selected.is_old == menu.is_old)
            .map(|selected| (selected.file.clone(), selected.line, selected.is_old))
            .collect::<Vec<_>>();

        if selected_lines.iter().any(|(_, line, _)| *line == menu.line) {
            selected_lines.sort();
            selected_lines.dedup();
            return selected_lines;
        }

        return vec![(menu.file.clone(), menu.line, menu.is_old)];
    }

    if let Some(selected_range) = app.git_diff_selected_range.clone().map(normalize_range) {
        return (selected_range.start..=selected_range.end)
            .map(|line| (selected_range.file.clone(), line, selected_range.is_old))
            .collect();
    }

    let mut selected_lines = app
        .git_diff_selected_lines
        .iter()
        .map(|selected| (selected.file.clone(), selected.line, selected.is_old))
        .collect::<Vec<_>>();
    selected_lines.sort();
    selected_lines.dedup();
    selected_lines
}

/// extend_stage_selection_for_lines 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn extend_stage_selection_for_lines(app: &mut App, lines: &[(String, usize, bool)]) {
    for (file, line, is_old) in lines {
        let target = if *is_old {
            &mut app.staged_old_lines_selected
        } else {
            &mut app.staged_lines_selected
        };
        let entry = (file.clone(), *line);
        if !target.iter().any(|(selected_file, selected_line)| {
            *selected_file == entry.0 && *selected_line == entry.1
        }) {
            target.push(entry);
        }
    }

    app.staged_lines_selected.sort();
    app.staged_lines_selected.dedup();
    app.staged_old_lines_selected.sort();
    app.staged_old_lines_selected.dedup();
}

/// clear_stage_selection_for_lines 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn clear_stage_selection_for_lines(app: &mut App, lines: &[(String, usize, bool)]) {
    for (file, line, is_old) in lines {
        if *is_old {
            app.staged_old_lines_selected.retain(|(selected_file, selected_line)| {
                selected_file != file || *selected_line != *line
            });
        } else {
            app.staged_lines_selected.retain(|(selected_file, selected_line)| {
                selected_file != file || *selected_line != *line
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// normalize_stage_line_selections 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn normalize_stage_line_selections(app: &mut App) {
    app.staged_lines_selected.sort();
    app.staged_lines_selected.dedup();
    app.staged_old_lines_selected.sort();
    app.staged_old_lines_selected.dedup();
}

#[cfg(not(target_arch = "wasm32"))]
/// replace_stage_selection_for_file_lines 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn replace_stage_selection_for_file_lines(
    app: &mut App,
    file: &str,
    old_changed: &std::collections::HashSet<usize>,
    new_changed: &std::collections::HashSet<usize>,
) {
    app.staged_lines_selected.retain(|(selected_file, _)| selected_file != file);
    app.staged_old_lines_selected.retain(|(selected_file, _)| selected_file != file);

    app.staged_lines_selected
        .extend(new_changed.iter().copied().map(|line| (file.to_string(), line)));
    app.staged_old_lines_selected
        .extend(old_changed.iter().copied().map(|line| (file.to_string(), line)));
}

#[cfg(not(target_arch = "wasm32"))]
/// invert_stage_selection_for_file_lines 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn invert_stage_selection_for_file_lines(
    app: &mut App,
    file: &str,
    old_changed: &std::collections::HashSet<usize>,
    new_changed: &std::collections::HashSet<usize>,
) {
    for line in new_changed.iter().copied() {
        if let Some(pos) =
            app.staged_lines_selected.iter().position(|(selected_file, selected_line)| {
                selected_file == file && *selected_line == line
            })
        {
            app.staged_lines_selected.remove(pos);
        } else {
            app.staged_lines_selected.push((file.to_string(), line));
        }
    }

    for line in old_changed.iter().copied() {
        if let Some(pos) =
            app.staged_old_lines_selected.iter().position(|(selected_file, selected_line)| {
                selected_file == file && *selected_line == line
            })
        {
            app.staged_old_lines_selected.remove(pos);
        } else {
            app.staged_old_lines_selected.push((file.to_string(), line));
        }
    }
}

/// persist_file_tree_expanded 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn persist_file_tree_expanded(app: &App) {
    let arr = serde_json::Value::Array(
        app.file_tree_expanded.iter().cloned().map(serde_json::Value::String).collect(),
    );
    set_config_field("file_tree_expanded", arr);
}
