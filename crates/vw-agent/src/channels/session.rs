//! # 通道会话管理模块
//!
//! 本模块提供通道（Channel）与会话（Session）之间的关联管理功能。
//!
//! ## 主要职责
//!
//! - **会话映射管理**：维护发送者（sender）到会话（session）的映射关系
//! - **会话生命周期**：处理会话的创建、查找和清理
//! - **标题生成**：为新会话生成初始标题，并支持异步刷新优化后的标题
//! - **项目范围绑定**：将通道会话与项目目录和范围 ID 关联
//! - **会话处理器集成**：运行会话处理器并处理流式事件
//!
//! ## 核心数据结构
//!
//! - `channel_project_dir_override_store`：项目目录覆盖存储
//! - `sender_session_store`：发送者到会话 ID 的映射存储
//!
//! ## 使用场景
//!
//! 当用户通过某个通道（如 Telegram、Discord）发送消息时，本模块负责：
//! 1. 查找该用户在当前项目中的现有会话
//! 2. 如不存在则创建新会话
//! 3. 维护会话与发送者的映射关系
//! 4. 生成和更新会话标题

use super::*;

/// 获取通道项目目录覆盖值的静态存储
///
/// 返回一个全局唯一的 `Mutex<Option<PathBuf>>` 引用，用于存储项目目录的覆盖值。
/// 该存储在测试场景中特别有用，允许临时覆盖项目目录以便进行隔离测试。
///
/// # 返回值
///
/// 返回指向静态 `Mutex<Option<PathBuf>>` 的引用，其中：
/// - `Some(path)` 表示已设置覆盖目录
/// - `None` 表示使用默认目录
///
/// # 线程安全
///
/// 使用 `OnceLock` 确保线程安全的单次初始化。
///
/// # 示例
///
/// ```ignore
/// // 设置测试覆盖目录
/// channel_project_dir_override_store()
///     .lock()
///     .unwrap()
///     .replace(PathBuf::from("/test/project"));
/// ```
pub(crate) fn channel_project_dir_override_store() -> &'static Mutex<Option<PathBuf>> {
    static STORE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

/// 获取通道的项目目录路径
///
/// 解析当前应该使用的项目目录。如果存在覆盖值（通常用于测试），
/// 则使用覆盖值；否则使用运行时上下文中提供的工作空间目录。
///
/// # 参数
///
/// - `ctx`：通道运行时上下文，包含工作空间目录等配置信息
///
/// # 返回值
///
/// 返回解析后的项目目录路径 `PathBuf`
///
/// # 优先级
///
/// 1. 如果存在覆盖值，使用覆盖值
/// 2. 否则使用上下文中的 `workspace_dir`
pub(crate) fn channel_project_directory(ctx: &ChannelRuntimeContext) -> PathBuf {
    channel_project_dir_override_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| ctx.workspace_dir.as_ref().clone())
}

/// 异步解析通道的项目范围标识符
///
/// 项目范围 ID 用于标识消息所属的项目上下文，是会话隔离的关键。
/// 该函数尝试从项目实例获取 ID，如果失败则回退到使用目录路径作为标识符。
///
/// # 参数
///
/// - `ctx`：通道运行时上下文，包含项目目录信息
///
/// # 返回值
///
/// 返回项目范围 ID 字符串：
/// - 成功时返回项目实例的 ID
/// - 失败时返回目录路径的字符串表示
///
/// # 异步行为
///
/// 该函数是异步的，因为需要与项目实例管理器进行交互。
pub(crate) async fn resolve_channel_project_scope_id(ctx: &ChannelRuntimeContext) -> String {
    let directory = channel_project_directory(ctx);
    let provided = crate::app::agent::project::instance::provide(directory.clone(), None, || {
        Box::pin(async move { crate::app::agent::project::instance::project().map(|p| p.id) })
    })
    .await
    .ok()
    .flatten();

    provided.unwrap_or_else(|| directory.to_string_lossy().to_string())
}

/// 获取发送者会话映射的全局存储
///
/// 返回一个全局唯一的哈希表存储，用于维护"发送者标识"到"会话 ID"的映射关系。
/// 键的格式为 `{project_scope_id}::{sender_key}`，确保不同项目和发送者的会话隔离。
///
/// # 返回值
///
/// 返回指向静态 `Mutex<HashMap<String, String>>` 的引用
///
/// # 线程安全
///
/// 使用 `OnceLock` 确保线程安全的单次初始化
///
/// # 存储结构
///
/// - 键：格式为 `{project_scope_id}::{sender_key}`
/// - 值：对应的会话 ID
pub(crate) fn sender_session_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 清除指定项目和发送者的会话映射
///
/// 从全局存储中移除特定项目范围和发送者的会话 ID 映射。
/// 这通常在用户希望开始新会话时调用。
///
/// # 参数
///
/// - `project_scope_id`：项目范围标识符
/// - `sender_key`：发送者标识键（由 `sender_session_key` 函数生成）
///
/// # 副作用
///
/// 移除存储中键为 `{project_scope_id}::{sender_key}` 的条目
pub(crate) fn clear_sender_session_id_for_scope(project_scope_id: &str, sender_key: &str) {
    let key = format!("{}::{}", project_scope_id, sender_key);
    sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
}

