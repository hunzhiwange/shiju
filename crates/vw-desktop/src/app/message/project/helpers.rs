//! 提供项目消息处理共享辅助函数，集中处理路径、时间和异步任务构造。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::message::project::ProjectMessage;
use crate::app::{App, Message, state::FindInFolderMatch};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
use image::{ImageFormat, RgbaImage};
use regex::RegexBuilder;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vw_gateway_client::vw_api_types::file::{
    CopyFileRequest, DeleteFileRequest, FileNodeDto, FileNodeKind, ListFilesRequest,
    MoveFileRequest,
};
use vw_gateway_client::vw_api_types::id::{ProjectId, WorktreeId};
use vw_gateway_client::vw_api_types::project::{
    ListProjectsRequest, ProjectDto, ResolveProjectRequest,
};
use vw_gateway_client::vw_api_types::worktree::{
    CreateWorktreeRequest, DeleteWorktreeRequest, ResetMode, ResetWorktreeRequest, WorktreeDto,
};
use vw_shared::message::types as agent_message;

/// is_supported_image_attachment 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn is_supported_image_attachment(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            )
        })
        .unwrap_or(false)
}

const CHAT_IMAGE_ATTACHMENT_DIR: &str = "chat_image_attachments";

fn chat_image_attachment_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|dirs| {
        vw_config_types::paths::home_config_dir(dirs.home_dir()).join(CHAT_IMAGE_ATTACHMENT_DIR)
    })
}

fn path_within_dir(path: &Path, dir: &Path) -> bool {
    match (path.canonicalize(), dir.canonicalize()) {
        (Ok(canonical_path), Ok(canonical_dir)) => canonical_path.starts_with(&canonical_dir),
        _ => false,
    }
}

fn sanitize_attachment_stem(path: &Path) -> String {
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("image");
    let sanitized = stem
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if sanitized.is_empty() { "image".to_string() } else { sanitized }
}

fn build_image_attachment_snapshot_path(
    source: &Path,
    metadata: &std::fs::Metadata,
    snapshot_dir: &Path,
) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(source.to_string_lossy().as_bytes());
    hasher.update(metadata.len().to_le_bytes());
    if let Ok(modified) = metadata.modified()
        && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
    {
        hasher.update(duration.as_secs().to_le_bytes());
        hasher.update(duration.subsec_nanos().to_le_bytes());
    }

    let digest = hex::encode(hasher.finalize());
    let short_digest = &digest[..16];
    let stem = sanitize_attachment_stem(source);
    let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("img");
    snapshot_dir.join(format!("{stem}-{short_digest}.{extension}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn build_clipboard_image_attachment_path(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
    snapshot_dir: &Path,
) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(rgba_bytes);

    let digest = hex::encode(hasher.finalize());
    snapshot_dir.join(format!("clipboard-{}.png", &digest[..16]))
}

fn stabilize_image_attachment(
    source: &Path,
    metadata: &std::fs::Metadata,
    project_root: Option<&Path>,
    snapshot_dir: &Path,
) -> Result<PathBuf, String> {
    if project_root.is_some_and(|root| path_within_dir(source, root))
        || path_within_dir(source, snapshot_dir)
    {
        return Ok(source.to_path_buf());
    }

    std::fs::create_dir_all(snapshot_dir).map_err(|error| {
        format!("创建图片附件目录失败：{}（{}）", snapshot_dir.display(), error)
    })?;

    let target = build_image_attachment_snapshot_path(source, metadata, snapshot_dir);
    if !target.exists() {
        std::fs::copy(source, &target)
            .map_err(|error| format!("复制图片附件失败：{}（{}）", source.display(), error))?;
    }

    Ok(target)
}

fn attachment_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

