//! Agent 运行时的轻量 JSON 存储层。
//!
//! 非 WASM 平台会把按 key 分层的数据写入本地 `storage` 目录，并通过读写锁
//! 保护同一路径的并发访问。WASM 平台显式提供空实现或 NotFound 错误，避免
//! 伪装支持不可用的本地文件系统能力。

use crate::app::agent::global;
use crate::app::agent::util::{lock, log};
#[cfg(not(target_arch = "wasm32"))]
use git2::Repository;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
#[cfg(not(target_arch = "wasm32"))]
use walkdir::WalkDir;

#[derive(Debug, Clone)]
/// 表示指定存储资源不存在。
pub struct NotFoundError {
    /// 面向调用方的错误说明。
    pub message: String,
}

impl fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotFoundError {}

#[derive(Debug)]
/// 存储层统一错误类型。
pub enum Error {
    /// key 对应的 JSON 资源不存在。
    NotFound(NotFoundError),
    /// 文件系统读写错误。
    Io(std::io::Error),
    /// JSON 序列化或反序列化错误。
    Json(serde_json::Error),
    /// 阻塞任务 join 失败。
    Join(tokio::task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Join(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(value: tokio::task::JoinError) -> Self {
        Error::Join(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
struct StorageState {
    dir: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
static STATE: LazyLock<OnceCell<StorageState>> = LazyLock::new(OnceCell::new);

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("storage".to_string()));
    log::create(Some(tags))
});

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(not(target_arch = "wasm32"))]
async fn state() -> &'static StorageState {
    STATE
        .get_or_init(|| async {
            let dir = storage_root_dir();
            let _ = tokio::fs::create_dir_all(&dir).await;

            let migration_path = dir.join("migration");
            let migration = tokio::fs::read_to_string(&migration_path)
                .await
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0);

            let migrations_len = 1usize;
            for index in migration..migrations_len {
                LOGGER.info(format!("running migration"), {
                    let mut extra = Map::new();
                    extra.insert("index".to_string(), Value::Number(index.into()));
                    Some(extra)
                });
                let result = match index {
                    0 => migration_0(dir.clone()).await,
                    _ => Ok(()),
                };
                if let Err(e) = result {
                    LOGGER.error("failed to run migration", {
                        let mut extra = Map::new();
                        extra.insert("index".to_string(), Value::Number(index.into()));
                        extra.insert("error".to_string(), Value::String(e.to_string()));
                        Some(extra)
                    });
                }
                let _ = tokio::fs::write(&migration_path, (index + 1).to_string()).await;
            }

            StorageState { dir }
        })
        .await
}

#[cfg(not(target_arch = "wasm32"))]
fn storage_root_dir() -> PathBuf {
    storage_root_dir_for_home(&global::paths().home)
}

#[cfg(not(target_arch = "wasm32"))]
fn storage_root_dir_for_home(home: &Path) -> PathBuf {
    vw_config_types::paths::home_config_dir(home).join("storage")
}

#[cfg(not(target_arch = "wasm32"))]
fn target_path(dir: &Path, key: &[&str]) -> PathBuf {
    let mut p = dir.to_path_buf();
    for k in key {
        p.push(k);
    }
    p.set_extension("json");
    p
}

#[cfg(not(target_arch = "wasm32"))]
fn not_found_for(path: &Path) -> Error {
    Error::NotFound(NotFoundError { message: format!("Resource not found: {}", path.display()) })
}

#[cfg(not(target_arch = "wasm32"))]
fn map_io_error(e: std::io::Error, path: &Path) -> Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        return not_found_for(path);
    }
    Error::Io(e)
}

