//! Session 模块 - 会话核心功能实现
//!
//! 本模块提供了 VibeWindow 代理系统中的会话（Session）管理功能。
//! 会话是代理与用户交互的基本单元，包含消息历史、元数据、权限控制等信息。
//!
//! # 主要功能
//!
//! - **会话生命周期管理**：创建、读取、更新、删除（CRUD）会话
//! - **消息管理**：支持消息的添加和查询
//! - **会话分支（Fork）**：支持从现有会话创建新分支
//! - **事件发布**：会话变更时通过事件总线发布通知
//! - **持久化存储**：会话数据通过存储层持久化
//!
//! # 核心类型
//!
//! - [`Session`]：内存中的会话对象，包含 ID 和消息列表
//! - [`Info`]：会话元数据，包含标题、时间戳、权限等信息
//! - [`Message`]：消息对象，包含角色和内容
//! - [`Role`]：消息角色（用户、助手、系统、工具）
//!
//! # 示例
//!
//! ```no_run
//! use app::agent::session::session::{create_next, CreateInput};
//!
//! async fn example() {
//!     let input = CreateInput {
//!         parent_id: None,
//!         title: Some("我的会话".to_string()),
//!         directory: "/path/to/project".to_string(),
//!         permission: None,
//!     };
//!     let session_info = create_next(input).await.unwrap();
//! }
//! ```

use crate::app::agent::bus;
use crate::app::agent::id;
use crate::app::agent::installation;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::project::instance;
use crate::app::agent::storage;
use crate::app::agent::util::log;
use std::sync::LazyLock;

use serde_json::{Map, Value, json};
use std::fmt;

/// 内存中的会话对象
///
/// 表示一个活跃的会话，包含会话 ID 和消息历史。
/// 主要用于运行时消息管理和上下文维护。
#[derive(Debug, Clone)]
pub struct Session {
    /// 会话唯一标识符（降序时间戳格式）
    pub id: String,
    /// 消息历史列表
    pub messages: Vec<Message>,
}

/// 消息对象
///
/// 表示会话中的一条消息，包含角色和内容。
#[derive(Debug, Clone, Hash)]
pub struct Message {
    /// 消息角色（用户、助手、系统、工具）
    pub role: Role,
    /// 消息文本内容
    pub content: String,
}

/// 消息角色枚举
///
/// 定义消息的可能角色类型，用于区分消息来源和用途。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// 用户消息
    User,
    /// 助手（AI）消息
    Assistant,
    /// 系统消息
    System,
    /// 开发者指令消息
    Developer,
    /// 工具调用消息
    Tool,
}

impl Session {
    /// 创建新的会话实例
    ///
    /// # 参数
    ///
    /// - `id`: 会话唯一标识符
    ///
    /// # 返回
    ///
    /// 返回一个空的会话实例，消息列表初始化为空
    ///
    /// # 示例
    ///
    /// ```
    /// use app::agent::session::session::{Session, Role};
    ///
    /// let session = Session::new("session-123".to_string());
    /// assert!(session.messages.is_empty());
    /// ```
    pub fn new(id: String) -> Self {
        Self { id, messages: Vec::new() }
    }

    /// 向会话添加消息
    ///
    /// # 参数
    ///
    /// - `role`: 消息角色
    /// - `content`: 消息内容
    ///
    /// # 示例
    ///
    /// ```
    /// use app::agent::session::session::{Session, Role};
    ///
    /// let mut session = Session::new("session-123".to_string());
    /// session.push(Role::User, "你好".to_string());
    /// assert_eq!(session.messages.len(), 1);
    /// ```
    pub fn push(&mut self, role: Role, content: String) {
        self.messages.push(Message { role, content });
    }
}

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
    use crate::app::agent::bus;

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