/// collect_local_attachments 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn collect_local_attachments(
    app: &App,
    picked: Vec<String>,
) -> (Vec<String>, Vec<String>) {
    let max_images = app.multimodal_settings.max_images.clamp(1, 16) as usize;
    let max_bytes =
        (app.multimodal_settings.max_image_size_mb.clamp(1, 20) as u64).saturating_mul(1024 * 1024);
    let mut seen = app.files.iter().cloned().collect::<HashSet<_>>();
    let project_root = app.project_path.as_deref().map(Path::new);
    let snapshot_dir = chat_image_attachment_dir();
    let mut image_count =
        app.files.iter().filter(|path| is_supported_image_attachment(Path::new(path))).count();
    let mut accepted = Vec::new();
    let mut errors = Vec::new();

    for raw in picked {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let source = Path::new(trimmed);
        let resolved = match source.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                errors.push(format!("读取失败：{}（{}）", source.display(), error));
                continue;
            }
        };

        if !resolved.is_file() {
            errors.push(format!("不支持文件夹：{}", resolved.display()));
            continue;
        }

        if is_supported_image_attachment(&resolved) {
            let metadata = match std::fs::metadata(&resolved) {
                Ok(metadata) => metadata,
                Err(error) => {
                    errors.push(format!("读取大小失败：{}（{}）", resolved.display(), error));
                    continue;
                }
            };

            if metadata.len() > max_bytes {
                errors.push(format!(
                    "图片过大：{}（超过 {} MB）",
                    attachment_name(&resolved),
                    app.multimodal_settings.max_image_size_mb.clamp(1, 20),
                ));
                continue;
            }

            if image_count >= max_images {
                errors.push(format!(
                    "已达图片上限：{}（最多 {} 张）",
                    attachment_name(&resolved),
                    max_images,
                ));
                continue;
            }

            let Some(snapshot_dir) = snapshot_dir.as_deref() else {
                errors.push("无法确定图片附件目录".to_string());
                continue;
            };

            let stable_path = match stabilize_image_attachment(
                &resolved,
                &metadata,
                project_root,
                snapshot_dir,
            ) {
                Ok(path) => path,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            let normalized = stable_path.to_string_lossy().to_string();
            if !seen.insert(normalized.clone()) {
                continue;
            }

            image_count += 1;
            accepted.push(normalized);
            continue;
        }

        let normalized = resolved.to_string_lossy().to_string();
        if !seen.insert(normalized.clone()) {
            continue;
        }

        accepted.push(normalized);
    }

    (accepted, errors)
}

/// append_local_attachments 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn append_local_attachments(app: &mut App, picked: Vec<String>) {
    let (accepted, errors) = collect_local_attachments(app, picked);
    if !accepted.is_empty() {
        app.files.extend(accepted);
    }
    if !errors.is_empty() {
        app.push_notification(errors.join("\n"));
    }
}

/// persist_clipboard_image_attachment 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn persist_clipboard_image_attachment(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
) -> Result<String, String> {
    let Some(snapshot_dir) = chat_image_attachment_dir() else {
        return Err("无法确定图片附件目录".to_string());
    };

    std::fs::create_dir_all(&snapshot_dir).map_err(|error| {
        format!("创建图片附件目录失败：{}（{}）", snapshot_dir.display(), error)
    })?;

    let target = build_clipboard_image_attachment_path(width, height, rgba_bytes, &snapshot_dir);
    if !target.exists() {
        let image = RgbaImage::from_raw(width, height, rgba_bytes.to_vec())
            .ok_or_else(|| "剪贴板图片数据无效".to_string())?;
        image
            .save_with_format(&target, ImageFormat::Png)
            .map_err(|error| format!("保存剪贴板图片失败：{}（{}）", target.display(), error))?;
    }

    Ok(target.to_string_lossy().to_string())
}

/// refresh_file_index 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn refresh_file_index(app: &App) -> Task<Message> {
    let Some(root) = app.project_path.clone() else {
        return Task::none();
    };
    // 耗时或平台相关操作交给异步任务，避免阻塞界面消息循环。
    Task::perform(async move { refresh_file_index_with_fallback(&root).await }, |files| {
        Message::Project(ProjectMessage::FileIndexReady(files))
    })
}

async fn refresh_file_index_with_fallback(project_path: &str) -> Vec<String> {
    match refresh_gateway_file_index(project_path).await {
        Ok(files) if !files.is_empty() => files,
        Ok(files) => {
            let fallback = refresh_local_file_index(project_path).await;
            if fallback.is_empty() { files } else { fallback }
        }
        Err(error) => {
            if vw_gateway_client::is_gateway_unavailable_error(&error) {
                tracing::debug!(
                    target: "vw_desktop",
                    project_path,
                    error = %error,
                    "failed to refresh file index from gateway, falling back to local scan"
                );
            } else {
                tracing::warn!(
                    target: "vw_desktop",
                    project_path,
                    error = %error,
                    "failed to refresh file index from gateway, falling back to local scan"
                );
            }
            refresh_local_file_index(project_path).await
        }
    }
}

async fn refresh_local_file_index(project_path: &str) -> Vec<String> {
    let path = project_path.to_string();
    crate::app::message::spawn_blocking_opt(move || Some(crate::app::refresh_file_index(&path)))
        .await
        .unwrap_or_default()
}

