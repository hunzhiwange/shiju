//! Credential precedence for an explicitly selected provider and current auth snapshot.
use axum::http::StatusCode;

pub fn credential(
    info: &model_types::Info,
    current_auth: Option<auth_types::Info>,
) -> Result<String, StatusCode> {
    let real_key = |key: &str| !key.trim().is_empty() && key.trim() != auth_types::OAUTH_DUMMY_KEY;
    if let Some(key) = info.key.as_deref().filter(|v| {
        real_key(v)
            && matches!(
                info.source,
                model_types::ProviderSource::Config | model_types::ProviderSource::Custom
            )
    }) {
        return Ok(key.trim().into());
    }
    for name in &info.env {
        if let Some(key) = agent_env::get(name).filter(|v| real_key(v)) {
            return Ok(key.trim().into());
        }
    }
    match current_auth {
        Some(auth_types::Info::Api(api)) if real_key(&api.key) => Ok(api.key.trim().into()),
        Some(auth_types::Info::Oauth(oauth))
            if real_key(&oauth.access) && oauth.expires > chrono::Utc::now().timestamp_millis() =>
        {
            Ok(oauth.access)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