/// 会话操作错误类型
///
/// 定义了会话模块可能返回的各种错误。
#[derive(Debug)]
pub enum Error {
    /// ID 生成错误
    Id(id::Error),
    /// 存储操作错误
    Storage(storage::Error),
    /// JSON 序列化/反序列化错误
    Json(serde_json::Error),
    /// 无活动项目上下文
    NoProjectContext,
    /// 异步运行时与同步存储之间的隔离执行失败
    Runtime(anyhow::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Id(e) => write!(f, "{}", e),
            Error::Storage(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::NoProjectContext => write!(f, "no active project context"),
            Error::Runtime(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<id::Error> for Error {
    fn from(value: id::Error) -> Self {
        Error::Id(value)
    }
}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Error::Storage(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Runtime(value)
    }
}

pub use vw_shared::session::info::{Info, RevertInfo, ShareInfo, Summary, TimeInfo};
pub use vw_shared::session::session_utils::{create_slug, is_default_title};

/// 创建会话的输入参数
///
/// 用于 `create_next` 函数的输入参数结构。
#[derive(Debug, Clone, Default)]
pub struct CreateInput {
    /// 父会话 ID（用于创建子会话）
    pub parent_id: Option<String>,
    /// 会话标题（可选，未提供则使用默认标题）
    pub title: Option<String>,
    /// 工作目录路径
    pub directory: String,
    /// 权限规则集（可选）
    pub permission: Option<permission_next::Ruleset>,
}

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
fn forked_title(title: &str) -> String {
    // 匹配已有的分支编号格式
    static RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^(.+) \(fork #(\d+)\)$").unwrap());
    if let Some(caps) = RE.captures(title) {
        // 提取基础标题和当前编号
        let base = caps.get(1).map(|m| m.as_str()).unwrap_or(title);
        let num = caps.get(2).and_then(|m| m.as_str().parse::<u64>().ok()).unwrap_or(0);
        // 递增编号
        return format!("{} (fork #{})", base, num + 1);
    }
    // 首次分支
    format!("{} (fork #1)", title)
}

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
/// use app::agent::session::session::{create_next, CreateInput};
///
/// async fn example() {
///     let input = CreateInput {
///         parent_id: None,
///         title: Some("新会话".to_string()),
///         directory: "/path".to_string(),
///         permission: None,
///     };
///     let info = create_next(input).await.unwrap();
/// }
/// ```
pub async fn create_next(input: CreateInput) -> Result<Info, Error> {
    let now = now_ms();
    let project_id = project_id()?;
    let id = id::descending(id::Prefix::Session, None)?;

    let info = Info {
        id: id.clone(),
        slug: create_slug(),
        project_id: project_id.clone(),
        directory: input.directory.clone(),
        parent_id: input.parent_id.clone(),
        summary: None,
        share: None,
        title: input.title.unwrap_or_else(|| create_default_title(input.parent_id.is_some())),
        version: installation::version(),
        time: TimeInfo { created: now, updated: now, compacting: None, archived: None },
        permission: input.permission,
        revert: None,
    };

    // 记录创建日志
    LOGGER.info(
        "created",
        Some(extra([
            ("id", Value::String(info.id.clone())),
            ("projectID", Value::String(info.project_id.clone())),
            ("directory", Value::String(info.directory.clone())),
        ])),
    );

    // 持久化到存储层
    let tenant = crate::session::ui_store::current_tenant_scope();
    let save_info = info.clone();
    let save_project_id = project_id.clone();
    crate::global_database::run_isolated(move || {
        crate::session::ui_store::save_agent_session_scoped_for_tenant(
            &tenant,
            &save_info,
            Some(&save_project_id),
        )
    })
    .await?
    .ok_or_else(|| {
        Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("failed to persist session: {}", info.id),
        }))
    })?;

    // 发布创建事件
    let _ = bus::publish(event::CREATED, json!({ "info": info }), instance_directory_opt());
    Ok(get(&id).await?)
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
pub async fn get(session_id: &str) -> Result<Info, Error> {
    let project_id = project_id()?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    crate::global_database::run_isolated({
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::load_agent_session_scoped_for_tenant(
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
    })
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
pub async fn touch(session_id: &str) -> Result<Info, Error> {
    update(session_id, |draft| {
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
pub async fn update(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    let project_id = project_id()?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let mut result = crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::load_agent_session_scoped_for_tenant(
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
    f(&mut result);
    result.time.updated = now_ms();
    let save_result = result.clone();
    crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::save_agent_session_scoped_for_tenant(
                &tenant,
                &save_result,
                Some(&project_id),
            )
            .ok_or_else(|| {
                Error::Storage(storage::Error::NotFound(storage::NotFoundError {
                    message: format!("failed to persist session: {}", session_id),
                }))
            })
        }
    })
    .await??;
    // 记录更新日志
    LOGGER.info(
        "updated",
        Some(extra([
            ("id", Value::String(result.id.clone())),
            ("projectID", Value::String(result.project_id.clone())),
        ])),
    );
    // 发布更新事件
    let _ =
        bus::publish(event::UPDATED, json!({ "info": result.clone() }), instance_directory_opt());
    Ok(result)
}

/// 列出当前项目的所有会话
///
/// 获取当前项目上下文中的所有会话，按最后更新时间降序排列。
///
/// # 返回
///
/// - `Ok(Vec<Info>)`: 成功返回会话列表
/// - `Err(Error)`: 读取失败
pub async fn list() -> Result<Vec<Info>, Error> {
    let project_id = project_id()?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let mut out = crate::global_database::run_isolated(move || {
        crate::session::ui_store::load_agent_sessions_scoped_for_tenant(&tenant, Some(&project_id))
    })
    .await?;
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
    let all = list().await?;
    // 过滤出匹配父会话 ID 的子会话
    Ok(all.into_iter().filter(|s| s.parent_id.as_deref() == Some(parent_id)).collect())
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
pub async fn list_all() -> Result<Vec<Info>, Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let mut out = crate::global_database::run_isolated(move || {
        crate::session::ui_store::load_agent_sessions_all_for_tenant(&tenant)
    })
    .await?;
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
pub async fn get_any(session_id: &str) -> Result<Info, Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(info) = crate::global_database::run_isolated({
        let session_id = session_id.clone();
        move || crate::session::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
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
pub async fn update_any(session_id: &str, f: impl FnOnce(&mut Info)) -> Result<Info, Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(mut result) = crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let session_id = session_id.clone();
        move || crate::session::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
    let project_id = result.project_id.clone();
    f(&mut result);
    result.time.updated = now_ms();
    let save_result = result.clone();
    crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::save_agent_session_scoped_for_tenant(
                &tenant,
                &save_result,
                Some(&project_id),
            )
            .ok_or_else(|| {
                Error::Storage(storage::Error::NotFound(storage::NotFoundError {
                    message: format!("failed to persist session: {}", session_id),
                }))
            })
        }
    })
    .await??;
    // 记录更新日志
    LOGGER.info(
        "updated",
        Some(extra([
            ("id", Value::String(result.id.clone())),
            ("projectID", Value::String(result.project_id.clone())),
        ])),
    );
    // 发布更新事件
    let _ =
        bus::publish(event::UPDATED, json!({ "info": result.clone() }), instance_directory_opt());
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
pub async fn remove(session_id: &str) -> Result<(), Error> {
    let project_id = project_id()?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    // 先读取会话信息（用于事件发布）
    let info = get(session_id).await?;
    #[cfg(not(target_arch = "wasm32"))]
    crate::tools::shell::processes::cancel_session(
        &serde_json::to_string(&tenant).expect("tenant serialization"),
        std::path::Path::new(&info.directory),
        &info.id,
    )
    .map_err(Error::Runtime)?;
    // 删除会话主数据
    let session_id = session_id.to_string();
    crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::delete_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?;
    super::compaction_checkpoint::remove(&session_id, &info.directory)
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
pub async fn remove_any(session_id: &str) -> Result<(), Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_string();
    let Some(info) = crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let session_id = session_id.clone();
        move || crate::session::ui_store::load_agent_session_any_for_tenant(&tenant, &session_id)
    })
    .await?
    else {
        return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
            message: format!("session not found: {}", session_id),
        })));
    };
    let project_id = info.project_id.clone();
    #[cfg(not(target_arch = "wasm32"))]
    crate::tools::shell::processes::cancel_session(
        &serde_json::to_string(&tenant).expect("tenant serialization"),
        std::path::Path::new(&info.directory),
        &info.id,
    )
    .map_err(Error::Runtime)?;
    // 删除会话主数据
    crate::global_database::run_isolated({
        let tenant = tenant.clone();
        let project_id = project_id.clone();
        let session_id = session_id.clone();
        move || {
            crate::session::ui_store::delete_agent_session_scoped_for_tenant(
                &tenant,
                &session_id,
                Some(&project_id),
            )
        }
    })
    .await?;
    super::compaction_checkpoint::remove(&session_id, &info.directory)
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