#[allow(dead_code)]
/// unique_name_in_dir 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn unique_name_in_dir(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let original = dir.join(name);
    if !original.exists() {
        return original;
    }

    let p = std::path::Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|s| s.to_str());

    for i in 1..=999 {
        let suffix = if i == 1 { "copy".to_string() } else { format!("copy {}", i) };
        let candidate_name = if let Some(ext) = ext {
            format!("{stem} {suffix}.{ext}")
        } else {
            format!("{stem} {suffix}")
        };
        let candidate = dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    original
}

#[allow(dead_code)]
/// copy_recursively 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn copy_recursively(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            copy_recursively(&from, &to)?;
        }
        Ok(())
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
        Ok(())
    }
}

/// find_session_project_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn find_session_project_path(app: &App, session_id: &str) -> Option<String> {
    app.project_sessions
        .iter()
        .find_map(|(path, sessions)| {
            sessions.iter().any(|s| s.id == session_id).then(|| path.clone())
        })
        .or_else(|| app.project_path.clone())
}

/// now_ms 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    crate::app::gateway_client()
}

#[derive(Debug, Clone)]
/// GatewayFileContext 保存该流程中跨函数传递的结构化数据。
///
/// 使用具名字段保留领域含义，避免在消息链路中传递松散的动态数据。
pub(crate) struct GatewayFileContext {
    pub(crate) project_id: ProjectId,
    pub(crate) worktree_id: Option<WorktreeId>,
    pub(crate) base_directory: String,
}

/// resolve_gateway_file_context 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn resolve_gateway_file_context(
    project_path: &str,
) -> Result<GatewayFileContext, String> {
    let project = resolve_gateway_project(project_path).await?;
    let current_directory = normalize_path(project_path);
    let project_directory = normalize_path(&project.directory);
    if current_directory == project_directory {
        return Ok(GatewayFileContext {
            project_id: project.id,
            worktree_id: None,
            base_directory: project.directory,
        });
    }

    let client = gateway_client()?;
    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let maybe_worktree = client
        .project_worktrees(&project_scope)
        .await?
        .items
        .into_iter()
        .find(|worktree| normalize_path(&worktree.directory) == current_directory);
    if let Some(worktree) = maybe_worktree {
        return Ok(GatewayFileContext {
            project_id: project.id,
            worktree_id: Some(worktree.id),
            base_directory: worktree.directory,
        });
    }

    Ok(GatewayFileContext {
        project_id: project.id,
        worktree_id: None,
        base_directory: project_path.to_string(),
    })
}

async fn refresh_gateway_file_index(project_path: &str) -> Result<Vec<String>, String> {
    let client = gateway_client()?;
    let context = resolve_gateway_file_context(project_path).await?;
    let response = client
        .file_list(&ListFilesRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            path: None,
            depth: None,
        })
        .await?;
    let mut files = Vec::new();
    collect_gateway_file_paths(&context.base_directory, &response.root, &mut files);
    files.sort();
    Ok(files)
}

fn collect_gateway_file_paths(base_directory: &str, node: &FileNodeDto, output: &mut Vec<String>) {
    match node.kind {
        FileNodeKind::File => {
            output.push(resolve_absolute_gateway_path(base_directory, &node.path));
        }
        FileNodeKind::Directory => {
            if let Some(children) = node.children.as_ref() {
                for child in children {
                    collect_gateway_file_paths(base_directory, child, output);
                }
            }
        }
    }
}

fn resolve_absolute_gateway_path(base_directory: &str, relative_path: &str) -> String {
    let relative = relative_path.trim();
    if relative.is_empty() || relative == "." {
        return base_directory.to_string();
    }
    std::path::PathBuf::from(base_directory).join(relative).to_string_lossy().to_string()
}

fn normalize_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}

fn map_timestamp(value: i64) -> u64 {
    value.max(0) as u64
}

fn map_gateway_project(project: ProjectDto) -> vw_shared::project::Info {
    vw_shared::project::Info {
        id: project.id.0,
        worktree: project.directory.clone(),
        vcs: project.git.is_repo.then_some(vw_shared::project::Vcs::Git),
        name: Some(project.name),
        icon: None,
        commands: None,
        time: vw_shared::project::TimeInfo {
            created: map_timestamp(project.created_at_ms.0),
            updated: map_timestamp(project.updated_at_ms.0),
            initialized: None,
        },
        sandboxes: vec![project.directory],
    }
}

