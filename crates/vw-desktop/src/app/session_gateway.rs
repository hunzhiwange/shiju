//! 封装会话与网关运行时之间的连接和请求逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::collections::HashSet;

use crate::app::config;
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

pub use desktop_session::message::chat::stream::save::SessionMessageDeltaSaveError;

#[cfg(test)]
fn gateway_client() -> Result<vw_gateway_client::GatewayClient, String> {
    config::gateway_client()
}

pub(crate) fn active_tenant_scope() -> TenantScopeDto {
    config::gateway_client_endpoint().tenant_scope().clone()
}

pub(crate) async fn gateway_task_plan_run_stop_with_client(
    client: &vw_gateway_client::GatewayClient,
    task_id: &str,
) -> Result<vw_api_task::TaskPlanRunStatusResponse, String> {
    desktop_session::session_gateway::gateway_task_plan_run_stop_with_client(client, task_id).await
}

pub(crate) async fn gateway_task_plan_run_stop_async(
    task_id: &str,
) -> Result<vw_api_task::TaskPlanRunStatusResponse, String> {
    desktop_session::session_gateway::gateway_task_plan_run_stop_async(config::gateway_client_endpoint(), task_id).await
}

pub(crate) async fn gateway_shiju_account_login_async() -> Result<ShijuLoginChallengeDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_login_async(config::gateway_client_endpoint()).await
}

pub(crate) async fn gateway_shiju_account_login_poll_async(
    state: &str,
) -> Result<ShijuLoginPollDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_login_poll_async(config::gateway_client_endpoint(), state).await
}

pub(crate) async fn gateway_shiju_account_profile_async(
    credential: &ShijuCredential,
) -> Result<ShijuAccountProfileDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_profile_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_account_monthly_quota_async(
    credential: &ShijuCredential,
) -> Result<gateway_shiju_account::ShijuMonthlyQuotaDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_monthly_quota_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_account_me_async(
    credential: &ShijuCredential,
) -> Result<ShijuAccountMeDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_me_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_account_update_async(
    credential: &ShijuCredential,
    request: &ShijuAccountUpdateRequest,
) -> Result<ShijuAccountUpdateResponseDto, String> {
    desktop_session::session_gateway::gateway_shiju_account_update_async(config::gateway_client_endpoint(), credential, request).await
}

pub(crate) async fn gateway_shiju_account_password_update_async(
    credential: &ShijuCredential,
    request: &ShijuAccountPasswordUpdateRequest,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_shiju_account_password_update_async(config::gateway_client_endpoint(), credential, request).await
}

