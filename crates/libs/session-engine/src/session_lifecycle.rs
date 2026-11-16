use agent_bus as bus;
use ordered_id as id;

// CreateInput permission types are owned by session-engine::session_io.
use agent_project::instance;
use agent_storage as storage;
use agent_log as log;
use std::sync::LazyLock;

use serde_json::{Map, Value, json};

pub use crate::session_memory::{Session, Message, Role};

/// 会话模块专用日志记录器
///
/// 使用 Lazy 初始化，带有 "session" 服务标签。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("session".to_string()));
    log::create(Some(tags))
});

/// 会话相关事件定义
///
/// 定义了会话生命周期中发布的各种事件类型，
/// 用于通过事件总线通知其他组件会话状态变更。
pub mod event {
    use agent_bus as bus;

    /// 会话创建事件
    pub const CREATED: bus::Definition = bus::Definition { r#type: "session.created" };
    /// 会话更新事件
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "session.updated" };
    /// 会话删除事件
    pub const DELETED: bus::Definition = bus::Definition { r#type: "session.deleted" };
    /// 会话差异事件（用于同步）
    pub const DIFF: bus::Definition = bus::Definition { r#type: "session.diff" };
    /// 会话错误事件
    pub const ERROR: bus::Definition = bus::Definition { r#type: "session.error" };
}

pub use session_info::Error;

pub use session_info::{Info, RevertInfo, ShareInfo, Summary, TimeInfo};
pub use session_text::{create_slug, is_default_title};

/// 创建会话的输入参数
///
/// 用于 `create_next` 函数的输入参数结构。
pub use crate::session_io::CreateInput;

/// 获取当前时间的 Unix 毫秒时间戳
///
/// 使用 `web_time` 以支持 WebAssembly 环境。
///
/// # 返回
///
/// 返回自 Unix 纪元以来的毫秒数
pub fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 父会话的默认标题前缀
const PARENT_TITLE_PREFIX: &str = "New session - ";
/// 子会话的默认标题前缀
const CHILD_TITLE_PREFIX: &str = "Child session - ";

/// 创建默认会话标题
///
/// 根据是否为子会话生成带时间戳的默认标题。
/// 格式：`{前缀}YYYY-MM-DDTHH:MM:SS.sssZ`
///
/// # 参数
///
/// - `is_child`: 是否为子会话（有父会话）
///
/// # 返回
///
/// 返回格式化的默认标题字符串
fn create_default_title(is_child: bool) -> String {
    let prefix = if is_child { CHILD_TITLE_PREFIX } else { PARENT_TITLE_PREFIX };
    let dt = time::OffsetDateTime::from_unix_timestamp_nanos((now_ms() as i128) * 1_000_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
        .to_offset(time::UtcOffset::UTC);
    let s = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        dt.year(),
        u8::from(dt.month()),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
        dt.millisecond()
    );
    format!("{}{}", prefix, s)
}

/// 生成分支会话的标题
///
/// 当从现有会话创建分支时，自动在标题后添加分支编号。
/// 如果标题已有分支编号，则递增编号；否则添加 "(fork #1)"。
///
/// # 参数
///
/// - `title`: 原始会话标题
///
/// # 返回
///
/// 返回带分支编号的新标题
#[cfg(test)]
fn forked_title(title: &str) -> String { crate::session_fork::forked_title(title) }

/// 获取当前项目 ID
///
/// 从项目实例上下文中获取当前项目 ID。
///
/// # 返回
///
/// - `Ok(String)`: 成功返回项目 ID
/// - `Err(Error::NoProjectContext)`: 无活动项目上下文
fn project_id() -> Result<String, Error> {
    let Some(project) = instance::project() else {
        return Err(Error::NoProjectContext);
    };
    if project.id.is_empty() {
        return Err(Error::NoProjectContext);
    }
    Ok(project.id)
}

/// 获取实例目录（可选）
///
/// 获取当前项目实例的工作目录路径，如果为空则返回 None。
///
/// # 返回
///
/// 返回 Some(directory) 或 None
fn instance_directory_opt() -> Option<String> {
    let d = instance::directory();
    if d.is_empty() { None } else { Some(d) }
}

/// 创建新会话
///
/// 根据输入参数创建一个新的会话，并持久化到存储层。
/// 创建成功后会发布 `session.created` 事件。
///
/// # 参数
///
/// - `input`: 创建会话的输入参数
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回创建的会话元数据
/// - `Err(Error)`: 创建失败
///
/// # 示例
///
/// ```no_run
/// use session_engine::session_lifecycle::{create_next, CreateInput};
///
/// async fn example() {
///     let input = CreateInput {
///         parent_id: None,
///         title: Some("新会话".to_string()),
///         directory: "/path".to_string(),
///         permission: None,
///     };
///     let info = create_next::<H>(input).await.unwrap();
/// }
/// ```
pub async fn create_next<H: LifecycleHost>(input: CreateInput) -> Result<Info, Error> {
    #[cfg(not(target_arch = "wasm32"))]
    let _worktree_use =
        worktree_core::registry::use_lease(std::path::Path::new(&input.directory))
            .map_err(|error| Error::Runtime(anyhow::Error::new(error)))?;
    let info = new_info::<H>(input)?;

    // 记录创建日志
    LOGGER.info(
        "created",
        Some(extra([
            ("id", Value::String(info.id.clone())),
            ("projectID", Value::String(info.project_id.clone())),
            ("directory", Value::String(info.directory.clone())),
        ])),
    );

    persist_info(&info).await?;

    // 发布创建事件
    let _ = bus::publish(event::CREATED, json!({ "info": info }), instance_directory_opt());
    get::<H>(&info.id).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn reserve_next<H: LifecycleHost>(input: CreateInput) -> Result<Info, Error> {
    reserve_info::<H>(new_info::<H>(input)?).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn reserve_visible_task<H: LifecycleHost>(input: CreateInput) -> Result<Info, Error> {
    let mut info = new_info::<H>(input)?;
    info.slug = Info::visible_task_slug(&info.slug);
    reserve_info::<H>(info).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn reserve_child_agent<H: LifecycleHost>(input: CreateInput) -> Result<Info, Error> {
    let mut info = new_info::<H>(input)?;
    info.mark_agent_tool();
    reserve_info::<H>(info).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn reserve_info<H: LifecycleHost>(info: Info) -> Result<Info, Error> {
    let _worktree_use = worktree_core::registry::use_lease(std::path::Path::new(&info.directory))
        .map_err(|error| Error::Runtime(anyhow::Error::new(error)))?;
    let tenant = session_store::ui_store::current_tenant_scope();
    let _metadata = lock_metadata_writes(&tenant).await;
    validate_parent(&tenant, &info).await?;
    Ok(agent_database::run_isolated(move || {
        session_store::ui_store::reserve_pending_session(&tenant, &info)
    })
    .await??)
}

fn new_info<H: LifecycleHost>(input: CreateInput) -> Result<Info, Error> {
    #[cfg(not(target_arch = "wasm32"))]
    worktree_core::environment::ensure_ready(&input.directory)
        .map_err(|error| Error::Runtime(anyhow::Error::new(error)))?;
    let now = now_ms();
    let project_id = project_id()?;
    let title = input.title.unwrap_or_else(|| create_default_title(input.parent_id.is_some()));
    Ok(Info {
        id: id::descending(id::Prefix::Session, None)?,
        slug: create_slug(),
        project_id,
        directory: input.directory,
        parent_id: input.parent_id,
        summary: None,
        share: None,
        title,
        version: H::version(),
        time: TimeInfo { created: now, updated: now, compacting: None, archived: None },
        permission: input.permission,
        fork_execution_started: None,
        revert: None,
    })
}

async fn persist_info(info: &Info) -> Result<(), Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    let _metadata = lock_metadata_writes(&tenant).await;
    validate_parent(&tenant, info).await?;
    let save_info = info.clone();
    agent_database::run_isolated(move || {
        session_store::ui_store::save_agent_session_scoped_for_tenant(
            &tenant,
            &save_info,
            Some(&save_info.project_id),
        )
    })
    .await?
    .ok_or_else(|| {
        Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("failed to persist session: {}", info.id),
        }))
    })?;

    Ok(())
}

/// Acquire after any history guards; never acquire history while holding this guard.
pub async fn lock_metadata_writes(
    tenant: &vw_api_transport::TenantScope,
) -> tokio::sync::OwnedMutexGuard<()> {
    super::message::lock_history(&format!("session-metadata/{}", json!(tenant))).await
}

async fn validate_parent(tenant: &vw_api_transport::TenantScope, info: &Info) -> Result<(), Error> {
    let Some(parent_id) = info.parent_id.clone() else {
        return Ok(());
    };
    let tenant = tenant.clone();
    let project_id = info.project_id.clone();
    let parent = agent_database::run_isolated(move || {
        session_store::ui_store::load_agent_session_strict(&tenant, &project_id, &parent_id)
    })
    .await??
    .ok_or_else(|| anyhow::anyhow!("parent session no longer exists"))?;
    let canonical = |directory: &str| {
        std::fs::canonicalize(directory).unwrap_or_else(|_| std::path::PathBuf::from(directory))
    };
    if canonical(&parent.directory) != canonical(&info.directory) {
        return Err(anyhow::anyhow!("parent session belongs to another directory").into());
    }
    Ok(())
}

/// 获取会话信息
///
/// 根据会话 ID 从当前项目上下文中读取会话元数据。
///
/// # 参数
///
/// - `session_id`: 会话 ID
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回会话元数据
/// - `Err(Error)`: 会话不存在或读取失败
pub async fn get<H: LifecycleHost>(session_id: &str) -> Result<Info, Error> {
    let project_id = project_id()?;
    let tenant = session_store::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let mut info = agent_database::run_isolated({
        let session_id = session_id.clone();
        move || {
            session_store::ui_store::load_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?
    .ok_or_else(|| {
        Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        }))
    })?;
    project_fork::<H>(&mut info).await?;
    Ok(info)
}

/// 更新会话的最后修改时间
///
/// 触碰会话以更新其 `time.updated` 字段为当前时间。
///
/// # 参数
///
/// - `session_id`: 会话 ID
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回更新后的会话信息
/// - `Err(Error)`: 更新失败
pub async fn touch<H: LifecycleHost>(session_id: &str) -> Result<Info, Error> {
    update::<H>(session_id, |draft| {
        draft.time.updated = now_ms();
    })
    .await
}

/// 更新会话信息
///
/// 使用闭包更新会话元数据，自动更新 `time.updated` 字段，
/// 并发布 `session.updated` 事件。
///
/// # 参数
///
/// - `session_id`: 会话 ID
/// - `f`: 更新闭包，接收可变引用的会话信息
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回更新后的会话信息
/// - `Err(Error)`: 更新失败
pub async fn update<H: LifecycleHost>(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    let project_id = project_id()?;
    let tenant = session_store::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let result = agent_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            session_store::ui_store::load_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?
    .ok_or_else(|| {
        Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        }))
    })?;
    apply_update::<H>(tenant, result, f).await
}

