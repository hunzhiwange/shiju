//! Pure generation comparison for already validated Codex credential bundles.
use crate::identity::{parse_codex_jwt_claims, token_subject, workspace_id};
use serde_json::Value;

/// Both bundles have already passed the private credential file schema check.
pub fn should_adopt_cli_rotation(managed: &Value, live: &Value) -> Result<bool, String> {
    let last_refresh = |auth: &Value| {
        let date = auth["last_refresh"].as_str()?;
        let millis = chrono::DateTime::parse_from_rfc3339(date).ok()?.timestamp_millis();
        (millis > 0).then_some(millis)
    };
    let old_date = last_refresh(managed);
    let live_date = last_refresh(live);
    let same_tokens = managed["tokens"] == live["tokens"];
    // Unchanged material can establish a date for a legacy bundle.
    if same_tokens && live_date <= old_date {
        return Ok(false);
    }

    let identity = |auth: &Value| {
        let tokens = &auth["tokens"];
        let workspace = tokens["account_id"].as_str()?;
        let claims = parse_codex_jwt_claims(tokens["id_token"].as_str()?)?;
        let subject = token_subject(&claims)?;
        let access_claims = parse_codex_jwt_claims(tokens["access_token"].as_str()?);
        if workspace_id(&claims).is_some_and(|id| id != workspace)
            || access_claims.as_ref().and_then(workspace_id).is_some_and(|id| id != workspace)
        {
            return None;
        }
        Some((workspace.to_owned(), subject.to_owned()))
    };
    if !identity(managed).is_some_and(|expected| identity(live).as_ref() == Some(&expected)) {
        return Err("rotated official account identity changed".into());
    }
    if !same_tokens && (old_date.is_none() || live_date <= old_date) {
        return Err("rotated official account generation is stale or ambiguous".into());
    }
    Ok(true)
}
