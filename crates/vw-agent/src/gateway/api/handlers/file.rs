//! 文件操作路由模块
//!
//! 本模块提供与文件系统交互的 HTTP API 路由，包括：
//! - 文本内容搜索（基于 ripgrep）
//! - 文件名搜索
//! - 目录列表
//! - 文件内容读取
//! - Git 状态查询
//!
//! # 路由端点
//!
//! - `GET /find` - 在文件中搜索文本内容
//! - `GET /find/file` - 按文件名搜索文件
//! - `GET /find/symbol` - 符号搜索（未实现）
//! - `GET /file` - 列出目录内容
//! - `GET /file/content` - 读取文件内容
//! - `GET /file/status` - 获取 Git 状态
use std::path::{Path, PathBuf};

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use base64::Engine;
use serde::Deserialize;
use serde::Serialize;
use vw_api_types::common::OperationAck;
use vw_api_types::file::{CopyFileRequest, DeleteFileRequest, MoveFileRequest, WriteFileResponse};

use crate::app::agent::file;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::api::handlers::misc::not_implemented;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::normalize_rel_path;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::gateway::instance::with_instance;
use crate::app::agent::project::instance;

/// 创建文件操作路由器
///
/// 返回配置了所有文件操作端点的 Axum 路由器实例。
///
/// # 路由说明
///
/// - `/find` - 文本内容搜索，使用 ripgrep 引擎
/// - `/find/file` - 文件名搜索，支持模糊匹配
/// - `/find/symbol` - 符号搜索（当前返回未实现响应）
/// - `/file` - 列出指定路径的目录内容
/// - `/file/content` - 读取指定文件的内容
/// - `/file/status` - 获取工作目录的 Git 状态
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/find", get(file_find_text))
        .route("/find/file", get(file_find_file))
        .route("/find/symbol", get(not_implemented))
        .route("/file", get(file_list))
        .route("/file/read", post(file_read_v1))
        .route("/file/content", get(file_read))
        .route("/file/write", post(file_write_v1))
        .route("/file/status", get(file_status))
        .route("/files/move", post(file_move_v1))
        .route("/files/copy", post(file_copy_v1))
        .route("/files/delete", post(file_delete_v1))
}

#[derive(Debug, Deserialize)]
struct FileReadBody {
    directory: Option<String>,
    agent_key: Option<String>,
    path: String,
}

#[derive(Debug, Serialize)]
struct FileReadBodyResponse {
    root_directory: String,
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct FileWriteBody {
    directory: Option<String>,
    agent_key: Option<String>,
    path: String,
    content: String,
    #[serde(default)]
    create_if_missing: bool,
}

#[derive(Debug, Serialize)]
struct FileWriteBodyResponse {
    ok: bool,
    root_directory: String,
    path: String,
    bytes_written: u64,
}

/// 文本搜索查询参数
///
/// 用于 `/find` 端点的查询字符串参数。
#[derive(Debug, Deserialize)]
struct FindTextQuery {
    /// 可选的目录路径，指定搜索范围
    /// 如果未提供，则使用实例的默认工作目录
    directory: Option<String>,

    /// 搜索模式/关键词
    /// 支持正则表达式语法（由 ripgrep 提供）
    pattern: String,
}

/// Ripgrep 匹配结果
///
/// 表示文本搜索中的单个匹配项，包含匹配的位置和内容信息。
#[derive(Debug, Serialize)]
struct RipgrepMatch {
    /// 匹配所在的文件路径（相对于工作目录）
    path: String,

    /// 匹配所在的行号（从 1 开始）
    #[serde(rename = "lineNumber")]
    line_number: usize,

    /// 匹配所在行的完整内容
    line: String,

    /// 匹配在行中的起始位置（从 0 开始的字符偏移）
    start: usize,