fn map_loaded_project_info(project_path: String, project: ProjectDto) -> super::LoadedProjectInfo {
    let current_branch = project.current_branch.clone().filter(|value| !value.trim().is_empty());
    super::LoadedProjectInfo { project_path, info: map_gateway_project(project), current_branch }
}

#[allow(dead_code)]
/// load_gateway_recent_projects 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn load_gateway_recent_projects()
-> Result<Vec<crate::app::RecentProjectMeta>, String> {
    let mut projects = gateway_client()?
        .project_list(
            &crate::app::session_gateway::active_tenant_scope(),
            &ListProjectsRequest { cursor: None, limit: Some(100), query: None, status: None },
        )
        .await?
        .items;
    projects.sort_by(|left, right| {
        right
            .updated_at_ms
            .0
            .cmp(&left.updated_at_ms.0)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.directory.cmp(&right.directory))
            .then_with(|| left.id.0.cmp(&right.id.0))
    });

    Ok(projects
        .into_iter()
        .filter(|project| crate::app::projects::is_visible_recent_project_path(&project.directory))
        .map(|project| crate::app::RecentProjectMeta {
            path: project.directory,
            name: project.name,
            task_board_settings: None,
            session_auto_refresh: crate::app::state::default_recent_project_session_auto_refresh(),
            session_refresh_interval_seconds:
                crate::app::state::default_recent_project_session_refresh_interval_seconds(),
            icon: None,
            icon_color: None,
            worktree_start_command: None,
        })
        .collect())
}

/// resolve_gateway_project 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) async fn resolve_gateway_project(project_path: &str) -> Result<ProjectDto, String> {
    gateway_client()?
        .project_resolve(
            &crate::app::session_gateway::active_tenant_scope(),
            &ResolveProjectRequest { directory: project_path.to_string(), create_if_missing: true },
        )
        .await
        .map(|response| response.project)
}

/// load_project_info_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn load_project_info_task(project_path: String) -> Task<Message> {
    Task::perform(
        async move {
            resolve_gateway_project(&project_path)
                .await
                .map(|project| map_loaded_project_info(project_path, project))
        },
        |res| Message::Project(ProjectMessage::ProjectInfoLoaded(res)),
    )
}

/// load_project_worktree_picker_options 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn load_project_worktree_picker_options(
    project_path: &str,
) -> Result<Vec<(String, String)>, String> {
    let client = gateway_client()?;
    let project = resolve_gateway_project(project_path).await?;
    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let mut worktrees = client.project_worktrees(&project_scope).await?.items;
    worktrees.sort_by(|left, right| {
        left.name.cmp(&right.name).then(left.directory.cmp(&right.directory))
    });

    let mut options = vec![(project.directory.clone(), "主工作区".to_string())];
    let mut seen = std::collections::HashSet::new();
    seen.insert(normalize_path(&project.directory));

    for worktree in worktrees {
        let key = normalize_path(&worktree.directory);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let label = if worktree.name.trim().is_empty() {
            let name = std::path::Path::new(&worktree.directory)
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("工作树");
            format!("独立工作区 ({name})")
        } else {
            format!("独立工作区 ({})", worktree.name)
        };
        options.push((worktree.directory, label));
    }

    options.push(("__create_worktree__".to_string(), "创建新的独立工作区".to_string()));
    Ok(options)
}

/// create_gateway_session_in_directory 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn create_gateway_session_in_directory(
    directory: String,
) -> Result<vw_shared::session::info::Info, String> {
    gateway_client()?
        .session_create::<vw_shared::session::info::Info>(
            &directory,
            &Some(vw_gateway_client::GatewaySessionCreateBody { parent_id: None, title: None }),
        )
        .await
}

/// create_gateway_worktree_session 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn create_gateway_worktree_session(
    project_path: String,
    requested_name: String,
) -> Result<vw_shared::session::info::Info, String> {
    let client = gateway_client()?;
    let project = resolve_gateway_project(&project_path).await?;
    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let worktree = client
        .project_worktree_create(
            &project_scope,
            &CreateWorktreeRequest {
                name: requested_name.clone(),
                branch: format!("vibewindow/{requested_name}"),
                from_ref: None,
                checkout: true,
            },
        )
        .await?
        .worktree;
    create_gateway_session_in_directory(worktree.directory).await
}

async fn find_gateway_worktree(project_path: &str, directory: &str) -> Result<WorktreeDto, String> {
    let client = gateway_client()?;
    let project = resolve_gateway_project(project_path).await?;
    let project_scope = crate::app::session_gateway::active_project_scope(project.id.clone())?;
    let target = normalize_path(directory);
    client
        .project_worktrees(&project_scope)
        .await?
        .items
        .into_iter()
        .find(|worktree| normalize_path(&worktree.directory) == target)
        .ok_or_else(|| "工作区不存在".to_string())
}

