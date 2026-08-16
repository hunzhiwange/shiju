//! 会话 UI 存储适配层，负责把桌面会话、快照、原始模型调用和归档状态落盘。

use crate::project;
use crate::session::session as agent_session;
#[cfg(not(target_arch = "wasm32"))]
use crate::session::store::SessionStore;
use crate::session::ui_config::{load_app_config, set_config_field};
/// 当前公开项是模块对外契约的一部分。
pub use crate::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, SessionTodoItem,
    ThinkTiming, TokenUsage, ToolTiming,
};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

use vw_shared::session::path;
use vw_shared::session::ui_store;

#[cfg(not(target_arch = "wasm32"))]
use crate::global_database::GlobalDatabase;

static SESSION_SCOPE: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static SESSION_DATA_DIR: LazyLock<PathBuf> = LazyLock::new(resolve_session_data_dir);
#[cfg(not(target_arch = "wasm32"))]
static RUNTIME_SESSION_STORE: LazyLock<Mutex<Option<SessionStore>>> =
    LazyLock::new(|| Mutex::new(None));

tokio::task_local! {
    static SESSION_TENANT_SCOPE: TenantScope;
}

/// 在显式租户作用域中执行异步会话操作。
pub async fn with_tenant_scope<T>(
    tenant: TenantScope,
    future: impl std::future::Future<Output = T>,
) -> T {
    SESSION_TENANT_SCOPE.scope(tenant, future).await
}

pub(crate) fn current_tenant_scope() -> TenantScope {
    SESSION_TENANT_SCOPE.try_with(Clone::clone).unwrap_or_default()
}

#[cfg(all(not(target_arch = "wasm32"), test))]
thread_local! {
    static TEST_SESSION_STORE: std::cell::RefCell<Option<SessionStore>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(all(not(target_arch = "wasm32"), test))]
struct TestSessionStoreOverride {
    previous: Option<SessionStore>,
}