    /// 匹配在行中的结束位置（从 0 开始的字符偏移）
    end: usize,
}

/// 处理文本搜索请求
///
/// 在指定目录的文件中搜索匹配指定模式的文本内容。
/// 使用 ripgrep 引擎进行高效搜索，支持正则表达式。
///
/// # 参数
///
/// - `query`: 查询参数，包含搜索目录和搜索模式
/// - `headers`: HTTP 请求头，用于解析实例信息
///
/// # 返回
///
/// 返回匹配结果列表，每个结果包含文件路径、行号、匹配内容和位置信息。
/// 最多返回 10 个匹配结果。
///
/// # 错误
///
/// - `ApiError::BadRequest`: 搜索过程失败时返回
async fn file_find_text(
    Query(query): Query<FindTextQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<RipgrepMatch>>, ApiError> {
    let FindTextQuery { directory, pattern } = query;

    // 解析并验证目录路径
    let dir = resolve_directory(&InstanceQuery { directory }, &headers);

    // 在实例上下文中执行搜索
    let result = with_instance(dir, move || {
        Box::pin(async move {
            // 获取当前工作目录
            let cwd = PathBuf::from(instance::directory());

            // 执行 ripgrep 搜索
            let matches = file::ripgrep::search(file::ripgrep::SearchInput {
                cwd,
                pattern,
                glob: None,      // 不限制文件类型
                limit: Some(10), // 限制最多 10 个结果
                follow: None,    // 不跟随符号链接
            })
            .map_err(|e| ApiError::bad_request(e.to_string()))?;

            // 转换为 API 响应格式
            Ok(matches
                .into_iter()
                .map(|m| RipgrepMatch {
                    path: m.path,
                    line_number: m.line_number,
                    line: m.line,
                    start: m.start,
                    end: m.end,
                })
                .collect::<Vec<_>>())
        })
    })
    .await?;

    Ok(Json(result))
}

/// 文件名搜索查询参数
///
/// 用于 `/find/file` 端点的查询字符串参数。
#[derive(Debug, Deserialize)]
struct FindFileQuery {
    /// 可选的目录路径，指定搜索范围
    /// 如果未提供，则使用实例的默认工作目录
    directory: Option<String>,

    /// 搜索查询字符串
    /// 将与文件名进行大小写不敏感的匹配
    query: String,

    /// 是否包含目录在搜索结果中
    /// 可选值："true"（默认）或 "false"
    dirs: Option<String>,

    /// 结果类型过滤器
    /// 可选值："file"（仅文件）或 "directory"（仅目录）
    r#type: Option<String>,

    /// 最大返回结果数
    /// 默认值：10，范围：1-200
    limit: Option<usize>,
}

/// 处理文件名搜索请求
///
/// 根据文件名或目录名搜索文件系统中的文件。
/// 支持模糊匹配、类型过滤和数量限制。
///
/// # 参数
///
/// - `query`: 查询参数，包含搜索目录、查询字符串、过滤选项等
/// - `headers`: HTTP 请求头，用于解析实例信息
///
/// # 返回
///
/// 返回匹配的文件/目录路径列表（相对于工作目录）。
///
/// # 搜索流程
///
/// 1. 使用 ripgrep 列出所有文件
/// 2. 根据查询字符串进行大小写不敏感的过滤
/// 3. 可选地遍历目录结构查找匹配的目录名
/// 4. 根据类型参数（file/directory）过滤结果
/// 5. 排序并截断到指定数量
///
/// # 错误
///
/// - `ApiError::BadRequest`: 文件扫描过程失败时返回
async fn file_find_file(
    Query(query): Query<FindFileQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<String>>, ApiError> {
    let FindFileQuery { directory, query, dirs, r#type, limit } = query;

    // 解析并验证目录路径
    let dir = resolve_directory(&InstanceQuery { directory }, &headers);

    // 预处理查询参数：转换为小写以实现大小写不敏感匹配
    let needle = query.to_ascii_lowercase();
    let include_dirs = dirs.as_deref() != Some("false");
    let filter_type = r#type;

    // 限制结果数量，范围在 1-200 之间
    let limit = limit.unwrap_or(10).clamp(1, 200);

    let result = with_instance(dir, move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let mut out = Vec::new();

            // 第一步：使用 ripgrep 获取所有文件列表
            let files = file::ripgrep::files(file::ripgrep::FilesInput {
                cwd: root.clone(),
                glob: None,          // 不限制文件类型
                hidden: Some(true),  // 包含隐藏文件
                follow: Some(false), // 不跟随符号链接
                max_depth: None,     // 不限制搜索深度
            })
            .map_err(|e| ApiError::bad_request(e.to_string()))?;

            // 第二步：过滤文件名匹配的文件
            for rel in files {
                if out.len() >= limit {
                    break;
                }
                // 大小写不敏感的文件名匹配
                if !rel.to_ascii_lowercase().contains(&needle) {
                    continue;
                }
                out.push(rel);
            }

            // 第三步：可选地搜索目录名
            if include_dirs {
                let mut dirs = Vec::new();

                // 遍历目录结构
                for entry in walkdir::WalkDir::new(&root)
                    .follow_links(false) // 不跟随符号链接
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    // 只处理目录
                    if entry.file_type().is_dir() {
                        let path = entry.path();

                        // 计算相对路径
                        let rel = path.strip_prefix(&root).unwrap_or(path);
                        let rel = rel.to_string_lossy().to_string().replace('\\', "/");

                        // 跳过根目录和 .git 目录
                        if rel.is_empty() || rel.starts_with(".git/") {
                            continue;
                        }

                        // 检查是否被 .gitignore 等忽略规则排除
                        if file::ignore::matches(
                            &format!("{}/", rel.trim_end_matches('/')),
                            None,
                            None,
                        ) {
                            continue;
                        }

                        // 大小写不敏感的目录名匹配
                        if rel.to_ascii_lowercase().contains(&needle) {
                            dirs.push(rel);
                            if dirs.len() >= limit {
                                break;
                            }
                        }
                    }
                }

                // 去重并添加到结果列表
                dirs.sort();
                dirs.dedup();
                out.extend(dirs);
            }

            // 第四步：根据类型参数过滤结果
            if let Some(t) = filter_type.as_deref() {
                if t == "file" {
                    // 仅保留文件
                    out.retain(|p| PathBuf::from(instance::directory()).join(p).is_file());
                } else if t == "directory" {
                    // 仅保留目录
                    out.retain(|p| PathBuf::from(instance::directory()).join(p).is_dir());
                }
            }

            // 第五步：排序并截断到指定数量
            out.sort();
            out.truncate(limit);

            Ok(out)
        })
    })
    .await?;