/// delete_gateway_worktree 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn delete_gateway_worktree(
    project_path: &str,
    directory: &str,
    force: bool,
) -> Result<(), String> {
    let client = gateway_client()?;
    let worktree = find_gateway_worktree(project_path, directory).await?;
    let project_scope =
        crate::app::session_gateway::active_project_scope(worktree.project_id.clone())?;
    let ack = client
        .worktree_delete(&project_scope, &worktree.id.0, &DeleteWorktreeRequest { force })
        .await?;
    if ack.ok {
        Ok(())
    } else {
        Err(ack.message.unwrap_or_else(|| "删除工作区失败".to_string()))
    }
}

/// reset_gateway_worktree 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn reset_gateway_worktree(
    project_path: &str,
    directory: &str,
) -> Result<(), String> {
    let client = gateway_client()?;
    let worktree = find_gateway_worktree(project_path, directory).await?;
    let project_scope =
        crate::app::session_gateway::active_project_scope(worktree.project_id.clone())?;
    let ack = client
        .worktree_reset(
            &project_scope,
            &worktree.id.0,
            &ResetWorktreeRequest { mode: ResetMode::Hard, target_ref: None },
        )
        .await?;
    if ack.ok {
        Ok(())
    } else {
        Err(ack.message.unwrap_or_else(|| "重置工作区失败".to_string()))
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// relative_to_project 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn relative_to_project(project_root: &str, path: &str) -> Option<String> {
    let root = std::path::Path::new(project_root);
    let target = std::path::Path::new(path);

    let normalize = |p: &std::path::Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        if s.is_empty() { ".".to_string() } else { s }
    };

    if let Ok(rel) = target.strip_prefix(root) {
        return Some(normalize(rel));
    }

    if let (Ok(root_canon), Ok(target_canon)) =
        (std::fs::canonicalize(root), std::fs::canonicalize(target))
        && let Ok(rel) = target_canon.strip_prefix(root_canon)
    {
        return Some(normalize(rel));
    }

    let root_norm = project_root.replace('\\', "/");
    let root_norm = root_norm.trim_end_matches('/');
    let target_norm = path.replace('\\', "/");
    if target_norm == root_norm {
        return Some(".".to_string());
    }
    let prefix = format!("{}/", root_norm);
    if let Some(rel) = target_norm.strip_prefix(&prefix) {
        return Some(if rel.is_empty() { ".".to_string() } else { rel.to_string() });
    }

    None
}

/// gateway_move_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_move_path(
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    let client = gateway_client()?;
    let context = resolve_gateway_file_context(project_path).await?;
    let from_relative = relative_to_project(&context.base_directory, from_path)
        .ok_or_else(|| "无法定位源路径".to_string())?;
    let to_relative = relative_to_project(&context.base_directory, to_path)
        .ok_or_else(|| "无法定位目标路径".to_string())?;
    let result = client
        .file_move(&MoveFileRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            from_path: from_relative,
            to_path: to_relative,
            overwrite: false,
        })
        .await?;
    if result.ok {
        Ok(())
    } else {
        Err(result.message.unwrap_or_else(|| "移动失败".to_string()))
    }
}

/// gateway_copy_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_copy_path(
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    let client = gateway_client()?;
    let context = resolve_gateway_file_context(project_path).await?;
    let from_relative = relative_to_project(&context.base_directory, from_path)
        .ok_or_else(|| "无法定位源路径".to_string())?;
    let to_relative = relative_to_project(&context.base_directory, to_path)
        .ok_or_else(|| "无法定位目标路径".to_string())?;
    let result = client
        .file_copy(&CopyFileRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            from_path: from_relative,
            to_path: to_relative,
            overwrite: false,
        })
        .await?;
    if result.ok {
        Ok(())
    } else {
        Err(result.message.unwrap_or_else(|| "复制失败".to_string()))
    }
}

/// gateway_delete_path 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) async fn gateway_delete_path(
    project_path: &str,
    path: &str,
    recursive: bool,
) -> Result<(), String> {
    let client = gateway_client()?;
    let context = resolve_gateway_file_context(project_path).await?;
    let relative = relative_to_project(&context.base_directory, path)
        .ok_or_else(|| "无法定位路径".to_string())?;
    let result = client
        .file_delete(&DeleteFileRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            path: relative,
            recursive,
        })
        .await?;
    if result.ok {
        Ok(())
    } else {
        Err(result.message.unwrap_or_else(|| "删除失败".to_string()))
    }
}

