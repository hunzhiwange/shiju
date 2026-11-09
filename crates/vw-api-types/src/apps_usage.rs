//! 应用余额、额度和套餐的脱敏查询结果。

use crate::TimestampMs;
use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

pub mod presentation;

/// 查询值允许负数和零；缺失值由外层 Option 表达。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct UsageNumber(f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonFiniteUsageNumber;

impl fmt::Display for NonFiniteUsageNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("usage number must be finite")
    }
}

impl std::error::Error for NonFiniteUsageNumber {}

impl TryFrom<f64> for UsageNumber {
    type Error = NonFiniteUsageNumber;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() { Ok(Self(value)) } else { Err(NonFiniteUsageNumber) }
    }
}

impl From<UsageNumber> for f64 {
    fn from(value: UsageNumber) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCredentialState {
    Valid,
    Expired,
    RefreshPending,
    NotFound,
    ParseError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageQueryKind {
    OfficialSubscription,
    Balance,
    CodingPlan,
    CustomScript,
}

/// Select one authorized usage resource. The tenant and principal come from the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageSnapshotQuery {
    pub project_id: crate::id::ProjectId,
    pub target_host_id: String,
    pub app_id: crate::agent_apps::AppId,
    pub provider_id: String,
    pub account_id: Option<String>,
    pub kind: Option<UsageQueryKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshotResponse {
    pub capabilities: Vec<UsageQueryKind>,
    pub snapshot: Option<UsageSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub result: UsageQueryResult,
    pub stale: bool,
    pub refresh_failure: Option<UsageQueryResult>,
}

/// 结果来源及其脱敏标识，由执行端根据实际适配器或数据源填写。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum UsageSource {
    Official(String),
    Provider(String),
    Script(String),
    Estimate(String),
    Billing(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQueryResult {
    pub source: UsageSource,
    /// 执行查询的 Unix 毫秒时间。未执行查询时为空。
    pub queried_at: Option<TimestampMs>,
    pub credential_state: UsageCredentialState,
    /// 安全的凭据状态说明。
    pub credential_message: Option<String>,
    pub outcome: UsageQueryOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UsageQueryOutcome {
    Success { data: UsageQueryData },
    Failure { kind: UsageQueryKind, error: UsageQueryError },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UsageQueryData {
    OfficialSubscription(UsageSubscription),
    Balance(Vec<UsageWallet>),
    CodingPlan(Vec<UsageQuotaWindow>),
    CustomScript(Vec<UsageScriptPackage>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSubscription {
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
    pub windows: Vec<UsageQuotaWindow>,
    pub extra_usage: Option<UsageExtraUsage>,
    /// 供应商报告的 Credits 数量。
    pub credits_balance: Option<UsageNumber>,
    /// 每项代表一次可用重置。保留顺序，空项表示永不过期。
    /// 外层为空表示未查到，空列表表示已查到但没有可用次数。
    pub reset_credits: Option<Vec<Option<String>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQuotaWindow {
    pub name: String,
    pub plan_label: Option<String>,
    /// 已用百分比，保留上游报告的有限值。
    pub utilization: Option<UsageNumber>,
    pub used: Option<UsageNumber>,
    pub total: Option<UsageNumber>,
    pub remaining: Option<UsageNumber>,
    /// 已用、总额及剩余的单位，例如 USD、tokens 或 requests。
    pub unit: Option<String>,
    /// ISO 8601 重置时间。
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageExtraUsage {
    pub is_enabled: bool,
    pub monthly_limit: Option<UsageNumber>,
    pub used_credits: Option<UsageNumber>,
    pub utilization: Option<UsageNumber>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWallet {
    pub currency: String,
    pub remaining: Option<UsageNumber>,
    pub total: Option<UsageNumber>,
    pub used: Option<UsageNumber>,
    pub granted: Option<UsageNumber>,
    pub topped_up: Option<UsageNumber>,
    /// Public wallet details page supplied by the native adapter. Never includes credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageScriptPackage {
    pub plan_name: Option<String>,
    /// 脚本输出经校验和脱敏后的附加说明。
    pub extra: Option<String>,
    pub is_valid: Option<bool>,
    pub invalid_message: Option<String>,
    pub total: Option<UsageNumber>,
    pub used: Option<UsageNumber>,
    pub remaining: Option<UsageNumber>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQueryError {
    pub code: UsageQueryErrorCode,
    /// 对外安全说明，不得包含凭据或上游原始响应。
    pub message: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageQueryErrorCode {
    CredentialUnavailable,
    MissingConfiguration,
    Unsupported,
    Timeout,
    Transport,
    UpstreamUnauthorized,
    UpstreamRateLimited,
    UpstreamUnavailable,
    UpstreamRejected,
    InvalidResponse,
}

/// Validate a recharge or wallet details link before offering it to the user.
pub fn recharge_detail_url(raw: &str) -> Option<Url> {
    let (scheme, location) = raw.split_once("://")?;
    let authority = location.split(['/', '?', '#']).next()?;
    if !scheme.eq_ignore_ascii_case("https")
        || authority.is_empty()
        || authority.contains('@')
        || raw.contains('\\')
        || raw.chars().any(|character| character.is_control() || character.is_whitespace())
    {
        return None;
    }
    Url::parse(raw).ok().filter(|url| url.host_str().is_some())
}