/// 列出当前项目的所有会话
///
/// 获取当前项目上下文中的所有会话，按最后更新时间降序排列。
///
/// # 返回
///
/// - `Ok(Vec<Info>)`: 成功返回会话列表
/// - `Err(Error)`: 读取失败
pub async fn list<H: LifecycleHost>() -> Result<Vec<Info>, Error> {
    let project_id = project_id()?;
    list_for_project::<H>(project_id).await
}

pub async fn list_for_project<H: LifecycleHost>(project_id: String) -> Result<Vec<Info>, Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    let mut out = agent_database::run_isolated(move || {
        session_store::ui_store::load_agent_sessions_strict(&tenant, &project_id)
    })
    .await??;
    for info in &mut out {
        project_fork::<H>(info).await?;
    }
    out.sort_by(|a, b| b.time.updated.cmp(&a.time.updated));
    Ok(out)
}

/// 获取指定父会话的所有子会话
///
/// 从当前项目的会话列表中筛选出指定父会话的子会话。
///
/// # 参数
///
/// - `parent_id`: 父会话 ID
///
/// # 返回
///
/// - `Ok(Vec<Info>)`: 成功返回子会话列表
/// - `Err(Error)`: 读取失败
pub async fn children(parent_id: &str) -> Result<Vec<Info>, Error> {
    let mut children = children_including_pending(parent_id).await?;
    children.retain(|info| !info.is_first_response_pending());
    Ok(children)
}