/// run_find_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn run_find_task(
    tab_id: String,
    title: String,
    scope_path: String,
    query: String,
    replace_text: String,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
    files: Vec<String>,
) -> Task<Message> {
    const MAX_FIND_RESULTS: usize = 3000;
    let tab_id_for_msg = tab_id.clone();
    let title_for_msg = title.clone();
    let scope_for_msg = scope_path.clone();
    let query_for_msg = query.clone();
    let replace_for_msg = replace_text.clone();

    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                let mut out = Vec::<FindInFolderMatch>::new();
                let mut error: Option<String> = None;
                let mut limit_reached = false;
                let mut compiled_regex = None;

                if use_regex {
                    let mut pattern = query.clone();
                    if whole_word {
                        pattern = format!(r"\b(?:{})\b", pattern);
                    }
                    let mut builder = RegexBuilder::new(&pattern);
                    builder.case_insensitive(!case_sensitive);
                    match builder.build() {
                        Ok(re) => compiled_regex = Some(re),
                        Err(e) => {
                            error = Some(format!("正则表达式错误: {}", e));
                        }
                    }
                }

                if error.is_none() {
                    let scope = std::path::Path::new(&scope_path);
                    let scope_prefix = scope.to_string_lossy().to_string();
                    let plain_query =
                        if case_sensitive { query.clone() } else { query.to_ascii_lowercase() };

                    'files: for path in files {
                        if !(path == scope_prefix
                            || path.starts_with(&(scope_prefix.clone() + "/")))
                        {
                            continue;
                        }
                        let Ok(content) = std::fs::read_to_string(&path) else {
                            continue;
                        };

                        for (li, line) in content.lines().enumerate() {
                            if let Some(re) = &compiled_regex {
                                for m in re.find_iter(line) {
                                    out.push(FindInFolderMatch {
                                        path: path.clone(),
                                        line: li + 1,
                                        column: m.start() + 1,
                                        preview: line.trim().to_string(),
                                        match_len: m.end().saturating_sub(m.start()),
                                    });
                                    if out.len() >= MAX_FIND_RESULTS {
                                        limit_reached = true;
                                        break 'files;
                                    }
                                }
                                continue;
                            }

                            if plain_query.is_empty() {
                                continue;
                            }

                            let (line_cmp, needle) = if case_sensitive {
                                (line.to_string(), plain_query.as_str())
                            } else {
                                (line.to_ascii_lowercase(), plain_query.as_str())
                            };

                            let mut start = 0usize;
                            while start <= line_cmp.len() {
                                let Some(pos) = line_cmp[start..].find(needle) else {
                                    break;
                                };
                                let st = start + pos;
                                let ed = st + needle.len();

                                if whole_word {
                                    let bytes = line_cmp.as_bytes();
                                    let left_ok = st == 0 || !is_word_byte(bytes[st - 1]);
                                    let right_ok = ed >= bytes.len() || !is_word_byte(bytes[ed]);
                                    if !(left_ok && right_ok) {
                                        start = st + needle.len();
                                        continue;
                                    }
                                }

                                out.push(FindInFolderMatch {
                                    path: path.clone(),
                                    line: li + 1,
                                    column: st + 1,
                                    preview: line.trim().to_string(),
                                    match_len: needle.len(),
                                });
                                if out.len() >= MAX_FIND_RESULTS {
                                    limit_reached = true;
                                    break 'files;
                                }
                                start = st + needle.len();
                            }
                        }
                    }
                }

                Some((out, error, limit_reached))
            })
            .await
            .unwrap_or((Vec::new(), Some("搜索失败".to_string()), false))
        },
        move |(matches, error, limit_reached)| {
            Message::Project(ProjectMessage::FileTreeFindCompleted {
                tab_id: tab_id_for_msg.clone(),
                title: title_for_msg.clone(),
                scope_path: scope_for_msg.clone(),
                query: query_for_msg.clone(),
                replace_text: replace_for_msg.clone(),
                case_sensitive,
                whole_word,
                use_regex,
                matches,
                error,
                limit_reached,
            })
        },
    )
}

