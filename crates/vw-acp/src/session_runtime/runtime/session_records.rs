//! 会话记录创建与复用逻辑。
//!
//! 本模块负责在 ACP 客户端会话与本地持久化记录之间建立映射，
//! 包括创建新记录、复用目录匹配的记录、检查运行中队列所有者，
//! 以及把请求的模型和会话选项写回记录。

use std::path::{Path, PathBuf};

use crate::session_mode_preference::set_current_model_id;
use crate::session_persistence::{
    FindSessionByDirectoryWalkOptions, absolute_path, find_git_repository_root,
    find_session_by_directory_walk, iso_now, list_sessions, normalize_name, write_session_record,
};
use crate::session_runtime::prompt_runner::SessionSetModelOptions;
use crate::types::{AuthPolicy, SESSION_RECORD_SCHEMA, SessionEnsureResult, SessionRecord};
use crate::{AcpClient, set_session_model, sync_advertised_model_state as sync_model_state};

use super::prompt::{apply_requested_model, resolve_agent_config};
use super::state::{fallback_event_log, persist_session_options};
use super::{
    SessionCreateOptions, SessionCreateWithClientResult, SessionEnsureOptions, SessionRuntimeError,
};

#[cfg(test)]
#[path = "session_records_tests.rs"]
mod session_records_tests;

async fn create_session_record_with_client(
    client: &AcpClient,
    options: &SessionCreateOptions,
) -> Result<SessionRecord, SessionRuntimeError> {
    let cwd = absolute_path(&options.cwd).to_string_lossy().into_owned();
    let session_info = if let Some(session_id) = options.resume_session_id.as_deref() {
        crate::session_runtime_helpers::with_timeout(
            client.load_session(session_id.to_string(), PathBuf::from(&cwd)),
            options.timeout_ms,
        )
        .await
        .map_err(SessionRuntimeError::from_source)?
        .map_err(SessionRuntimeError::from_source)?
    } else {
        crate::perf_metrics::measure_perf("runtime.session_create.create_session", || async {
            crate::session_runtime_helpers::with_timeout(
                client.create_session(PathBuf::from(&cwd)),
                options.timeout_ms,
            )
            .await
        })
        .await
        .map_err(SessionRuntimeError::from_source)?
        .map_err(SessionRuntimeError::from_source)?
    };

    let requested_model_applied = apply_requested_model(
        client,
        &session_info.session_id,
        &cwd,
        options.session_options.as_ref().and_then(|value| value.model.as_deref()),
        options.timeout_ms,
    )
    .await
    .unwrap_or(false);

    let now = iso_now();
    let conversation = crate::session_conversation_model::create_session_conversation(Some(&now));
    let mut record = SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: session_info.session_id.clone(),
        acp_session_id: session_info.session_id.clone(),
        agent_session_id: None,
        agent_command: options.agent_command.clone(),
        agent_config: options.agent_config.clone(),
        cwd: cwd.clone(),
        name: normalize_name(options.name.as_deref()),
        created_at: now.clone(),
        last_used_at: now.clone(),
        last_seq: 0,
        last_request_id: None,
        event_log: fallback_event_log(&session_info.session_id),
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
        vwacp: Some(crate::SessionAcpxState {
            current_mode_id: None,
            desired_mode_id: None,
            current_model_id: None,
            available_models: None,
            available_commands: None,
            config_options: None,
            desired_config_options: None,
            session_options: None,
        }),
    };

    persist_session_options(&mut record, options.session_options.as_ref());
    sync_model_state(&mut record, None);
    if requested_model_applied {
        set_current_model_id(
            &mut record,
            options.session_options.as_ref().and_then(|value| value.model.as_deref()),
        );
    }

    write_session_record(&record).await.map_err(SessionRuntimeError::from_source)?;
    Ok(record)
}

/// 创建会话记录并保留已初始化的客户端。
///
/// 函数会根据选项创建或加载 ACP 会话，写入本地 [`SessionRecord`]，
/// 并把可继续使用的 [`AcpClient`] 一并返回给调用方。客户端创建、代理调用、
/// 持久化或超时失败时返回 [`SessionRuntimeError`]。
pub async fn create_session_with_client(
    options: SessionCreateOptions,
) -> Result<SessionCreateWithClientResult, SessionRuntimeError> {
    let client = AcpClient::new(
        options.agent_command.clone(),
        resolve_agent_config(&options.agent_command, options.agent_config.clone()),
    )
    .with_mcp_servers(options.mcp_servers.clone().unwrap_or_default())
    .with_permission_mode(options.permission_mode)
    .with_non_interactive_permissions(options.non_interactive_permissions)
    .with_auth_credentials(options.auth_credentials.clone().unwrap_or_default())
    .with_auth_policy(options.auth_policy.unwrap_or(AuthPolicy::Skip))
    .with_verbose(options.verbose)
    .with_session_options(options.session_options.clone());
    let record = create_session_record_with_client(&client, &options).await?;

    Ok(SessionCreateWithClientResult { record, client })
}

