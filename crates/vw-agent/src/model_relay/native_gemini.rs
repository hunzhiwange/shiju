//! Native Gemini URI controls for the authorized resident route.
use axum::http::{StatusCode, Uri};

pub(crate) struct Request {
    pub(crate) model: String,
    pub(crate) streaming: bool,
}

impl Request {
    pub(crate) fn parse(uri: &Uri) -> Option<Self> {
        let path = uri.path().strip_prefix("/gemini").unwrap_or(uri.path());
        let resource = path
            .strip_prefix("/v1beta/models/")
            .or_else(|| uri.path().strip_prefix("/gemini/v1/models/"))?;
        let (model, action) = resource.rsplit_once(':')?;
        let streaming = match action {
            "generateContent" => false,
            "streamGenerateContent" => true,
            _ => return None,
        };
        let model = model.strip_prefix("models/").unwrap_or(model);
        if model.is_empty()
            || !model.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return None;
        }
        if uri.query().is_some_and(|query| query != "alt=sse" || !streaming) {
            return None;
        }
        Some(Self { model: model.into(), streaming })
    }

    pub(crate) fn url(
        &self,
        target: &reqwest::Url,
        model: &str,
    ) -> Result<reqwest::Url, StatusCode> {
        // Retain the trusted provider origin/version; the URI only selects a
        // model and one of two generation actions, never a destination.
        super::transport::gemini_url(
            &crate::provider::provider::ApiInfo {
                protocols: None,
                url: target.to_string(),
                id: model.into(),
                adapter: "openai-compatible".into(),
            },
            self.streaming,
        )
    }
}

/// Shared pure Schema policy, scoped to the authorized resident data plane.
pub(crate) fn prepare(body: &serde_json::Value) -> Result<serde_json::Value, StatusCode> {
    vw_agent_apps::routing::gemini_schema::prepare_native_request(body)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}