#[cfg(all(not(target_arch = "wasm32"), test))]
impl Drop for TestSessionStoreOverride {
    fn drop(&mut self) {
        TEST_SESSION_STORE.with(|slot| {
            *slot.borrow_mut() = self.previous.take();
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), test))]
fn override_session_store_for_test(store: SessionStore) -> TestSessionStoreOverride {
    let previous = TEST_SESSION_STORE.with(|slot| slot.borrow_mut().replace(store));
    TestSessionStoreOverride { previous }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopeProjectMetadata {
    scope: String,
    scope_key: String,
    project_id: String,
    directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vcs: Option<project::Vcs>,
    sandboxes: Vec<String>,
    time: project::TimeInfo,
}

impl ScopeProjectMetadata {
    fn from_project(scope: &str, info: &project::Info) -> Self {
        Self {
            scope: scope.to_string(),
            scope_key: path::session_scope_key(scope),
            project_id: info.id.clone(),
            directory: info.worktree.clone(),
            name: info.name.clone(),
            vcs: info.vcs.clone(),
            sandboxes: info.sandboxes.clone(),
            time: info.time.clone(),
        }
    }
}

fn session_scope() -> Option<String> {
    SESSION_SCOPE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// 执行 current_session_scope 操作，并返回调用方需要的结果。
pub fn current_session_scope() -> Option<String> {
    session_scope()
}

/// 执行 set_session_scope 操作，并返回调用方需要的结果。
pub fn set_session_scope(path: Option<&str>) {
    let mut lock = SESSION_SCOPE.lock().unwrap_or_else(|e| e.into_inner());
    *lock = path.map(|s| s.to_string());
}

fn data_dir() -> &'static Path {
    SESSION_DATA_DIR.as_path()
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_session_store() -> Result<SessionStore> {
    #[cfg(test)]
    if let Some(store) = TEST_SESSION_STORE.with(|slot| slot.borrow().clone()) {
        return Ok(store);
    }

    let mut cached = RUNTIME_SESSION_STORE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(store) = cached.as_ref() {
        return Ok(store.clone());
    }

    #[cfg(test)]
    let database = GlobalDatabase::new(
        std::env::temp_dir()
            .join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
    )
    .context("open isolated test session database")?;

    #[cfg(not(test))]
    let database = {
        project::runtime_store().context("initialize project store before session store")?;
        GlobalDatabase::for_home(&crate::global::paths().home)
            .context("open shared global database for session store")?
    };
    let store = SessionStore::new(database).context("initialize global session store")?;
    *cached = Some(store.clone());
    Ok(store)
}

#[cfg(not(target_arch = "wasm32"))]
fn project_scope(tenant: &TenantScope, scope: Option<&str>) -> Result<ProjectScope> {
    let project_id = scope.map(str::trim).filter(|value| !value.is_empty()).ok_or_else(|| {
        anyhow::anyhow!("native session operation requires a non-empty project scope")
    })?;
    ProjectScope::new(tenant.clone(), ProjectId::from(project_id.to_owned()))
        .context("build project session scope")
}

#[cfg(not(target_arch = "wasm32"))]
fn log_session_store_error(
    operation: &str,
    tenant: &TenantScope,
    scope: Option<&str>,
    error: &anyhow::Error,
) {
    tracing::error!(
        target: "vw_agent",
        operation,
        platform_id = tenant.platform_id().as_ref(),
        company_id = tenant.company_id().as_ref(),
        project_id = scope.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("<missing>"),
        error = %error,
        "native session store operation failed"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn run_project_operation<T>(
    operation: &str,
    tenant: &TenantScope,
    scope: Option<&str>,
    callback: impl FnOnce(&SessionStore, &ProjectScope) -> Result<T>,
) -> Option<T> {
    let scope = scope.map(str::trim).filter(|value| !value.is_empty())?;
    let result = (|| {
        let project_scope = project_scope(tenant, Some(scope))?;
        let store = runtime_session_store()?;
        callback(&store, &project_scope)
    })();
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            log_session_store_error(operation, tenant, Some(scope), &error);
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_project_operation_fallible<T>(
    operation: &str,
    tenant: &TenantScope,
    scope: Option<&str>,
    callback: impl FnOnce(&SessionStore, &ProjectScope) -> Result<T>,
) -> Result<T> {
    let scope = scope.map(str::trim).filter(|value| !value.is_empty()).ok_or_else(|| {
        anyhow::anyhow!("native session operation requires a non-empty project scope")
    })?;
    let result = (|| {
        let project_scope = project_scope(tenant, Some(scope))?;
        let store = runtime_session_store()?;
        callback(&store, &project_scope)
    })();
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            log_session_store_error(operation, tenant, Some(scope), &error);
            Err(error)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_tenant_operation<T>(
    operation: &str,
    tenant: &TenantScope,
    callback: impl FnOnce(&SessionStore, &TenantScope) -> Result<T>,
) -> Option<T> {
    let result = (|| {
        let store = runtime_session_store()?;
        callback(&store, tenant)
    })();
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            log_session_store_error(operation, tenant, Some("*"), &error);
            None
        }
    }
}

fn resolve_session_data_dir() -> PathBuf {
    let paths = crate::global::paths();
    let session_data_dir = vw_config_types::paths::home_config_dir(&paths.home);
    let _ = std::fs::create_dir_all(session_data_dir.join("storage").join("session"));
    backfill_scope_project_metadata(&session_data_dir, &paths.data.join("storage").join("project"));
    tracing::debug!(
        target: "vw_agent",
        session_data_dir = %session_data_dir.display(),
        "session ui data directory ready"
    );
    session_data_dir
}

fn project_storage_file_path(project_id: &str) -> PathBuf {
    crate::global::paths().data.join("storage").join("project").join(format!("{project_id}.json"))
}

fn load_project_info(project_id: &str) -> Option<project::Info> {
    let path = project_storage_file_path(project_id);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<project::Info>(&content).ok()
}

fn scope_project_metadata_path(data_dir: &Path, scope: &str) -> Option<PathBuf> {
    Some(path::sessions_dir_for_scope(data_dir, Some(scope))?.join("project.json"))
}

fn write_scope_project_metadata(
    data_dir: &Path,
    scope: &str,
    info: &project::Info,
) -> Option<PathBuf> {
    let metadata_path = scope_project_metadata_path(data_dir, scope)?;
    if let Some(parent) = metadata_path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let metadata = ScopeProjectMetadata::from_project(scope, info);
    let content = serde_json::to_string_pretty(&metadata).ok()?;
    std::fs::write(&metadata_path, content).ok()?;
    Some(metadata_path)
}

fn ensure_scope_project_metadata(scope: Option<&str>) {
    let Some(scope) = scope.map(str::trim).filter(|scope| !scope.is_empty()) else {
        return;
    };
    let Some(info) = load_project_info(scope) else {
        return;
    };
    let _ = write_scope_project_metadata(data_dir(), scope, &info);
}

fn backfill_scope_project_metadata(session_data_dir: &Path, project_storage_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(project_storage_dir) else {
        return;
    };
    let mut written = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(info) = serde_json::from_str::<project::Info>(&content) else {
            continue;
        };
        if write_scope_project_metadata(session_data_dir, &info.id, &info).is_some() {
            written += 1;
        }
    }
    if written > 0 {
        tracing::debug!(
            target: "vw_agent",
            written,
            project_storage_dir = %project_storage_dir.display(),
            "backfilled scoped project metadata files"
        );
    }
}

/// 执行 session_file_path 操作，并返回调用方需要的结果。
pub fn session_file_path(id: &str) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        let scope = session_scope();
        let store = match runtime_session_store() {
            Ok(store) => store,
            Err(error) => {
                log_session_store_error(
                    "resolve session database path",
                    &current_tenant_scope(),
                    scope.as_deref(),
                    &error,
                );
                return None;
            }
        };
        store.storage_path().map(Path::to_path_buf)
    }

    #[cfg(target_arch = "wasm32")]
    {
        path::session_file_path(data_dir(), id, session_scope().as_deref())
    }
}

/// 执行 session_step_snapshot_file_path 操作，并返回调用方需要的结果。
pub fn session_step_snapshot_file_path(
    session_id: &str,
    step_index: u32,
    kind: &str,
) -> Option<PathBuf> {
    path::session_step_snapshot_file_path(
        data_dir(),
        session_id,
        step_index,
        kind,
        session_scope().as_deref(),
    )
}

/// 执行 session_step_llm_raw_file_path 操作，并返回调用方需要的结果。
pub fn session_step_llm_raw_file_path(session_id: &str, step_index: u32) -> Option<PathBuf> {
    path::session_step_llm_raw_file_path(
        data_dir(),
        session_id,
        step_index,
        session_scope().as_deref(),
    )
}

/// 执行 session_step_llm_raw_file_path_scoped 操作，并返回调用方需要的结果。
pub fn session_step_llm_raw_file_path_scoped(
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> Option<PathBuf> {
    path::session_step_llm_raw_file_path_scoped(data_dir(), session_id, step_index, scope)
}

/// 执行 load_llm_raw_step_scoped 操作，并返回调用方需要的结果。
pub fn load_llm_raw_step_scoped(
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> std::io::Result<Option<serde_json::Value>> {
    #[cfg(not(target_arch = "wasm32"))]
    return {
        let default_scope = session_scope();
        let Some(scope) = scope.or_else(|| default_scope.as_deref()) else {
            return Ok(None);
        };
        let tenant = current_tenant_scope();
        let project_scope = project_scope(&tenant, Some(scope))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let store = runtime_session_store().map_err(|error| io::Error::other(error.to_string()))?;
        store
            .load_session_step_llm_raw(&project_scope, session_id, step_index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    };

    #[cfg(target_arch = "wasm32")]
    ui_store::load_llm_raw_step(data_dir(), session_id, step_index, scope)
}

/// 按显式租户和项目 scope 读取单个步骤的原始 LLM 载荷。
pub(crate) fn load_llm_raw_step_scoped_for_tenant(
    tenant: &TenantScope,
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> std::io::Result<Option<serde_json::Value>> {
    #[cfg(not(target_arch = "wasm32"))]
    return {
        let project_scope = project_scope(tenant, scope)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let store = runtime_session_store()
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        store
            .load_session_step_llm_raw(&project_scope, session_id, step_index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    };

    #[cfg(target_arch = "wasm32")]
    {
        let _ = tenant;
        ui_store::load_llm_raw_step(data_dir(), session_id, step_index, scope)
    }
}

pub(crate) fn load_llm_raw_turn_step_scoped_for_tenant(
    tenant: &TenantScope,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> std::io::Result<Option<serde_json::Value>> {
    #[cfg(not(target_arch = "wasm32"))]
    return {
        let project_scope = project_scope(tenant, scope)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        runtime_session_store()
            .map_err(|error| io::Error::other(error.to_string()))?
            .load_session_turn_step_llm_raw(&project_scope, session_id, turn_id, step_index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    };
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (tenant, turn_id);
        ui_store::load_llm_raw_step(data_dir(), session_id, step_index, scope)
    }
}

/// 按显式租户和项目 scope 列出会话已有的原始 LLM step 索引。
pub(crate) fn load_llm_raw_step_indices_scoped_for_tenant(
    tenant: &TenantScope,
    session_id: &str,
    scope: Option<&str>,
) -> std::io::Result<Vec<u32>> {
    #[cfg(not(target_arch = "wasm32"))]
    return {
        let project_scope = project_scope(tenant, scope)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let store = runtime_session_store().map_err(|error| io::Error::other(error.to_string()))?;
        store
            .load_session_llm_raw_step_indices(&project_scope, session_id)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    };

    #[cfg(target_arch = "wasm32")]
    {
        let _ = (tenant, session_id, scope);
        Ok(Vec::new())
    }
}

/// 执行 save_session_step_snapshot 操作，并返回调用方需要的结果。
pub fn save_session_step_snapshot(
    session: &ChatSession,
    step_index: u32,
    kind: &str,
) -> Option<PathBuf> {
    ui_store::save_session_step_snapshot(
        data_dir(),
        session,
        step_index,
        kind,
        session_scope().as_deref(),
    )
}

/// 持久化原始 LLM 步骤，并保留 scope、存储初始化与数据库写入错误。
pub(crate) fn try_persist_llm_raw_step(
    session_id: &str,
    step_index: u32,
    payload: &serde_json::Value,
    scope: Option<&str>,
) -> anyhow::Result<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    return {
        let default_scope = session_scope();
        let scope = scope
            .or_else(|| default_scope.as_deref())
            .ok_or_else(|| anyhow::anyhow!("persist LLM raw step requires a project scope"))?;
        let tenant = current_tenant_scope();
        let project_scope = project_scope(&tenant, Some(scope))?;
        let store = runtime_session_store()?;
        store.save_session_step_llm_raw(&project_scope, session_id, step_index, payload)?;
        Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
    };

    #[cfg(target_arch = "wasm32")]
    ui_store::persist_llm_raw_step(data_dir(), session_id, step_index, payload, scope)
        .ok_or_else(|| anyhow::anyhow!("persist LLM raw step failed"))
}

/// 执行 persist_llm_raw_step 操作，并返回调用方需要的结果。
pub fn persist_llm_raw_step(
    session_id: &str,
    step_index: u32,
    payload: &serde_json::Value,
    scope: Option<&str>,
) -> Option<PathBuf> {
    try_persist_llm_raw_step(session_id, step_index, payload, scope).ok()
}

/// 执行 resolve_session_scope_id 操作，并返回调用方需要的结果。
pub fn resolve_session_scope_id(path: Option<&str>, project_id: Option<&str>) -> Option<String> {
    resolve_session_scope_id_for_tenant(&current_tenant_scope(), path, project_id)
}

/// 按显式租户解析目录对应的项目 scope。
pub fn resolve_session_scope_id_for_tenant(
    tenant: &TenantScope,
    path: Option<&str>,
    project_id: Option<&str>,
) -> Option<String> {
    if let Some(id) = project_id.map(str::trim).filter(|s| !s.is_empty()) {
        ensure_scope_project_metadata(Some(id));
        return Some(id.to_string());
    }
    let path = path.map(str::trim).filter(|s| !s.is_empty())?;
    let path_buf = PathBuf::from(path);

    #[cfg(target_arch = "wasm32")]
    {
        let _ = path_buf;
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let fut = async move { project::from_directory(tenant, path_buf).await.ok() };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return tokio::task::block_in_place(|| handle.block_on(fut)).map(|(info, _)| {
                let scope = info.id.clone();
                let _ = write_scope_project_metadata(data_dir(), &scope, &info);
                scope
            });
        }

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .and_then(|rt| rt.block_on(fut))
            .map(|(info, _)| {
                let scope = info.id.clone();
                let _ = write_scope_project_metadata(data_dir(), &scope, &info);
                scope
            })
    }
}

/// 执行 load_session_todos 操作，并返回调用方需要的结果。
pub fn load_session_todos(session_id: &str) -> Vec<SessionTodoItem> {
    load_session_todos_scoped(session_id, session_scope().as_deref())
}

/// 执行 load_session_todos_scoped 操作，并返回调用方需要的结果。
pub fn load_session_todos_scoped(session_id: &str, scope: Option<&str>) -> Vec<SessionTodoItem> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation(
            "load session todos",
            &current_tenant_scope(),
            scope,
            |store, project_scope| store.load_session_todos(project_scope, session_id),
        )
        .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_session_todos_scoped(data_dir(), session_id, scope)
    }
}

/// 执行 save_session_todos 操作，并返回调用方需要的结果。
pub fn save_session_todos(session_id: &str, todos: &[SessionTodoItem]) -> Option<PathBuf> {
    save_session_todos_scoped(session_id, todos, session_scope().as_deref())
}

/// 执行 save_session_todos_scoped 操作，并返回调用方需要的结果。
pub fn save_session_todos_scoped(
    session_id: &str,
    todos: &[SessionTodoItem],
    scope: Option<&str>,
) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation(
            "save session todos",
            &current_tenant_scope(),
            scope,
            |store, project_scope| {
                store.save_session_todos(project_scope, session_id, todos)?;
                Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
            },
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::save_session_todos_scoped(data_dir(), session_id, todos, scope)
    }
}

/// 执行 load_sessions_scoped 操作，并返回调用方需要的结果。
pub fn load_sessions_scoped(scope: Option<&str>) -> Vec<ChatSessionMeta> {
    load_sessions_scoped_for_tenant(&current_tenant_scope(), scope)
}

/// 按显式租户列出项目会话。
pub fn load_sessions_scoped_for_tenant(
    tenant: &TenantScope,
    scope: Option<&str>,
) -> Vec<ChatSessionMeta> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("list sessions", tenant, scope, SessionStore::list_sessions)
            .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_sessions_scoped(data_dir(), scope)
    }
}

