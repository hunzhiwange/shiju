//! Address-book operations on the existing provider record; no network or live writes.
use vw_api_types::app_providers::endpoints::{
    AppProviderEndpoint, AppProviderEndpointChange, AppProviderEndpointKind,
};

fn normalize(raw: &str, kind: AppProviderEndpointKind) -> Result<String, String> {
    let value = raw.trim();
    super::validation::endpoint(value)?;
    let url = reqwest::Url::parse(value).map_err(|_| "invalid application provider endpoint")?;
    match kind {
        AppProviderEndpointKind::BaseUrl => Ok(value.trim_end_matches('/').into()),
        AppProviderEndpointKind::FullUrl => {
            if url.path().trim_matches('/').is_empty() {
                return Err("full application provider endpoint requires a request path".into());
            }
            // Request paths are not guessed, appended or stripped, including the final slash.
            Ok(value.into())
        }
    }
}

pub fn read(stored: &serde_json::Value) -> Result<Vec<AppProviderEndpoint>, String> {
    if !stored.is_object() {
        return Err("invalid stored application provider profile".into());
    }
    let mut rows: Vec<AppProviderEndpoint> = match stored.get("customEndpoints") {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|_| "invalid stored application provider profile")?,
    };
    if rows.len() > 64 {
        return Err("invalid stored application provider profile".into());
    }
    for (index, row) in rows.iter().enumerate() {
        if normalize(&row.url, row.kind).as_deref() != Ok(row.url.as_str())
            || rows[..index].iter().any(|other| other.url == row.url && other.kind == row.kind)
        {
            return Err("invalid stored application provider profile".into());
        }
    }
    rows.sort_by(|a, b| b.added_at.cmp(&a.added_at).then_with(|| a.url.cmp(&b.url)));
    Ok(rows)
}

pub fn update(
    stored: &serde_json::Value,
    request: &AppProviderEndpointChange,
    now: u64,
) -> Result<serde_json::Value, String> {
    use AppProviderEndpointChange::*;
    let (raw, kind) = match request {
        Add { url, kind } | Remove { url, kind } | MarkUsed { url, kind } => (url, *kind),
    };
    let url = normalize(raw, kind)?;
    let mut rows = read(stored)?;
    let position = rows.iter().position(|row| row.url == url && row.kind == kind);
    match request {
        Add { .. } => {
            if position.is_some() {
                return Ok(stored.clone());
            }
            if rows.len() >= 64 {
                return Err("too many application provider endpoints".into());
            }
            rows.push(AppProviderEndpoint { url, kind, added_at: now, last_used: None });
        }
        Remove { .. } => {
            let Some(index) = position else {
                return Ok(stored.clone());
            };
            rows.remove(index);
        }
        MarkUsed { .. } => {
            let index = position.ok_or("application provider endpoint not found")?;
            rows[index].last_used =
                Some(now.max(rows[index].added_at).max(rows[index].last_used.unwrap_or_default()));
        }
    }
    let mut updated = stored.clone();
    updated.as_object_mut().ok_or("invalid stored application provider profile")?.insert(
        "customEndpoints".into(),
        serde_json::to_value(rows).map_err(|_| "invalid stored application provider profile")?,
    );
    Ok(updated)
}
