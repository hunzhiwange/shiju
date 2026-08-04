//! ACP 客户端缓存与会话记录管理。
//!
//! 本模块把“如何复用 ACP 客户端”和“如何找到或创建 ACP 会话记录”集中在一处，
//! 请求入口只需要按本地会话 id 获取可用记录。这里会写入 vwacp 会话记录，因此错误会
//! 向上包装为统一 API 错误。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use tokio::sync::mpsc;
use vw_acp::{
    AcpJsonRpcMessage, AcpSessionOptions, AuthPolicy, DEFAULT_EVENT_MAX_SEGMENTS,
    DEFAULT_EVENT_SEGMENT_MAX_BYTES, PermissionMode, SessionAcpxState, SessionEventLog,
    SessionRecord, SessionStateOptions, apply_lifecycle_snapshot_to_record,
    create_session_conversation, default_session_event_log, session_event_log, set_desired_mode_id,
    write_session_record,
};

use crate::app::agent::config;

use super::config::build_acp_command_line;
use super::{
    ACP_CLIENT_CACHE, CachedAcpClient, CachedAcpClientEntry, Error, ParsedAcpOptions, to_api_error,
};

/// 复制非空工具名列表。
///
/// 返回 `None` 表示没有有效工具名，调用方据此避免写入空白白名单。
fn clone_non_empty_tools(entries: &[String]) -> Option<Vec<String>> {
    let allowed_tools = entries
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<Vec<_>>();
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}

/// 检查取消信号是否已经触发。
///
/// 没有传入接收端时返回 `false`；调用方仍需在异步等待点继续监听变化。
pub(crate) fn should_abort(rx: Option<&tokio::sync::watch::Receiver<bool>>) -> bool {
    rx.is_some_and(|r| *r.borrow())
}

/// 将本地会话 id 转为 ACP 会话名。
///
/// 空白输入返回 `None`，非空输入会加上 `vw-session:` 前缀，确保和用户手动命名空间区分。
pub(crate) fn acp_session_name(session_id: &str) -> Option<String> {
    let trimmed = session_id.trim();
    (!trimmed.is_empty()).then(|| format!("vw-session:{trimmed}"))
}

/// 生成 ACP 客户端缓存键。
///
/// 缓存键包含命令、参数、环境、cwd 和会话权限选项；这些值任一变化都可能改变代理行为，
/// 因此必须使用独立客户端。
fn acp_client_cache_key(
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
    cwd: &Path,
    parsed_options: &ParsedAcpOptions,
) -> String {
    let mut env = acp_cfg.env.iter().collect::<Vec<_>>();
    env.sort_by(|left, right| left.0.cmp(right.0));

    json!({
        "agentName": acp_agent_name,
        "command": acp_cfg.command,
        "args": acp_cfg.args,
        "env": env.into_iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        "cwd": cwd,
        "permissionMode": parsed_options.permission_mode,
        "fullAccess": parsed_options.full_access,
        "nonInteractivePermissions": parsed_options.non_interactive_permissions,
        "authPolicy": parsed_options.auth_policy,
        "sessionOptions": parsed_options.session_options,
    })
    .to_string()
}