/// 后台读取 UI Store，并将完整历史和首屏分块合并为一个冷打开结果。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_prepared_session_open_from_ui_store_task(
    project_path: String,
    session_id: String,
    open_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    let target_session_id = session_id.clone();
    let target_project_path = project_path.clone();
    Task::perform(
        async move {
            let client = crate::app::gateway_client()?;
            let mut loaded = client
                .session_ui_get_with_raw(&session_id, Some(&project_path))
                .await?
                .filter(|loaded| !loaded.session.messages.is_empty())
                .ok_or_else(|| "session UI store has no message history".to_string())?;
            let step_llm_raw = std::mem::take(&mut loaded.step_llm_raw);
            let llm_raw_step_indices = std::mem::take(&mut loaded.llm_raw_step_indices);
            let mut session = loaded.session;

            let missing_message_model_count =
                session.messages.iter().filter(|message| message.model.is_none()).count();
            crate::app::session::backfill_message_models_from_steps(
                &mut session.messages,
                &session.steps,
            );
            if session.messages.iter().filter(|message| message.model.is_none()).count()
                < missing_message_model_count
            {
                crate::app::session_gateway::gateway_save_session_async(
                    &session,
                    Some(&project_path),
                )
                .await?;
            }

            crate::app::message::spawn_blocking_opt(move || {
                let mut prepared = crate::app::session::prepare_session_open_from_ui_store(
                    session,
                    show_reasoning_summary,
                );
                prepared.step_llm_raw = step_llm_raw;
                prepared.llm_raw_step_indices = llm_raw_step_indices;
                Some(prepared)
            })
            .await
            .ok_or_else(|| "session UI preparation task did not complete".to_string())
        },
        move |result| {
            Message::Project(ProjectMessage::SessionOpenPrepared {
                target_session_id,
                project_path: target_project_path,
                open_generation,
                source: super::PreparedSessionOpenSource::UiStore,
                result,
            })
        },
    )
}

/// 后台读取远端历史，并在同一后台边界内完成协议转换和首屏 UI 准备。
pub(crate) fn load_prepared_session_open_from_gateway_task(
    project_path: String,
    session_id: String,
    open_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    let target_session_id = session_id.clone();
    let target_project_path = project_path.clone();
    Task::perform(
        async move {
            tracing::info!(
                target: "vw_desktop",
                session_id = %session_id,
                project_path = %project_path,
                "loading prepared session history from gateway"
            );
            let client = crate::app::gateway_client()?;
            let (messages, session_info, session_ui) = tokio::join!(
                client.session_messages::<Vec<agent_message::WithParts>>(
                    &session_id,
                    Some(&project_path),
                ),
                client.session_get::<vw_shared::session::info::Info>(
                    &session_id,
                    Some(&project_path),
                ),
                client.session_ui_get_with_raw(&session_id, Some(&project_path)),
            );
            let messages = messages?;
            session_info?;
            let (step_llm_raw, llm_raw_step_indices) = match session_ui {
                    Ok(Some(loaded)) => (loaded.step_llm_raw, loaded.llm_raw_step_indices),
                    Ok(None) => (std::collections::HashMap::new(), Vec::new()),
                    Err(error) => {
                        tracing::warn!(
                            target: "vw_desktop",
                            session_id = %session_id,
                            error = %error,
                            "session raw step payloads were unavailable"
                        );
                        (std::collections::HashMap::new(), Vec::new())
                    }
                };

            crate::app::message::spawn_blocking_opt(move || {
                let usage = messages.iter().fold(
                    crate::app::models::TokenUsage::default(),
                    |mut usage, message| {
                        if let agent_message::Info::Assistant(assistant) = &message.info {
                            usage.input_tokens += assistant.tokens.input;
                            usage.output_tokens += assistant.tokens.output;
                            usage.cached_tokens +=
                                assistant.tokens.cache.read + assistant.tokens.cache.write;
                            usage.reasoning_tokens += assistant.tokens.reasoning;
                        }
                        usage
                    },
                );
                let session_model = super::latest_assistant_model_label(&messages);
                let steps = super::steps_from_gateway_messages(&messages);
                let (chat, message_ids) = super::loaded_chat_from_gateway_messages(messages);
                let mut prepared = crate::app::session::prepare_session_open(
                    session_id,
                    chat,
                    message_ids,
                    usage,
                    steps,
                    Vec::new(),
                    0,
                    session_model,
                    show_reasoning_summary,
                );
                prepared.step_llm_raw = step_llm_raw;
                prepared.llm_raw_step_indices = llm_raw_step_indices;
                Some(prepared)
            })
            .await
            .ok_or_else(|| "session history preparation task did not complete".to_string())
        },
        move |result| {
            Message::Project(ProjectMessage::SessionOpenPrepared {
                target_session_id,
                project_path: target_project_path,
                open_generation,
                source: super::PreparedSessionOpenSource::RemoteHistory,
                result,
            })
        },
    )
}