pub(crate) async fn gateway_shiju_app_catalog_async(
    credential: &ShijuCredential,
) -> Result<ShijuAppCatalogDto, String> {
    desktop_session::session_gateway::gateway_shiju_app_catalog_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_apps_async(
    credential: &ShijuCredential,
) -> Result<ShijuAppListDto, String> {
    desktop_session::session_gateway::gateway_shiju_apps_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_app_api_key_binding_async(
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppApiKeyBindingDto, String> {
    desktop_session::session_gateway::gateway_shiju_app_api_key_binding_async(config::gateway_client_endpoint(), credential, installation_id).await
}

pub(crate) async fn gateway_shiju_app_api_keys_async(
    credential: &ShijuCredential,
) -> Result<ShijuIamApiKeyListDto, String> {
    desktop_session::session_gateway::gateway_shiju_app_api_keys_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_app_save_api_key_binding_async(
    credential: &ShijuCredential,
    installation_id: &str,
    request: &ShijuAppApiKeyBindingSaveRequest,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_shiju_app_save_api_key_binding_async(config::gateway_client_endpoint(), credential, installation_id, request).await
}

pub(crate) async fn gateway_shiju_app_prepare_launch_async(
    credential: &ShijuCredential,
    installation_id: &str,
) -> Result<ShijuAppLaunchDto, String> {
    desktop_session::session_gateway::gateway_shiju_app_prepare_launch_async(config::gateway_client_endpoint(), credential, installation_id).await
}

pub(crate) async fn gateway_shiju_skill_search_async(
    credential: &ShijuCredential,
    query: &ShijuSkillSearchQuery,
) -> Result<ShijuSkillCursorPageDto, String> {
    desktop_session::session_gateway::gateway_shiju_skill_search_async(config::gateway_client_endpoint(), credential, query).await
}

pub(crate) async fn gateway_shiju_skill_installed_async(
    credential: &ShijuCredential,
) -> Result<Vec<ShijuInstalledSkillDto>, String> {
    desktop_session::session_gateway::gateway_shiju_skill_installed_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_system_skills_async(
    credential: &ShijuCredential,
    query: &ShijuSystemSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuSystemSkillDto>, String> {
    desktop_session::session_gateway::gateway_shiju_system_skills_async(config::gateway_client_endpoint(), credential, query).await
}

pub(crate) async fn gateway_shiju_maintained_skills_async(
    credential: &ShijuCredential,
    query: &ShijuMaintainedSkillQuery,
) -> Result<ShijuSkillPageDto<ShijuTenantSkillDto>, String> {
    desktop_session::session_gateway::gateway_shiju_maintained_skills_async(config::gateway_client_endpoint(), credential, query).await
}

pub(crate) async fn gateway_shiju_skill_tags_async(
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    desktop_session::session_gateway::gateway_shiju_skill_tags_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_tenant_skill_tags_async(
    credential: &ShijuCredential,
) -> Result<ShijuSkillTagCatalogDto, String> {
    desktop_session::session_gateway::gateway_shiju_tenant_skill_tags_async(config::gateway_client_endpoint(), credential).await
}

pub(crate) async fn gateway_shiju_skill_detail_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillDetailDto, String> {
    desktop_session::session_gateway::gateway_shiju_skill_detail_async(config::gateway_client_endpoint(), credential, skill_id).await
}

#[allow(dead_code)]
pub(crate) async fn gateway_shiju_skill_readme_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<ShijuSkillReadmeDto, String> {
    desktop_session::session_gateway::gateway_shiju_skill_readme_async(config::gateway_client_endpoint(), credential, skill_id).await
}

pub(crate) async fn gateway_shiju_skill_versions_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<Vec<ShijuSkillVersionDto>, String> {
    desktop_session::session_gateway::gateway_shiju_skill_versions_async(config::gateway_client_endpoint(), credential, skill_id).await
}

pub(crate) async fn gateway_shiju_skill_remote_install_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_shiju_skill_remote_install_async(config::gateway_client_endpoint(), credential, skill_id).await
}

#[allow(dead_code)]
pub(crate) async fn gateway_shiju_skill_remote_install_version_async(
    credential: &ShijuCredential,
    skill_id: u64,
    version_id: u64,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_shiju_skill_remote_install_version_async(config::gateway_client_endpoint(), credential, skill_id, version_id).await
}

pub(crate) async fn gateway_shiju_skill_remote_uninstall_async(
    credential: &ShijuCredential,
    skill_id: u64,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_shiju_skill_remote_uninstall_async(config::gateway_client_endpoint(), credential, skill_id).await
}

pub(crate) fn active_project_scope(project_id: ProjectId) -> Result<ProjectScopeDto, String> {
    desktop_session::session_gateway::active_project_scope(config::gateway_client_endpoint(), project_id)
}

pub(crate) fn active_agent_scheduler_scope(
    project_id: ProjectId,
) -> Result<AgentSchedulerScopeDto, String> {
    desktop_session::session_gateway::active_agent_scheduler_scope(config::gateway_client_endpoint(), project_id)
}

/// 执行 gateway_load_session_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_session_async(id: &str) -> Result<Option<ChatSession>, String> {
    desktop_session::session_gateway::gateway_load_session_async(config::gateway_client_endpoint(), id).await
}

pub(crate) async fn gateway_bounded_session_open_with_client(
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
    desktop_session::session_gateway::gateway_bounded_session_open_with_client(client, session_id, directory).await
}

pub(crate) async fn gateway_session_message_page_before_with_client(
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
    desktop_session::session_gateway::gateway_session_message_page_before_with_client(client, session_id, directory, cursor).await
}

/// 执行 gateway_load_session_any_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_session_any_async(id: &str) -> Result<Option<ChatSession>, String> {
    desktop_session::session_gateway::gateway_load_session_any_async(config::gateway_client_endpoint(), id).await
}

/// 无目录限制地读取会话 UI 数据，并保留每步原始 LLM 载荷。
pub async fn gateway_load_session_any_with_raw_async(
    id: &str,
) -> Result<Option<gateway_session::SessionUiLoadResult>, String> {
    desktop_session::session_gateway::gateway_load_session_any_with_raw_async(config::gateway_client_endpoint(), id).await
}

/// 执行 gateway_load_sessions_scoped_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_sessions_scoped_async(
    scope: Option<&str>,
) -> Result<Vec<ChatSessionMeta>, String> {
    desktop_session::session_gateway::gateway_load_sessions_scoped_async(config::gateway_client_endpoint(), scope).await
}

/// 执行 gateway_load_archived_session_ids_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_load_archived_session_ids_async(
    scope: Option<&str>,
) -> Result<HashSet<String>, String> {
    desktop_session::session_gateway::gateway_load_archived_session_ids_async(config::gateway_client_endpoint(), scope).await
}

/// 执行 gateway_session_preview_meta_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_preview_meta_async(
    id: &str,
) -> Result<Option<ChatSessionMeta>, String> {
    desktop_session::session_gateway::gateway_session_preview_meta_async(config::gateway_client_endpoint(), id).await
}

/// 执行 gateway_session_file_path_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_file_path_async(
    id: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    desktop_session::session_gateway::gateway_session_file_path_async(config::gateway_client_endpoint(), id).await
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
    desktop_session::session_gateway::gateway_session_step_llm_raw_async(config::gateway_client_endpoint(), id, turn_id, step_index, directory).await
}