/// 标准化会话名。
fn normalize_session_name(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// 构造可缓存的 ACP 客户端。
///
/// 输出回调只把原始 JSON-RPC 消息转发到当前请求通道，真正的事件解释由 updates 模块处理。
fn build_cached_acp_client(
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
    parsed_options: &ParsedAcpOptions,
) -> Arc<CachedAcpClient> {
    let output_tx =
        Arc::new(parking_lot::Mutex::new(None::<mpsc::UnboundedSender<AcpJsonRpcMessage>>));
    let callback_output_tx = output_tx.clone();
    let callback = Arc::new(move |_direction, message| {
        if let Some(tx) = callback_output_tx.lock().as_ref() {
            let _ = tx.send(message);
        }
    });

    let client = vw_acp::AcpClient::new(
        acp_agent_name.to_string(),
        vw_acp::AcpAgentConfig {
            command: acp_cfg.command.clone(),
            args: acp_cfg.args.clone(),
            env: acp_cfg.env.clone(),
        },
    )
    .with_permission_mode(parsed_options.permission_mode.unwrap_or(PermissionMode::ApproveAll))
    .with_full_access(parsed_options.full_access)
    .with_non_interactive_permissions(parsed_options.non_interactive_permissions.clone())
    .with_auth_policy(parsed_options.auth_policy.unwrap_or(AuthPolicy::Skip))
    .with_session_options(parsed_options.session_options.clone())
    .with_acp_output_message_callback(Some(callback))
    .with_verbose(false);

    Arc::new(CachedAcpClient {
        client: Arc::new(client),
        prompt_lock: Arc::new(tokio::sync::Mutex::new(())),
        output_tx,
    })
}

/// 获取或创建与配置完全匹配的 ACP 客户端。
///
/// 返回值是共享 `Arc`；函数本身不执行网络请求，错误处理发生在后续会话和 prompt 调用中。
pub(super) fn get_cached_acp_client(
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
    cwd: &Path,
    parsed_options: &ParsedAcpOptions,
) -> CachedAcpClientEntry {
    let key = acp_client_cache_key(acp_agent_name, acp_cfg, cwd, parsed_options);
    let mut cache = ACP_CLIENT_CACHE.lock();
    if let Some(existing) = cache.get(&key) {
        return CachedAcpClientEntry { key, client: existing.clone() };
    }

    let client = build_cached_acp_client(acp_agent_name, acp_cfg, parsed_options);
    cache.insert(key.clone(), client.clone());
    CachedAcpClientEntry { key, client }
}

/// 关闭一次性 ACP 客户端缓存项。
///
/// 只有缓存中仍是同一个客户端且没有其他请求持有它时才会移除并关闭，避免误关正在
/// 复用同一连接的交互会话。
pub(super) async fn close_cached_acp_client_if_unshared(entry: &CachedAcpClientEntry) {
    let removed = {
        let mut cache = ACP_CLIENT_CACHE.lock();
        let should_remove = cache.get(&entry.key).is_some_and(|existing| {
            Arc::ptr_eq(existing, &entry.client) && Arc::strong_count(existing) <= 2
        });
        if should_remove { cache.remove(&entry.key) } else { None }
    };

    if let Some(client) = removed {
        let _ = client.client.close().await;
    }
}

/// 为会话记录构造事件日志配置。
///
/// 当默认路径推导失败时仍返回一个可写入记录的保守配置，保证记录创建路径不因日志目录
/// 推导失败而整体中断。
fn session_event_log_for_record(session_id: &str) -> SessionEventLog {
    default_session_event_log(session_id).unwrap_or_else(|| SessionEventLog {
        active_path: session_event_log(session_id, PathBuf::new()).active_path,
        segment_count: DEFAULT_EVENT_MAX_SEGMENTS,
        max_segment_bytes: DEFAULT_EVENT_SEGMENT_MAX_BYTES,
        max_segments: DEFAULT_EVENT_MAX_SEGMENTS,
        last_write_at: None,
        last_write_error: None,
    })
}

/// 从 ACP 会话选项中提取需要持久化的状态选项。
///
/// 没有模型、工具或最大轮次时返回 `None`，避免在会话记录里写入空状态对象。
fn session_state_options(
    session_options: Option<&AcpSessionOptions>,
) -> Option<SessionStateOptions> {
    let session_options = session_options?;
    if session_options.model.is_none()
        && session_options.allowed_tools.is_none()
        && session_options.max_turns.is_none()
    {
        return None;
    }

    Some(SessionStateOptions {
        model: session_options.model.clone(),
        allowed_tools: session_options
            .allowed_tools
            .as_ref()
            .and_then(|entries| clone_non_empty_tools(entries)),
        max_turns: session_options.max_turns,
    })
}

/// 在当前 ACP 客户端上创建新的会话记录。
///
/// 会调用 ACP 代理创建会话，并把本地记录写入 vwacp 存储。ACP 调用或记录写入失败时，
/// 返回统一的 `Error`。
async fn create_cached_session_record(
    cached_client: &CachedAcpClient,
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
    cwd: &Path,
    session_name: Option<String>,
    parsed_options: &ParsedAcpOptions,
) -> Result<SessionRecord, Error> {
    let session_info = cached_client.client.create_session(cwd).await.map_err(to_api_error)?;
    let session_id = session_info.session_id;
    let now = vw_acp::iso_now();
    let conversation = create_session_conversation(Some(&now));
    let cwd_text = cwd.to_string_lossy().into_owned();
    let mut record = SessionRecord {
        schema: vw_acp::SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: session_id.clone(),
        acp_session_id: session_id.clone(),
        agent_session_id: None,
        agent_command: build_acp_command_line(acp_cfg),
        agent_config: Some(vw_acp::AcpAgentConfig {
            command: acp_cfg.command.clone(),
            args: acp_cfg.args.clone(),
            env: acp_cfg.env.clone(),
        }),
        cwd: cwd_text,
        name: normalize_session_name(session_name.as_deref()),
        created_at: now.clone(),
        last_used_at: now.clone(),
        last_seq: 0,
        last_request_id: None,
        event_log: session_event_log_for_record(&session_id),
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: conversation.title,
        messages: conversation.messages,
        updated_at: conversation.updated_at,
        cumulative_token_usage: conversation.cumulative_token_usage,
        request_token_usage: conversation.request_token_usage,
        vwacp: None,
    };

    // desired_mode/session_options 先写入记录，后续请求入口会再向 ACP 代理应用实际值。
    // 这样即使代理生命周期快照稍后才更新，本地记录也能表达用户期望状态。
    set_desired_mode_id(&mut record, parsed_options.session_mode.as_deref());
    if let Some(session_options) = session_state_options(parsed_options.session_options.as_ref()) {
        let state = record.vwacp.get_or_insert(SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: None,
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: None,
        });
        state.session_options = Some(session_options);
    }

    apply_lifecycle_snapshot_to_record(
        &mut record,
        &cached_client.client.get_agent_lifecycle_snapshot(),
    );
    write_session_record(&record).await.map_err(to_api_error)?;

    tracing::debug!(
        target: "vw_agent",
        acp_agent = %acp_agent_name,
        acp_session_id = %record.acp_session_id,
        "created ACP session record on cached client"
    );

    Ok(record)
}