/// 异步清除发送者的会话 ID 映射
///
/// 根据通道消息上下文，清除当前发送者在当前项目中的会话映射。
/// 这是一个便捷函数，内部调用 `clear_sender_session_id_for_scope`。
///
/// # 参数
///
/// - `ctx`：通道运行时上下文
/// - `msg`：通道消息，包含发送者信息
///
/// # 异步行为
///
/// 需要异步执行以解析项目范围 ID
pub(crate) async fn clear_sender_session_id(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) {
    let project_scope_id = resolve_channel_project_scope_id(ctx).await;
    clear_sender_session_id_for_scope(&project_scope_id, &sender_session_key(msg));
}

/// 为消息生成初始会话标题
///
/// 从消息内容提取并格式化一个简短的标题，用于新会话的初始标识。
/// 标题会进行规范化处理（去除多余空格）并限制长度。
///
/// # 参数
///
/// - `msg`：通道消息，包含内容和通道/发送者信息
///
/// # 返回值
///
/// 返回生成的标题字符串：
/// - 如果消息有内容：返回前 50 个字符（超出时添加 "..."）
/// - 如果消息内容为空：返回 `{channel} {sender}` 格式的标题
///
/// # 处理规则
///
/// 1. 规范化空白字符（将连续空格合并为单个空格）
/// 2. 去除首尾空白
/// 3. 限制最多 50 个字符
/// 4. 如果截断，在末尾添加 "..."
pub(crate) fn initial_session_title_for_message(msg: &traits::ChannelMessage) -> String {
    let normalized = msg.content.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return format!("{} {}", msg.channel, msg.sender);
    }
    let mut chars = trimmed.chars();
    let title: String = chars.by_ref().take(50).collect();
    if chars.next().is_some() { format!("{}...", title.trim_end()) } else { title }
}

/// 异步生成并刷新会话标题
///
/// 在后台任务中异步生成优化的会话标题，并更新到会话记录中。
/// 这允许系统使用 AI 模型根据消息内容生成更有意义的标题，
/// 同时不阻塞消息处理流程。
///
/// # 参数
///
/// - `session_id`：需要更新标题的会话 ID
/// - `first_user_content`：用户的初始消息内容，用于生成标题
/// - `preferred_model`：可选的首选模型标识符，用于标题生成
///
/// # 行为说明
///
/// 该函数立即返回，实际工作在异步任务中执行：
/// 1. 调用 `title::generate_from_content` 生成新标题
/// 2. 如果生成成功，更新会话的 title 字段
/// 3. 忽略任何错误（后台任务失败不应影响主流程）
pub(crate) fn spawn_channel_session_title_refresh(
    session_id: String,
    first_user_content: String,
    preferred_model: Option<String>,
) {
    tokio::spawn(async move {
        let generated = crate::app::agent::session::title::generate_from_content(
            session_id.clone(),
            first_user_content,
            preferred_model,
            None,
        )
        .await;
        let Ok(title) = generated else { return };
        let _ =
            crate::app::agent::session::session::update_any(&session_id, |s| s.title = title).await;
    });
}

/// 异步解析或创建发送者的会话 ID
///
/// 查找指定发送者在当前项目中的现有会话，如果不存在则创建新会话。
/// 这是通道消息处理的核心函数，负责会话的连续性管理。
///
/// # 参数
///
/// - `ctx`：通道运行时上下文，包含项目目录和工作空间信息
/// - `msg`：通道消息，包含发送者标识
///
/// # 返回值
///
/// 返回会话 ID 字符串
///
/// # 行为流程
///
/// 1. 解析项目目录和项目范围 ID
/// 2. 构建复合键 `{project_scope_id}::{sender_key}`
/// 3. 查找现有会话：
///    - 如果找到且项目 ID 匹配，直接返回现有会话 ID
///    - 否则继续创建新会话
/// 4. 创建新会话：
///    - 使用消息内容生成初始标题
///    - 在项目上下文中创建会话记录
/// 5. 更新发送者会话映射
/// 6. 返回会话 ID
///
/// # 错误处理
///
/// 如果会话创建失败，回退到使用复合键作为会话 ID
pub(crate) async fn resolve_or_create_sender_session_id(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> String {
    let project_dir = channel_project_directory(ctx);
    let project_scope_id = resolve_channel_project_scope_id(ctx).await;
    let key = format!("{}::{}", project_scope_id, sender_session_key(msg));

    // 尝试查找现有会话
    let existing =
        sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned();
    if let Some(existing) = existing {
        // 验证现有会话是否属于当前项目
        if let Ok(info) = crate::app::agent::session::session::get_any(&existing).await {
            if info.project_id == project_scope_id {
                return existing;
            }
        }
    }

    // 创建新会话
    let directory = project_dir.to_string_lossy().to_string();
    let title = initial_session_title_for_message(msg);
    let create = crate::app::agent::project::instance::provide(project_dir, None, move || {
        let directory = directory.clone();
        let title = title.clone();
        Box::pin(async move {
            crate::app::agent::session::session::create_next(
                crate::app::agent::session::session::CreateInput {
                    parent_id: None,
                    title: Some(title),
                    directory,
                    permission: None,
                },
            )
            .await
        })
    })
    .await;

    // 提取会话 ID，创建失败时使用复合键作为回退
    let session_id = match create {
        Ok(Ok(info)) => info.id,
        Err(_) => key.clone(),
        Ok(Err(_)) => key.clone(),
    };

    // 更新映射存储
    sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, session_id.clone());
    session_id
}