/// 执行 load_sessions 操作，并返回调用方需要的结果。
pub fn load_sessions() -> Vec<ChatSessionMeta> {
    load_sessions_scoped(session_scope().as_deref())
}

/// 执行 load_archived_session_ids_scoped 操作，并返回调用方需要的结果。
pub fn load_archived_session_ids_scoped(scope: Option<&str>) -> HashSet<String> {
    load_archived_session_ids_scoped_for_tenant(&current_tenant_scope(), scope)
}

/// 按显式租户读取项目归档集合。
pub fn load_archived_session_ids_scoped_for_tenant(
    tenant: &TenantScope,
    scope: Option<&str>,
) -> HashSet<String> {
    let cfg = load_app_config(tenant);
    let key = if let Some(scope) = scope {
        format!("archived_session_ids:{}", path::session_scope_key(scope))
    } else {
        "archived_session_ids".to_string()
    };
    cfg.get(&key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<HashSet<String>>()
        })
        .unwrap_or_default()
}

/// 执行 load_archived_session_ids 操作，并返回调用方需要的结果。
pub fn load_archived_session_ids() -> HashSet<String> {
    load_archived_session_ids_scoped(session_scope().as_deref())
}

/// 执行 save_archived_session_ids 操作，并返回调用方需要的结果。
pub fn save_archived_session_ids(ids: &HashSet<String>) {
    save_archived_session_ids_scoped(ids, session_scope().as_deref());
}