    Ok(Json(result))
}

/// 目录列表查询参数
///
/// 用于 `/file` 端点的查询字符串参数。
#[derive(Debug, Deserialize)]
struct FileListQuery {
    /// 可选的目录路径，指定列出哪个实例的文件
    /// 如果未提供，则使用实例的默认工作目录
    directory: Option<String>,

    /// 要列出的目录路径（相对于工作目录）
    /// 可以是空字符串表示根目录
    path: String,
}

/// 处理目录列表请求
///
/// 列出指定目录下的所有文件和子目录。
///
/// # 参数
///
/// - `query`: 查询参数，包含目录路径和要列出的路径
/// - `headers`: HTTP 请求头，用于解析实例信息
///
/// # 返回
///
/// 返回文件节点列表（`file::Node`），每个节点包含：
/// - 名称
/// - 类型（文件/目录）
/// - 大小（仅文件）
/// - 修改时间等元数据
///
/// # 安全性
///
/// - 路径会被规范化以防止目录遍历攻击
/// - 只能访问实例工作目录内的文件
///
/// # 错误
///
/// - `ApiError::BadRequest`: 路径无效或列出操作失败时返回
async fn file_list(
    Query(query): Query<FileListQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<file::Node>>, ApiError> {
    let FileListQuery { directory, path } = query;

    // 解析并验证目录路径
    let dir = resolve_directory(&InstanceQuery { directory }, &headers);

    let result = with_instance(dir, move || {
        Box::pin(async move {
            // 获取工作目录根路径
            let root = PathBuf::from(instance::directory());

            // 规范化相对路径，防止目录遍历攻击
            let rel = normalize_rel_path(&root, &path);

            // 列出目录内容
            file::list(
                root,
                // 处理空路径的情况
                if rel.as_deref().is_some_and(|s| s.is_empty()) { None } else { rel.as_deref() },
            )
            .map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;

    Ok(Json(result))
}

/// 文件读取查询参数
///
/// 用于 `/file/content` 端点的查询字符串参数。
#[derive(Debug, Deserialize)]
struct FileReadQuery {
    /// 可选的目录路径，指定读取哪个实例的文件
    /// 如果未提供，则使用实例的默认工作目录
    directory: Option<String>,

    /// 要读取的文件路径（相对于工作目录）
    path: String,
}

/// 处理文件内容读取请求
///
/// 读取指定文件的完整内容。
///
/// # 参数
///
/// - `query`: 查询参数，包含目录路径和文件路径
/// - `headers`: HTTP 请求头，用于解析实例信息
///
/// # 返回
///
/// 返回文件内容（`file::Content`），包含：
/// - 文件内容（文本或二进制）
/// - 文件大小
/// - 编码信息等
///
/// # 安全性
///
/// - 路径会被规范化以防止目录遍历攻击
/// - 如果规范化失败，会尝试使用原始路径（但仍在工作目录约束内）
/// - 只能访问实例工作目录内的文件
///
/// # 错误
///
/// - `ApiError::BadRequest`: 路径无效或读取操作失败时返回
async fn file_read(
    Query(query): Query<FileReadQuery>,
    headers: HeaderMap,
) -> Result<Json<file::Content>, ApiError> {
    let FileReadQuery { directory, path } = query;

    // 解析并验证目录路径
    let dir = resolve_directory(&InstanceQuery { directory }, &headers);

    let result = with_instance(dir, move || {
        Box::pin(async move {
            // 获取工作目录根路径
            let root = PathBuf::from(instance::directory());

            // 规范化相对路径，防止目录遍历攻击
            // 如果规范化失败（例如路径包含特殊字符），则使用原始路径
            let rel = normalize_rel_path(&root, &path).unwrap_or(path);

            // 读取文件内容
            file::read(root, &rel).map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;

    Ok(Json(result))
}

async fn file_read_v1(
    headers: HeaderMap,
    Json(body): Json<FileReadBody>,
) -> Result<Json<FileReadBodyResponse>, ApiError> {
    let root_directory = resolve_directory_or_agent_root(
        &InstanceQuery { directory: body.directory.clone() },
        body.agent_key.as_deref(),
        &headers,
    );
    let path = body.path.clone();

    let content = with_instance(root_directory.clone(), move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let rel = normalize_rel_path(&root, &path).unwrap_or(path.clone());
            let full = root.join(&rel);
            if !contains_path(&root, &full) {
                return Err(ApiError::bad_request("path escapes workspace root"));
            }
            let content = match std::fs::read_to_string(&full) {
                Ok(content) => content,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(err) => return Err(ApiError::bad_request(err.to_string())),
            };
            Ok((rel, content))
        })
    })
    .await?;

    Ok(Json(FileReadBodyResponse { root_directory, path: content.0, content: content.1 }))
}

async fn file_write_v1(
    headers: HeaderMap,
    Json(body): Json<FileWriteBody>,
) -> Result<Json<FileWriteBodyResponse>, ApiError> {
    let root_directory = resolve_directory_or_agent_root(
        &InstanceQuery { directory: body.directory.clone() },
        body.agent_key.as_deref(),
        &headers,
    );
    let path = body.path.clone();
    let content = body.content.clone();
    let create_if_missing = body.create_if_missing;

    let response = with_instance(root_directory.clone(), move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let rel = normalize_rel_path(&root, &path).unwrap_or(path.clone());
            let full = root.join(&rel);
            if !contains_path(&root, &full) {
                return Err(ApiError::bad_request("path escapes workspace root"));
            }

            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| ApiError::bad_request(err.to_string()))?;
            }
            if !create_if_missing && !full.exists() {
                return Err(ApiError::bad_request(format!("file not found: {}", rel)));
            }
            std::fs::write(&full, content.as_bytes())
                .map_err(|err| ApiError::bad_request(err.to_string()))?;

            Ok(WriteFileResponse { ok: true, path: rel, bytes_written: content.len() as u64 })
        })
    })
    .await?;

    Ok(Json(FileWriteBodyResponse {
        ok: response.ok,
        root_directory,
        path: response.path,
        bytes_written: response.bytes_written,
    }))
}

/// 处理 Git 状态查询请求
///
/// 获取工作目录的 Git 状态信息，包括已修改、已暂存、未跟踪的文件等。
///
/// # 参数
///
/// - `query`: 查询参数，包含目录路径
/// - `headers`: HTTP 请求头，用于解析实例信息
///
/// # 返回
///
/// 返回文件信息列表（`file::Info`），每个条目包含：
/// - 文件路径
/// - Git 状态（已修改/已暂存/未跟踪/冲突等）
/// - 文件大小、修改时间等元数据
///
/// # 使用场景
///
/// - 查看工作目录的变更状态
/// - 准备提交前检查修改的文件
/// - 代码审查时识别变更范围
///
/// # 错误
///
/// - 如果工作目录不是 Git 仓库，将返回空列表
/// - `ApiError::BadRequest`: 查询过程失败时返回
async fn file_status(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<file::Info>>, ApiError> {
    // 解析并验证目录路径
    let dir = resolve_directory(&query, &headers);

    let result = with_instance(dir, move || {
        Box::pin(async move {
            // 获取 worktree 路径并查询 Git 状态
            // worktree 是实际的工作树路径，可能与主仓库分离
            Ok(file::status_git(PathBuf::from(instance::worktree())))
        })
    })
    .await?;

    Ok(Json(result))
}

async fn file_move_v1(Json(body): Json<MoveFileRequest>) -> Result<Json<OperationAck>, ApiError> {
    let root_directory =
        resolve_file_root(&body.project_id.0, body.worktree_id.as_ref().map(|id| id.0.as_str()))
            .await?;
    let from_path = body.from_path.clone();
    let to_path = body.to_path.clone();
    let overwrite = body.overwrite;

    with_instance(root_directory, move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let from_full = resolve_workspace_path(&root, &from_path)?;
            let to_full = resolve_workspace_path(&root, &to_path)?;

            if !from_full.exists() {
                return Err(ApiError::bad_request(format!("file not found: {}", from_path)));
            }
            if to_full.exists() && !overwrite {
                return Err(ApiError::bad_request(format!("path already exists: {}", to_path)));
            }
            if let Some(parent) = to_full.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| ApiError::bad_request(err.to_string()))?;
            }
            if to_full.exists() {
                remove_existing_path(&to_full, true)?;
            }
            std::fs::rename(&from_full, &to_full)
                .map_err(|err| ApiError::bad_request(err.to_string()))?;
            Ok::<(), ApiError>(())
        })
    })
    .await?;

    Ok(Json(OperationAck { ok: true, message: None }))
}

