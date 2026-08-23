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
//! - 复用的 `reqwest::Client`

mod agent_scheduler_api;
mod apiql_api;
#[cfg(test)]
#[path = "agent_scheduler_api_tests.rs"]
mod agent_scheduler_api_tests;
mod config_api;
mod control_api;
#[cfg(test)]
#[path = "control_api_tests.rs"]
mod control_api_tests;
mod cron_api;
pub mod desktop_settings_api;
mod file_api;
mod git_api;
mod knowledge_api;
mod inference_api;
mod mcp_api;
mod operations_api;
#[cfg(test)]
#[path = "operations_api_tests.rs"]
mod operations_api_tests;
mod permission_api;
mod pty_api;
mod project_api;
mod provider_api;
mod question_api;
mod request_helpers;
mod session_api;
mod shiju_account_api;
#[cfg(test)]
#[path = "shiju_account_api_tests.rs"]
mod shiju_account_api_tests;
mod shiju_app_api;
#[cfg(test)]
#[path = "shiju_app_api_tests.rs"]
mod shiju_app_api_tests;
mod shiju_skill_api;
#[cfg(test)]
#[path = "shiju_skill_api_tests.rs"]
mod shiju_skill_api_tests;
mod skill_market_api;
mod staff_api;
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

pub use self::cron_api::{CronAddRequest, CronJobDto, CronRunDto, CronUpdateRequest};
pub use self::apiql_api::ApiQlQuery;
pub use self::desktop_settings_api::{
    DesktopPluginCapabilitiesDto, DesktopPluginCatalogEntryDto, DesktopPluginDetailDto,
    DesktopPluginPathDto, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto, DesktopSkillPathDto,
    DesktopServiceCommand, ExternalAppsStateDto, HookAuditEntryDto, HookCatalogEntryDto, HookMutationRequestDto,
    HookMutationResponseDto,
};
pub use self::permission_api::{
    PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto,
};
pub use self::inference_api::{
    GatewayAgentRequest, GatewayAgentResponse, GatewayChatWebSocketEvent,
};
#[cfg(not(target_arch = "wasm32"))]
pub use self::inference_api::GatewayChatWebSocket;
pub use self::pty_api::{
    PtyCreateRequest, PtyInfoDto, PtySizeDto, PtyStatusDto, PtyUpdateRequest,
};
#[cfg(not(target_arch = "wasm32"))]
pub use self::pty_api::{PtyConnection, PtyWebSocketEvent};
pub use self::session_api::SessionUiLoadResult;
pub use self::shiju_account_api::{
    SHIJU_CREDENTIAL_HEADER, ShijuAccountDepartmentDto, ShijuAccountMeDto,
    ShijuAccountPasswordUpdateRequest, ShijuAccountProfileDto, ShijuAccountRoleDto,
    ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto, ShijuCredential,
    ShijuLoginChallengeDto, ShijuLoginPollDto,
};
pub use self::shiju_app_api::{
    ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuAppPersonalApiKeyBindingDto, ShijuCatalogAppDto,
    ShijuIamApiKeyDto, ShijuIamApiKeyListDto, ShijuInstalledAppDto,
};
pub use self::shiju_skill_api::{
    ShijuCompanySkillQuery, ShijuInstalledSkillDto, ShijuMaintainedSkillQuery,
    ShijuMarketplaceSkillDto, ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto,
    ShijuSkillReadmeDto, ShijuSkillScanResultDto, ShijuSkillSearchQuery, ShijuSkillTagDto,
    ShijuSkillVersionContentDto, ShijuSkillVersionDto, ShijuSkillVisibilityDto,
    ShijuSkillVisibilityScopeDto,
    ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto,
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
}

impl GatewayClient {
    /// 使用给定端点创建客户端，并在原生平台上配置默认请求超时。
    ///
    /// # 错误
    ///
    /// 当底层 `reqwest::Client` 构建失败时，返回字符串化后的错误信息。
    pub fn new(endpoint: GatewayEndpoint) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS));
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder();
        let client = builder.build().map_err(|err| err.to_string())?;
        Ok(Self { endpoint, client })
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