/// 执行 save_archived_session_ids_scoped 操作，并返回调用方需要的结果。
pub fn save_archived_session_ids_scoped(ids: &HashSet<String>, scope: Option<&str>) {
    save_archived_session_ids_scoped_for_tenant(&current_tenant_scope(), ids, scope);
}

/// 按显式租户保存项目归档集合。
pub fn save_archived_session_ids_scoped_for_tenant(
    tenant: &TenantScope,
    ids: &HashSet<String>,
    scope: Option<&str>,
) {
    let arr = serde_json::Value::Array(
        ids.iter().map(|v| serde_json::Value::String(v.clone())).collect(),
    );
    let key = if let Some(scope) = scope {
        format!("archived_session_ids:{}", path::session_scope_key(scope))
    } else {
        "archived_session_ids".to_string()
    };
    set_config_field(tenant, &key, arr);
}

/// 执行 save_sessions_scoped 操作，并返回调用方需要的结果。
pub fn save_sessions_scoped(sessions: &[ChatSessionMeta], scope: Option<&str>) {
    ensure_scope_project_metadata(scope);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = run_project_operation(
            "replace session index",
            &current_tenant_scope(),
            scope,
            |store, project_scope| store.replace_session_index(project_scope, sessions),
        );
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::save_sessions_scoped(data_dir(), sessions, scope);
    }
}