async fn file_copy_v1(Json(body): Json<CopyFileRequest>) -> Result<Json<OperationAck>, ApiError> {
    let root_directory =
        resolve_file_root(&body.project_id.0, body.worktree_id.as_ref().map(|id| id.0.as_str()))
            .await?;
    let from_path = body.from_path.clone();
    let to_path = body.to_path.clone();
    let overwrite = body.overwrite;

    with_instance(root_directory, move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let from_full = resolve_workspace_path(&root, &from_path)?;
            let to_full = resolve_workspace_path(&root, &to_path)?;

            if !from_full.exists() {
                return Err(ApiError::bad_request(format!("file not found: {}", from_path)));
            }
            if to_full.exists() && !overwrite {
                return Err(ApiError::bad_request(format!("path already exists: {}", to_path)));
            }
            if to_full.exists() {
                remove_existing_path(&to_full, true)?;
            }
            copy_path_recursive(&from_full, &to_full)?;
            Ok::<(), ApiError>(())
        })
    })
    .await?;

    Ok(Json(OperationAck { ok: true, message: None }))
}

async fn file_delete_v1(
    Json(body): Json<DeleteFileRequest>,
) -> Result<Json<OperationAck>, ApiError> {
    let root_directory =
        resolve_file_root(&body.project_id.0, body.worktree_id.as_ref().map(|id| id.0.as_str()))
            .await?;
    let path = body.path.clone();
    let recursive = body.recursive;

    with_instance(root_directory, move || {
        Box::pin(async move {
            let root = PathBuf::from(instance::directory());
            let full = resolve_workspace_path(&root, &path)?;
            remove_existing_path(&full, recursive)?;
            Ok::<(), ApiError>(())
        })
    })
    .await?;

    Ok(Json(OperationAck { ok: true, message: None }))
}