/// 分叉（Fork）会话
///
/// 从现有会话创建一个新分支，复制原始会话的消息历史直到并包含指定消息（可选）。
/// 新会话的标题会自动添加或递增分支编号。
///
/// # 参数
///
/// - `session_id`: 原始会话 ID
/// - `message_id`: 可选的截止消息 ID（复制到此消息为止，并包含此消息）
///
/// # 返回
///
/// - `Ok(Info)`: 成功返回新创建的分支会话信息
/// - `Err(Error)`: 分叉失败
///
/// # 示例
///
/// ```no_run
/// use app::agent::session::session::fork;
///
/// async fn example() {
///     // 复制整个会话
///     let new_session = fork("original-session-id", None).await.unwrap();
///
///     // 只复制到特定消息
///     let partial_session = fork("original-session-id", Some("msg-123")).await.unwrap();
/// }
/// ```
pub async fn fork(session_id: &str, message_id: Option<&str>) -> Result<Info, Error> {
    fork_with_parent(session_id, message_id, None).await
}

pub(crate) async fn fork_with_parent(
    session_id: &str,
    message_id: Option<&str>,
    parent_id: Option<String>,
) -> Result<Info, Error> {
    // 读取原始会话信息
    let original = get(session_id).await?;
    // 生成分支标题
    let title = forked_title(&original.title);
    let directory = instance_directory_opt().unwrap_or_else(|| original.directory.clone());
    // 在创建目标前确认截点确实属于源会话。
    let mut msgs = super::message::messages(session_id, None).await?;
    if let Some(until) = message_id {
        if !msgs.iter().any(|msg| msg.info.id() == until) {
            return Err(Error::Storage(storage::Error::NotFound(storage::NotFoundError {
                message: format!("message {until} not found in source thread"),
            })));
        }
    }
    // 创建新会话
    let created = create_next(CreateInput {
        parent_id,
        title: Some(title),
        directory,
        permission: original.permission.clone(),
    })
    .await?;

    // 获取原始会话的所有消息，按时间正序复制，保证父消息映射可用。
    msgs.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    // 建立旧消息 ID 到新消息 ID 的映射
    let mut id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 复制消息和部分
    for msg in msgs {
        let reached_target = message_id.is_some_and(|until| msg.info.id() == until);

        // 生成新的消息 ID
        let new_id = id::ascending(id::Prefix::Message, None)?;
        id_map.insert(msg.info.id().to_string(), new_id.clone());

        // 对于助手消息，需要更新父消息 ID 的映射
        let parent_id = match &msg.info {
            super::message::Info::Assistant(a) => id_map.get(&a.parent_id).cloned(),
            _ => None,
        };

        // 克隆并更新消息信息
        let mut cloned_info = msg.info.clone();
        cloned_info.set_session_id(&created.id);
        cloned_info.set_id(&new_id);
        if let Some(parent_id) = parent_id {
            cloned_info.set_parent_id(&parent_id);
        }
        super::message::update_message(&cloned_info).await?;

        // 复制消息的所有部分
        for part in msg.parts {
            let mut new_part = part.clone();
            new_part.set_session_id(&created.id);
            new_part.set_message_id(&new_id);
            new_part.set_id(&id::ascending(id::Prefix::Part, None)?);
            super::message::update_part(&new_part).await?;
        }

        if reached_target {
            break;
        }
    }

    super::compaction_checkpoint::copy(session_id, &created.id)
        .await
        .map_err(|error| Error::Runtime(anyhow::anyhow!(error)))?;
    Ok(created)
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
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
