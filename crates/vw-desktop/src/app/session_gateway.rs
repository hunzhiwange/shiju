//! 封装会话与网关运行时之间的连接和请求逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::collections::HashSet;

use crate::app::config;
use vw_gateway_client::DesktopPluginCatalogEntryDto;
use vw_gateway_client::DesktopSkillCatalogEntryDto;
use vw_gateway_client::DesktopSkillDetailDto;
use vw_gateway_client::ExternalAppsStateDto;
use vw_gateway_client::HookAuditEntryDto;
use vw_gateway_client::HookCatalogEntryDto;
use vw_gateway_client::HookMutationResponseDto;
use vw_gateway_client::PendingPermissionReplyDto;
use vw_gateway_client::PendingPermissionRequestDto;
use vw_gateway_client::ProjectChangeRecordDto;
use vw_gateway_client::SkillMarketCategoryDto;
use vw_gateway_client::SkillMarketCoordinateDto;
use vw_gateway_client::SkillMarketInstallSelectionDto;
use vw_gateway_client::SkillMarketSkillDetailDto;
use vw_gateway_client::SkillMarketSkillDto;
use vw_gateway_client::SkillMarketSkillFileDto;
use vw_gateway_client::SkillMarketSkillLocatorDto;
use vw_gateway_client::SkillMarketSkillPageDto;
use vw_gateway_client::SkillMarketSkillQuery;
use vw_gateway_client::SkillMarketSkillSetDetailDto;
use vw_gateway_client::SkillMarketSkillSetPageDto;
use vw_gateway_client::SkillMarketSourceDto;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::ProjectId;
use vw_gateway_client::vw_api_types::project::{ProjectScopeDto, TenantScopeDto};
use vw_gateway_client::{
    ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest, ShijuAccountProfileDto,
    ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto, ShijuAppApiKeyBindingDto,
    ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto,
    ShijuCredential, ShijuIamApiKeyListDto, ShijuInstalledSkillDto, ShijuLoginChallengeDto,
    ShijuLoginPollDto, ShijuMaintainedSkillQuery, ShijuSkillCursorPageDto, ShijuSkillDetailDto,
    ShijuSkillPageDto, ShijuSkillReadmeDto, ShijuSkillSearchQuery, ShijuSkillTagCatalogDto,
    ShijuSkillVersionDto, ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta, SessionMessageDelta};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMessageDeltaSaveError {
    Stale { stored_updated_ms: u64, stored_message_count: usize },
    Rejected(String),
    Other(String),
}

impl std::fmt::Display for SessionMessageDeltaSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale { stored_updated_ms, .. } => {
                write!(formatter, "stale session message delta at {stored_updated_ms}")
            }
            Self::Rejected(error) | Self::Other(error) => formatter.write_str(error),
        }
    }
}

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

fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    config::gateway_client()
}

pub(crate) fn active_tenant_scope() -> TenantScopeDto {
    config::gateway_client_endpoint().tenant_scope().clone()
}

pub(crate) async fn gateway_task_plan_run_stop_with_client(
    client: &vw_gateway_client::GatewayClient,
    task_id: &str,
) -> Result<vw_gateway_client::vw_api_types::task::TaskPlanRunStatusResponse, String> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err("plan-run task id 不能为空".to_string());
    }
    client.task_plan_run_stop(task_id).await
}

pub(crate) async fn gateway_task_plan_run_stop_async(
    task_id: &str,
) -> Result<vw_gateway_client::vw_api_types::task::TaskPlanRunStatusResponse, String> {
    let client = gateway_client()?;
    gateway_task_plan_run_stop_with_client(&client, task_id).await
}

pub(crate) async fn gateway_shiju_account_login_async() -> Result<ShijuLoginChallengeDto, String> {
    let client = gateway_client()?;
    client.shiju_account_login().await
}

pub(crate) async fn gateway_shiju_account_login_poll_async(
    state: &str,
) -> Result<ShijuLoginPollDto, String> {
    let client = gateway_client()?;
    client.shiju_account_login_poll(state).await
}

pub(crate) async fn gateway_shiju_account_profile_async(
    credential: &ShijuCredential,
) -> Result<ShijuAccountProfileDto, String> {
    let client = gateway_client()?;
    client.shiju_account_profile(credential).await
}

pub(crate) async fn gateway_shiju_account_monthly_quota_async(
    credential: &ShijuCredential,
) -> Result<vw_gateway_client::ShijuMonthlyQuotaDto, String> {
    gateway_client()?.shiju_account_monthly_quota(credential).await
}

pub(crate) async fn gateway_shiju_account_me_async(
    credential: &ShijuCredential,
) -> Result<ShijuAccountMeDto, String> {
    let client = gateway_client()?;
    client.shiju_account_me(credential).await
}

pub(crate) async fn gateway_shiju_account_update_async(
    credential: &ShijuCredential,
    request: &ShijuAccountUpdateRequest,
) -> Result<ShijuAccountUpdateResponseDto, String> {
    let client = gateway_client()?;
    client.shiju_account_update(credential, request).await
}

pub(crate) async fn gateway_shiju_account_password_update_async(
    credential: &ShijuCredential,
    request: &ShijuAccountPasswordUpdateRequest,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.shiju_account_password_update(credential, request).await
}

pub(crate) async fn gateway_shiju_app_catalog_async(
    credential: &ShijuCredential,
) -> Result<ShijuAppCatalogDto, String> {
    let client = gateway_client()?;
    client.shiju_app_catalog(credential).await
}

