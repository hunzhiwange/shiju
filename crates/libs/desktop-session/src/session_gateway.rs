//! 会话网关请求；端点由桌面壳显式传入。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::collections::HashSet;

use gateway_desktop_settings::DesktopPluginCatalogEntryDto;
use gateway_desktop_settings::DesktopSkillCatalogEntryDto;
use gateway_desktop_settings::DesktopSkillDetailDto;
use gateway_desktop_settings::ExternalAppsStateDto;
use gateway_desktop_settings::HookAuditEntryDto;
use gateway_desktop_settings::HookCatalogEntryDto;
use gateway_desktop_settings::HookMutationResponseDto;
use gateway_interaction::PendingPermissionReplyDto;
use gateway_interaction::PendingPermissionRequestDto;
use vw_api_project::ProjectChangeRecordDto;
use gateway_skill_market::SkillMarketCategoryDto;
use gateway_skill_market::SkillMarketCoordinateDto;
use gateway_skill_market::SkillMarketInstallSelectionDto;
use gateway_skill_market::SkillMarketSkillDetailDto;
use gateway_skill_market::SkillMarketSkillDto;
use gateway_skill_market::SkillMarketSkillFileDto;
use gateway_skill_market::SkillMarketSkillLocatorDto;
use gateway_skill_market::SkillMarketSkillPageDto;
use gateway_skill_market::SkillMarketSkillQuery;
use gateway_skill_market::SkillMarketSkillSetDetailDto;
use gateway_skill_market::SkillMarketSkillSetPageDto;
use gateway_skill_market::SkillMarketSourceDto;
use vw_api_agent_scheduler::AgentSchedulerScopeDto;
use vw_api_transport::id::ProjectId;
use vw_api_project::{ProjectScopeDto, TenantScopeDto};
use {gateway_shiju_account::ShijuAccountMeDto, gateway_shiju_account::ShijuAccountPasswordUpdateRequest, gateway_shiju_account::ShijuAccountProfileDto, gateway_shiju_account::ShijuAccountUpdateRequest, gateway_shiju_account::ShijuAccountUpdateResponseDto, gateway_shiju_account::ShijuCredential, gateway_shiju_account::ShijuLoginChallengeDto, gateway_shiju_account::ShijuLoginPollDto};
use gateway_shiju_skill::{ShijuInstalledSkillDto, ShijuMaintainedSkillQuery, ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillReadmeDto, ShijuSkillSearchQuery, ShijuSkillTagCatalogDto, ShijuSkillVersionDto, ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto};
use gateway_shiju_app::{ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto, ShijuIamApiKeyListDto};
use session_ui_types::{ChatSession, ChatSessionMeta, SessionMessageDelta};

pub use crate::message::chat::stream::save::SessionMessageDeltaSaveError;

#[cfg(not(target_arch = "wasm32"))]
fn block_on_gateway<F, T>(fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            rt.block_on(fut)
        }
    }
}

pub async fn gateway_task_plan_run_stop_with_client(
    client: &vw_gateway_client::GatewayClient,
    task_id: &str,
) -> Result<vw_api_task::TaskPlanRunStatusResponse, String> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err("plan-run task id 不能为空".to_string());
    }
    gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone()).task_plan_run_stop(task_id).await
}

pub async fn gateway_task_plan_run_stop_async(endpoint: gateway_endpoint::GatewayEndpoint,
    task_id: &str,
) -> Result<vw_api_task::TaskPlanRunStatusResponse, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_task_plan_run_stop_with_client(&client, task_id).await
}

pub fn active_project_scope(endpoint: gateway_endpoint::GatewayEndpoint, project_id: ProjectId) -> Result<ProjectScopeDto, String> {
    ProjectScopeDto::new(endpoint.tenant_scope().clone(), project_id).map_err(|error| error.to_string())
}

pub fn active_agent_scheduler_scope(endpoint: gateway_endpoint::GatewayEndpoint,
    project_id: ProjectId,
) -> Result<AgentSchedulerScopeDto, String> {
    Ok(AgentSchedulerScopeDto::new(active_project_scope(endpoint.clone(), project_id)?))
}

pub async fn gateway_load_session_async(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Result<Option<ChatSession>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_get(id, None).await
}

pub async fn gateway_bounded_session_open_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
) -> Result<
    (
        vw_api_session::GatewaySessionUiSummary,
        vw_api_session::GatewaySessionMessagePage<
            session_ui_types::SessionMessageRecord,
        >,
    ),
    String,