/// 执行 save_sessions 操作，并返回调用方需要的结果。
pub fn save_sessions(sessions: &[ChatSessionMeta]) {
    save_sessions_scoped(sessions, session_scope().as_deref())
}

/// 执行 load_session_scoped 操作，并返回调用方需要的结果。
pub fn load_session_scoped(id: &str, scope: Option<&str>) -> Option<ChatSession> {
    load_session_scoped_for_tenant(&current_tenant_scope(), id, scope)
}

/// 按显式租户读取项目会话。
pub fn load_session_scoped_for_tenant(
    tenant: &TenantScope,
    id: &str,
    scope: Option<&str>,
) -> Option<ChatSession> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("load session", tenant, scope, |store, project_scope| {
            store.load_session(project_scope, id)
        })
        .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_session_scoped(data_dir(), id, scope)
    }
}

/// 按显式租户读取项目会话，并保留存储初始化与查询错误。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_session_scoped_for_tenant_fallible(
    tenant: &TenantScope,
    id: &str,
    scope: Option<&str>,
) -> anyhow::Result<Option<ChatSession>> {
    run_project_operation_fallible("load session", tenant, scope, |store, project_scope| {
        store.load_session(project_scope, id)
    })
}

/// 执行 load_session 操作，并返回调用方需要的结果。
pub fn load_session(id: &str) -> Option<ChatSession> {
    load_session_scoped(id, session_scope().as_deref())
}

