//! 网关客户端封装。
//!
//! 本 crate 负责为桌面端、CLI 与其他调用方提供统一的网关访问层，覆盖：
//! - 普通 HTTP JSON 接口请求
//! - 聊天流式 SSE 接口消费
//! - 会话、项目、文件、桌面设置等常用业务 API
//! - 网关端点与认证参数的统一描述
//! - 向上层透出稳定的请求/响应类型别名
//!
//! # 模块划分
//!
//! - `client`: 对外暴露的主客户端实现
//! - `endpoint`: 网关地址与认证模型
//! - `http`: 通用请求、日志与错误处理辅助函数
//! - `session`: 会话相关类型导出
//! - `stream`: 流式聊天请求与 SSE 事件解析
//!
//! # 设计目标
//!
//! - 对上层屏蔽具体 HTTP 细节
//! - 保持错误文本稳定，便于直接展示
//! - 尽量复用 vw-api-types 中的类型定义，减少重复建模
//! - 同时兼容异步与少量阻塞调用场景

mod client;
mod endpoint;
mod http;
mod session;
mod stream;

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use client::GatewayChatWebSocket;
/// 网关 HTTP 与流式接口的统一客户端入口。
///
/// 调用方通常只需要持有这个类型，即可访问本 crate 暴露的大部分能力。
pub use client::GatewayClient;
/// 保留每步原始 LLM 载荷的会话 UI 批量加载结果。
pub use client::SessionUiLoadResult;
/// Channel QR-code authorization transport contract.
pub use client::{ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto};
pub use client::{CronAddRequest, CronJobDto, CronRunDto, CronUpdateRequest};
/// 桌面端可用外部应用状态的精简传输对象。
///
/// 该结构会把原始 JSON 响应规整为更适合 UI 层消费的字段。
pub use client::{
    DesktopPluginCapabilitiesDto, DesktopPluginCatalogEntryDto, DesktopPluginDetailDto,
    DesktopPluginPathDto, DesktopServiceCommand, DesktopSkillCatalogEntryDto,
    DesktopSkillDetailDto, DesktopSkillPathDto, ExternalAppsStateDto, HookAuditEntryDto,
    HookCatalogEntryDto, HookMutationRequestDto, HookMutationResponseDto,
    PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto,
    PtyCreateRequest, PtyInfoDto, PtySizeDto, PtyStatusDto, PtyUpdateRequest,
};
pub use client::{GatewayAgentRequest, GatewayAgentResponse, GatewayChatWebSocketEvent};
/// Workflow streaming 请求与事件类型。
pub use client::{
    GatewayWorkflowDefaultStreamRequest, GatewayWorkflowResponseMode, GatewayWorkflowRunOutcome,
    GatewayWorkflowStreamError, GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest,
};
/// Provider 列表接口的响应结构。
///
/// 包含全部 provider、默认路由与已连接 provider 三部分信息。
pub use client::{
    ProviderAuthMethodDto, ProviderAuthMethodsResponse, ProviderListResponse,
    ProviderOAuthAuthorizationDto,
};
#[cfg(not(target_arch = "wasm32"))]
pub use client::{PtyConnection, PtyWebSocketEvent};
/// SHIJU account gateway transport contract.
pub use client::{
    SHIJU_CREDENTIAL_HEADER, ShijuAccountDepartmentDto, ShijuAccountMeDto,
    ShijuAccountPasswordUpdateRequest, ShijuAccountProfileDto, ShijuAccountRoleDto,
    ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto, ShijuCredential,
    ShijuLoginChallengeDto, ShijuLoginPollDto, ShijuModelTokenDto,
};
/// SHIJU application gateway transport contract.
pub use client::{
    ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuAppPersonalApiKeyBindingDto, ShijuCatalogAppDto,
    ShijuIamApiKeyDto, ShijuIamApiKeyListDto, ShijuInstalledAppDto,
};
/// SHIJU private-skill gateway transport contract.
pub use client::{
    ShijuInstalledSkillDto, ShijuMaintainedSkillQuery, ShijuMarketplaceSkillDto,
    ShijuSkillCursorPageDto, ShijuSkillDetailDto, ShijuSkillPageDto, ShijuSkillReadmeDto,
    ShijuSkillScanResultDto, ShijuSkillSearchQuery, ShijuSkillTagCatalogDto, ShijuSkillTagDto,
    ShijuSkillVersionContentDto, ShijuSkillVersionDto, ShijuSkillVisibilityDto,
    ShijuSkillVisibilityScopeDto, ShijuSystemSkillDto, ShijuSystemSkillQuery, ShijuTenantSkillDto,
};
/// Unified skill-market transport contract.
pub use client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketEnvelope,
    SkillMarketInstallScopeDto, SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto,
    SkillMarketSecurityDto, SkillMarketSkillDetailDto, SkillMarketSkillDto,
    SkillMarketSkillFileDto, SkillMarketSkillLocatorDto, SkillMarketSkillPageDto,
    SkillMarketSkillQuery, SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto,
    SkillMarketSkillSetPageDto, SkillMarketSort, SkillMarketSourceDto,
};
/// 网关认证参数与基础端点配置。
///
/// 用于描述请求应发送到哪个网关实例，以及需要附加哪些认证信息。
pub use endpoint::{GatewayAuth, GatewayEndpoint};
/// 判断错误文本是否表示网关当前不可达。
///
/// 调用方可在有本地或默认回退的读取路径中用它避免把预期离线状态记录为告警。
pub use http::is_gateway_unavailable_error;
/// 会话相关请求与响应类型。
///
/// 这些类型直接复用 `vw-api-types` 中的定义，避免调用方重复依赖多个 crate。
pub use session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageDeltaResponse, GatewaySessionMessageListQuery, GatewaySessionMessagePage,
    GatewaySessionMessagePageQuery, GatewaySessionPatchBody, GatewaySessionPatchTime,
    GatewaySessionResetBody, GatewaySessionScopeBody, GatewaySessionSummarizeBody,
    GatewaySessionTitleGenerateBody, GatewaySessionTitleGenerateResponse, GatewaySessionTodoItem,
    GatewaySessionTodoPutBody, GatewaySessionUiSummary, SessionMessagePageDirection,
};
#[allow(deprecated)]
/// 兼容旧命名的会话类型别名。
///
/// 仅用于平滑迁移旧调用点，新代码应优先使用 `Gateway*` 前缀类型。
pub use session::{
    SessionCreateBody, SessionDiffQuery, SessionForkBody, SessionMessageListQuery,
    SessionPatchBody, SessionPatchTime, SessionResetBody, SessionSummarizeBody, SessionTodoItem,
    SessionTodoPutBody,
};
#[allow(deprecated)]
/// 兼容旧命名的流式聊天类型别名。
///
/// 仅用于保留旧 API 兼容性。
pub use stream::{ChatStreamEvent, ChatStreamRequest};
/// 流式聊天请求与事件类型。
///
/// 对应网关 `/v1/chat/stream` 接口的请求体与事件枚举。
pub use stream::{
    GatewayChatStepFinishEvent, GatewayChatStepStartEvent, GatewayChatStreamEvent,
    GatewayChatStreamRequest, GatewayChatToolEvent, GatewayChatToolEventKind,
    GatewayChatToolStatus, GatewayChatUsage, GatewayJsonSseEvent, GatewayStreamAttemptResult,
    GatewayStreamEvent, GatewayStreamResume, GatewayTypedChatStreamEvent, LAST_EVENT_ID_HEADER,
    STREAM_REQUEST_ID_HEADER, normalize_chat_stream_event, normalize_chat_tool_event,
};
/// 对外透出共享的 API 类型定义。
///
/// 方便上层从单一依赖入口访问完整的接口类型集合。
pub use vw_api_types;
pub use vw_api_types::knowledge::{
    KnowledgeCapabilitiesDto, KnowledgeChunkDto, KnowledgeChunkType, KnowledgeChunkingMode,
    KnowledgeDatasetCreateRequest, KnowledgeDatasetDto, KnowledgeDocumentCreateRequest,
    KnowledgeDocumentDto, KnowledgeDocumentImportLimitsDto, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentImportResponse,
    KnowledgeDocumentImportTypeDto, KnowledgeDocumentReindexRequest, KnowledgeDocumentStatus,
    KnowledgeEmbeddingPresetDto, KnowledgeEmbeddingStatusDto, KnowledgeEmbeddingTestRequest,
    KnowledgeEmbeddingTestResponse, KnowledgeImportSupportDto, KnowledgeIndexJobDto,
    KnowledgeIndexJobStatus, KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode,
    KnowledgeRetrieveRequest, KnowledgeRetrieveResponse, KnowledgeRuntimeStatus,
};
pub use vw_api_types::project::{
    ListProjectChangeRecordsRequest, ListProjectChangeRecordsResponse, ProjectChangeRecordDto,
};
pub use vw_api_types::workflow::{
    WorkflowNodeRunDto, WorkflowNodeRunStatus, WorkflowRecord, WorkflowRecordDeleteResponse,
    WorkflowRecordSummary, WorkflowRecordUpsertBody, WorkflowRunRequest, WorkflowRunResponse,
    WorkflowRunStatus,
};

pub use client::{
    ShijuConnector, ShijuConnectorAuthorizationStatus, ShijuConnectorAuthorizeRequest,
    ShijuConnectorBinding, ShijuConnectorChallenge, ShijuConnectorPoll, ShijuConnectorPollRequest,
    ShijuConnectorSaveRequest, ShijuConnectorVariable,
};