/// 执行 gateway_resolve_session_scope_id_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_resolve_session_scope_id_async(
    path: Option<&str>,
    project_id: Option<&str>,
) -> Result<Option<String>, String> {
    desktop_session::session_gateway::gateway_resolve_session_scope_id_async(config::gateway_client_endpoint(), path, project_id).await
}

/// 执行 gateway_permission_list_owned_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_permission_list_owned_async(
    directory: Option<String>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    desktop_session::session_gateway::gateway_permission_list_owned_async(config::gateway_client_endpoint(), directory).await
}

/// 执行 gateway_save_session_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_save_session_async(
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    desktop_session::session_gateway::gateway_save_session_async(config::gateway_client_endpoint(), session, directory).await
}

/// 原子提交有界的会话消息增量。
pub async fn gateway_session_message_delta_async(
    session_id: &str,
    directory: Option<&str>,
    delta: &SessionMessageDelta,
) -> Result<(), SessionMessageDeltaSaveError> {
    desktop_session::session_gateway::gateway_session_message_delta_async(config::gateway_client_endpoint(), session_id, directory, delta).await
}

/// 执行 gateway_set_session_scope_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_set_session_scope_async(scope: Option<&str>) -> Result<(), String> {
    desktop_session::session_gateway::gateway_set_session_scope_async(config::gateway_client_endpoint(), scope).await
}

/// 执行 gateway_current_session_scope_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_current_session_scope_async() -> Result<Option<String>, String> {
    desktop_session::session_gateway::gateway_current_session_scope_async(config::gateway_client_endpoint()).await
}

