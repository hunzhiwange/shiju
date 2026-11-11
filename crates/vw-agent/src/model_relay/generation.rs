//! Protocol and authentication primitives for one explicitly selected generation target.

use axum::http::{StatusCode, header};
use reqwest::Url;
use vw_agent_apps::providers::validation;
use vw_provider_resolver::remote_models::is_real_bearer;

use super::transport::{
    chat_url, gemini_url, messages_url, official_alibaba_messages_url,
    official_deepseek_messages_url, responses_url,
};
use crate::provider::provider::ApiInfo;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationProtocol {
    Chat,
    Messages,
    Responses,
    Gemini,
}

impl GenerationProtocol {
    pub(crate) fn url(
        self,
        endpoint: &str,
        model: &str,
        full_url: bool,
    ) -> Result<Url, StatusCode> {
        let invalid = StatusCode::UNPROCESSABLE_ENTITY;
        validation::endpoint(endpoint).map_err(|_| invalid)?;
        if full_url && self != Self::Gemini {
            return Url::parse(endpoint).map_err(|_| invalid);
        }
        let api = ApiInfo {
            url: endpoint.to_owned(),
            id: model.to_owned(),
            adapter: "openai-compatible".into(),
        };
        let url = match self {
            Self::Chat => chat_url(&api),
            Self::Messages => match official_deepseek_messages_url(&api)
                .or_else(|| official_alibaba_messages_url(&api))
            {
                Some(url) => Ok(url),
                None => messages_url(&api),
            },
            Self::Responses => responses_url(&api),
            Self::Gemini => gemini_url(&api, false),
        }?;
        if full_url && Url::parse(endpoint).ok().as_ref() != Some(&url) {
            return Err(invalid);
        }
        Ok(url)
    }

    pub(crate) fn headers(
        self,
        credential: &str,
        messages_bearer: bool,
    ) -> Option<header::HeaderMap> {
        if !is_real_bearer(credential) {
            return None;
        }
        let credential = credential.trim();
        let (name, value) = match self {
            Self::Messages if !messages_bearer => (
                header::HeaderName::from_static("x-api-key"),
                header::HeaderValue::from_str(credential),
            ),
            Self::Gemini => (
                header::HeaderName::from_static("x-goog-api-key"),
                header::HeaderValue::from_str(credential),
            ),
            _ => (
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {credential}")),
            ),
        };
        let mut value = value.ok()?;
        value.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(name, value);
        if self == Self::Messages {
            headers.insert("anthropic-version", header::HeaderValue::from_static("2023-06-01"));
        }
        Some(headers)
    }
}
