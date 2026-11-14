//! Account-scoped official quota observations; never queries or refreshes credentials.
use std::{collections::HashMap, sync::{Mutex, OnceLock}, time::{Duration, Instant}};
use axum::http::{HeaderMap, StatusCode};
use vw_api_types::apps_usage::{UsageCredentialState, UsageQueryData, UsageQueryOutcome, UsageQueryResult};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Account(pub vw_api_types::id::TenantScope, pub String, pub String, pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision { Unknown, Available, Exhausted }
struct Reading { at: Instant, windows: Vec<(f64, Option<i64>)> }
fn readings() -> &'static Mutex<HashMap<Account, Reading>> {
    static CACHE: OnceLock<Mutex<HashMap<Account, Reading>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}
fn store(account: Account, windows: Vec<(f64, Option<i64>)>) {
    let mut cache = readings().lock().unwrap_or_else(|e| e.into_inner());
    cache.retain(|_, reading| reading.at.elapsed() < Duration::from_secs(60));
    if cache.len() >= 4096 && !cache.contains_key(&account) {
        if let Some(oldest) = cache.iter().min_by_key(|(_, r)| r.at).map(|(k, _)| k.clone()) {
            cache.remove(&oldest);
        }
    }
    cache.insert(account, Reading { at: Instant::now(), windows });
}
pub(crate) fn decision(account: &Account) -> Decision {
    let cache = readings().lock().unwrap_or_else(|e| e.into_inner());
    let Some(reading) = cache.get(account).filter(|r| r.at.elapsed() < Duration::from_secs(60)) else {
        return Decision::Unknown;
    };
    let now = chrono::Utc::now().timestamp();
    let windows: Vec<_> = reading.windows.iter().filter(|(_, reset)| reset.is_none_or(|r| r > now)).collect();
    if windows.iter().any(|(used, _)| *used >= 100.0) { Decision::Exhausted }
    else if windows.is_empty() { Decision::Unknown } else { Decision::Available }
}
pub(crate) fn admit(account: &Account) -> Result<(), StatusCode> {
    match decision(account) {
        Decision::Exhausted => Err(StatusCode::TOO_MANY_REQUESTS),
        Decision::Unknown | Decision::Available => Ok(()),
    }
}
/// Only successful official Codex results are authoritative. Never use stale fallback data.
pub(crate) fn publish(account: Account, result: &UsageQueryResult) {
    if result.credential_state != UsageCredentialState::Valid { return; }
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } = &result.outcome else { return; };
    let windows = data.windows.iter().filter(|w| matches!(w.name.as_str(), "five_hour" | "seven_day"))
        .filter_map(|w| {
            let used = f64::from(w.utilization?);
            if !(0.0..=100.0).contains(&used) { return None; }
            let reset = match w.resets_at.as_deref() {
                Some(value) => Some(chrono::DateTime::parse_from_rfc3339(value).ok()?.timestamp()),
                None => None,
            };
            Some((used, reset))
        }).collect();
    store(account, windows);
}
/// Called before interpreting success/error bodies, including 401 and 429 responses.
pub(crate) fn observe(account: Account, headers: &HeaderMap) {
    let text = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let active = text("x-codex-active-limit").filter(|s| !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
    let mut windows = Vec::new();
    for window in ["primary", "secondary"] {
        let named = active.map(|s| format!("x-{s}-{window}"));
        let base = format!("x-codex-{window}");
        let prefix = named.as_deref().filter(|p| text(&format!("{p}-used-percent")).is_some()).unwrap_or(&base);
        let Some(used) = text(&format!("{prefix}-used-percent")).and_then(|s| s.parse::<f64>().ok()).filter(|v| v.is_finite() && (0.0..=100.0).contains(v)) else { continue; };
        if !matches!(text(&format!("{prefix}-window-minutes")), Some("300" | "10080")) { continue; }
        let reset = text(&format!("{prefix}-reset-at")).and_then(|s| s.parse::<i64>().ok())
            .or_else(|| text(&format!("{prefix}-reset-after-seconds")).and_then(|s| s.parse::<i64>().ok()).filter(|v| *v >= 0).and_then(|v| chrono::Utc::now().timestamp().checked_add(v)));
        windows.push((used, reset));
    }
    // Missing or malformed headers cannot erase a known exhausted reading.
    if !windows.is_empty() { store(account, windows); }
}