fn resolve_directory_or_agent_root(
    query: &InstanceQuery,
    agent_key: Option<&str>,
    headers: &HeaderMap,
) -> String {
    if let Some(directory) = query.directory.as_deref()
        && !directory.trim().is_empty()
    {
        return resolve_directory(query, headers);
    }

    resolve_agent_workspace_root(agent_key).unwrap_or_else(|| resolve_directory(query, headers))
}

fn resolve_agent_workspace_root(agent_key: Option<&str>) -> Option<String> {
    let normalized_agent_key = agent_key.unwrap_or("main").trim();
    let suffix = if normalized_agent_key.is_empty() || normalized_agent_key == "main" {
        String::new()
    } else {
        format!("-{normalized_agent_key}")
    };

    if let Ok(workspace) = std::env::var("VIBEWINDOW_WORKSPACE") {
        let workspace = workspace.trim();
        if !workspace.is_empty() {
            return Some(format!("{workspace}{suffix}"));
        }
    }

    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| {
            vw_config_types::paths::home_config_dir(home).join(format!("workspace{suffix}"))
        })
        .map(|path| path.to_string_lossy().into_owned())
}

fn contains_path(root: &Path, candidate: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let candidate = if candidate.exists() {
        candidate.canonicalize().unwrap_or_else(|_| candidate.to_path_buf())
    } else {
        let mut missing_parts = Vec::new();
        let mut current = candidate;
        while !current.exists() {
            if let Some(name) = current.file_name() {
                missing_parts.push(name.to_os_string());
            }
            let Some(parent) = current.parent() else {
                break;
            };
            current = parent;
        }
        let mut base = current.canonicalize().unwrap_or_else(|_| current.to_path_buf());
        for part in missing_parts.iter().rev() {
            base.push(part);
        }
        base
    };
    candidate.starts_with(&root)
}