/// Internal parent lookup includes reservations still running their first turn.
pub async fn children_including_pending(parent_id: &str) -> Result<Vec<Info>, Error> {
    let project_id = project_id()?;
    let tenant = session_store::ui_store::current_tenant_scope();
    let parent_id = parent_id.to_owned();
    Ok(agent_database::run_isolated(move || {
        session_store::ui_store::load_agent_children(&tenant, &project_id, &parent_id)
    })
    .await??)
}

/// 列出所有项目的所有会话
///
/// 跨项目列出所有会话，按最后更新时间降序排列。
/// 主要用于管理界面或全局搜索。
///
/// # 返回
///
/// - `Ok(Vec<Info>)`: 成功返回所有会话列表
/// - `Err(Error)`: 读取失败
pub async fn list_all<H: LifecycleHost>() -> Result<Vec<Info>, Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    let mut out = agent_database::run_isolated(move || {
        session_store::ui_store::load_agent_sessions_all_strict(&tenant)
    })
    .await??;
    for info in &mut out {
        project_fork::<H>(info).await?;
    }
    out.sort_by(|a, b| b.time.updated.cmp(&a.time.updated));
    Ok(out)
}

/// 获取任意会话信息（跨项目）
///
/// 不限定当前项目上下文，跨项目查找并返回会话信息。
///
/// # 参数
///
/// - `session_id`: 会话 ID
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回会话元数据
/// - `Err(Error)`: 会话不存在或读取失败
pub async fn get_any<H: LifecycleHost>(session_id: &str) -> Result<Info, Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(mut info) = agent_database::run_isolated({
        let session_id = session_id.clone();
        move || session_store::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
    project_fork::<H>(&mut info).await?;
    Ok(info)
}

/// 更新任意会话信息（跨项目）
///
/// 不限定当前项目上下文，跨项目查找并更新会话信息。
///
/// # 参数
///
/// - `session_id`: 会话 ID
/// - `f`: 更新闭包
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回更新后的会话信息
/// - `Err(Error)`: 会话不存在或更新失败
pub async fn update_any<H: LifecycleHost>(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(result) = agent_database::run_isolated({
        let tenant = tenant.clone();
        let session_id = session_id.clone();
        move || session_store::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
    apply_update::<H>(tenant, result, f).await
}

async fn apply_update<H: LifecycleHost>(
    tenant: vw_api_transport::TenantScope,
    info: Info,
    update: impl FnOnce(&mut Info),
) -> Result<Info, Error> {
    let metadata = lock_metadata_writes(&tenant).await;
    let mut info = agent_database::run_isolated({
        let tenant = tenant.clone();
        move || session_store::ui_store::load_agent_session_strict(&tenant, &info.project_id, &info.id)
    })
    .await??
    .ok_or_else(|| anyhow::anyhow!("session no longer exists"))?;
    let session_id = info.id.clone();
    let project_id = info.project_id.clone();
    let previous_title = info.title.clone();
    let previous_parent = info.parent_id.clone();
    let previous_directory = info.directory.clone();
    update(&mut info);
    if info.id != session_id || info.project_id != project_id {
        return Err(anyhow::anyhow!("session identity cannot change during an update").into());
    }
    if info.parent_id != previous_parent || info.directory != previous_directory {
        validate_parent(&tenant, &info).await?;
    }
    let explicit_title = info.title != previous_title;
    info.time.updated = now_ms();
    let result = agent_database::run_isolated(move || {
        session_store::ui_store::update_agent_session(&tenant, info, explicit_title)
    })
    .await??
    .ok_or_else(|| {
        Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {session_id}"),
        }))
    })?;
    drop(metadata);
    if result.time.archived.is_some() {
        H::close_hooks(&result.id, std::path::Path::new(&result.directory)).await;
        H::close_mcp(
            &result.id,
            std::path::Path::new(&result.directory),
        );
    }
    // 记录更新日志
    LOGGER.info(
        "updated",
        Some(extra([
            ("id", Value::String(result.id.clone())),
            ("projectID", Value::String(result.project_id.clone())),
        ])),
    );
    // 发布更新事件
    if !result.is_first_response_pending() {
        let _ = bus::publish(
            event::UPDATED,
            json!({ "info": result.clone() }),
            instance_directory_opt(),
        );
    }
    Ok(result)
}