/// 执行 gateway_question_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_list_async() -> Result<Vec<question_types::Request>, String> {
    desktop_session::session_gateway::gateway_question_list_async(config::gateway_client_endpoint()).await
}

/// 执行 gateway_session_todo_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_session_todo_list_async(
    id: &str,
    directory: Option<&str>,
) -> Result<Vec<todo_types::Todo>, String> {
    desktop_session::session_gateway::gateway_session_todo_list_async(config::gateway_client_endpoint(), id, directory).await
}

/// 执行 gateway_question_reply_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_reply_async(
    request_id: &str,
    answers: Vec<Vec<String>>,
) -> Result<(), String> {
    desktop_session::session_gateway::gateway_question_reply_async(config::gateway_client_endpoint(), request_id, answers).await
}

/// 执行 gateway_question_reject_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_question_reject_async(request_id: &str) -> Result<(), String> {
    desktop_session::session_gateway::gateway_question_reject_async(config::gateway_client_endpoint(), request_id).await
}

/// 执行 gateway_permission_list_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_permission_list_async(
    directory: Option<&str>,
) -> Result<Vec<PendingPermissionRequestDto>, String> {
    desktop_session::session_gateway::gateway_permission_list_async(config::gateway_client_endpoint(), directory).await
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
    desktop_session::session_gateway::gateway_permission_reply_async(config::gateway_client_endpoint(), request_id, reply, directory).await
}

/// 执行 gateway_save_agent_session_scoped_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_save_agent_session_scoped_async(
    info: &session_info::Info,
    scope: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    desktop_session::session_gateway::gateway_save_agent_session_scoped_async(config::gateway_client_endpoint(), info, scope).await
}

/// 执行 gateway_external_apps_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_apps_async() -> Result<ExternalAppsStateDto, String> {
    desktop_session::session_gateway::gateway_external_apps_async(config::gateway_client_endpoint()).await
}

/// 执行 gateway_external_open_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_open_async(path: &str, target: &str) -> Result<(), String> {
    desktop_session::session_gateway::gateway_external_open_async(config::gateway_client_endpoint(), path, target).await
}

/// 执行 gateway_external_reveal_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_external_reveal_async(path: &str) -> Result<(), String> {
    desktop_session::session_gateway::gateway_external_reveal_async(config::gateway_client_endpoint(), path).await
}

/// 执行 gateway_skills_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skills_async(
    project_path: Option<&str>,
) -> Result<Vec<DesktopSkillCatalogEntryDto>, String> {
    desktop_session::session_gateway::gateway_skills_async(config::gateway_client_endpoint(), project_path).await
}

/// 执行 gateway_skill_detail_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_detail_async(
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<DesktopSkillDetailDto, String> {
    desktop_session::session_gateway::gateway_skill_detail_async(config::gateway_client_endpoint(), project_path, skill_id).await
}

/// 执行 gateway_skill_create_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_create_async(project_path: &str) -> Result<String, String> {
    desktop_session::session_gateway::gateway_skill_create_async(config::gateway_client_endpoint(), project_path).await
}

/// 执行 gateway_skill_install_builtin_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_install_builtin_async(
    project_path: &str,
    skill_id: &str,
) -> Result<String, String> {
    desktop_session::session_gateway::gateway_skill_install_builtin_async(config::gateway_client_endpoint(), project_path, skill_id).await
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
    desktop_session::session_gateway::gateway_skill_set_enabled_async(config::gateway_client_endpoint(), project_path, skill_id, enabled).await
}

/// 执行 gateway_skill_delete_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_skill_delete_async(
    project_path: Option<&str>,
    skill_id: &str,
) -> Result<String, String> {
    desktop_session::session_gateway::gateway_skill_delete_async(config::gateway_client_endpoint(), project_path, skill_id).await
}