#[cfg(not(target_arch = "wasm32"))]
/// 删除指定 key 对应的 JSON 文件。
///
/// # 参数
///
/// - `key`: 分层 key，每个片段映射为一级路径，最终文件扩展名为 `.json`。
///
/// # 返回值
///
/// 删除成功或文件原本不存在时返回 `Ok(())`。
///
/// # 错误
///
/// 除 NotFound 外的文件系统错误会被映射为 `Error::Io`。
pub async fn remove(key: &[&str]) -> Result<(), Error> {
    let dir = state().await.dir.clone();
    let target = target_path(&dir, key);
    let lock_key = target.to_string_lossy().to_string();
    let _guard = lock::write(&lock_key).await;
    match tokio::fs::remove_file(&target).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(map_io_error(e, &target)),
    }
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台的删除占位实现。
///
/// 本地存储不可用，因此删除操作保持幂等成功。
pub async fn remove(_key: &[&str]) -> Result<(), Error> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 读取指定 key 的 JSON 并反序列化为目标类型。
///
/// # 错误
///
/// 文件不存在返回 `Error::NotFound`；读取失败返回 `Error::Io`；
/// JSON 解析失败返回 `Error::Json`。
pub async fn read<T: DeserializeOwned>(key: &[&str]) -> Result<T, Error> {
    let dir = state().await.dir.clone();
    let target = target_path(&dir, key);
    let lock_key = target.to_string_lossy().to_string();
    let _guard = lock::read(&lock_key).await;
    let content = tokio::fs::read_to_string(&target).await.map_err(|e| map_io_error(e, &target))?;
    Ok(serde_json::from_str::<T>(&content)?)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台的读取占位实现。
///
/// 明确返回 NotFound，避免调用方误以为本地持久化可用。
pub async fn read<T: DeserializeOwned>(_key: &[&str]) -> Result<T, Error> {
    Err(Error::NotFound(NotFoundError { message: "Not supported on WASM".to_string() }))
}

#[cfg(not(target_arch = "wasm32"))]
/// 将可序列化内容写入指定 key。
///
/// 写入前会创建父目录，并对目标路径加写锁。
///
/// # 错误
///
/// 目录创建、JSON 序列化或文件写入失败时返回对应错误。
pub async fn write<T: Serialize>(key: &[&str], content: &T) -> Result<(), Error> {
    let dir = state().await.dir.clone();
    let target = target_path(&dir, key);
    let lock_key = target.to_string_lossy().to_string();
    let _guard = lock::write(&lock_key).await;
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let out = serde_json::to_string_pretty(content)?;
    tokio::fs::write(&target, out).await.map_err(|e| map_io_error(e, &target))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台的写入占位实现。
///
/// 当前不持久化数据，调用方需要通过平台能力另行保存。
pub async fn write<T: Serialize>(_key: &[&str], _content: &T) -> Result<(), Error> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 原子地读取、修改并写回指定 key 的 JSON 内容。
///
/// # 参数
///
/// - `key`: 要更新的分层 key。
/// - `f`: 在内存中修改反序列化值的闭包。
///
/// # 返回值
///
/// 返回写回后的值。
///
/// # 错误
///
/// 文件不存在、解析失败、序列化失败或写回失败时返回错误。
pub async fn update<T: DeserializeOwned + Serialize>(
    key: &[&str],
    f: impl FnOnce(&mut T),
) -> Result<T, Error> {
    let dir = state().await.dir.clone();
    let target = target_path(&dir, key);
    let lock_key = target.to_string_lossy().to_string();
    let _guard = lock::write(&lock_key).await;
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = tokio::fs::read_to_string(&target).await.map_err(|e| map_io_error(e, &target))?;
    let mut value = serde_json::from_str::<T>(&content)?;
    f(&mut value);
    let out = serde_json::to_string_pretty(&value)?;
    tokio::fs::write(&target, out).await.map_err(|e| map_io_error(e, &target))?;
    Ok(value)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台的更新占位实现。
///
/// 明确返回 NotFound，避免隐藏本地存储缺失。
pub async fn update<T: DeserializeOwned + Serialize>(
    _key: &[&str],
    _f: impl FnOnce(&mut T),
) -> Result<T, Error> {
    Err(Error::NotFound(NotFoundError { message: "Not supported on WASM".to_string() }))
}

#[cfg(not(target_arch = "wasm32"))]
/// 列出指定前缀下的所有 JSON key。
///
/// # 参数
///
/// - `prefix`: 要枚举的 key 前缀。
///
/// # 返回值
///
/// 返回排序后的完整 key 列表，文件扩展名会从最后一段中移除。
///
/// # 错误
///
/// 阻塞枚举任务 join 失败时返回 `Error::Join`。
pub async fn list(prefix: &[&str]) -> Result<Vec<Vec<String>>, Error> {
    let dir = state().await.dir.clone();
    let mut base = dir.clone();
    for p in prefix {
        base.push(p);
    }
    let prefix_vec: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();

    let result = tokio::task::spawn_blocking(move || {
        if !base.is_dir() {
            return Vec::<Vec<String>>::new();
        }
        let mut out: Vec<Vec<String>> = Vec::new();
        for entry in WalkDir::new(&base).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Ok(rel) = p.strip_prefix(&base) else { continue };
            let mut parts: Vec<String> = prefix_vec.clone();
            for comp in rel.components() {
                let s = comp.as_os_str().to_string_lossy().to_string();
                parts.push(s);
            }
            if let Some(last) = parts.last_mut() {
                if last.ends_with(".json") {
                    *last = last.trim_end_matches(".json").to_string();
                }
            }
            out.push(parts);
        }
        out.sort();
        out
    })
    .await?;

    Ok(result)
}

#[cfg(target_arch = "wasm32")]
/// WASM 平台的列表占位实现。
///
/// 返回空列表，表示没有可枚举的本地 JSON 存储。
pub async fn list(_prefix: &[&str]) -> Result<Vec<Vec<String>>, Error> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
async fn migration_0(storage_dir: PathBuf) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || migration_0_blocking(&storage_dir)).await?
}