/// 删除会话
///
/// 删除当前项目上下文中的指定会话及其关联数据（消息、部分、分享信息等）。
/// 删除成功后会发布 `session.deleted` 事件。
///
/// # 参数
///
/// - `session_id`: 要删除的会话 ID
///
/// # 返回
///
/// - `Ok(())`: 删除成功
/// - `Err(Error)`: 删除失败
pub async fn remove<H: LifecycleHost>(session_id: &str) -> Result<(), Error> {
    let _history = super::message::lock_history(session_id).await;
    let project_id = project_id()?;
    let tenant = session_store::ui_store::current_tenant_scope();
    // 先读取会话信息（用于事件发布）
    let info = get::<H>(session_id).await?;
    H::close_hooks(&info.id, std::path::Path::new(&info.directory)).await;
    H::close_mcp(&info.id, std::path::Path::new(&info.directory));
    #[cfg(not(target_arch = "wasm32"))]
    tool_shell::shell::processes::cancel_session(
        &serde_json::to_string(&tenant).expect("tenant serialization"),
        std::path::Path::new(&info.directory),
        &info.id,
    )
    .map_err(Error::Runtime)?;
    // 删除会话主数据
    let session_id = session_id.to_string();
    agent_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            session_store::ui_store::delete_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?;
    crate::compaction_checkpoint::remove(&checkpoint_scope(&info.directory), &session_id)
        .await
        .map_err(|error| Error::Runtime(anyhow::anyhow!(error)))?;
    // 删除关联的分享信息
    storage::remove(&["share", &session_id]).await.ok();
    // 删除关联的消息和部分
    remove_messages_and_parts(&session_id).await.ok();
    // 发布删除事件
    LOGGER.info(
        "deleted",
        Some(extra([
            ("id", Value::String(info.id.clone())),
            ("projectID", Value::String(info.project_id.clone())),
        ])),
    );
    let _ = bus::publish(event::DELETED, json!({ "info": info }), instance_directory_opt());
    Ok(())
}