> {
    let page_query = vw_api_session::GatewaySessionMessagePageQuery {
        directory: Some(directory.to_owned()),
        cursor: None,
        limit: Some(64),
        direction: vw_api_session::SessionMessagePageDirection::Older,
    };
    let session_client = gateway_session::GatewaySessionClient::new(client.transport().clone());
    let (summary, page) = tokio::join!(
        session_client.session_ui_summary(session_id, Some(directory)),
        session_client.session_message_page(session_id, &page_query),
    );
    Ok((summary?, page?))
}

pub async fn gateway_session_message_page_before_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
    cursor: String,
) -> Result<
    vw_api_session::GatewaySessionMessagePage<
        session_ui_types::SessionMessageRecord,
    >,
    String,
> {
    gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_message_page(
            session_id,
            &vw_api_session::GatewaySessionMessagePageQuery {
                directory: Some(directory.to_owned()),
                cursor: Some(cursor),
                limit: Some(64),
                direction: vw_api_session::SessionMessagePageDirection::Older,
            },
        )
        .await
}

pub async fn gateway_load_session_any_async(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Result<Option<ChatSession>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_get_any(id).await
}

pub async fn gateway_load_session_any_with_raw_async(endpoint: gateway_endpoint::GatewayEndpoint,
    id: &str,
) -> Result<Option<gateway_session::SessionUiLoadResult>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_get_any_with_raw(id).await
}

pub async fn gateway_load_sessions_scoped_async(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: Option<&str>,
) -> Result<Vec<ChatSessionMeta>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_previews(scope).await
}

pub async fn gateway_load_archived_session_ids_async(endpoint: gateway_endpoint::GatewayEndpoint,
    scope: Option<&str>,
) -> Result<HashSet<String>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_archived_get(scope).await.map(|ids| ids.into_iter().collect())
}

pub async fn gateway_session_preview_meta_async(endpoint: gateway_endpoint::GatewayEndpoint,
    id: &str,
) -> Result<Option<ChatSessionMeta>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_preview_meta_get(id, None).await
}

pub async fn gateway_session_file_path_async(endpoint: gateway_endpoint::GatewayEndpoint,
    id: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_path_get(id, None).await.map(|path| path.map(std::path::PathBuf::from))
}

pub async fn gateway_session_step_llm_raw_async(endpoint: gateway_endpoint::GatewayEndpoint,
    id: &str,
    turn_id: &str,
    step_index: u32,
    directory: Option<&str>,
) -> Result<Option<serde_json::Value>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_step_llm_raw_get(id, turn_id, step_index, directory).await
}

pub async fn gateway_resolve_session_scope_id_async(endpoint: gateway_endpoint::GatewayEndpoint,
    path: Option<&str>,
    project_id: Option<&str>,
) -> Result<Option<String>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_scope_get(path, project_id).await
}

pub async fn gateway_permission_list_owned_async(endpoint: gateway_endpoint::GatewayEndpoint,
    directory: Option<String>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).permission_list(directory.as_deref()).await
}

pub async fn gateway_save_session_async(endpoint: gateway_endpoint::GatewayEndpoint,
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let session_to_save = session_ui_content_snapshot(session);
    let omitted_call_count = session.calls.len();
    let omitted_step_count = session.steps.len();
    if omitted_call_count > 0 || omitted_step_count > 0 {
        tracing::debug!(
            target: "vw_desktop",
            session_id = %session_to_save.id,
            omitted_call_count,
            omitted_step_count,
            "omitted runtime trajectory from session ui snapshot"
        );
    }
    tracing::debug!(
        target: "vw_desktop",
        session_id = %session_to_save.id,
        directory = ?directory,
        message_count = session_to_save.messages.len(),
        "desktop saving session ui content"
    );
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_save(&session_to_save.id, directory, &session_to_save).await?;
    Ok(Some(std::path::PathBuf::from(&session_to_save.id)))
}

pub async fn gateway_session_message_delta_async(endpoint: gateway_endpoint::GatewayEndpoint,
    session_id: &str,
    directory: Option<&str>,
    delta: &SessionMessageDelta,
) -> Result<(), SessionMessageDeltaSaveError> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone()).map_err(SessionMessageDeltaSaveError::Other)?;
    gateway_session_message_delta_with_client(&client, session_id, directory, delta).await
}