/// 创建会话记录并关闭临时客户端。
///
/// 适用于只需要持久化记录、不需要继续复用客户端的调用点。
/// 成功时返回写入后的 [`SessionRecord`]；失败时返回 [`SessionRuntimeError`]。
pub async fn create_session(
    options: SessionCreateOptions,
) -> Result<SessionRecord, SessionRuntimeError> {
    let SessionCreateWithClientResult { record, client } =
        create_session_with_client(options).await?;
    let _ = client.close().await;
    Ok(record)
}

async fn finalize_ensured_record(
    mut record: SessionRecord,
    options: &SessionEnsureOptions,
) -> Result<SessionEnsureResult, SessionRuntimeError> {
    if let Some(requested_model) =
        options.session_options.as_ref().and_then(|value| value.model.as_deref())
    {
        let result = set_session_model(SessionSetModelOptions {
            session_id: record.vwacp_record_id.clone(),
            model_id: requested_model.to_string(),
            mcp_servers: options.mcp_servers.clone(),
            non_interactive_permissions: options.non_interactive_permissions,
            auth_credentials: options.auth_credentials.clone(),
            auth_policy: options.auth_policy,
            timeout_ms: options.timeout_ms,
            verbose: options.verbose,
        })
        .await
        .map_err(SessionRuntimeError::from_source)?;
        record = result.record;
    }

    Ok(SessionEnsureResult { record, created: false })
}

fn is_within_walk_boundary(boundary: &Path, target: &Path) -> bool {
    target.strip_prefix(boundary).is_ok()
}

async fn find_live_session_by_directory_walk(
    cwd: &str,
    name: Option<&str>,
    boundary: Option<&str>,
) -> Result<Option<SessionRecord>, SessionRuntimeError> {
    let normalized_name = normalize_name(name);
    let normalized_start = absolute_path(cwd);
    let requested_boundary =
        boundary.map(absolute_path).unwrap_or_else(|| normalized_start.clone());
    let walk_boundary = if is_within_walk_boundary(&requested_boundary, &normalized_start) {
        requested_boundary
    } else {
        normalized_start.clone()
    };
    let sessions = list_sessions().await.map_err(SessionRuntimeError::from_source)?;
    let mut current = normalized_start.clone();
    let walk_root = current.ancestors().last().map(Path::to_path_buf).unwrap_or(current.clone());

    loop {
        let mut candidates = sessions
            .iter()
            .filter(|record| {
                Path::new(&record.cwd) == current
                    && record.name.as_deref() == normalized_name.as_deref()
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| right.last_used_at.cmp(&left.last_used_at));

        for candidate in candidates {
            let health =
                crate::queue_ipc_health::probe_queue_owner_health(&candidate.vwacp_record_id).await;
            if health.has_lease && health.pid_alive && health.socket_reachable {
                return Ok(Some(candidate));
            }
        }

        if current == walk_boundary || current == walk_root {
            return Ok(None);
        }

        let Some(parent) = current.parent().map(Path::to_path_buf) else {
            return Ok(None);
        };
        if parent == current {
            return Ok(None);
        }
        current = parent;
        if !is_within_walk_boundary(&walk_boundary, &current) {
            return Ok(None);
        }
    }
}

/// 确保当前目录上下文存在可用会话。
///
/// 函数优先通过持久化目录查找复用记录，再检查同目录向上查找范围内
/// 是否存在仍有健康队列所有者的会话；都不存在时创建新会话。
/// 返回值中的 `created` 标识是否实际创建了新记录。
pub async fn ensure_session(
    options: SessionEnsureOptions,
) -> Result<SessionEnsureResult, SessionRuntimeError> {
    let cwd = absolute_path(&options.cwd).to_string_lossy().into_owned();
    let git_root = find_git_repository_root(&cwd).map(|path| path.to_string_lossy().into_owned());
    let walk_boundary = options.walk_boundary.clone().or(git_root.clone()).unwrap_or(cwd.clone());
    let existing = find_session_by_directory_walk(&FindSessionByDirectoryWalkOptions {
        agent_command: options.agent_command.clone(),
        cwd: cwd.clone(),
        name: options.name.clone(),
        boundary: Some(walk_boundary),
    })
    .await
    .map_err(SessionRuntimeError::from_source)?;

    if let Some(existing) = existing {
        return finalize_ensured_record(existing, &options).await;
    }

    if let Some(existing_live) = find_live_session_by_directory_walk(
        &cwd,
        options.name.as_deref(),
        options.walk_boundary.as_deref().or(git_root.as_deref()),
    )
    .await?
    {
        return finalize_ensured_record(existing_live, &options).await;
    }

    let record = create_session(SessionCreateOptions {
        agent_command: options.agent_command,
        agent_config: options.agent_config,
        cwd: cwd.clone(),
        name: options.name,
        resume_session_id: options.resume_session_id,
        mcp_servers: options.mcp_servers,
        permission_mode: options.permission_mode,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials,
        auth_policy: options.auth_policy,
        verbose: options.verbose,
        session_options: options.session_options,
        timeout_ms: options.timeout_ms,
    })
    .await?;

    Ok(SessionEnsureResult { record, created: true })
}