/// 执行 load_session_any 操作，并返回调用方需要的结果。
pub fn load_session_any(id: &str) -> Option<ChatSession> {
    load_session_any_for_tenant(&current_tenant_scope(), id)
}

/// 按显式租户跨项目读取会话。
pub fn load_session_any_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSession> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_tenant_operation("load session across projects", tenant, |store, tenant| {
            store.load_session_any(tenant, id)
        })
        .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_session_any(data_dir(), id, session_scope().as_deref())
    }
}

/// 执行 load_agent_session_scoped 操作，并返回调用方需要的结果。
pub fn load_agent_session_scoped(
    session_id: &str,
    scope: Option<&str>,
) -> Option<agent_session::Info> {
    load_agent_session_scoped_for_tenant(&current_tenant_scope(), session_id, scope)
}

/// 按显式租户读取项目 agent 会话。
pub fn load_agent_session_scoped_for_tenant(
    tenant: &TenantScope,
    session_id: &str,
    scope: Option<&str>,
) -> Option<agent_session::Info> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("load agent session", tenant, scope, |store, project_scope| {
            store.load_agent_session(project_scope, session_id)
        })
        .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_agent_session_scoped(data_dir(), session_id, scope)
    }
}

/// 执行 load_agent_sessions_scoped 操作，并返回调用方需要的结果。
pub fn load_agent_sessions_scoped(scope: Option<&str>) -> Vec<agent_session::Info> {
    load_agent_sessions_scoped_for_tenant(&current_tenant_scope(), scope)
}

/// 按显式租户列出项目 agent 会话。
pub fn load_agent_sessions_scoped_for_tenant(
    tenant: &TenantScope,
    scope: Option<&str>,
) -> Vec<agent_session::Info> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation(
            "list agent sessions",
            tenant,
            scope,
            SessionStore::load_agent_sessions,
        )
        .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_agent_sessions_scoped(data_dir(), scope)
    }
}

/// 执行 load_agent_sessions_all 操作，并返回调用方需要的结果。
pub fn load_agent_sessions_all() -> Vec<agent_session::Info> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        load_agent_sessions_all_for_tenant(&current_tenant_scope())
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_agent_sessions_all(data_dir(), session_scope().as_deref())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_agent_sessions_all_for_tenant(tenant: &TenantScope) -> Vec<agent_session::Info> {
    run_tenant_operation("list agent sessions across projects", tenant, |store, tenant| {
        store.load_agent_sessions_all(tenant)
    })
    .unwrap_or_default()
}

/// 执行 load_agent_session_any 操作，并返回调用方需要的结果。
pub fn load_agent_session_any(session_id: &str) -> Option<agent_session::Info> {
    load_agent_session_any_for_tenant(&current_tenant_scope(), session_id)
}

/// 按显式租户跨项目读取 agent 会话。
pub fn load_agent_session_any_for_tenant(
    tenant: &TenantScope,
    session_id: &str,
) -> Option<agent_session::Info> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_tenant_operation("load agent session across projects", tenant, |store, tenant| {
            store.load_agent_session_any(tenant, session_id)
        })
        .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::load_agent_session_any(data_dir(), session_id, session_scope().as_deref())
    }
}

