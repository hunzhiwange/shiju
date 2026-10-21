//! 会话列表、详情与网关会话操作相关类型。
//!
//! 本模块承载“会话”这一顶层业务域的协议结构，覆盖：
//! - 会话摘要与完整详情
//! - 创建、更新、分叉、归档、标题生成等请求响应
//! - 会话绑定的工作树、分支、活跃请求与 token 用量
//! - 与兼容网关交互时使用的请求体和查询参数
//!
//! # 主要类型
//!
//! - [`SessionSummaryDto`][]: 用于列表视图的会话摘要
//! - [`SessionDto`][]: 用于详情页或状态同步的完整会话对象
//! - [`CreateSessionRequest`][]: 创建新会话的标准请求
//! - [`UpdateSessionRequest`][]: 局部更新会话属性的请求
//! - [`ListSessionsResponse`][]: 标准分页列表响应

use crate::common::{JsonMap, PaginatedResponse, TimestampMs};
use crate::id::{MessageId, ProjectId, RequestId, SessionId, WorktreeId};
use crate::todo::{TodoPriority, TodoStatus};
use serde::{Deserialize, Serialize};

/// 会话当前所处状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 当前没有执行中的请求。
    Idle,
    /// 当前有请求正在进行。
    Running,
    /// 最近一次关键流程以错误结束。
    Error,
    /// 会话已归档，不再作为活跃会话展示。
    Archived,
}

/// 会话累计的模型用量信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionUsageDto {
    /// 输入 token 数量。
    pub input_tokens: u64,
    /// 输出 token 数量。
    pub output_tokens: u64,
    /// 推理阶段 token 数量。
    #[serde(default)]
    pub reasoning_tokens: u64,
    /// 总 token 数量。
    pub total_tokens: u64,
    /// 可选的美元成本估算。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// 会话当前活跃请求的摘要信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveRequestDto {
    /// 请求 ID。
    pub request_id: RequestId,
    /// 请求状态字符串。
    pub status: String,
    /// 请求启动时间。
    pub started_at_ms: TimestampMs,
}

/// 会话附加元数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionMetadataDto {
    /// 会话关联分支名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// 会话绑定的工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 额外透传字段。
    #[serde(flatten)]
    pub extra: JsonMap,
}

/// 会话列表项摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummaryDto {
    /// 会话 ID。
    pub id: SessionId,
    /// 关联项目 ID；会话可能独立于项目存在。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 会话标题。
    pub title: String,
    /// 会话状态。
    pub status: SessionStatus,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 最后更新时间。
    pub updated_at_ms: TimestampMs,
    /// 最近一条消息时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_message_at_ms: Option<TimestampMs>,
    /// 消息总数。
    pub message_count: u32,
    /// 是否置顶。
    #[serde(default)]
    pub pinned: bool,
    /// 是否已归档。
    #[serde(default)]
    pub archived: bool,
}

/// 会话完整详情。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDto {
    /// 会话 ID。
    pub id: SessionId,
    /// 关联项目 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 会话标题。
    pub title: String,
    /// 会话状态。
    pub status: SessionStatus,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 更新时间。
    pub updated_at_ms: TimestampMs,
    /// 最近一条消息时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_message_at_ms: Option<TimestampMs>,
    /// 消息总数。
    pub message_count: u32,
    /// 会话摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 用量统计。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<SessionUsageDto>,
    /// 当前活跃请求。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_request: Option<ActiveRequestDto>,
    /// 扩展元数据。
    #[serde(default)]
    pub metadata: SessionMetadataDto,
}

/// 查询会话列表的请求参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListSessionsRequest {
    /// 按项目过滤。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 按工作目录过滤。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// 按状态过滤。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<SessionStatus>,
    /// 分页游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 分页大小。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// 会话列表响应。
pub type ListSessionsResponse = PaginatedResponse<SessionSummaryDto>;

/// 创建会话请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreateSessionRequest {
    /// 关联项目 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 初始标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 初始用户消息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_message: Option<String>,
    /// 附加元数据。
    #[serde(default)]
    pub metadata: SessionMetadataDto,
}

