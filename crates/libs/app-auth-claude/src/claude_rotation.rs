//! Claude credential generation and expiration rules.
use app_config_write::write_guard::WriteError;
use serde_json::Value;
use vw_api_transport::TimestampMs;

pub fn check_generation(
    before: Option<&Value>,
    incoming: Option<&Value>,
) -> Result<(), WriteError> {
    if before == incoming {
        return Ok(());
    }
    let incoming_expiry = expiry(incoming.and_then(|oauth| oauth.get("expiresAt")))?;
    let Some(before_expiry) = expiry(before.and_then(|oauth| oauth.get("expiresAt")))? else {
        // Older Claude snapshots may lack a generation timestamp. The locked
        // default slot remains authoritative for these same-owner snapshots.
        return Ok(());
    };
    let incoming_expiry = incoming_expiry.ok_or(WriteError::Conflict)?;
    let token_changed = ["accessToken", "refreshToken"].into_iter().any(|key| {
        before.and_then(|oauth| oauth.get(key)) != incoming.and_then(|oauth| oauth.get(key))
    });
    if incoming_expiry < before_expiry || (incoming_expiry == before_expiry && token_changed) {
        return Err(WriteError::Conflict);
    }
    Ok(())
}

pub fn expiry(value: Option<&Value>) -> Result<Option<TimestampMs>, WriteError> {
    let Some(value) = value else { return Ok(None) };
    let at = value
        .as_i64()
        .or_else(|| {
            value.as_f64().filter(|&at| at >= 1.0 && at < i64::MAX as f64).map(|at| at as i64)
        })
        .filter(|&at| at > 0)
        .ok_or(WriteError::InvalidResource)?;
    Ok(Some(TimestampMs(if at < 100_000_000_000 { at * 1000 } else { at })))
}