async fn gateway_session_message_delta_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: Option<&str>,
    delta: &SessionMessageDelta,
) -> Result<(), SessionMessageDeltaSaveError> {
    match gateway_session::GatewaySessionClient::new(client.transport().clone()).session_message_delta(session_id, directory, delta).await {
        Ok(_) => Ok(()),
        Err(error) if error.contains("session history conflict") => {
            Err(SessionMessageDeltaSaveError::Rejected(error))
        }
        Err(error)
            if error.contains("409 Conflict")
                || error.contains("session message delta would create a seq_no gap") =>
        {
            let summary = gateway_session::GatewaySessionClient::new(client.transport().clone()).session_ui_summary(session_id, directory).await.map_err(
                |refresh_error| {
                    SessionMessageDeltaSaveError::Other(format!(
                        "{error}; failed to refresh stale session version: {refresh_error}"
                    ))
                },
            )?;
            Err(SessionMessageDeltaSaveError::Stale {
                stored_updated_ms: summary.updated_ms,
                stored_message_count: summary.message_count,
            })
        }
        Err(error)
            if [
                "400 Bad Request",
                "401 Unauthorized",
                "403 Forbidden",
                "404 Not Found",
                "413 Payload Too Large",
                "422 Unprocessable Entity",
            ]
            .iter()
            .any(|status| error.contains(status)) =>
        {
            Err(SessionMessageDeltaSaveError::Rejected(error))
        }
        Err(error) => Err(SessionMessageDeltaSaveError::Other(error)),
    }
}

fn session_ui_content_snapshot(session: &ChatSession) -> ChatSession {
    // calls 与 steps 由会话运行时独立持久化；UI 快照只携带消息和元数据。
    let mut snapshot = session.clone();
    snapshot.calls.clear();
    snapshot.steps.clear();
    snapshot
}

pub async fn gateway_set_session_scope_async(endpoint: gateway_endpoint::GatewayEndpoint, scope: Option<&str>) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_scope_put(scope).await.map(|_| ())
}

pub async fn gateway_current_session_scope_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Option<String>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_scope_get(None, None).await
}

pub async fn gateway_question_list_async(endpoint: gateway_endpoint::GatewayEndpoint) -> Result<Vec<question_types::Request>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).question_list().await
}

pub async fn gateway_session_todo_list_async(endpoint: gateway_endpoint::GatewayEndpoint,
    id: &str,
    directory: Option<&str>,
) -> Result<Vec<todo_types::Todo>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_todo_get(id, directory).await
}

pub async fn gateway_question_reply_async(endpoint: gateway_endpoint::GatewayEndpoint,
    request_id: &str,
    answers: Vec<Vec<String>>,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).question_reply(request_id, answers).await.map(|_| ())
}

pub async fn gateway_question_reject_async(endpoint: gateway_endpoint::GatewayEndpoint, request_id: &str) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).question_reject(request_id).await.map(|_| ())
}

pub async fn gateway_permission_list_async(endpoint: gateway_endpoint::GatewayEndpoint,
    directory: Option<&str>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).permission_list(directory).await
}

pub async fn gateway_permission_reply_async(endpoint: gateway_endpoint::GatewayEndpoint,
    request_id: &str,
    reply: PendingPermissionReplyDto,
    directory: Option<&str>,
) -> Result<(), String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_interaction::GatewayInteractionClient::new(client.transport().clone()).permission_reply(request_id, reply, directory, None).await.map(|_| ())
}

pub async fn gateway_save_agent_session_scoped_async(endpoint: gateway_endpoint::GatewayEndpoint,
    info: &session_info::Info,
    scope: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_update::<()>(
            &info.id,
            scope,
            &vw_api_session::GatewaySessionPatchBody {
                title: Some(info.title.clone()),
                time: None,
            },
        )
        .await?;
    Ok(Some(std::path::PathBuf::from(&info.id)))
}

pub async fn gateway_project_change_records_async(endpoint: gateway_endpoint::GatewayEndpoint,
    directory: &str,
) -> Result<Vec<ProjectChangeRecordDto>, String> {
    let client = vw_gateway_client::GatewayClient::new(endpoint.clone())?;
    let project = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
            &endpoint.tenant_scope().clone(),
            &vw_api_project::ResolveProjectRequest {
                directory: directory.to_string(),
                create_if_missing: false,
            },
        )
        .await?
        .project;
    let scope = active_project_scope(endpoint.clone(), project.id)?;
    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_change_records(&scope, directory).await.map(|response| response.items)
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_save_session_result_from_future<F>(
    future: F,
) -> Result<Option<std::path::PathBuf>, String>
where
    F: std::future::Future<Output = Result<Option<std::path::PathBuf>, String>>,
{
    block_on_gateway(future)
}

pub fn is_permanent_session_save_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("413 payload too large")
}