pub(crate) async fn gateway_shiju_apps_async(
    credential: &ShijuCredential,
) -> Result<ShijuAppListDto, String> {
    let client = gateway_client()?;
    client.shiju_apps(credential).await
}

pub(crate) async fn gateway_shiju_app_api_key_binding_async(
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppApiKeyBindingDto, String> {
    let client = gateway_client()?;
    client.shiju_app_api_key_binding(credential, installation_id).await
}

pub(crate) async fn gateway_shiju_app_api_keys_async(
    credential: &ShijuCredential,
) -> Result<ShijuIamApiKeyListDto, String> {
    let client = gateway_client()?;
    client.shiju_app_api_keys(credential).await
}

pub(crate) async fn gateway_shiju_app_save_api_key_binding_async(
    credential: &ShijuCredential,
    installation_id: &str,
    request: &ShijuAppApiKeyBindingSaveRequest,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.shiju_app_save_api_key_binding(credential, installation_id, request).await
}

pub(crate) async fn gateway_shiju_app_prepare_launch_async(
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppLaunchDto, String> {
    let client = gateway_client()?;
    client.shiju_app_prepare_launch(credential, installation_id).await
}

pub(crate) async fn gateway_shiju_skill_search_async(
    credential: &ShijuCredential,
    query: &ShijuSkillSearchQuery,
) -> Result<ShijuSkillCursorPageDto, String> {
    let client = gateway_client()?;
    client.shiju_skill_search(credential, query).await
}

pub(crate) async fn gateway_shiju_skill_installed_async(
    credential: &ShijuCredential,
) -> Result<Vec<ShijuInstalledSkillDto>, String> {
    let client = gateway_client()?;
    client.shiju_skill_installed(credential).await
}

pub(crate) async fn gateway_shiju_system_skills_async(
    credential: &ShijuCredential,
    query: &ShijuSystemSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
    let client = gateway_client()?;
    client.shiju_system_skills(credential, query).await
}

pub(crate) async fn gateway_shiju_maintained_skills_async(
    credential: &ShijuCredential,
    query: &ShijuMaintainedSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    let client = gateway_client()?;
    client.shiju_maintained_skills(credential, query).await
}

pub(crate) async fn gateway_shiju_skill_tags_async(
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = gateway_client()?;
    client.shiju_skill_tags(credential).await
}

pub(crate) async fn gateway_shiju_tenant_skill_tags_async(
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    let client = gateway_client()?;
    client.shiju_tenant_skill_tags(credential).await
}

pub(crate) async fn gateway_shiju_skill_detail_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillDetailDto, String> {
    let client = gateway_client()?;
    client.shiju_skill_detail(credential, skill_id).await
}

#[allow(dead_code)]
pub(crate) async fn gateway_shiju_skill_readme_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillReadmeDto, String> {
    let client = gateway_client()?;
    client.shiju_skill_readme(credential, skill_id).await
}

pub(crate) async fn gateway_shiju_skill_versions_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<Vec<ShijuSkillVersionDto>, String> {
    let client = gateway_client()?;
    client.shiju_skill_versions(credential, skill_id).await
}

pub(crate) async fn gateway_shiju_skill_remote_install_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.shiju_skill_remote_install(credential, skill_id).await
}

#[allow(dead_code)]
pub(crate) async fn gateway_shiju_skill_remote_install_version_async(
    credential: &ShijuCredential,
    skill_id: u64,
    version_id: u64,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.shiju_skill_remote_install_version(credential, skill_id, version_id).await
}

pub(crate) async fn gateway_shiju_skill_remote_uninstall_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.shiju_skill_remote_uninstall(credential, skill_id).await
}

pub(crate) fn active_project_scope(project_id: ProjectId) -> Result<ProjectScopeDto, String> {
    ProjectScopeDto::new(active_tenant_scope(), project_id).map_err(|error| error.to_string())
}

pub(crate) fn active_agent_scheduler_scope(
    project_id: ProjectId,
) -> Result<AgentSchedulerScopeDto, String> {
    Ok(AgentSchedulerScopeDto::new(active_project_scope(project_id)?))
}

/// 执行 gateway_load_session_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_session_async(id: &str) -> Result<Option<ChatSession>, String> {
    let client = gateway_client()?;
    client.session_ui_get(id, None).await
}

pub(crate) async fn gateway_bounded_session_open_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
) -> Result<
    (
        vw_gateway_client::GatewaySessionUiSummary,
        vw_gateway_client::GatewaySessionMessagePage<
            vw_shared::session::ui_types::SessionMessageRecord,
        >,
    ),
    String,
> {
    let page_query = vw_gateway_client::GatewaySessionMessagePageQuery {
        directory: Some(directory.to_owned()),
        cursor: None,
        limit: Some(64),
        direction: vw_gateway_client::SessionMessagePageDirection::Older,
    };
    let (summary, page) = tokio::join!(
        client.session_ui_summary(session_id, Some(directory)),
        client.session_message_page(session_id, &page_query),
    );
    Ok((summary?, page?))
}

pub(crate) async fn gateway_session_message_page_before_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
    cursor: String,
) -> Result<
    vw_gateway_client::GatewaySessionMessagePage<
        vw_shared::session::ui_types::SessionMessageRecord,
    >,
    String,
> {
    client
        .session_message_page(
            session_id,
            &vw_gateway_client::GatewaySessionMessagePageQuery {
                directory: Some(directory.to_owned()),
                cursor: Some(cursor),
                limit: Some(64),
                direction: vw_gateway_client::SessionMessagePageDirection::Older,
            },
        )
        .await
}