async fn resolve_file_root(
    project_id: &str,
    worktree_id: Option<&str>,
) -> Result<String, ApiError> {
    if let Some(worktree_id) = worktree_id {
        return decode_worktree_directory(worktree_id);
    }

    let info = crate::app::agent::gateway::instance::load_local_project(project_id).await?;

    if !info.worktree.trim().is_empty() {
        return Ok(info.worktree);
    }

    info.sandboxes
        .into_iter()
        .find(|path| !path.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("project directory missing"))
}

fn decode_worktree_directory(worktree_id: &str) -> Result<String, ApiError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(worktree_id)
        .map_err(|_| ApiError::not_found("worktree not found"))?;
    String::from_utf8(bytes).map_err(|_| ApiError::not_found("worktree not found"))
}

fn resolve_workspace_path(root: &PathBuf, path: &str) -> Result<PathBuf, ApiError> {
    let rel = normalize_rel_path(root, path).unwrap_or_else(|| path.to_string());
    let full = root.join(&rel);
    if !contains_path(root, &full) {
        return Err(ApiError::bad_request("path escapes workspace root"));
    }
    Ok(full)
}

fn remove_existing_path(path: &Path, recursive: bool) -> Result<(), ApiError> {
    if !path.exists() {
        return Err(ApiError::bad_request(format!("file not found: {}", path.display())));
    }

    let metadata =
        std::fs::symlink_metadata(path).map_err(|err| ApiError::bad_request(err.to_string()))?;
    if metadata.is_dir() {
        if !recursive {
            return Err(ApiError::bad_request(format!(
                "directory requires recursive delete: {}",
                path.display()
            )));
        }
        std::fs::remove_dir_all(path).map_err(|err| ApiError::bad_request(err.to_string()))?;
    } else {
        std::fs::remove_file(path).map_err(|err| ApiError::bad_request(err.to_string()))?;
    }
    Ok(())
}

fn copy_path_recursive(from: &Path, to: &Path) -> Result<(), ApiError> {
    let metadata =
        std::fs::symlink_metadata(from).map_err(|err| ApiError::bad_request(err.to_string()))?;

    if metadata.is_dir() {
        std::fs::create_dir_all(to).map_err(|err| ApiError::bad_request(err.to_string()))?;
        for entry in
            std::fs::read_dir(from).map_err(|err| ApiError::bad_request(err.to_string()))?
        {
            let entry = entry.map_err(|err| ApiError::bad_request(err.to_string()))?;
            let child_from = entry.path();
            let child_to = to.join(entry.file_name());
            copy_path_recursive(&child_from, &child_to)?;
        }
        return Ok(());
    }

    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|err| ApiError::bad_request(err.to_string()))?;
    }
    std::fs::copy(from, to).map_err(|err| ApiError::bad_request(err.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod file_tests;