/// Lists the skill-market sources registered by the active gateway.
pub async fn gateway_skill_market_sources_async() -> Result<Vec<SkillMarketSourceDto>, String> {
    desktop_session::session_gateway::gateway_skill_market_sources_async(config::gateway_client_endpoint()).await
}

/// Lists recommended skills from one registered market source.
pub async fn gateway_skill_market_recommended_async(
    source: &str,
) -> Result<Vec<SkillMarketSkillDto>, String> {
    desktop_session::session_gateway::gateway_skill_market_recommended_async(config::gateway_client_endpoint(), source).await
}

/// Queries skills from one registered market source.
pub async fn gateway_skill_market_skills_async(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillPageDto, String> {
    desktop_session::session_gateway::gateway_skill_market_skills_async(config::gateway_client_endpoint(), source, query).await
}

/// Lists categories from one registered market source.
pub async fn gateway_skill_market_categories_async(
    source: &str,
) -> Result<Vec<SkillMarketCategoryDto>, String> {
    desktop_session::session_gateway::gateway_skill_market_categories_async(config::gateway_client_endpoint(), source).await
}

/// Reads one source-aware market skill detail.
pub async fn gateway_skill_market_skill_detail_async(
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillDetailDto, String> {
    desktop_session::session_gateway::gateway_skill_market_skill_detail_async(config::gateway_client_endpoint(), locator).await
}

/// Reads the canonical Markdown file for one source-aware market skill.
pub async fn gateway_skill_market_skill_file_async(
    locator: &SkillMarketSkillLocatorDto,
) -> Result<SkillMarketSkillFileDto, String> {
    desktop_session::session_gateway::gateway_skill_market_skill_file_async(config::gateway_client_endpoint(), locator).await
}

/// Installs one canonical market skill into the active project.
pub async fn gateway_skill_market_install_async(
    project_path: &str,
    coordinate: &SkillMarketCoordinateDto,
) -> Result<String, String> {
    desktop_session::session_gateway::gateway_skill_market_install_async(config::gateway_client_endpoint(), project_path, coordinate).await
}

/// Installs one canonical market skill into explicit Agent targets.
pub async fn gateway_skill_market_install_to_targets_async(
    project_path: Option<&str>,
    coordinate: &SkillMarketCoordinateDto,
    installations: &[SkillMarketInstallSelectionDto],
) -> Result<Vec<String>, String> {
    desktop_session::session_gateway::gateway_skill_market_install_to_targets_async(config::gateway_client_endpoint(), project_path, coordinate, installations).await
}

/// Queries skill sets from one registered market source.
pub async fn gateway_skill_market_skill_sets_async(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Result<SkillMarketSkillSetPageDto, String> {
    desktop_session::session_gateway::gateway_skill_market_skill_sets_async(config::gateway_client_endpoint(), source, query).await
}

/// Reads one skill-set detail from a registered market source.
pub async fn gateway_skill_market_skill_set_detail_async(
    source: &str,
    slug: &str,
) -> Result<SkillMarketSkillSetDetailDto, String> {
    desktop_session::session_gateway::gateway_skill_market_skill_set_detail_async(config::gateway_client_endpoint(), source, slug).await
}

/// Installs one market skill set into the active project atomically.
pub async fn gateway_skill_market_skill_set_install_async(
    project_path: &str,
    source: &str,
    slug: &str,
) -> Result<Vec<String>, String> {
    desktop_session::session_gateway::gateway_skill_market_skill_set_install_async(config::gateway_client_endpoint(), project_path, source, slug).await
}

/// 执行 gateway_plugins_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugins_async() -> Result<Vec<DesktopPluginCatalogEntryDto>, String> {
    desktop_session::session_gateway::gateway_plugins_async(config::gateway_client_endpoint()).await
}

/// 读取当前已发现插件声明的 hook 清单。
pub async fn gateway_hooks_async() -> Result<Vec<HookCatalogEntryDto>, String> {
    desktop_session::session_gateway::gateway_hooks_async(config::gateway_client_endpoint()).await
}

/// 使用服务端重新解析后的稳定哈希信任当前 hook 声明。
pub async fn gateway_hook_trust_async(
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    desktop_session::session_gateway::gateway_hook_trust_async(config::gateway_client_endpoint(), plugin_id, hook_hash).await
}

/// 使用服务端重新解析后的稳定哈希撤销当前 hook 声明。
pub async fn gateway_hook_revoke_async(
    plugin_id: &str,
    hook_hash: &str,
) -> Result<HookMutationResponseDto, String> {
    desktop_session::session_gateway::gateway_hook_revoke_async(config::gateway_client_endpoint(), plugin_id, hook_hash).await
}

/// 读取只包含时间、工具名、耗时和成功状态的有界审计记录。
pub async fn gateway_hook_audit_async() -> Result<Vec<HookAuditEntryDto>, String> {
    desktop_session::session_gateway::gateway_hook_audit_async(config::gateway_client_endpoint()).await
}

/// 执行 gateway_plugin_set_enabled_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugin_set_enabled_async(
    plugin_id: &str,
    enabled: bool,
) -> Result<String, String> {
    desktop_session::session_gateway::gateway_plugin_set_enabled_async(config::gateway_client_endpoint(), plugin_id, enabled).await
}

/// 执行 gateway_plugin_delete_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_plugin_delete_async(plugin_id: &str) -> Result<String, String> {
    desktop_session::session_gateway::gateway_plugin_delete_async(config::gateway_client_endpoint(), plugin_id).await
}