#[cfg(test)]
#[path = "session_gateway_tests.rs"]
mod session_gateway_tests;

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_load_session(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Option<ChatSession> {
    block_on_gateway(gateway_load_session_async(endpoint.clone(), id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_load_session_any(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Option<ChatSession> {
    block_on_gateway(gateway_load_session_any_async(endpoint.clone(), id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_load_sessions_scoped(endpoint: gateway_endpoint::GatewayEndpoint, scope: Option<&str>) -> Vec<ChatSessionMeta> {
    block_on_gateway(gateway_load_sessions_scoped_async(endpoint.clone(), scope)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_load_archived_session_ids(endpoint: gateway_endpoint::GatewayEndpoint, scope: Option<&str>) -> HashSet<String> {
    block_on_gateway(gateway_load_archived_session_ids_async(endpoint.clone(), scope)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_session_preview_meta(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Option<ChatSessionMeta> {
    block_on_gateway(gateway_session_preview_meta_async(endpoint.clone(), id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_session_file_path(endpoint: gateway_endpoint::GatewayEndpoint, id: &str) -> Option<std::path::PathBuf> {
    block_on_gateway(gateway_session_file_path_async(endpoint.clone(), id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_resolve_session_scope_id(endpoint: gateway_endpoint::GatewayEndpoint,
    path: Option<&str>,
    project_id: Option<&str>,
) -> Option<String> {
    block_on_gateway(gateway_resolve_session_scope_id_async(endpoint.clone(), path, project_id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_save_session_result(endpoint: gateway_endpoint::GatewayEndpoint,
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    gateway_save_session_result_from_future(gateway_save_session_async(endpoint.clone(), session, directory))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_save_session(endpoint: gateway_endpoint::GatewayEndpoint,
    session: &ChatSession,
    directory: Option<&str>,
) -> Option<std::path::PathBuf> {
    gateway_save_session_result(endpoint, session, directory)
        .ok()
        .map(|_| std::path::PathBuf::from(&session.id))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_set_session_scope(endpoint: gateway_endpoint::GatewayEndpoint, scope: Option<&str>) {
    let _ = block_on_gateway(gateway_set_session_scope_async(endpoint.clone(), scope));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_current_session_scope(endpoint: gateway_endpoint::GatewayEndpoint) -> Option<String> {
    block_on_gateway(gateway_current_session_scope_async(endpoint.clone())).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_question_list(endpoint: gateway_endpoint::GatewayEndpoint) -> Vec<question_types::Request> {
    block_on_gateway(gateway_question_list_async(endpoint.clone())).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_question_reply(endpoint: gateway_endpoint::GatewayEndpoint, request_id: &str, answers: Vec<Vec<String>>) {
    let _ = block_on_gateway(gateway_question_reply_async(endpoint.clone(), request_id, answers));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_question_reject(endpoint: gateway_endpoint::GatewayEndpoint, request_id: &str) {
    let _ = block_on_gateway(gateway_question_reject_async(endpoint.clone(), request_id));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_permission_list(endpoint: gateway_endpoint::GatewayEndpoint, directory: Option<&str>) -> Vec<PendingPermissionRequestDto> {
    block_on_gateway(gateway_permission_list_async(endpoint.clone(), directory)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_permission_reply(endpoint: gateway_endpoint::GatewayEndpoint,
    request_id: &str,
    reply: PendingPermissionReplyDto,
    directory: Option<&str>,
) {
    let _ = block_on_gateway(gateway_permission_reply_async(endpoint.clone(), request_id, reply, directory));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_save_agent_session_scoped(endpoint: gateway_endpoint::GatewayEndpoint,
    info: &session_info::Info,
    scope: Option<&str>,
) -> Option<std::path::PathBuf> {
    let result = block_on_gateway(gateway_save_agent_session_scoped_async(endpoint.clone(), info, scope));
    if result.is_ok() { Some(std::path::PathBuf::from(&info.id)) } else { None }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn gateway_project_change_records(endpoint: gateway_endpoint::GatewayEndpoint, directory: &str) -> Vec<ProjectChangeRecordDto> {
    block_on_gateway(gateway_project_change_records_async(endpoint.clone(), directory)).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub fn gateway_project_change_records(_endpoint: gateway_endpoint::GatewayEndpoint, _directory: &str) -> Vec<ProjectChangeRecordDto> {
    Vec::new()
}

#[path = "session_gateway/account.rs"]
mod account;
pub use account::*;

#[path = "session_gateway/settings.rs"]
mod settings;
pub use settings::*;