/// 删除任意会话（跨项目）
///
/// 不限定当前项目上下文，跨项目查找并删除会话及其关联数据。
///
/// # 参数
///
/// - `session_id`: 要删除的会话 ID
///
/// # 返回
///
/// - `Ok(())`: 删除成功
/// - `Err(Error)`: 会话不存在或删除失败
pub async fn remove_any<H: LifecycleHost>(session_id: &str) -> Result<(), Error> {
    let _history = super::message::lock_history(session_id).await;
    let tenant = session_store::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(info) = agent_database::run_isolated({
        let tenant = tenant.clone();
        let session_id = session_id.clone();
        move || session_store::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
    let project_id = info.project_id.clone();
    H::close_hooks(&info.id, std::path::Path::new(&info.directory)).await;
    H::close_mcp(&info.id, std::path::Path::new(&info.directory));
    #[cfg(not(target_arch = "wasm32"))]
    tool_shell::shell::processes::cancel_session(
        &serde_json::to_string(&tenant).expect("tenant serialization"),
        std::path::Path::new(&info.directory),
        &info.id,
    )
    .map_err(Error::Runtime)?;
    // 删除会话主数据
    agent_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            session_store::ui_store::delete_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?;
    crate::compaction_checkpoint::remove(&checkpoint_scope(&info.directory), &session_id)
        .await
        .map_err(|error| Error::Runtime(anyhow::anyhow!(error)))?;
    // 删除关联的分享信息
    storage::remove(&["share", &session_id]).await.ok();
    // 删除关联的消息和部分
    remove_messages_and_parts(&session_id).await.ok();
    // 发布删除事件
    LOGGER.info(
        "deleted",
        Some(extra([
            ("id", Value::String(info.id.clone())),
            ("projectID", Value::String(info.project_id.clone())),
        ])),
    );
    let _ = bus::publish(event::DELETED, json!({ "info": info }), instance_directory_opt());
    Ok(())
}

/// Strict lifecycle cleanup for callers already holding history and metadata guards.
pub async fn remove_archived_locked<H: LifecycleHost>(info: Info) -> Result<(), Error> {
    let tenant = session_store::ui_store::current_tenant_scope();
    H::close_hooks(&info.id, std::path::Path::new(&info.directory)).await;
    H::close_mcp(&info.id, std::path::Path::new(&info.directory));
    #[cfg(not(target_arch = "wasm32"))]
    tool_shell::shell::processes::cancel_session(
        &serde_json::to_string(&tenant).expect("tenant serialization"),
        std::path::Path::new(&info.directory),
        &info.id,
    )
    .map_err(Error::Runtime)?;
    crate::compaction_checkpoint::remove_strict(&checkpoint_scope(&info.directory), &info.id)
        .await
        .map_err(|error| Error::Runtime(anyhow::anyhow!(error)))?;
    storage::remove(&["share", &info.id]).await?;
    remove_messages_and_parts_strict(&info.id).await?;
    let deleted = agent_database::run_isolated({
        let info = info.clone();
        move || session_store::ui_store::delete_agent_session_strict(&tenant, &info.project_id, &info.id)
    })
    .await??;
    if !deleted {
        return Err(anyhow::anyhow!("session disappeared during archived deletion").into());
    }
    let _ = bus::publish(event::DELETED, json!({ "info": info }), instance_directory_opt());
    Ok(())
}

async fn remove_messages_and_parts_strict(session_id: &str) -> Result<(), Error> {
    for key in storage::list_strict(&["message", session_id]).await? {
        if let Some(message_id) = key.get(2) {
            for prefix in
                [vec!["part", message_id.as_str()], vec!["part", session_id, message_id.as_str()]]
            {
                for part_key in storage::list_strict(&prefix).await? {
                    let refs = part_key.iter().map(String::as_str).collect::<Vec<_>>();
                    storage::remove(&refs).await?;
                }
            }
            super::message::remove_message(session_id, message_id).await?;
        }
    }
    Ok(())
}

/// 删除会话关联的所有消息和部分
///
/// 级联删除会话下的所有消息以及每条消息的所有部分数据。
///
/// # 参数
///
/// - `session_id`: 会话 ID
///
/// # 返回
///
/// - `Ok(())`: 删除成功
/// - `Err(Error)`: 删除失败
async fn remove_messages_and_parts(session_id: &str) -> Result<(), Error> {
    // 获取会话下的所有消息键
    let mut msg_keys = storage::list(&["message", session_id]).await?;
    msg_keys.sort();
    // 遍历每条消息
    for key in msg_keys {
        // 键格式：["message", session_id, message_id]
        if key.len() < 3 {
            continue;
        }
        let message_id = key[2].clone();
        // 获取消息下的所有部分键
        let part_keys = storage::list(&["part", &message_id]).await?;
        // 删除每个部分
        for pkey in part_keys {
            // 键格式：["part", message_id, part_id]
            if pkey.len() < 3 {
                continue;
            }
            let part_id = pkey[2].clone();
            super::message::remove_part(session_id, &message_id, &part_id).await.ok();
        }
        // 删除消息
        super::message::remove_message(session_id, &message_id).await.ok();
    }
    Ok(())
}

/// 构建日志额外字段映射
///
/// 从键值对数组构建 JSON Map，用于日志记录。
///
/// # 参数
///
/// - `pairs`: 键值对数组
///
/// # 返回
///
/// 返回构建好的 JSON Map
fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

/// Host installation and plugin IO; metadata and lifecycle policy stay here.
pub trait LifecycleHost: Send + Sync + 'static {
    fn home() -> std::path::PathBuf;
    fn version() -> String;
    fn close_hooks(id: &str, directory: &std::path::Path) -> impl std::future::Future<Output = ()> + Send;
    fn close_mcp(id: &str, directory: &std::path::Path);
}
fn checkpoint_scope(directory: &str) -> String {
    crate::compaction_checkpoint::scope_for_directory(&session_store::ui_store::current_tenant_scope(), directory)
}
async fn project_fork<H: LifecycleHost>(info: &mut Info) -> anyhow::Result<()> {
    info.fork_execution_started = if info.source_slug().starts_with("turn-fork-") {
        crate::fork_execution::load_scoped(H::home(), session_store::ui_store::current_tenant_scope(), &info.project_id, &info.id).await?.map(|state| state.started)
    } else { None };
    Ok(())
}

#[cfg(test)]
#[path = "session_lifecycle_tests.rs"]
mod session_lifecycle_tests;