/// 执行 gateway_project_change_records_async 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub async fn gateway_project_change_records_async(
    directory: &str,
) -> Result<Vec<ProjectChangeRecordDto>, String> {
    desktop_session::session_gateway::gateway_project_change_records_async(config::gateway_client_endpoint(), directory).await
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_session 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_session(id: &str) -> Option<ChatSession> {
    desktop_session::session_gateway::gateway_load_session(config::gateway_client_endpoint(), id)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_session_any 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_session_any(id: &str) -> Option<ChatSession> {
    desktop_session::session_gateway::gateway_load_session_any(config::gateway_client_endpoint(), id)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_sessions_scoped 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_sessions_scoped(scope: Option<&str>) -> Vec<ChatSessionMeta> {
    desktop_session::session_gateway::gateway_load_sessions_scoped(config::gateway_client_endpoint(), scope)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_load_archived_session_ids 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_load_archived_session_ids(scope: Option<&str>) -> HashSet<String> {
    desktop_session::session_gateway::gateway_load_archived_session_ids(config::gateway_client_endpoint(), scope)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_session_preview_meta 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_session_preview_meta(id: &str) -> Option<ChatSessionMeta> {
    desktop_session::session_gateway::gateway_session_preview_meta(config::gateway_client_endpoint(), id)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_session_file_path 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_session_file_path(id: &str) -> Option<std::path::PathBuf> {
    desktop_session::session_gateway::gateway_session_file_path(config::gateway_client_endpoint(), id)
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
    desktop_session::session_gateway::gateway_resolve_session_scope_id(config::gateway_client_endpoint(), path, project_id)
}

#[cfg(not(target_arch = "wasm32"))]
/// 同步保存会话，并保留网关返回的原始错误。
pub fn gateway_save_session_result(
    session: &ChatSession,
    directory: Option<&str>,
) -> Result<Option<std::path::PathBuf>, String> {
    desktop_session::session_gateway::gateway_save_session_result(config::gateway_client_endpoint(), session, directory)
}

/// 判断会话保存错误是否无法通过后续快照重试恢复。
pub fn is_permanent_session_save_error(error: &str) -> bool {
    desktop_session::session_gateway::is_permanent_session_save_error(error)
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
    desktop_session::session_gateway::gateway_save_session(config::gateway_client_endpoint(), session, directory)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_set_session_scope 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_set_session_scope(scope: Option<&str>) {
    desktop_session::session_gateway::gateway_set_session_scope(config::gateway_client_endpoint(), scope)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_current_session_scope 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_current_session_scope() -> Option<String> {
    desktop_session::session_gateway::gateway_current_session_scope(config::gateway_client_endpoint())
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_list 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_list() -> Vec<question_types::Request> {
    desktop_session::session_gateway::gateway_question_list(config::gateway_client_endpoint())
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_reply 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_reply(request_id: &str, answers: Vec<Vec<String>>) {
    desktop_session::session_gateway::gateway_question_reply(config::gateway_client_endpoint(), request_id, answers)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_question_reject 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_question_reject(request_id: &str) {
    desktop_session::session_gateway::gateway_question_reject(config::gateway_client_endpoint(), request_id)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_permission_list 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_permission_list(directory: Option<&str>) -> Vec<PendingPermissionRequestDto> {
    desktop_session::session_gateway::gateway_permission_list(config::gateway_client_endpoint(), directory)
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
    desktop_session::session_gateway::gateway_permission_reply(config::gateway_client_endpoint(), request_id, reply, directory)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_save_agent_session_scoped 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_save_agent_session_scoped(
    info: &session_info::Info,
    scope: Option<&str>,
) -> Option<std::path::PathBuf> {
    desktop_session::session_gateway::gateway_save_agent_session_scoped(config::gateway_client_endpoint(), info, scope)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 gateway_project_change_records 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_project_change_records(directory: &str) -> Vec<ProjectChangeRecordDto> {
    desktop_session::session_gateway::gateway_project_change_records(config::gateway_client_endpoint(), directory)
}

#[cfg(target_arch = "wasm32")]
/// 执行 gateway_project_change_records 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn gateway_project_change_records(_directory: &str) -> Vec<ProjectChangeRecordDto> {
    desktop_session::session_gateway::gateway_project_change_records(config::gateway_client_endpoint(), _directory)
}

#[cfg(test)]
#[path = "session_gateway_tests.rs"]
mod session_gateway_tests;

pub async fn gateway_plugin_prepare_async(
    source: String,
    update_id: Option<String>,
) -> Result<gateway_desktop_settings::DesktopPluginPreviewDto, String> {
    desktop_session::session_gateway::gateway_plugin_prepare_async(config::gateway_client_endpoint(), source, update_id).await
}
pub async fn gateway_plugin_activate_async(token: String, update: bool) -> Result<String, String> {
    desktop_session::session_gateway::gateway_plugin_activate_async(config::gateway_client_endpoint(), token, update).await
}
pub async fn gateway_plugin_discard_async(token: String) -> Result<(), String> {
    desktop_session::session_gateway::gateway_plugin_discard_async(config::gateway_client_endpoint(), token).await
}

/// Resolve the selected project without creating it; repository authorization remains server-side.
pub async fn gateway_skill_repos_scope(directory: &str, host: String) -> Result<vw_api_app_skills::repos_api::SkillReposQuery, String> {
    desktop_session::session_gateway::gateway_skill_repos_scope(config::gateway_client_endpoint(), directory, host).await
}

pub async fn gateway_skill_repos_request(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    version: u64,
    change: Option<(vw_api_app_skills::repos_api::SkillRepoEntry, bool)>,
) -> Result<vw_api_app_skills::repos_api::SkillReposResponse, String> {
    desktop_session::session_gateway::gateway_skill_repos_request(config::gateway_client_endpoint(), scope, version, change).await
}

/// Only fixed Claude actions cross this boundary; credentials never enter messages.
pub(crate) async fn gateway_claude_config_request(
    scope: vw_api_app_skills::claude_api::ClaudeConfigQuery,
    token: String,
    change: Option<(bool, vw_api_app_skills::claude_api::ClaudeConfigAction)>,
) -> Result<(vw_api_app_skills::claude_api::ClaudeConfigResponse, Option<String>), String> {
    desktop_session::session_gateway::gateway_claude_config_request(config::gateway_client_endpoint(), scope, token, change).await
}

/// Read-only skills requests; credentials stay outside UI messages.
pub(crate) async fn gateway_skills_market_request(
    project: String, host: String, query: String, token: String,
) -> Result<crate::app::message::settings::skills::market_view::Results, String> {
    desktop_session::session_gateway::gateway_skills_market_request(config::gateway_client_endpoint(), project, host, query, token).await
}

/// Installation is submitted once; observe the same durable task until terminal.
pub(crate) async fn gateway_skills_install_view(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    request: Option<vw_api_app_skills::install_api::InstallSkillRequest>,
    token: String,
) -> Result<Option<vw_api_app_skills::inventory_api::SkillInventoryResponse>, String> {
    desktop_session::session_gateway::gateway_skills_install_view(config::gateway_client_endpoint(), scope, request, token).await
}

/// Re-scan between writes: each selection is bound to one server revision.
pub(crate) async fn gateway_skills_import_request(
    project: String, host: String,
    apps: Vec<vw_api_app_identity::AppId>,
    selected: Vec<vw_api_app_skills::import_api::UnmanagedSkillItem>,
    token: String,
) -> (Vec<(String, Result<(), String>)>, Result<vw_api_app_skills::import_api::UnmanagedSkillsResponse, String>) {
    desktop_session::session_gateway::gateway_skills_import_request(config::gateway_client_endpoint(), project, host, apps, selected, token).await
}

pub(crate) async fn gateway_skills_job(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    id: vw_api_transport::TaskId,
    token: String,
    cancel: bool,
) -> Result<vw_api_app_skills::jobs_api::SkillJobResponse, String> {
    desktop_session::session_gateway::gateway_skills_job(config::gateway_client_endpoint(), scope, id, token, cancel).await
}

pub(crate) async fn gateway_skills_submit(
    request: vw_api_app_skills::install_api::InstallSkillRequest,
    token: String,
) -> Result<vw_api_app_skills::install_api::InstallSkillResponse, String> {
    desktop_session::session_gateway::gateway_skills_submit(config::gateway_client_endpoint(), request, token).await
}

pub(crate) async fn gateway_skill_bindings_request(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: Option<crate::app::message::settings::skills::bindings_view::Change>,
) -> Result<crate::app::message::settings::skills::bindings_view::Readback, String> {
    desktop_session::session_gateway::gateway_skill_bindings_request(config::gateway_client_endpoint(), scope, token, change).await
}

/// Mutation and polling use the same authorized gateway instance and scope.
pub(crate) async fn gateway_skills_mutation(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    operation: (crate::app::message::settings::skills::updates_view::Operation, u64, Option<String>),
    token: String,
) -> Result<crate::app::message::settings::skills::updates_view::Outcome, String> {
    desktop_session::session_gateway::gateway_skills_mutation(config::gateway_client_endpoint(), scope, operation, token).await
}

pub(crate) async fn gateway_skill_storage_request(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: crate::app::message::settings::skills::storage_modal::Change,
) -> Result<crate::app::message::settings::skills::storage_modal::Readback, String> {
    desktop_session::session_gateway::gateway_skill_storage_request(config::gateway_client_endpoint(), scope, token, change).await
}

/// Keep successful mutations separate from readback so a failed refresh cannot undo them.
pub(crate) async fn gateway_skill_backups_request(
    scope: vw_api_app_skills::repos_api::SkillReposQuery,
    token: String,
    change: crate::app::message::settings::skills::backup_modal::Change,
) -> Result<crate::app::message::settings::skills::backup_modal::Readback, String> {
    desktop_session::session_gateway::gateway_skill_backups_request(config::gateway_client_endpoint(), scope, token, change).await
}