/// 执行 session_preview_meta 操作，并返回调用方需要的结果。
pub fn session_preview_meta(id: &str) -> Option<ChatSessionMeta> {
    session_preview_meta_for_tenant(&current_tenant_scope(), id)
}

/// 按显式租户读取会话预览。
pub fn session_preview_meta_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSessionMeta> {
    let scope = session_scope();
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation(
            "load session preview",
            tenant,
            scope.as_deref(),
            |store, project_scope| store.load_session_meta(project_scope, id),
        )
        .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::session_preview_meta(data_dir(), id, scope.as_deref())
    }
}

/// 执行 save_session 操作，并返回调用方需要的结果。
pub fn save_session(session: &ChatSession) -> Option<PathBuf> {
    save_session_scoped(session, session_scope().as_deref())
}

/// 执行 save_agent_session_scoped 操作，并返回调用方需要的结果。
pub fn save_agent_session_scoped(
    info: &agent_session::Info,
    scope: Option<&str>,
) -> Option<PathBuf> {
    save_agent_session_scoped_for_tenant(&current_tenant_scope(), info, scope)
}

/// 按显式租户保存 agent 会话。
pub fn save_agent_session_scoped_for_tenant(
    tenant: &TenantScope,
    info: &agent_session::Info,
    scope: Option<&str>,
) -> Option<PathBuf> {
    let scope = scope.or(Some(&info.project_id));
    ensure_scope_project_metadata(scope);
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("save agent session", tenant, scope, |store, project_scope| {
            store.save_agent_session(project_scope, info)?;
            Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::save_agent_session_scoped(data_dir(), info, scope)
    }
}

/// 执行 save_session_scoped 操作，并返回调用方需要的结果。
pub fn save_session_scoped(session: &ChatSession, scope: Option<&str>) -> Option<PathBuf> {
    save_session_scoped_for_tenant(&current_tenant_scope(), session, scope)
}

/// 按显式租户保存项目会话。
pub fn save_session_scoped_for_tenant(
    tenant: &TenantScope,
    session: &ChatSession,
    scope: Option<&str>,
) -> Option<PathBuf> {
    ensure_scope_project_metadata(scope);
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("save session", tenant, scope, |store, project_scope| {
            store.save_session(project_scope, session)?;
            Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::save_session_scoped(data_dir(), session, scope)
    }
}

/// 按显式租户保存 UI 会话，并将持久化失败返回给调用方。
pub fn save_session_ui_scoped_for_tenant(
    tenant: &TenantScope,
    session: &ChatSession,
    scope: Option<&str>,
    preserve_calls: bool,
) -> anyhow::Result<PathBuf> {
    ensure_scope_project_metadata(scope);
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation_fallible("save session UI", tenant, scope, |store, project_scope| {
            if preserve_calls {
                store.save_session_ui_snapshot(project_scope, session)?;
            } else {
                store.save_session(project_scope, session)?;
            }
            Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = preserve_calls;
        ui_store::save_session_scoped(data_dir(), session, scope)
            .ok_or_else(|| anyhow::anyhow!("save session UI failed"))
    }
}

/// 按显式租户原子提交 UI 快照与 Gateway 已声明的原始 LLM 工件。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn commit_session_ui_snapshot_scoped_for_tenant(
    tenant: &TenantScope,
    session: &ChatSession,
    scope: Option<&str>,
    raw_artifacts: &[crate::session::llm_raw::LlmRawArtifact],
    expected_finished_steps: &[crate::session::store::LlmRawStepKey],
) -> anyhow::Result<PathBuf> {
    ensure_scope_project_metadata(scope);
    run_project_operation_fallible(
        "commit session UI raw snapshot",
        tenant,
        scope,
        |store, project_scope| {
            store.commit_session_ui_snapshot(
                project_scope,
                session,
                raw_artifacts,
                expected_finished_steps,
            )?;
            Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
        },
    )
}

/// 按显式租户保存 UI 快照，但保留独立持久化的原始调用记录。
pub fn save_session_ui_snapshot_scoped_for_tenant(
    tenant: &TenantScope,
    session: &ChatSession,
    scope: Option<&str>,
) -> Option<PathBuf> {
    ensure_scope_project_metadata(scope);
    #[cfg(not(target_arch = "wasm32"))]
    {
        run_project_operation("save session UI snapshot", tenant, scope, |store, project_scope| {
            store.save_session_ui_snapshot(project_scope, session)?;
            Ok(store.storage_path().map_or_else(|| data_dir().to_path_buf(), Path::to_path_buf))
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::save_session_scoped(data_dir(), session, scope)
    }
}

/// 执行 delete_session 操作，并返回调用方需要的结果。
pub fn delete_session(id: &str) {
    delete_session_scoped(id, session_scope().as_deref());
}

/// 执行 delete_agent_session_scoped 操作，并返回调用方需要的结果。
pub fn delete_agent_session_scoped(id: &str, scope: Option<&str>) {
    delete_agent_session_scoped_for_tenant(&current_tenant_scope(), id, scope);
}

/// 按显式租户删除 agent 会话。
pub fn delete_agent_session_scoped_for_tenant(tenant: &TenantScope, id: &str, scope: Option<&str>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ =
            run_project_operation("delete agent session", tenant, scope, |store, project_scope| {
                store.delete_agent_session(project_scope, id)
            });
        release_image_assets_for_deleted_session(id, scope);
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::delete_agent_session_scoped(data_dir(), id, scope);
    }
}

/// 执行 delete_session_scoped 操作，并返回调用方需要的结果。
pub fn delete_session_scoped(id: &str, scope: Option<&str>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = run_project_operation(
            "delete session",
            &current_tenant_scope(),
            scope,
            |store, project_scope| store.delete_session(project_scope, id),
        );
        release_image_assets_for_deleted_session(id, scope);
    }

    #[cfg(target_arch = "wasm32")]
    {
        ui_store::delete_session_scoped(data_dir(), id, scope);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn release_image_assets_for_deleted_session(session_id: &str, scope: Option<&str>) {
    use crate::app::agent::multimodal::assets::{ImageAssetScope, ImageAssetStore};

    let scope_key = scope.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(session_id);
    let Ok(asset_scope) = ImageAssetScope::new(scope_key) else {
        return;
    };
    let store = ImageAssetStore::from_global_paths();
    if let Err(error) = store.release_session_refs(&asset_scope, session_id) {
        tracing::warn!(
            target: "vw_agent",
            session_id = %session_id,
            error = %error,
            "failed to release image asset references after session delete"
        );
    }
}

/// 执行 append_session_call 操作，并返回调用方需要的结果。
pub fn append_session_call(session: &str, payload: &serde_json::Value) -> Option<PathBuf> {
    let mut s = load_session(session).unwrap_or(ChatSession {
        id: session.to_string(),
        title: "新会话".to_string(),
        messages: vec![],
        message_ids: vec![],
        calls: vec![],
        steps: vec![],
        created_ms: agent_session::now_ms(),
        updated_ms: 0,
    });
    s.calls.push(payload.clone());
    s.updated_ms = agent_session::now_ms();
    save_session(&s)
}

/// 执行 persist_ai_call_payload 操作，并返回调用方需要的结果。
pub fn persist_ai_call_payload(
    session: &str,
    stream: u64,
    payload: &serde_json::Value,
    scope: Option<&str>,
) -> Option<PathBuf> {
    let mut s = load_session_scoped(session, scope).unwrap_or(ChatSession {
        id: session.to_string(),
        title: "新会话".to_string(),
        messages: vec![],
        message_ids: vec![],
        calls: vec![],
        steps: vec![],
        created_ms: agent_session::now_ms(),
        updated_ms: 0,
    });
    let stream_id =
        payload.get("stream_id").and_then(serde_json::Value::as_u64).or(Some(stream)).unwrap_or(0);
    if stream_id != 0 {
        if let Some(pos) = s.calls.iter().rposition(|c| {
            c.get("stream_id")
                .and_then(serde_json::Value::as_u64)
                .map(|id| id == stream_id)
                .unwrap_or(false)
        }) {
            s.calls[pos] = payload.clone();
        } else {
            s.calls.push(payload.clone());
        }
    } else {
        s.calls.push(payload.clone());
    }
    s.updated_ms = agent_session::now_ms();
    save_session_scoped(&s, scope)
}

/// 执行 save_session_snapshot 操作，并返回调用方需要的结果。
pub fn save_session_snapshot(session: &ChatSession) -> Option<PathBuf> {
    save_session(session)
}
#[cfg(test)]
#[path = "ui_store_tests.rs"]
mod ui_store_tests;
