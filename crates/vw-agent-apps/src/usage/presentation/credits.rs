use super::tiers::QuotaTone;
use chrono::{DateTime, NaiveDate, TimeDelta, TimeZone, Utc};
use std::collections::BTreeMap;
use vw_api_types::apps_usage::UsageNumber;

const RESET_EXPIRING_SOON: TimeDelta = TimeDelta::days(3);
// Display conversion from the upstream quota rules.
const CODEX_USD_PER_CREDIT: f64 = 0.04;

/// Approximate USD equivalent of Codex Credits for display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CreditsUsdEstimate {
    UnderOneDollar,
    RoundedDollars(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreditsLine {
    pub balance: UsageNumber,
    pub usd_estimate: CreditsUsdEstimate,
}

pub fn credits_line(balance: Option<UsageNumber>) -> Option<CreditsLine> {
    let balance = balance.filter(|balance| f64::from(*balance) > 0.0)?;
    let dollars = (f64::from(balance) * CODEX_USD_PER_CREDIT).round();
    let usd_estimate = if dollars < 1.0 {
        CreditsUsdEstimate::UnderOneDollar
    } else {
        CreditsUsdEstimate::RoundedDollars(dollars)
    };
    Some(CreditsLine { balance, usd_estimate })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetCreditsLine {
    pub count: usize,
    pub tone: QuotaTone,
    /// Earliest groups first, with the undated group last.
    pub groups: Vec<ResetCreditGroup>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetCreditGroup {
    pub expiry_day: Option<NaiveDate>,
    pub earliest_expiry: Option<DateTime<Utc>>,
    pub count: usize,
    pub tone: QuotaTone,
}

/// Recheck cached credits against `now` and group them in its display time zone.
/// Missing or unparseable expiry values are kept as undated credits.
pub fn reset_credits_line<Tz: TimeZone>(
    expiries: Option<&[Option<String>]>,
    now: &DateTime<Tz>,
) -> Option<ResetCreditsLine> {
    let zone = now.timezone();
    let mut groups = BTreeMap::new();
    for expiry in expiries? {
        let expiry = expiry
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|at| at.to_utc());
        if expiry.is_some_and(|at| at <= *now) {
            continue;
        }

        let expiry_day = expiry.map(|at| at.with_timezone(&zone).date_naive());
        let group = groups.entry(expiry_day).or_insert(ResetCreditGroup {
            expiry_day,
            earliest_expiry: expiry,
            count: 0,
            tone: QuotaTone::Normal,
        });
        group.count += 1;
        group.earliest_expiry = group.earliest_expiry.min(expiry);
        if expiry.is_some_and(|at| at.signed_duration_since(now) < RESET_EXPIRING_SOON) {
            group.tone = QuotaTone::Warning;
        }
    }

    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by_key(|group| (group.earliest_expiry.is_none(), group.earliest_expiry));
    Some(ResetCreditsLine {
        count: groups.iter().map(|group| group.count).sum(),
        tone: groups.first()?.tone,
        groups,
    })
}