/// load_session_messages_task_scoped 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn load_session_messages_task_scoped(
    project_path: Option<String>,
    id: String,
) -> Task<Message> {
    Task::perform(
        async move {
            tracing::info!(
                target: "vw_desktop",
                session_id = %id,
                project_path = project_path.as_deref().unwrap_or("<none>"),
                "loading session messages from gateway"
            );
            let client = match crate::app::gateway_client() {
                Ok(client) => client,
                Err(err) => return Err(err),
            };
            let msgs = client
                .session_messages::<Vec<agent_message::WithParts>>(&id, project_path.as_deref())
                .await;
            let info = client
                .session_get::<vw_shared::session::info::Info>(&id, project_path.as_deref())
                .await;
            match (msgs, info) {
                (Ok(msgs), Ok(_info)) => {
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %id,
                        project_path = project_path.as_deref().unwrap_or("<none>"),
                        message_count = msgs.len(),
                        "loaded session messages from gateway"
                    );
                    let mut usage = crate::app::models::TokenUsage::default();
                    for m in &msgs {
                        if let agent_message::Info::Assistant(a) = &m.info {
                            usage.input_tokens += a.tokens.input;
                            usage.output_tokens += a.tokens.output;
                            usage.cached_tokens += a.tokens.cache.read + a.tokens.cache.write;
                            usage.reasoning_tokens += a.tokens.reasoning;
                        }
                    }
                    Ok((id, msgs, usage))
                }
                (Err(e), _) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %id,
                        project_path = project_path.as_deref().unwrap_or("<none>"),
                        error = %e,
                        "failed to load session messages from gateway"
                    );
                    Err(e)
                }
                (_, Err(e)) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %id,
                        project_path = project_path.as_deref().unwrap_or("<none>"),
                        error = %e,
                        "failed to load session info while loading messages"
                    );
                    Err(e)
                }
            }
        },
        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
    )
}

/// prepare_session_ui_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn prepare_session_ui_task(
    session_id: String,
    chat: crate::app::session::SharedChatMessages,
    chunk_start_idx: usize,
    is_base: bool,
    show_reasoning_summary: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                let (chunk_start_idx, chunk_end_idx) =
                    crate::app::session::chat_ui_chunk_bounds(chat.len(), chunk_start_idx);
                if chunk_start_idx >= chunk_end_idx {
                    return None;
                }
                Some((
                    session_id,
                    crate::app::session::prepare_chat_ui_chunk_phase(
                        &chat[chunk_start_idx..chunk_end_idx],
                        chunk_start_idx,
                        is_base,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, phase)) => {
                Message::Project(ProjectMessage::SessionUiPrepared {
                    session_id,
                    stream_generation: None,
                    phase,
                })
            }
            None => Message::None,
        },
    )
}

pub(crate) fn prepare_stream_message_ui_task(
    session_id: String,
    message: crate::app::models::ChatMessage,
    message_idx: usize,
    stream_generation: u64,
    show_reasoning_summary: bool,
) -> Task<Message> {
    Task::perform(
        async move {
            crate::app::message::spawn_blocking_opt(move || {
                Some((
                    session_id,
                    stream_generation,
                    crate::app::session::prepare_chat_ui_message_phase(
                        &message,
                        message_idx,
                        show_reasoning_summary,
                    ),
                ))
            })
            .await
        },
        |res| match res {
            Some((session_id, stream_generation, phase)) => {
                Message::Project(ProjectMessage::SessionUiPrepared {
                    session_id,
                    stream_generation: Some(stream_generation),
                    phase,
                })
            }
            None => Message::None,
        },
    )
}

/// prepare_session_ui_chunks_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn prepare_session_ui_chunks_task(
    session_id: String,
    chat: crate::app::session::SharedChatMessages,
    chunk_starts: Vec<usize>,
    base_chunk_start: Option<usize>,
    show_reasoning_summary: bool,
) -> Task<Message> {
    if chunk_starts.is_empty() {
        return Task::none();
    }

    Task::batch(chunk_starts.into_iter().map(|chunk_start_idx| {
        prepare_session_ui_task(
            session_id.clone(),
            chat.clone(),
            chunk_start_idx,
            base_chunk_start == Some(chunk_start_idx),
            show_reasoning_summary,
        )
    }))
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