/// 执行 gateway_load_session_any_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_session_any_async(id: &str) -> Result<Option<ChatSession>, String> {
    let client = gateway_client()?;
    client.session_ui_get_any(id).await
}

/// 无目录限制地读取会话 UI 数据，并保留每步原始 LLM 载荷。
pub async fn gateway_load_session_any_with_raw_async(
    id: &str,
) -> Result<Option<vw_gateway_client::SessionUiLoadResult>, String> {
    let client = gateway_client()?;
    client.session_ui_get_any_with_raw(id).await
}

/// 执行 gateway_load_sessions_scoped_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_sessions_scoped_async(
    scope: Option<&str>,
) -> Result<Vec<ChatSessionMeta>, String> {
    let client = gateway_client()?;
    client.session_ui_previews(scope).await
}

/// 执行 gateway_load_archived_session_ids_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_archived_session_ids_async(
    scope: Option<&str>,
) -> Result<HashSet<String>, String> {
    let client = gateway_client()?;
    client.session_archived_get(scope).await.map(|ids| ids.into_iter().collect())
}

/// 执行 gateway_session_preview_meta_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_preview_meta_async(
    id: &str,
) -> Result<Option<ChatSessionMeta>, String> {
    let client = gateway_client()?;
    client.session_preview_meta_get(id, None).await
}

/// 执行 gateway_session_file_path_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_file_path_async(
    id: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = gateway_client()?;
    client.session_path_get(id, None).await.map(|path| path.map(std::path::PathBuf::from))
}

/// 执行 gateway_session_step_llm_raw_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_step_llm_raw_async(
    id: &str,
    turn_id: &str,
    step_index: u32,
    directory: Option<&str>,
) -> Result<Option<serde_json::Value>, String> {
    let client = gateway_client()?;
    client.session_step_llm_raw_get(id, turn_id, step_index, directory).await
}

/// 执行 gateway_resolve_session_scope_id_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_resolve_session_scope_id_async(
    path: Option<&str>,
    project_id: Option<&str>,
) -> Result<Option<String>, String> {
    let client = gateway_client()?;
    client.session_scope_get(path, project_id).await
}

/// 执行 gateway_permission_list_owned_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_permission_list_owned_async(
    directory: Option<String>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    let client = gateway_client()?;
    client.permission_list(directory.as_deref()).await
}

/// 执行 gateway_save_session_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_save_session_async(
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = gateway_client()?;
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
    client.session_ui_save(&session_to_save.id, directory, &session_to_save).await?;
    Ok(Some(std::path::PathBuf::from(&session_to_save.id)))
}

/// 原子提交有界的会话消息增量。
pub async fn gateway_session_message_delta_async(
    session_id: &str,
    directory: Option<&str>,
    delta: &SessionMessageDelta,
) -> Result<(), SessionMessageDeltaSaveError> {
    let client = gateway_client().map_err(SessionMessageDeltaSaveError::Other)?;
    gateway_session_message_delta_with_client(&client, session_id, directory, delta).await
}

async fn gateway_session_message_delta_with_client(
    client: &vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: Option<&str>,
    delta: &SessionMessageDelta,
) -> Result<(), SessionMessageDeltaSaveError> {
    match client.session_message_delta(session_id, directory, delta).await {
        Ok(_) => Ok(()),
        Err(error) if error.contains("session history conflict") => {
            Err(SessionMessageDeltaSaveError::Rejected(error))
        }
        Err(error)
            if error.contains("409 Conflict")
                || error.contains("session message delta would create a seq_no gap") =>
        {
            let summary = client.session_ui_summary(session_id, directory).await.map_err(
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

/// 执行 gateway_set_session_scope_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_set_session_scope_async(scope: Option<&str>) -> Result<(), String> {
    let client = gateway_client()?;
    client.session_scope_put(scope).await.map(|_| ())
}

/// 执行 gateway_current_session_scope_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_current_session_scope_async() -> Result<Option<String>, String> {
    let client = gateway_client()?;
    client.session_scope_get(None, None).await
}

/// 执行 gateway_question_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_list_async() -> Result<Vec<vw_shared::question::Request>, String> {
    let client = gateway_client()?;
    client.question_list().await
}

/// 执行 gateway_session_todo_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_todo_list_async(
    id: &str,
    directory: Option<&str>,
) -> Result<Vec<vw_shared::todo::Todo>, String> {
    let client = gateway_client()?;
    client.session_todo_get(id, directory).await
}

/// 执行 gateway_question_reply_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_reply_async(
    request_id: &str,
    answers: Vec<Vec<String>>,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.question_reply(request_id, answers).await.map(|_| ())
}

/// 执行 gateway_question_reject_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_reject_async(request_id: &str) -> Result<(), String> {
    let client = gateway_client()?;
    client.question_reject(request_id).await.map(|_| ())
}

/// 执行 gateway_permission_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_permission_list_async(
    directory: Option<&str>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    let client = gateway_client()?;
    client.permission_list(directory).await
}

/// 执行 gateway_permission_reply_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_permission_reply_async(
    request_id: &str,
    reply: PendingPermissionReplyDto,
    directory: Option<&str>,
) -> Result<(), String> {
    let client = gateway_client()?;
    client.permission_reply(request_id, reply, directory, None).await.map(|_| ())
}

/// 执行 gateway_save_agent_session_scoped_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_save_agent_session_scoped_async(
    info: &vw_shared::session::info::Info,
    scope: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    let client = gateway_client()?;
    client
        .session_update::<()>(
            &info.id,
            scope,
            &vw_gateway_client::GatewaySessionPatchBody {
                title: Some(info.title.clone()),
                time: None,
            },
        )
        .await?;
    Ok(Some(std::path::PathBuf::from(&info.id)))
}

