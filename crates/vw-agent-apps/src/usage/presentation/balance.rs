use reqwest::Url;
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryError, UsageQueryOutcome, UsageScriptPackage,
    },
};

use super::tiers::QuotaTone;
use crate::usage::cache::UsageCacheResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceLine {
    pub remaining: UsageNumber,
    pub bar_percent: Option<f64>,
    pub tone: QuotaTone,
}

/// Keep the signed amount. Only a known, positive total defines a progress bar.
pub fn balance_line(
    remaining: Option<UsageNumber>,
    total: Option<UsageNumber>,
) -> Option<BalanceLine> {
    let remaining = remaining?;
    let amount = f64::from(remaining);
    let bar_percent = total
        .map(f64::from)
        .filter(|total| *total > 0.0)
        .map(|total| (amount / total).clamp(0.0, 1.0) * 100.0);
    let tone = if amount <= 0.0 { QuotaTone::Danger } else { QuotaTone::Normal };
    Some(BalanceLine { remaining, bar_percent, tone })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptPackageStatus<'a> {
    Available(Option<BalanceLine>),
    Expired { reason: Option<&'a str> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptPackageLine<'a> {
    pub package: &'a UsageScriptPackage,
    pub status: ScriptPackageStatus<'a>,
}

/// Keep the reported status of each package, including mixed success and expiry.
pub fn script_package_lines(packages: &[UsageScriptPackage]) -> Vec<ScriptPackageLine<'_>> {
    packages
        .iter()
        .map(|package| {
            let status = if package.is_valid == Some(false) {
                let reason = package.invalid_message.as_deref().map(str::trim);
                ScriptPackageStatus::Expired { reason: reason.filter(|reason| !reason.is_empty()) }
            } else {
                ScriptPackageStatus::Available(balance_line(package.remaining, package.total))
            };
            ScriptPackageLine { package, status }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryStatusLine<'a> {
    /// Query time of the displayed snapshot, including the original time for stale data.
    pub queried_at: Option<TimestampMs>,
    pub stale: bool,
    pub failure: Option<&'a UsageQueryError>,
    pub credential_state: UsageCredentialState,
    pub credential_message: Option<&'a str>,
}

/// Keep refresh failure and credential status visible even when displaying a cached success.
pub fn query_status(result: &UsageCacheResult) -> QueryStatusLine<'_> {
    let (displayed, latest, stale) = match result {
        UsageCacheResult::Current(current) => (current, current, false),
        UsageCacheResult::Stale { last_success, failure } => (last_success.as_ref(), failure, true),
    };
    let failure = match &latest.outcome {
        UsageQueryOutcome::Success { .. } => None,
        UsageQueryOutcome::Failure { error, .. } => Some(error),
    };
    QueryStatusLine {
        queried_at: displayed.queried_at,
        stale,
        failure,
        credential_state: latest.credential_state,
        credential_message: latest.credential_message.as_deref(),
    }
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
