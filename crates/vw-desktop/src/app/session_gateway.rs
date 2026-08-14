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
    ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuCompanySkillQuery, ShijuCredential,
    ShijuInstalledSkillDto, ShijuLoginChallengeDto, ShijuLoginPollDto, ShijuMaintainedSkillQuery,
    ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillSearchQuery,
    ShijuSkillVersionDto, ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

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

pub(crate) async fn gateway_shiju_company_skills_async(
    credential: &ShijuCredential,
    query: &ShijuCompanySkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    let client = gateway_client()?;
    client.shiju_company_skills(credential, query).await
}

pub(crate) async fn gateway_shiju_maintained_skills_async(
    credential: &ShijuCredential,
    query: &ShijuMaintainedSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    let client = gateway_client()?;
    client.shiju_maintained_skills(credential, query).await
}

pub(crate) async fn gateway_shiju_skill_detail_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillDetailDto, String> {
    let client = gateway_client()?;
    client.shiju_skill_detail(credential, skill_id).await
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
    step_index: u32,
) -> Result<Option<serde_json::Value>, String> {
    let client = gateway_client()?;
    client.session_step_llm_raw_get(id, step_index, None).await
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
    // 原始 calls 由网关/流处理器独立持久化；UI 快照不再重复携带它们。
    let mut session_to_save = session.clone();
    let omitted_call_count = session_to_save.calls.len();
    session_to_save.calls.clear();
    if omitted_call_count > 0 {
        tracing::debug!(
            target: "vw_desktop",
            session_id = %session_to_save.id,
            omitted_call_count,
            "omitted raw call records from session ui snapshot"
        );
    }
    tracing::debug!(
        target: "vw_desktop",
        session_id = %session_to_save.id,
        directory = ?directory,
        message_count = session_to_save.messages.len(),
        step_count = session_to_save.steps.len(),
        call_count = session_to_save.calls.len(),
        "desktop saving session ui snapshot"
    );
    client.session_ui_save(&session_to_save.id, directory, &session_to_save).await?;
    Ok(Some(std::path::PathBuf::from(&session_to_save.id)))
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
    client.plugins_get().await
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
