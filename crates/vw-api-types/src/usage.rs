//! 统一请求账本协议；不同于 apps_usage 的余额与套餐查询。
//!
//! DTO 不包含原始日志正文、凭据或可打开的文件路径。身份与访问范围由服务端绑定。

use serde::{Deserialize, Serialize};

/// UTC Unix 秒；与通用 TimestampMs 明确区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UsageUnixSeconds(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageProvider {
    Unknown,
    Known { provider_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSourceKind {
    Native,
    Relay,
    Shiju,
}

/// 不透明来源关联 ID；实际主机、scope 和文件身份只保存在服务端。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSourceRef {
    pub source_id: String,
    pub kind: UsageSourceKind,
    pub app: String,
    pub native_request_id: Option<String>,
    pub native_session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageInputSemantics {
    /// 输入值不含缓存读取与创建。
    Fresh,
    /// 输入值包含缓存读取与创建，不得再次相加。
    Total,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageReasoningSemantics {
    IncludedInOutput,
    SeparateFromOutput,
    Unknown,
}

/// 空值表示来源未提供，不合成零。cache_creation_1h 是 creation 的子集。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTokens {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_creation: Option<u64>,
    pub cache_creation_1h: Option<u64>,
    pub input_semantics: UsageInputSemantics,
    pub reasoning_semantics: UsageReasoningSemantics,
}

/// 金额为十进制字符串；未知金额为 null，已知零为字符串 "0"。
/// 账本金额均为 USD；不使用浮点数传输费用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCost {
    pub source: UsageCostSource,
    /// source 为 Unknown 时必须为空；服务端校验来源与金额一致性。
    pub amount_usd: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCostSource {
    Unknown,
    Native,
    Calculated,
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageModels {
    pub request_model: Option<String>,
    pub response_model: Option<String>,
    pub pricing_model: Option<String>,
    /// 十进制倍率；无证据不默认成 1。
    pub pricing_multiplier: Option<String>,
    pub multiplier_basis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageMeasurementKind {
    Measured,
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageMeasurement {
    pub value: f64,
    pub kind: UsageMeasurementKind,
}

/// 每个指标单独标记实测/估算；缺少指标时为 null。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsagePerformance {
    pub first_token_ms: Option<UsageMeasurement>,
    pub latency_ms: Option<UsageMeasurement>,
    pub output_tokens_per_second: Option<UsageMeasurement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageRequestStatus {
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub started_at_unix_seconds: UsageUnixSeconds,
    pub provider: UsageProvider,
    pub sources: Vec<UsageSourceRef>,
    pub models: UsageModels,
    pub status: UsageRequestStatus,
    /// 无 usage 的失败请求仍可记录，但不能因此计为计费请求。
    pub billable: bool,
    pub tokens: Option<UsageTokens>,
    pub cost: UsageCost,
    pub performance: UsagePerformance,
    /// 无稳定关联时仅提示疑似重复，不直接丢弃或合并。
    pub suspected_duplicate_of: Option<String>,
}

/// 范围选择不携带 tenant/project/host；只能查询认证上下文已授权的账本。
/// 拒绝未知字段，避免调用方误以为传入身份字段能改变查询范围。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageRange {
    All,
    /// 半开区间 [start, end)。服务端校验 start < end。
    Absolute {
        start_unix_seconds: UsageUnixSeconds,
        end_unix_seconds: UsageUnixSeconds,
    },
    /// 自定义半开区间；缺省起点为当前时刻前 24h，live_end_time 时忽略固定终点。
    Custom {
        start_unix_seconds: Option<UsageUnixSeconds>,
        end_unix_seconds: Option<UsageUnixSeconds>,
        live_end_time: bool,
    },
    /// 每次请求重新计算的持续时间，必须大于零。
    Rolling { seconds: u64 },
    /// IANA 时区下的日历范围，不能用固定 24h 相减。
    Calendar {
        period: UsageCalendarPeriod,
        time_zone: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCalendarPeriod {
    Today,
    /// 含今天在内的当地日历日，每次查询重新计算。
    Last7Days,
    Last14Days,
    Last30Days,
    Yesterday,
    ThisWeek,
    ThisMonth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageResolvedRange {
    pub start_unix_seconds: Option<UsageUnixSeconds>,
    pub end_unix_seconds: UsageUnixSeconds,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct UsageFilter {
    pub app: Option<String>,
    pub source_kind: Option<UsageSourceKind>,
    pub provider: Option<UsageProvider>,
    pub request_model: Option<String>,
    pub response_model: Option<String>,
    pub pricing_model: Option<String>,
    pub native_session_id: Option<String>,
    pub status: Option<UsageRequestStatus>,
    pub billable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageQuery {
    pub range: UsageRange,
    pub filter: UsageFilter,
    /// 不透明游标，服务端须绑定授权范围及过滤条件。
    pub cursor: Option<String>,
    /// 服务端校验并限制单页大小。
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsagePage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub resolved_range: UsageResolvedRange,
    pub revision: u64,
}

/// 保持总请求与计费请求计数独立，未知费用不折算为零。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRequestCounts {
    pub total_requests: u64,
    pub billable_requests: u64,
    pub unknown_cost_requests: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageErrorCode {
    InvalidParameters,
    Unauthenticated,
    Forbidden,
    NotFound,
    RevisionConflict,
    SourceUnavailable,
}

/// 消息仅包含可公开说明，不得回传日志正文、凭据或主机路径。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageError {
    pub code: UsageErrorCode,
    pub message: String,
}