#[cfg(not(target_arch = "wasm32"))]
fn migration_0_blocking(storage_dir: &Path) -> Result<(), Error> {
    let project_dir = global::paths().data.join("project");
    if !project_dir.is_dir() {
        return Ok(());
    }

    let Ok(entries) = std::fs::read_dir(&project_dir) else { return Ok(()) };
    for entry in entries.flatten() {
        let full_project_dir = entry.path();
        if !full_project_dir.is_dir() {
            continue;
        }
        let Some(project_name) =
            full_project_dir.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
        else {
            continue;
        };

        LOGGER.info(format!("migrating project {}", project_name), None);

        let mut project_id = project_name.clone();

        if project_id != "global" {
            // 旧版项目目录名不一定稳定；通过会话消息中的 worktree 找到 Git 根提交，
            // 用根提交作为迁移后的项目 ID，避免同一仓库换路径后生成多个项目。
            let message_root = full_project_dir.join("storage").join("session").join("message");
            if !message_root.is_dir() {
                continue;
            }

            let mut found_root: Option<String> = None;
            for e in WalkDir::new(&message_root).max_depth(3).into_iter().flatten() {
                if !e.file_type().is_file() {
                    continue;
                }
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(s) = std::fs::read_to_string(p) {
                    if let Ok(v) = serde_json::from_str::<Value>(&s) {
                        if let Some(root) =
                            v.get("path").and_then(|x| x.get("root")).and_then(|x| x.as_str())
                        {
                            found_root = Some(root.to_string());
                            break;
                        }
                    }
                }
            }
            let Some(worktree) = found_root else {
                continue;
            };

            if !Path::new(&worktree).is_dir() {
                continue;
            }

            let Ok(repo) = Repository::discover(&worktree) else { continue };
            let Some(id) = root_commit_id(&repo) else { continue };
            project_id = id.clone();

            let project_out_dir = storage_dir.join("project");
            let _ = std::fs::create_dir_all(&project_out_dir);
            let project_out = project_out_dir.join(format!("{}.json", project_id));
            let now = now_ms();
            let mut obj = Map::new();
            obj.insert("id".to_string(), Value::String(id));
            obj.insert("vcs".to_string(), Value::String("git".to_string()));
            obj.insert("worktree".to_string(), Value::String(worktree.clone()));
            let mut time = Map::new();
            time.insert("created".to_string(), Value::Number(now.into()));
            time.insert("initialized".to_string(), Value::Number(now.into()));
            obj.insert("time".to_string(), Value::Object(time));
            let _ = std::fs::write(
                &project_out,
                serde_json::to_string(&Value::Object(obj)).unwrap_or_default(),
            );

            LOGGER.info(format!("migrating session content for project {}", project_id), None);

            let session_message_root =
                full_project_dir.join("storage").join("session").join("message");
            let Ok(session_entries) = std::fs::read_dir(&session_message_root) else { continue };
            for session_entry in session_entries.flatten() {
                let msg_dir = session_entry.path();
                if !msg_dir.is_dir() {
                    continue;
                }
                let Some(session_id) =
                    msg_dir.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
                else {
                    continue;
                };
                LOGGER.info(format!("migrating messages for session {}", session_id), None);

                let Ok(msg_entries) = std::fs::read_dir(&msg_dir) else { continue };
                for msg_entry in msg_entries.flatten() {
                    let msg_file = msg_entry.path();
                    if msg_file.extension().and_then(|s| s.to_str()) != Some("json") {
                        continue;
                    }
                    let Some(msg_basename) =
                        msg_file.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
                    else {
                        continue;
                    };
                    let dest = storage_dir.join("message").join(&session_id).join(&msg_basename);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let Ok(msg_content) = std::fs::read_to_string(&msg_file) else { continue };
                    let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) else {
                        continue;
                    };
                    let _ =
                        std::fs::write(&dest, serde_json::to_string(&msg_json).unwrap_or_default());

                    let Some(message_id) =
                        msg_json.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
                    else {
                        continue;
                    };
                    LOGGER.info(format!("migrating parts for message {}", message_id), None);

                    let part_dir = full_project_dir
                        .join("storage")
                        .join("session")
                        .join("part")
                        .join(&session_id)
                        .join(&message_id);
                    let Ok(part_entries) = std::fs::read_dir(&part_dir) else { continue };
                    for part_entry in part_entries.flatten() {
                        let part_file = part_entry.path();
                        if part_file.extension().and_then(|s| s.to_str()) != Some("json") {
                            continue;
                        }
                        let Some(part_basename) =
                            part_file.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
                        else {
                            continue;
                        };
                        let dest = storage_dir.join("part").join(&message_id).join(&part_basename);
                        if let Some(parent) = dest.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let Ok(part_content) = std::fs::read_to_string(&part_file) else {
                            continue;
                        };
                        let Ok(part_json) = serde_json::from_str::<Value>(&part_content) else {
                            continue;
                        };
                        let _ = std::fs::write(
                            &dest,
                            serde_json::to_string(&part_json).unwrap_or_default(),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn root_commit_id(repo: &Repository) -> Option<String> {
    let mut revwalk = repo.revwalk().ok()?;
    let Ok(mut refs) = repo.references() else { return None };
    while let Some(r) = refs.next() {
        let Ok(r) = r else { continue };
        let Some(oid) = r.target() else { continue };
        let _ = revwalk.push(oid);
    }

    let mut roots: Vec<String> = Vec::new();
    for oid in revwalk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else { continue };
        if commit.parent_count() == 0 {
            roots.push(commit.id().to_string());
        }
    }
    roots.sort();
    roots.into_iter().next()
}
#[cfg(test)]
mod tests;