/// 查找当前目录边界内可复用的 ACP 会话记录，必要时创建新记录。
///
/// `force_new_session` 为真时会先关闭匹配的旧记录，再创建新会话。查找、关闭、创建或写入
/// 过程中出现的底层错误都会映射为统一 `Error`。
pub(crate) async fn find_or_create_cached_session_record(
    cached_client: &CachedAcpClient,
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
    cwd: &Path,
    walk_boundary: &str,
    session_name: Option<String>,
    parsed_options: &ParsedAcpOptions,
    force_new_session: bool,
) -> Result<SessionRecord, Error> {
    let agent_command = build_acp_command_line(acp_cfg);
    let cwd_text = cwd.to_string_lossy().into_owned();

    if force_new_session
        && let Some(existing) =
            vw_acp::find_session_by_directory_walk(&vw_acp::FindSessionByDirectoryWalkOptions {
                agent_command: agent_command.clone(),
                cwd: cwd_text.clone(),
                name: session_name.clone(),
                boundary: Some(walk_boundary.to_string()),
            })
            .await
            .map_err(to_api_error)?
    {
        vw_acp::close_session(&existing.vwacp_record_id).await.map_err(to_api_error)?;
    }

    if let Some(existing) =
        vw_acp::find_session_by_directory_walk(&vw_acp::FindSessionByDirectoryWalkOptions {
            agent_command,
            cwd: cwd_text,
            name: session_name.clone(),
            boundary: Some(walk_boundary.to_string()),
        })
        .await
        .map_err(to_api_error)?
    {
        return Ok(existing);
    }

    create_cached_session_record(
        cached_client,
        acp_agent_name,
        acp_cfg,
        cwd,
        session_name,
        parsed_options,
    )
    .await
}

/// 构造未找到 ACP 会话时的用户可读错误。
///
/// 返回值包含建议执行的 `vwacp sessions new` 命令；该函数不检查命令是否存在，也不执行 IO。
pub(crate) fn missing_session_error(
    walk_boundary: &str,
    acp_agent_name: &str,
    session_name: Option<&str>,
) -> Error {
    let create_cmd = if let Some(name) = session_name.filter(|value| !value.trim().is_empty()) {
        format!("vwacp {acp_agent_name} sessions new --name {name}")
    } else {
        format!("vwacp {acp_agent_name} sessions new")
    };
    Error::Api(crate::app::agent::session::message::AssistantError::Unknown {
        message: format!(
            "acp: ⚠ No vwacp session found (searched up to {walk_boundary}).\nCreate one: {create_cmd}"
        ),
    })
}
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