/// 执行 gateway_external_apps_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_apps_async() -> Result<ExternalAppsStateDto, String> {
    let client = gateway_client()?;
    client.desktop_external_apps_get().await
}

/// 执行 gateway_external_open_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_open_async(path: &str, target: &str) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_external_app_open(path, target).await
}

/// 执行 gateway_external_reveal_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_reveal_async(path: &str) -> Result<(), String> {
    let client = gateway_client()?;
    client.desktop_external_path_reveal(path).await
}

/// 执行 gateway_skills_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skills_async(
    project_path: Option<&str>,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    let client = gateway_client()?;
    client.skills_get(project_path).await
}

/// 执行 gateway_skill_detail_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_detail_async(
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<DesktopSkillDetailDto, String> {
    let client = gateway_client()?;
    client.skill_detail_get(project_path, skill_id).await
}

/// 执行 gateway_skill_create_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_create_async(project_path: &str) -> Result<String, String> {
    let client = gateway_client()?;
    client.skill_create(project_path).await
}

/// 执行 gateway_skill_install_builtin_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_install_builtin_async(
    project_path: &str,
    skill_id: &str,
) -> Result<String, String> {
    let client = gateway_client()?;
    client.skill_install_builtin(project_path, skill_id).await
}

/// 执行 gateway_skill_set_enabled_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_set_enabled_async(
    project_path: Option<&str>,
    skill_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = gateway_client()?;
    client.skill_set_enabled(project_path, skill_id, enabled).await
}

/// 执行 gateway_skill_delete_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_delete_async(
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<String, String> {
    let client = gateway_client()?;
    client.skill_delete(project_path, skill_id).await
}

/// Lists the skill-market sources registered by the active gateway.
pub async fn gateway_skill_market_sources_async() -> Result<Vec<SkillMarketSourceDto>, String> {
    let client = gateway_client()?;
    client.skill_market_sources().await
}

/// Lists recommended skills from one registered market source.
pub async fn gateway_skill_market_recommended_async(
    source: &str,
) -> Result<Vec<SkillMarketSkillDto>, String> {
    let client = gateway_client()?;
    client.skill_market_recommended(source).await
}