/// 网关创建会话请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionCreateBody {
    /// 父会话或父消息链标识。
    #[serde(rename = "parentID", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// 可选标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// 创建会话响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    /// 新建后的会话详情。
    pub session: SessionDto,
}

/// 更新会话请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UpdateSessionRequest {
    /// 更新标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 更新置顶状态。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    /// 更新归档状态。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    /// 更新元数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SessionMetadataDto>,
}

/// 网关更新会话请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionPatchBody {
    /// 更新标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 归档等时间相关变更。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<GatewaySessionPatchTime>,
}

/// 网关会话时间字段补丁。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionPatchTime {
    /// 归档时间戳；为空表示不更新。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<u64>,
}

/// 网关分叉会话请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionForkBody {
    /// 作为分叉起点的消息 ID。
    #[serde(rename = "messageID", default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

/// 网关重置会话请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewaySessionResetBody {
    /// 重置到指定消息。
    #[serde(rename = "messageID")]
    pub message_id: String,
    /// 是否回滚生成的代码变更。
    #[serde(rename = "revertCode")]
    pub revert_code: bool,
}

/// 网关触发摘要生成请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewaySessionSummarizeBody {
    /// 从该消息开始生成摘要。
    #[serde(rename = "messageID")]
    pub message_id: String,
}

/// 网关侧待办项格式。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewaySessionTodoItem {
    /// 待办 ID。
    pub id: String,
    /// 待办内容。
    pub content: String,
    /// 待办状态。
    pub status: TodoStatus,
    /// 待办优先级。
    pub priority: TodoPriority,
}

/// 批量覆盖会话待办列表的请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionTodoPutBody {
    /// 完整待办列表。
    #[serde(default)]
    pub todos: Vec<GatewaySessionTodoItem>,
}

/// 查询会话 diff 的参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionDiffQuery {
    /// 限定目录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// 指定消息对应的变更点。
    #[serde(rename = "messageID", default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

/// 查询会话消息列表的参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionMessageListQuery {
    /// 限定目录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// 返回消息数量上限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// 不透明分页游标；省略时读取最近一页。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// UI 消息页的读取方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionMessagePageDirection {
    /// 从当前边界向更旧消息读取。
    #[default]
    Older,
}

/// 查询有界 UI 消息页的参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GatewaySessionMessagePageQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub direction: SessionMessagePageDirection,
}

/// 不包含消息、调用和步骤正文的 UI 会话摘要。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewaySessionUiSummary {
    pub id: String,
    pub title: String,
    pub updated_ms: u64,
    pub message_count: usize,
    pub call_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_content: Option<String>,
}

/// 有界消息页响应；消息记录类型由使用协议的 crate 提供。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewaySessionMessagePage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_cursor: Option<String>,
    pub total_count: usize,
}

/// 增量写入结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewaySessionMessageDeltaResponse {
    pub upserted_rows: usize,
    pub truncated_rows: usize,
}

/// 从现有会话分叉创建新会话的请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ForkSessionRequest {
    /// 新会话标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 分叉起点消息 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_from_message_id: Option<MessageId>,
}

/// 分叉会话响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkSessionResponse {
    /// 新创建的会话。
    pub session: SessionDto,
}

/// 获取会话消息列表请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GetSessionMessagesRequest {
    /// 分页游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 单页消息数量。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// 网关保存会话 UI 状态请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionUiSaveBody {
    /// 序列化后的 UI 状态内容。
    pub session: String,
}

/// 网关批量归档请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionArchivedBody {
    /// 需要归档的会话 ID 列表。
    pub ids: Vec<String>,
}

/// 网关会话作用域请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewaySessionScopeBody {
    /// 会话作用域字符串。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// 网关生成会话标题请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewaySessionTitleGenerateBody {
    /// 用于生成标题的上下文内容。
    pub content: String,
    /// 优先使用的模型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<String>,
    /// 优先使用的 ACP 代理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
}

/// 网关生成标题响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewaySessionTitleGenerateResponse {
    /// 生成出的标题。
    pub title: String,
}