/// 异步为通道运行会话处理器
///
/// 启动会话处理器并处理其产生的流式事件，将增量内容收集并可选地转发到外部通道。
/// 这是将底层会话处理器与通道集成的主要接口。
///
/// # 参数
///
/// - `req`：会话处理器请求，包含执行所需的所有信息
/// - `delta_tx`：可选的增量内容发送器，用于实时转发输出内容
///
/// # 返回值
///
/// - `Ok(String)`：返回完整的输出内容
/// - `Err(anyhow::Error)`：处理器出错或异常退出
///
/// # 事件处理
///
/// 处理以下流式事件：
/// - `Delta`：增量内容，追加到输出并通过 `delta_tx` 转发（如果提供）
/// - `Done`：处理完成，返回累积的输出
/// - `Error`：处理错误，立即返回错误
/// - `StepStart` / `StepFinish`：步骤事件，当前被忽略
///
/// # 并发模型
///
/// 使用 `spawn_blocking` 在独立线程中运行同步的会话处理器，
/// 通过无界通道异步接收事件
pub(crate) async fn run_session_processor_for_channel(
    req: crate::app::agent::session::processor::Request,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
) -> anyhow::Result<String> {
    let mut req = req;
    if req.persist_app_session_artifacts && req.app_session_scope.is_none() {
        req.app_session_scope =
            crate::app::agent::session::processor::app_session_scope_from_root_async(
                req.root.as_deref(),
            )
            .await;
    }

    // 创建无界通道用于事件传递
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<
        crate::app::agent::session::processor::StreamEvent,
    >();

    // 在阻塞任务中运行处理器
    tokio::task::spawn_blocking(move || {
        crate::app::agent::session::processor::run(req, move |ev| event_tx.send(ev).is_ok());
    });

    // 处理流式事件
    let mut output = String::new();
    while let Some(ev) = event_rx.recv().await {
        match ev {
            crate::app::agent::session::processor::StreamEvent::LlmRawArtifact(_) => {}
            crate::app::agent::session::processor::StreamEvent::Delta(delta) => {
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            crate::app::agent::session::processor::StreamEvent::Tool(event) => {
                let delta = format!("tool {}\n{}\n", event.name, event.payload);
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            crate::app::agent::session::processor::StreamEvent::Done(_) => return Ok(output),
            crate::app::agent::session::processor::StreamEvent::Error(err) => {
                anyhow::bail!(err)
            }
            crate::app::agent::session::processor::StreamEvent::StepStart { .. }
            | crate::app::agent::session::processor::StreamEvent::StepFinish { .. }
            | crate::app::agent::session::processor::StreamEvent::PostToolRound { .. }
            | crate::app::agent::session::processor::StreamEvent::ContextLoaded { .. } => {}
        }
    }

    // 如果没有收到终止事件，视为异常
    anyhow::bail!("session processor exited without terminal event")
}

/// 将通道聊天消息转换为会话历史格式
///
/// 将通道模块的 `ChatMessage` 转换为模型层的 `ChatMessage` 格式，
/// 用于传递给会话处理器或 AI 模型。
///
/// # 参数
///
/// - `turns`：通道聊天消息切片，包含对话轮次
///
/// # 返回值
///
/// 返回转换后的模型层消息向量 `Vec<crate::session::ui_types::ChatMessage>`
///
/// # 角色映射
///
/// - `"user"` -> `ChatRole::User`
/// - `"assistant"` -> `ChatRole::Assistant`
/// - `"system"` -> `ChatRole::System`
/// - `"tool"` -> `ChatRole::Tool`
/// - 其他 -> 跳过（过滤掉）
///
/// # 注意
///
/// - `think_timing` 字段被初始化为空向量
/// - 无法识别的角色会被过滤掉
pub(crate) fn to_session_history(
    turns: &[ChatMessage],
) -> Vec<crate::session::ui_types::ChatMessage> {
    turns
        .iter()
        .filter_map(|turn| {
            let role = match turn.role.as_str() {
                "user" => crate::session::ui_types::ChatRole::User,
                "assistant" => crate::session::ui_types::ChatRole::Assistant,
                "system" => crate::session::ui_types::ChatRole::System,
                "tool" => crate::session::ui_types::ChatRole::Tool,
                _ => return None,
            };
            Some(crate::session::ui_types::ChatMessage {
                role,
                content: turn.content.clone(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
