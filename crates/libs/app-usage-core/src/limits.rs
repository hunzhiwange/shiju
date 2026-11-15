//! Quota and token limit rules.
use vw_api_usage::{UsageInputSemantics, UsageReasoningSemantics, UsageTokens};
use vw_api_apps_usage::apps_usage::UsageQuotaWindow;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitState {
    Unknown,
    Available,
    Exhausted,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitWindow {
    pub name: String,
    pub unit: String,
    pub used: Option<String>,
    pub limit: Option<String>,
    pub resets_at: Option<String>,
    pub state: LimitState,
}

/// Period boundaries resolved by the service in the configured calendar timezone.
/// This is trusted provider configuration, never a client-supplied usage count.
pub struct LedgerLimitPeriod {
    pub name: String,
    pub start_unix_seconds: i64,
    pub reset_unix_seconds: i64,
    pub usd: Option<Decimal>,
    pub tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderLimitStatus {
    pub provider_id: String,
    pub state: LimitState,
    pub windows: Vec<LimitWindow>,
}

pub fn combine(states: impl Iterator<Item = LimitState>) -> LimitState {
    let states: Vec<_> = states.collect();
    if states.contains(&LimitState::Exhausted) {
        LimitState::Exhausted
    } else if states.contains(&LimitState::Failed) {
        LimitState::Failed
    } else if states.is_empty() || states.contains(&LimitState::Unknown) {
        LimitState::Unknown
    } else {
        LimitState::Available
    }
}

pub fn quota_window(window: &UsageQuotaWindow, now: i64) -> LimitWindow {
    let current = match window.resets_at.as_deref() {
        None => true,
        Some(reset) => {
            chrono::DateTime::parse_from_rfc3339(reset).is_ok_and(|reset| reset.timestamp() > now)
        }
    };
    let remaining = window.remaining.map(f64::from).or_else(|| {
        window.total.zip(window.used).map(|(total, used)| f64::from(total) - f64::from(used))
    });
    let state = if !current {
        LimitState::Unknown
    } else if remaining.is_some_and(|v| v <= 0.0)
        || window.utilization.is_some_and(|v| f64::from(v) >= 100.0)
    {
        LimitState::Exhausted
    } else if remaining.is_some_and(|v| v > 0.0)
        || window.utilization.is_some_and(|v| (0.0..100.0).contains(&f64::from(v)))
    {
        LimitState::Available
    } else {
        LimitState::Unknown
    };
    LimitWindow {
        name: window.name.clone(),
        unit: window.unit.clone().unwrap_or_default(),
        used: window.used.map(|v| f64::from(v).to_string()),
        limit: window.total.map(|v| f64::from(v).to_string()),
        resets_at: window.resets_at.clone(),
        state,
    }
}

pub fn total_tokens(tokens: &UsageTokens) -> Option<u64> {
    let input = match tokens.input_semantics {
        UsageInputSemantics::Total => tokens.input?,
        UsageInputSemantics::Fresh => {
            tokens.input?.checked_add(tokens.cache_read?)?.checked_add(tokens.cache_creation?)?
        }
        UsageInputSemantics::Unknown => return None,
    };
    let output = match tokens.reasoning_semantics {
        UsageReasoningSemantics::IncludedInOutput => tokens.output?,
        UsageReasoningSemantics::SeparateFromOutput => {
            tokens.output?.checked_add(tokens.reasoning?)?
        }
        UsageReasoningSemantics::Unknown => return None,
    };
    input.checked_add(output)
}