/// Queries skills from one registered market source.
pub async fn gateway_skill_market_skills_async(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillPageDto, String> {
    let client = gateway_client()?;
    client.skill_market_skills(source, query).await
}

/// Lists categories from one registered market source.
pub async fn gateway_skill_market_categories_async(
    source: &str,
) -> Result<Vec<SkillMarketCategoryDto>, String> {
    let client = gateway_client()?;
    client.skill_market_categories(source).await
}

/// Reads one source-aware market skill detail.
pub async fn gateway_skill_market_skill_detail_async(
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillDetailDto, String> {
    let client = gateway_client()?;
    client.skill_market_skill_detail(locator).await
}

/// Reads the canonical Markdown file for one source-aware market skill.
pub async fn gateway_skill_market_skill_file_async(
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillFileDto, String> {
    let client = gateway_client()?;
    client.skill_market_skill_file(locator).await
}

/// Installs one canonical market skill into the active project.
pub async fn gateway_skill_market_install_async(
    project_path: &str,
    coordinate: &SkillMarketCoordinateDto,
) -> Result<String, String> {
    let client = gateway_client()?;
    client.skill_market_install(project_path, coordinate).await
}

/// Installs one canonical market skill into explicit Agent targets.
pub async fn gateway_skill_market_install_to_targets_async(
    project_path: Option<&str>,
    coordinate: &SkillMarketCoordinateDto,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<String>, String> {
    let client = gateway_client()?;
    client.skill_market_install_to_targets(project_path, coordinate, installations).await
}

/// Queries skill sets from one registered market source.
pub async fn gateway_skill_market_skill_sets_async(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillSetPageDto, String> {
    let client = gateway_client()?;
    client.skill_market_skill_sets(source, query).await
}

/// Reads one skill-set detail from a registered market source.
pub async fn gateway_skill_market_skill_set_detail_async(
    source: &str,
    slug: &str,
) -> Result<SkillMarketSkillSetDetailDto, String> {
    let client = gateway_client()?;
    client.skill_market_skill_set_detail(source, slug).await
}

/// Installs one market skill set into the active project atomically.
pub async fn gateway_skill_market_skill_set_install_async(
    project_path: &str,
    source: &str,
    slug: &str,
) -> Result<Vec<String>, String> {
    let client = gateway_client()?;
    client.skill_market_skill_set_install(project_path, source, slug).await
}

/// 执行 gateway_plugins_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugins_async() -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
    let client = gateway_client()?;
    client.plugins_status_get().await
}

/// 读取当前已发现插件声明的 hook 清单。
pub async fn gateway_hooks_async() -> Result<Vec<HookCatalogEntryDto>, String> {
    let client = gateway_client()?;
    client.hooks_get().await
}

/// 使用服务端重新解析后的稳定哈希信任当前 hook 声明。
pub async fn gateway_hook_trust_async(
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = gateway_client()?;
    client.hook_trust(plugin_id, hook_hash).await
}

/// 使用服务端重新解析后的稳定哈希撤销当前 hook 声明。
pub async fn gateway_hook_revoke_async(
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    let client = gateway_client()?;
    client.hook_revoke(plugin_id, hook_hash).await
}

/// 读取只包含时间、工具名、耗时和成功状态的有界审计记录。
pub async fn gateway_hook_audit_async() -> Result<Vec<HookAuditEntryDto>, String> {
    let client = gateway_client()?;
    client.hook_audit_get().await
}

/// 执行 gateway_plugin_set_enabled_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugin_set_enabled_async(
    plugin_id: &str,
    enabled: bool,
) -> Result<String, String> {
    let client = gateway_client()?;
    client.plugin_set_enabled(plugin_id, enabled).await
}

/// 执行 gateway_plugin_delete_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugin_delete_async(plugin_id: &str) -> Result<String, String> {
    let client = gateway_client()?;
    client.plugin_delete(plugin_id).await
}

/// 执行 gateway_project_change_records_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_project_change_records_async(
    directory: &str,
) -> Result<Vec<ProjectChangeRecordDto>, String> {
    let client = gateway_client()?;
    let project = client
        .project_resolve(
            &active_tenant_scope(),
            &vw_gateway_client::vw_api_types::project::ResolveProjectRequest {
                directory: directory.to_string(),
                create_if_missing: false,
            },
        )
        .await?
        .project;
    let scope = active_project_scope(project.id)?;
    client.project_change_records(&scope, directory).await.map(|response| response.items)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_session 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_session(id: &str) -> Option<ChatSession> {
    block_on_gateway(gateway_load_session_async(id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_session_any 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_session_any(id: &str) -> Option<ChatSession> {
    block_on_gateway(gateway_load_session_any_async(id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_sessions_scoped 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_sessions_scoped(scope: Option<&str>) -> Vec<ChatSessionMeta> {
    block_on_gateway(gateway_load_sessions_scoped_async(scope)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_archived_session_ids 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_archived_session_ids(scope: Option<&str>) -> HashSet<String> {
    block_on_gateway(gateway_load_archived_session_ids_async(scope)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_session_preview_meta 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_session_preview_meta(id: &str) -> Option<ChatSessionMeta> {
    block_on_gateway(gateway_session_preview_meta_async(id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_session_file_path 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_session_file_path(id: &str) -> Option<std::path::PathBuf> {
    block_on_gateway(gateway_session_file_path_async(id)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_resolve_session_scope_id 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_resolve_session_scope_id(
    path: Option<&str>,
    project_id: Option<&str>,
) -> Option<String> {
    block_on_gateway(gateway_resolve_session_scope_id_async(path, project_id)).ok().flatten()
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

#[cfg(not(target_arch = "wasm32"))]
/// 同步保存会话，并保留网关返回的原始错误。
pub fn gateway_save_session_result(
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    gateway_save_session_result_from_future(gateway_save_session_async(session, directory))
}

/// 判断会话保存错误是否无法通过后续快照重试恢复。
pub fn is_permanent_session_save_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("413 payload too large")
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_save_session 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_save_session(
    session: &ChatSession,
    directory: Option<&str>,
) -> Option<std::path::PathBuf> {
    gateway_save_session_result(session, directory)
        .ok()
        .map(|_| std::path::PathBuf::from(&session.id))
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_set_session_scope 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_set_session_scope(scope: Option<&str>) {
    let _ = block_on_gateway(gateway_set_session_scope_async(scope));
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_current_session_scope 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_current_session_scope() -> Option<String> {
    block_on_gateway(gateway_current_session_scope_async()).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_list 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_list() -> Vec<vw_shared::question::Request> {
    block_on_gateway(gateway_question_list_async()).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_reply 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_reply(request_id: &str, answers: Vec<Vec<String>>) {
    let _ = block_on_gateway(gateway_question_reply_async(request_id, answers));
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_reject 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_reject(request_id: &str) {
    let _ = block_on_gateway(gateway_question_reject_async(request_id));
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_permission_list 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_permission_list(directory: Option<&str>) -> Vec<PendingPermissionRequestDto> {
    block_on_gateway(gateway_permission_list_async(directory)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_permission_reply 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_permission_reply(
    request_id: &str,
    reply: PendingPermissionReplyDto,
    directory: Option<&str>,
) {
    let _ = block_on_gateway(gateway_permission_reply_async(request_id, reply, directory));
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_save_agent_session_scoped 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_save_agent_session_scoped(
    info: &vw_shared::session::info::Info,
    scope: Option<&str>,
) -> Option<std::path::PathBuf> {
    let result = block_on_gateway(gateway_save_agent_session_scoped_async(info, scope));
    if result.is_ok() { Some(std::path::PathBuf::from(&info.id)) } else { None }
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_project_change_records 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_project_change_records(directory: &str) -> Vec<ProjectChangeRecordDto> {
    block_on_gateway(gateway_project_change_records_async(directory)).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
/// 执行 gateway_project_change_records 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_project_change_records(_directory: &str) -> Vec<ProjectChangeRecordDto> {
    Vec::new()
}

#[cfg(test)]
#[path = "session_gateway_tests.rs"]
mod session_gateway_tests;

pub async fn gateway_plugin_prepare_async(
    source: String,
    update_id: Option<String>,
) -> Result<vw_gateway_client::DesktopPluginPreviewDto, String> {
    let client = gateway_client()?;
    match update_id {
        Some(id) => {
            client
                .plugin_prepare_update(&id, (!source.trim().is_empty()).then_some(source.trim()))
                .await
        }
        None => client.plugin_prepare_install(source.trim()).await,
    }
}
pub async fn gateway_plugin_activate_async(token: String, update: bool) -> Result<String, String> {
    let client = gateway_client()?;
    if update {
        client.plugin_update(&token, true).await
    } else {
        client.plugin_install(&token, true).await
    }
}
pub async fn gateway_plugin_discard_async(token: String) -> Result<(), String> {
    gateway_client()?.plugin_cancel(&token).await
}

/// Resolve the selected project without creating it; repository authorization remains server-side.
pub async fn gateway_skill_repos_scope(directory: &str, host: String) -> Result<vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery, String> {
    let project = gateway_client()?.project_resolve(&active_tenant_scope(),
        &vw_gateway_client::vw_api_types::project::ResolveProjectRequest {
            directory: directory.to_owned(), create_if_missing: false,
        }).await?.project;
    Ok(vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery {
        project_id: project.id, target_host_id: host,
    })
}

pub async fn gateway_skill_repos_request(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    version: u64,
    change: Option<(vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillRepoEntry, bool)>,
) -> Result<vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposResponse, String> {
    use vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::*;
    let client = gateway_client()?;
    match change {
        None => client.skill_repos_list(&scope).await,
        Some((repo, true)) => client.skill_repo_remove(&repo.owner, &repo.name,
            &RemoveSkillRepoRequest { scope, expected_version: version }).await,
        Some((repo, false)) => client.skill_repo_save(&SaveSkillRepoRequest {
            scope, expected_version: version, repo,
        }).await,
    }
}

/// Only fixed Claude actions cross this boundary; credentials never enter messages.
pub(crate) async fn gateway_claude_config_request(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::ClaudeConfigQuery,
    token: String,
    change: Option<(bool, vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::ClaudeConfigAction)>,
) -> Result<(vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::ClaudeConfigResponse, Option<String>), String> {
    use vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::*;
    let client = gateway_client()?;
    let notice = if let Some((onboarding, action)) = change {
        let request = ClaudeConfigRequest { scope: scope.clone(), action };
        let response = if onboarding { client.claude_onboarding(&request, &token).await? }
            else { client.claude_config_marker(&request, &token).await? };
        if response.scope.project_id != scope.project_id || response.scope.target_host_id != scope.target_host_id {
            return Err("Claude response scope mismatch".into());
        }
        Some(if response.changed { "操作已保存" } else { "配置已符合所选操作" }.to_owned())
    } else { None };
    Ok((client.claude_config_status(&scope, &token).await?, notice))
}

/// Read-only skills requests; credentials stay outside UI messages.
pub(crate) async fn gateway_skills_market_request(
    project: String, host: String, query: String, token: String,
) -> Result<crate::app::message::settings::skills::market_view::Results, String> {
    use vw_gateway_client::vw_api_types::agent_apps_skills::{repos_api::SkillReposQuery,
        discovery_api::DiscoverSkillsRequest, search_api::MarketSearchQuery};
    use crate::app::message::settings::skills::market_view::Results;
    let client = gateway_client()?;
    if query.is_empty() {
        client.agent_apps_skills_discover(&DiscoverSkillsRequest {
            scope: SkillReposQuery { project_id: project.into(), target_host_id: host },
            repo_id: None, force_refresh: false,
        }, &token).await.map(Results::Discovery).map_err(|e| e.to_string())
    } else {
        client.agent_apps_market_search(&MarketSearchQuery {
            project_id: project.into(), target_host_id: host, query, limit: 100, offset: 0,
        }, &token).await.map(Results::Search).map_err(|e| e.to_string())
    }
}

/// Installation is submitted once; observe the same durable task until terminal.
pub(crate) async fn gateway_skills_install_view(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    request: Option<vw_gateway_client::vw_api_types::agent_apps_skills::install_api::InstallSkillRequest>,
    token: String,
) -> Result<Option<vw_gateway_client::vw_api_types::agent_apps_skills::inventory_api::SkillInventoryResponse>, String> {
    use vw_gateway_client::vw_api_types::agent_apps_skills::jobs_api::SkillJobPhase;
    let client = gateway_client()?;
    let Some(request) = request else {
        return client.agent_apps_skills_inventory(&scope, &token).await.map(Some).map_err(|e| e.to_string());
    };
    let response = client.agent_apps_skills_install(&request, &token).await.map_err(|e| e.to_string())?;
    if response.scope != scope { return Err("安装响应目标不匹配".into()); }
    loop {
        let job = client.agent_apps_skills_job(&scope, &response.task_id, &token).await.map_err(|e| e.to_string())?;
        if job.scope != scope || job.task_id != response.task_id { return Err("安装任务目标不匹配".into()); }
        match job.phase {
            SkillJobPhase::Completed if !job.requires_reconciliation => return Ok(None),
            SkillJobPhase::Completed | SkillJobPhase::Failed | SkillJobPhase::Interrupted =>
                return Err("安装未完成或需要核对目标目录，请刷新后检查".into()),
            _ => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
        }
    }
}

/// Re-scan between writes: each selection is bound to one server revision.
pub(crate) async fn gateway_skills_import_request(
    project: String, host: String,
    apps: Vec<vw_gateway_client::vw_api_types::agent_apps::AppId>,
    selected: Vec<vw_gateway_client::vw_api_types::agent_apps_skills::import_api::UnmanagedSkillItem>,
    token: String,
) -> (Vec<(String, Result<(), String>)>, Result<vw_gateway_client::vw_api_types::agent_apps_skills::import_api::UnmanagedSkillsResponse, String>) {
    use vw_gateway_client::vw_api_types::agent_apps_skills::{import_api::*, repos_api::SkillReposQuery};
    let mut outcomes = Vec::new();
    let result = async {
        let client = gateway_client()?;
        let app_names = apps.iter().map(|id| {
            serde_json::to_value(id).map_err(|_| "应用标识无效".to_owned())?
                .as_str().map(str::to_owned).ok_or_else(|| "应用标识无效".to_owned())
        }).collect::<Result<Vec<_>, String>>()?;
        let query = UnmanagedSkillsQuery { project_id: project.into(), target_host_id: host.clone(),
            apps: app_names.join(",") };
        let mut report = client.agent_apps_skills_unmanaged(&query, &token).await.map_err(|e| e.to_string())?;
        for row in selected {
            if report.scope.project_id != query.project_id || report.scope.target_host_id != host {
                return Err("扫描响应作用域不匹配".to_owned());
            }
            let result = async {
                let current = report.skills.iter().find(|s| s.directory == row.directory
                    && s.content_hash == row.content_hash && !s.managed && !s.conflict && !s.external_link)
                    .ok_or_else(|| "技能来源已变化，请重新扫描".to_owned())?;
                let selection_id = current.selection_id.clone().ok_or_else(|| "技能不可导入".to_owned())?;
                let targets: Vec<_> = apps.iter().copied().filter(|id| current.apps.contains(id)).collect();
                if targets.is_empty() { return Err("未选择来源应用".to_owned()); }
                let imported = client.agent_apps_skills_import(&ImportSkillRequest {
                    scope: SkillReposQuery { project_id: query.project_id.clone(), target_host_id: host.clone() },
                    expected_version: report.version, selection_id, apps: targets,
                }, &token).await.map_err(|e| e.to_string())?;
                if imported.scope.project_id != query.project_id || imported.scope.target_host_id != host
                    || imported.directory != row.directory {
                    return Err("导入响应目标不匹配，请重新扫描确认实际状态".to_owned());
                }
                Ok(())
            }.await;
            outcomes.push((row.directory, result));
            report = client.agent_apps_skills_unmanaged(&query, &token).await.map_err(|e| e.to_string())?;
        }
        Ok(report)
    }.await;
    (outcomes, result)
}

pub(crate) async fn gateway_skills_job(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    id: vw_gateway_client::vw_api_types::id::TaskId,
    token: String,
    cancel: bool,
) -> Result<vw_gateway_client::vw_api_types::agent_apps_skills::jobs_api::SkillJobResponse, String> {
    let client = gateway_client()?;
    if cancel { client.agent_apps_skills_cancel_job(&scope, &id, &token).await }
    else { client.agent_apps_skills_job(&scope, &id, &token).await }.map_err(|e| e.to_string())
}

pub(crate) async fn gateway_skills_submit(
    request: vw_gateway_client::vw_api_types::agent_apps_skills::install_api::InstallSkillRequest,
    token: String,
) -> Result<vw_gateway_client::vw_api_types::agent_apps_skills::install_api::InstallSkillResponse, String> {
    gateway_client()?.agent_apps_skills_install(&request, &token).await.map_err(|e| e.to_string())
}

pub(crate) async fn gateway_skill_bindings_request(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    token: String,
    change: Option<crate::app::message::settings::skills::bindings_view::Change>,
) -> Result<crate::app::message::settings::skills::bindings_view::Readback, String> {
    use crate::app::message::settings::skills::bindings_view::{Change, Readback, complete_success};
    let client = crate::app::config::gateway_client()?;
    let changed = change.is_some();
    let mut resync_targets = None;
    let mut results = Vec::new();
    let mut error = None;
    match change {
        Some(Change::Save(request)) => {
            if client.agent_apps_skill_binding_targets(&scope, Some(&request), &token).await.is_err() {
                error = Some("保存目标偏好失败，请刷新后重试".into());
            }
        }
        Some(Change::Bind(id, request)) => {
            match client.agent_apps_skills_bindings(&id, &request, &token).await {
                Ok(response) if response.scope == scope => {
                    if !complete_success(&response.results, &request.targets, &id, request.enabled) {
                        error = Some("绑定未全部成功，请检查逐项结果".into());
                    }
                    results = response.results;
                }
                _ => error = Some("绑定失败，请重读确认实际状态".into()),
            }
        }
        Some(Change::Resync(request)) => {
            resync_targets = Some(request.targets.clone());
            match client.agent_apps_skills_resync(&request, &token).await {
                Ok(response) if response.scope == scope => {
                    // Resync can legitimately have no pairs when no skills are installed.
                    if response.results.iter().any(|r| r.status != vw_gateway_client::vw_api_types::agent_apps_skills::bindings_api::SkillBindingStatus::Enabled) {
                        error = Some("重新同步未全部成功，请检查逐项结果".into());
                    }
                    results = response.results;
                }
                _ => error = Some("重新同步失败，请刷新确认".into()),
            }
        }
        None => {}
    }
    let snapshot = match client.agent_apps_skill_binding_targets(&scope, None, &token).await {
        Ok(snapshot) => Some(snapshot),
        Err(cause) => {
            error.get_or_insert_with(|| format!("读取绑定目标失败：{cause}；请刷新确认实际状态"));
            None
        }
    };
    let inventory = match client.agent_apps_skills_inventory(&scope, &token).await {
        Ok(inventory) => Some(inventory),
        Err(cause) => {
            error.get_or_insert_with(|| format!("读取技能库存失败：{cause}；请刷新确认实际状态"));
            None
        }
    };
    if let (Some(targets), Some(inventory)) = (resync_targets, inventory.as_ref()) {
        let skills: Vec<_> = inventory.skills.iter().filter(|s| s.present).collect();
        if results.len() != skills.len() * targets.len() || skills.iter().any(|skill|
            !complete_success(&results.iter().filter(|r| r.directory == skill.id).cloned().collect::<Vec<_>>(), &targets, &skill.id, true)) {
            error = Some("重新同步未全部成功，请检查逐项结果".into());
        }
    }
    if snapshot.is_none() || inventory.is_none() {
        error.get_or_insert_with(|| "重读失败，已保留逐项写入结果，请刷新确认实际状态".into());
    }
    Ok(Readback { snapshot, inventory, results, error, changed })
}

/// Mutation and polling use the same authorized gateway instance and scope.
pub(crate) async fn gateway_skills_mutation(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    operation: (crate::app::message::settings::skills::updates_view::Operation, u64, Option<String>),
    token: String,
) -> Result<crate::app::message::settings::skills::updates_view::Outcome, String> {
    use crate::app::message::settings::skills::updates_view::{Operation, Outcome};
    use vw_gateway_client::vw_api_types::agent_apps_skills::{updates_api::*, uninstall_api::*};
    let client = gateway_client()?;
    let (operation, expected_version, hash) = operation;
    match operation {
        Operation::Uninstall(id) => {
            let response = client.agent_apps_skills_uninstall(&id, &UninstallSkillRequest { scope: scope.clone(), expected_version }, &token).await.map_err(|e| e.to_string())?;
            if response.scope != scope { return Err("卸载响应目标不匹配".into()); }
            Ok(Outcome::Uninstalled(id, response))
        }
        Operation::Update(id) => {
            let expected_hash = hash.ok_or("缺少已检查的当前哈希")?;
            let mut task = client.agent_apps_skills_update(&id, &UpdateSkillRequest { scope: scope.clone(), expected_version, expected_hash }, &token).await.map_err(|e| e.to_string())?;
            let task_id = task.task_id.clone();
            for _ in 0..300 {
                if task.scope != scope || task.task_id != task_id || task.directory.as_deref().is_some_and(|d| d != id) {
                    return Err("更新任务目标不匹配".into());
                }
                match task.phase {
                    SkillUpdatePhase::Completed | SkillUpdatePhase::Failed | SkillUpdatePhase::Interrupted => return Ok(Outcome::Updated(task)),
                    _ => {}
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                task = client.agent_apps_skills_update_task(task_id.as_ref(), &scope, &token).await.map_err(|e| e.to_string())?;
            }
            Err("更新仍在执行，请稍后刷新核对实际状态".into())
        }
    }
}

pub(crate) async fn gateway_skill_storage_request(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    token: String,
    change: crate::app::message::settings::skills::storage_modal::Change,
) -> Result<crate::app::message::settings::skills::storage_modal::Readback, String> {
    use crate::app::message::settings::skills::storage_modal::{Change, Readback};
    let client = gateway_client()?;
    match change {
        Change::OpenDirectory(path) => {
            tokio::task::spawn_blocking(move || {
                let home = crate::app::components::system_settings_apps::local::launch_directory(&client, &scope.target_host_id, &token, None)
                    .ok_or_else(|| "目标主机与本机授权不匹配".to_string())?;
                crate::app::message::settings::skills::storage_modal::open_local(&home, &path)
                    .map_err(|_| "无法打开主副本目录".to_string())?;
                Ok(Readback::Opened)
            }).await.map_err(|_| "打开目录任务失败".to_string())?
        }
        Change::Read => client.agent_apps_skills_storage(&scope, &token).await.map(Readback::Storage).map_err(|e| e.to_string()),
        Change::Mode(request) => client.agent_apps_skills_storage_patch(&request, &token).await.map(Readback::Storage).map_err(|e| e.to_string()),
        Change::Move(mut request) => {
            let targets = client.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if targets.scope != scope { return Err("目标作用域不匹配".into()); }
            request.targets = targets.targets;
            client.agent_apps_skills_storage_move(&request, &token).await.map(Readback::Move).map_err(|e| e.to_string())
        }
        Change::Resync(mut request) => {
            let targets = client.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if targets.scope != scope || targets.targets.is_empty() { return Err("请先配置应用绑定目标".into()); }
            request.targets = targets.targets;
            client.agent_apps_skills_resync(&request, &token).await.map(Readback::Sync).map_err(|e| e.to_string())
        }
    }
}

/// Keep successful mutations separate from readback so a failed refresh cannot undo them.
pub(crate) async fn gateway_skill_backups_request(
    scope: vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
    token: String,
    change: crate::app::message::settings::skills::backup_modal::Change,
) -> Result<crate::app::message::settings::skills::backup_modal::Readback, String> {
    use crate::app::message::settings::skills::backup_modal::{Change, Readback};
    use vw_gateway_client::vw_api_types::agent_apps_skills::backups_api::DeleteSkillBackupRequest;
    let client = crate::app::config::gateway_client().map_err(|_| "无法连接备份服务".to_string())?;
    match change {
        Change::Read => {
            let list = client.agent_apps_skills_backups(&scope, &token).await.map_err(|e| e.to_string())?;
            let inventory = client.agent_apps_skills_inventory(&scope, &token).await.map_err(|e| e.to_string())?;
            let bindings = client.agent_apps_skill_binding_targets(&scope, None, &token).await.map_err(|e| e.to_string())?;
            if list.scope != scope || inventory.scope != scope || bindings.scope != scope {
                return Err("备份读取作用域不匹配".into());
            }
            Ok(Readback::List(list, inventory.version, bindings.targets))
        }
        Change::Delete(id) => client.agent_apps_skills_backup_delete(&id, &DeleteSkillBackupRequest { scope }, &token)
            .await.map(Readback::Deleted).map_err(|e| e.to_string()),
        Change::Restore(id, request) => client.agent_apps_skills_backup_restore(&id, &request, &token)
            .await.map(Readback::Restored).map_err(|e| e.to_string()),
    }
}
