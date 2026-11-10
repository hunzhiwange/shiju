//! Native catalog options. This operation never projects headers or starts a proxy.
use super::validation::custom_user_agent;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{request_options::*, validation::ProviderRequestOptions},
};
pub fn read(app: AppId, row: &Value) -> Result<ProviderRequestOptions, String> {
    let native = match app {
        AppId::Claude => "claude",
        AppId::Codex => "codex",
        _ => return Err("unsupported request options application".into()),
    };
    if !row.get(native).is_some_and(Value::is_object) {
        return Err("unsupported native request options record".into());
    }
    let options = match row.get("requestOptions") {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|_| "invalid stored application provider profile")?,
        None => ProviderRequestOptions::default(),
    };
    super::validation::validate(app, &options)?;
    Ok(options)
}
pub fn view(app: AppId, row: &Value) -> Result<RequestOptionsView, String> {
    let options = read(app, row)?;
    Ok(RequestOptionsView {
        headers_configured: options.request_headers.as_ref().is_some_and(|v| !v.is_empty()),
        overrides_configured: options.local_proxy_request_overrides.as_ref().is_some_and(|v| {
            v.headers.as_ref().is_some_and(|headers| {
                headers.iter().any(|(name, value)| {
                    !name.trim().eq_ignore_ascii_case(USER_AGENT.as_str())
                        || matches!(custom_user_agent(Some(value)), Ok(Some(_)))
                })
            }) || v.body.as_ref().is_some_and(|v| !v.is_empty())
        }),
        application: RequestOptionsApplication::NotApplied,
    })
}
pub fn update(app: AppId, row: &Value, incoming: &ProviderRequestOptions) -> Result<Value, String> {
    let old = read(app, row)?;
    let options = super::validation::merge(app, Some(&old), incoming)?;
    let mut updated = row.clone();
    updated.as_object_mut().ok_or("invalid stored application provider profile")?.insert(
        "requestOptions".into(),
        serde_json::to_value(options).map_err(|_| "invalid request options")?,
    );
    Ok(updated)
}
