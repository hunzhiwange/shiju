//! 网关客户端主模块。
//!
//! 本模块负责聚合各业务 API 子模块，并提供统一的 [`GatewayClient`] 入口。
//! 业务能力按领域拆分到多个文件中，例如：
//! - `config_api`: 配置读写
//! - `project_api`: 项目与 worktree 管理
//! - `session_api`: 会话与 UI 持久化
//! - `file_api`: 文件树与文件操作
//! - `desktop_settings_api`: 桌面偏好与外部应用集成
//!
//! [`GatewayClient`] 本身只保存两类状态：
//! - 目标网关端点配置
//! - 复用的 `reqwest::Client`，宿主请求额外禁用重定向与重试

#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_accounts;
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_accounts::AgentAppsAuthLogout;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_accounts_tests;
#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_flow;
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_flow::{AgentAppsAuthError, AgentAppsAuthError as AgentAppsAuthFlowError};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_flow_tests;
#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_read;
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_read::{
    AgentAppsAuthBinding, AgentAppsAuthCredentials, AgentAppsAuthReadError,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_read_tests;
mod app_routing;
pub use app_routing::AppRoutingError;
mod agent_apps_lifecycle_api;
pub use agent_apps_lifecycle_api::AgentAppsLifecycleError;
#[cfg(test)]
mod agent_apps_lifecycle_api_tests;
mod agent_apps_read_api;
pub use agent_apps_read_api::AgentAppsReadError;
mod agent_apps_api;
#[cfg(test)]
#[path = "agent_apps_api_tests.rs"]
mod agent_apps_api_tests;
#[cfg(test)]
mod agent_apps_read_api_tests;
mod agent_scheduler_api;
#[cfg(test)]
#[path = "agent_scheduler_api_tests.rs"]
mod agent_scheduler_api_tests;
#[cfg(not(target_arch = "wasm32"))]
mod apps_usage_read_api;
#[cfg(not(target_arch = "wasm32"))]
pub use apps_usage_read_api::{
    AppsUsageCredentialRefs, AppsUsageDisplay, AppsUsageReadError, AppsUsageSettings,
    AppsUsageSigningCredentialRefs, AppsUsageTemplate,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_read_api_tests;
#[cfg(not(target_arch = "wasm32"))]
mod apps_usage_write_api;
#[cfg(not(target_arch = "wasm32"))]
pub use apps_usage_write_api::{
    AppsUsageOperation, AppsUsageOperationItem, AppsUsageScriptTrial, AppsUsageWriteError,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_write_api_tests;
mod channel_qr_api;
#[cfg(test)]
#[path = "channel_qr_api_tests.rs"]
mod channel_qr_api_tests;
mod config_api;
mod control_api;
#[cfg(test)]
#[path = "control_api_tests.rs"]
mod control_api_tests;
mod cron_api;
pub mod desktop_settings_api;
#[cfg(not(target_arch = "wasm32"))]
mod dynamic_tools_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "dynamic_tools_api_tests.rs"]
mod dynamic_tools_api_tests;
mod file_api;
mod git_api;
mod inference_api;
mod knowledge_api;
mod mcp_api;
pub mod mcp_management_distribution;
#[cfg(test)]
mod mcp_management_distribution_tests;
mod mcp_management_probe;
#[cfg(test)]
mod mcp_management_probe_tests;
mod mcp_management_read;
#[cfg(test)]
mod mcp_management_read_tests;
mod mcp_management_write;
#[cfg(test)]
mod mcp_management_write_tests;
mod operations_api;
#[cfg(test)]
#[path = "operations_api_tests.rs"]
mod operations_api_tests;
mod permission_api;
mod project_api;
mod provider_api;
mod pty_api;
mod question_api;
mod request_helpers;
mod session_api;
mod shiju_account_api;
#[cfg(test)]
#[path = "shiju_account_api_tests.rs"]
mod shiju_account_api_tests;
mod shiju_connector_api;
mod worktree_management_api;
pub use shiju_connector_api::*;
mod shiju_knowledge_api;
#[cfg(test)]
#[path = "shiju_knowledge_api_tests.rs"]
mod shiju_knowledge_api_tests;
pub use shiju_knowledge_api::*;
mod shiju_app_api;
#[cfg(test)]
#[path = "shiju_app_api_tests.rs"]
mod shiju_app_api_tests;
mod shiju_skill_api;
#[cfg(test)]
#[path = "shiju_skill_api_tests.rs"]
mod shiju_skill_api_tests;
mod skill_market_api;
mod system_api;
#[cfg(test)]
#[path = "system_api_tests.rs"]
mod system_api_tests;
mod task_lifecycle_api;
mod task_pool_api;
#[cfg(test)]
pub(crate) mod test_support;
mod tools_api;
mod workflow_api;

pub use self::channel_qr_api::{
    ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto,
};
pub use self::cron_api::{
    CronAddRequest, CronHistoryResponse, CronHistoryRunDto, CronJobDto, CronPreviewRequest,
    CronPreviewResponse, CronRunDto, CronUpdateRequest,
};
pub use self::desktop_settings_api::{
    DesktopPluginCapabilitiesDto, DesktopPluginCatalogEntryDto, DesktopPluginDependencyDto,
    DesktopPluginDetailDto, DesktopPluginPathDto, DesktopPluginPreviewDto, DesktopPluginSourceDto,
    DesktopServiceCommand, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto, DesktopSkillPathDto,
    ExternalAppsStateDto, HookAuditEntryDto, HookCatalogEntryDto, HookMutationRequestDto,
    HookMutationResponseDto,
};
#[cfg(not(target_arch = "wasm32"))]
pub use self::inference_api::GatewayChatWebSocket;
pub use self::inference_api::{
    GatewayAgentRequest, GatewayAgentResponse, GatewayChatWebSocketEvent,
};
pub use self::permission_api::{
    PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto,
};
#[cfg(not(target_arch = "wasm32"))]
pub use self::pty_api::{PtyConnection, PtyWebSocketEvent};
pub use self::pty_api::{PtyCreateRequest, PtyInfoDto, PtySizeDto, PtyStatusDto, PtyUpdateRequest};
pub use self::session_api::SessionUiLoadResult;
pub use self::shiju_account_api::{
    SHIJU_CREDENTIAL_HEADER, ShijuAccountDepartmentDto, ShijuAccountMeDto,
    ShijuAccountPasswordUpdateRequest, ShijuAccountProfileDto, ShijuAccountRoleDto,
    ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto, ShijuCredential,
    ShijuLoginChallengeDto, ShijuLoginPollDto, ShijuModelTokenDto, ShijuMonthlyQuotaDto,
};
pub use self::shiju_app_api::{
    ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuAppPersonalApiKeyBindingDto, ShijuCatalogAppDto,
    ShijuIamApiKeyDto, ShijuIamApiKeyListDto, ShijuInstalledAppDto,
};
pub use self::shiju_skill_api::{
    ShijuInstalledSkillDto, ShijuMaintainedSkillQuery, ShijuMarketplaceSkillDto,
    ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillReadmeDto,
    ShijuSkillScanResultDto, ShijuSkillSearchQuery, ShijuSkillTagCatalogDto, ShijuSkillTagDto,
    ShijuSkillVersionContentDto, ShijuSkillVersionDto, ShijuSkillVisibilityDto,
    ShijuSkillVisibilityScopeDto, ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto,
};
pub use self::skill_market_api::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketEnvelope,
    SkillMarketInstallScopeDto, SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto,
    SkillMarketSecurityDto, SkillMarketSkillDetailDto, SkillMarketSkillDto,
    SkillMarketSkillFileDto, SkillMarketSkillLocatorDto, SkillMarketSkillPageDto,
    SkillMarketSkillQuery, SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto,
    SkillMarketSkillSetPageDto, SkillMarketSort, SkillMarketSourceDto,
};
pub use self::workflow_api::{
    GatewayWorkflowDefaultStreamRequest, GatewayWorkflowResponseMode, GatewayWorkflowRunOutcome,
    GatewayWorkflowStreamError, GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest,
};
use crate::endpoint::GatewayEndpoint;
#[cfg(not(target_arch = "wasm32"))]
use crate::http::REQUEST_TIMEOUT_SECS;
pub use provider_api::{
    ProviderAuthMethodDto, ProviderAuthMethodsResponse, ProviderListResponse,
    ProviderOAuthAuthorizationDto,
};

/// 网关客户端，封装统一的 reqwest 实例与端点配置。
///
/// 该类型是所有业务 API 的宿主。调用方创建一次后，可以重复复用，避免为每个请求重复构建 HTTP 客户端。
#[derive(Debug, Clone)]
pub struct GatewayClient {
    endpoint: GatewayEndpoint,
    client: reqwest::Client,
    #[cfg(not(target_arch = "wasm32"))]
    dynamic_host_client: reqwest::Client,
}

impl GatewayClient {
    /// 使用给定端点创建客户端，并在原生平台上配置默认请求超时。
    ///
    /// # 错误
    ///
    /// 当底层 `reqwest::Client` 构建失败时，返回字符串化后的错误信息。
    pub fn new(endpoint: GatewayEndpoint) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let builder = || {
            let builder = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS));
            if endpoint.is_loopback() { builder.no_proxy() } else { builder }
        };
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder;
        #[cfg(not(target_arch = "wasm32"))]
        let dynamic_host_client = builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|err| err.to_string())?;
        let client = builder().build().map_err(|err| err.to_string())?;
        Ok(Self {
            endpoint,
            client,
            #[cfg(not(target_arch = "wasm32"))]
            dynamic_host_client,
        })
    }

    // Browser fetch cannot enforce the native no-redirect/no-retry transport policy.
    fn native_transport(&self) -> Result<&reqwest::Client, String> {
        #[cfg(target_arch = "wasm32")]
        {
            Err("authorized application requests require native transport".into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(&self.dynamic_host_client)
        }
    }

    /// 返回当前客户端绑定的网关端点。
    ///
    /// 可用于日志、诊断或生成派生请求配置。
    pub fn endpoint(&self) -> &GatewayEndpoint {
        &self.endpoint
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式建立聊天流，并逐条回调返回事件。
    ///
    /// 适用于不方便引入异步运行时的调用点，语义与异步版本保持一致。
    pub fn stream_chat_blocking(
        endpoint: &GatewayEndpoint,
        directory: Option<&str>,
        body: &crate::stream::GatewayChatStreamRequest,
        on_event: impl FnMut(crate::stream::GatewayChatStreamEvent) -> bool,
    ) -> Result<(), String> {
        crate::stream::stream_chat_blocking(endpoint, directory, body, on_event)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式建立聊天流，并在连续无响应超过 `idle_timeout` 时结束请求。
    ///
    /// 该超时只限制响应空窗，不限制整次流的最长运行时间。
    pub fn stream_chat_blocking_with_idle_timeout(
        endpoint: &GatewayEndpoint,
        directory: Option<&str>,
        body: &crate::stream::GatewayChatStreamRequest,
        idle_timeout: std::time::Duration,
        on_event: impl FnMut(crate::stream::GatewayChatStreamEvent) -> bool,
    ) -> Result<(), String> {
        crate::stream::stream_chat_blocking_with_idle_timeout(
            endpoint,
            directory,
            body,
            idle_timeout,
            on_event,
        )
    }

    /// 以异步方式建立聊天流，并逐条回调返回事件。
    ///
    /// 回调返回 `false` 时会主动结束消费，常用于 UI 已停止监听或调用方拿到终止事件后提前退出。
    pub async fn stream_chat(
        endpoint: &GatewayEndpoint,
        directory: Option<&str>,
        body: &crate::stream::GatewayChatStreamRequest,
        on_event: impl FnMut(crate::stream::GatewayChatStreamEvent) -> bool,
    ) -> Result<(), String> {
        crate::stream::stream_chat(endpoint, directory, body, on_event).await
    }

    /// 执行一次可恢复聊天流订阅，并返回明确的尝试结束原因。
    pub async fn stream_chat_resumable(
        endpoint: &GatewayEndpoint,
        directory: Option<&str>,
        body: &crate::stream::GatewayChatStreamRequest,
        resume: &crate::stream::GatewayStreamResume,
        on_event: impl FnMut(
            crate::stream::GatewayStreamEvent<crate::stream::GatewayChatStreamEvent>,
        ) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        crate::stream::stream_chat_resumable(endpoint, directory, body, resume, on_event).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以异步方式建立聊天流，并在连续无响应超过 `idle_timeout` 时结束请求。
    ///
    /// 该超时面向任务执行这类自动化场景，只限制响应空窗，不限制整次流的最长运行时间。
    pub async fn stream_chat_with_idle_timeout(
        endpoint: &GatewayEndpoint,
        directory: Option<&str>,
        body: &crate::stream::GatewayChatStreamRequest,
        idle_timeout: std::time::Duration,
        on_event: impl FnMut(crate::stream::GatewayChatStreamEvent) -> bool,
    ) -> Result<(), String> {
        crate::stream::stream_chat_with_idle_timeout(
            endpoint,
            directory,
            body,
            idle_timeout,
            on_event,
        )
        .await
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

mod app_providers_api;
#[cfg(test)]
mod app_providers_api_tests;

mod agent_apps_settings_api;
pub use agent_apps_settings_api::AgentAppsSettingsError;
#[cfg(test)]
mod agent_apps_settings_api_tests;

mod model_catalog_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_catalog_api_tests;

mod model_catalog_edits_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_catalog_edits_api_tests;

mod model_discovery_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_discovery_api_tests;

#[cfg(not(target_arch = "wasm32"))]
mod voice_api;
#[cfg(not(target_arch = "wasm32"))]
pub use voice_api::VoiceApiError;

mod aggregation_members_api;
pub use aggregation_members_api::AggregationApiError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_members_api_tests;

mod aggregation_omo_api;
pub use aggregation_omo_api::AggregationOmoError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_omo_api_tests;

mod aggregation_native_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_native_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_desktop_api_tests;

mod aggregation_desktop_api;

mod agent_apps_diagnostics_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_diagnostics_api_tests;

mod aggregation_publish_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_publish_api_tests;

mod aggregation_codex_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_codex_api_tests;
