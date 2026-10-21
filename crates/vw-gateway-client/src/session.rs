/// 直接透出网关会话相关类型，避免调用方重复依赖 vw-api-types。
pub use vw_api_types::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageDeltaResponse, GatewaySessionMessageListQuery, GatewaySessionMessagePage,
    GatewaySessionMessagePageQuery, GatewaySessionPatchBody, GatewaySessionPatchTime,
    GatewaySessionResetBody, GatewaySessionScopeBody, GatewaySessionSummarizeBody,
    GatewaySessionTitleGenerateBody, GatewaySessionTitleGenerateResponse, GatewaySessionTodoItem,
    GatewaySessionTodoPutBody, GatewaySessionUiSummary, SessionMessagePageDirection,
};

#[deprecated(note = "use GatewaySessionCreateBody")]
/// 兼容旧命名的会话创建请求体。
pub type SessionCreateBody = GatewaySessionCreateBody;
#[deprecated(note = "use GatewaySessionDiffQuery")]
/// 兼容旧命名的会话 diff 查询参数。
pub type SessionDiffQuery = GatewaySessionDiffQuery;
#[deprecated(note = "use GatewaySessionForkBody")]
/// 兼容旧命名的会话分叉请求体。
pub type SessionForkBody = GatewaySessionForkBody;
#[deprecated(note = "use GatewaySessionMessageListQuery")]
/// 兼容旧命名的消息列表查询参数。
pub type SessionMessageListQuery = GatewaySessionMessageListQuery;
#[deprecated(note = "use GatewaySessionPatchBody")]
/// 兼容旧命名的会话更新请求体。
pub type SessionPatchBody = GatewaySessionPatchBody;
#[deprecated(note = "use GatewaySessionPatchTime")]
/// 兼容旧命名的会话时间字段。
pub type SessionPatchTime = GatewaySessionPatchTime;
#[deprecated(note = "use GatewaySessionResetBody")]
/// 兼容旧命名的会话重置请求体。
pub type SessionResetBody = GatewaySessionResetBody;
#[deprecated(note = "use GatewaySessionTodoItem")]
/// 兼容旧命名的待办项结构。
pub type SessionTodoItem = GatewaySessionTodoItem;
#[deprecated(note = "use GatewaySessionSummarizeBody")]
/// 兼容旧命名的摘要请求体。
pub type SessionSummarizeBody = GatewaySessionSummarizeBody;
#[deprecated(note = "use GatewaySessionTodoPutBody")]
/// 兼容旧命名的待办更新请求体。
pub type SessionTodoPutBody = GatewaySessionTodoPutBody;

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
