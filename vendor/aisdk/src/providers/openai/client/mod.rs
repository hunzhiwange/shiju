//! This module provides the OpenAI client, an HTTP client for interacting with the OpenAI API.
//! It is a thin wrapper around the `reqwest` crate.
//! HTTP requests have this parts:

pub(crate) mod types;

#[cfg(test)]
mod tests;

pub(crate) use types::*;

use crate::core::client::{EmbeddingClient, LanguageModelClient, merge_body, merge_headers};
use crate::error::Error;
use crate::providers::openai::{ModelName, OpenAI};
use reqwest::header::CONTENT_TYPE;
use reqwest_eventsource::Event;

impl<M: ModelName> LanguageModelClient for OpenAI<M> {
    type Response = types::OpenAIResponse;
    type StreamEvent = types::OpenAiStreamEvent;

    fn path(&self) -> String {
        self.settings
            .path
            .clone()
            .unwrap_or_else(|| "/v1/responses".to_string())
    }

    fn method(&self) -> reqwest::Method {
        reqwest::Method::POST
    }

    fn headers(&self) -> crate::error::Result<reqwest::header::HeaderMap> {
        // Default headers
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        // Authorization
        default_headers.insert(
            "Authorization",
            format!("Bearer {}", self.settings.api_key.clone())
                .parse()
                .unwrap(),
        );

        merge_headers(
            default_headers,
            self.settings.headers.as_ref(),
            self.lm_options.extra_headers.as_ref(),
        )
    }

    fn query_params(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }

    fn body(&self) -> crate::error::Result<reqwest::Body> {
        merge_body(
            &self.lm_options,
            self.settings.body.as_ref(),
            self.lm_options.extra_body.as_ref(),
        )
    }

    fn parse_stream_sse(
        event: std::result::Result<Event, reqwest_eventsource::Error>,
    ) -> crate::error::Result<Self::StreamEvent> {
        match event {
            Ok(event) => match event {
                Event::Open => Ok(types::OpenAiStreamEvent::NotSupported("{}".to_string())),
                Event::Message(msg) => {
                    if msg.data.trim() == "[DONE]" || msg.data.is_empty() {
                        return Ok(types::OpenAiStreamEvent::NotSupported("[END]".to_string()));
                    }

                    let value: serde_json::Value =
                        serde_json::from_str(&msg.data).map_err(|e| Error::ApiError {
                            status_code: None,
                            details: format!("Invalid JSON in SSE data: {e}"),
                        })?;

                    // Unknown or malformed native calls must never disappear as ignored items.
                    let native = value
                        .get("item")
                        .and_then(|item| item.get("type"))
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|kind| kind.starts_with("tool_search"));
                    if native
                        && serde_json::from_value::<types::OpenAiStreamEvent>(value.clone())
                            .is_err()
                    {
                        return Err(Error::InvalidInput(
                            "Unsupported native tool_search stream item".into(),
                        ));
                    }
                    Ok(serde_json::from_value::<types::OpenAiStreamEvent>(value)
                        .unwrap_or(types::OpenAiStreamEvent::NotSupported(msg.data)))
                }
            },
            Err(e) => {
                // Extract status code if it's an InvalidStatusCode error
                let status_code = match &e {
                    reqwest_eventsource::Error::InvalidStatusCode(status, _) => Some(*status),
                    _ => None,
                };
                Err(Error::ApiError {
                    status_code,
                    details: e.to_string(),
                })
            }
        }
    }

    fn end_stream(event: &Self::StreamEvent) -> bool {
        matches!(
            event,
            types::OpenAiStreamEvent::ResponseCompleted { .. }
                | types::OpenAiStreamEvent::ResponseIncomplete { .. }
                | types::OpenAiStreamEvent::ResponseFailed { .. }
        ) || matches!(event, types::OpenAiStreamEvent::NotSupported(json) if json == "[END]")
            || matches!(event, types::OpenAiStreamEvent::ResponseError { .. })
    }
}

impl<M: ModelName> EmbeddingClient for OpenAI<M> {
    type Response = types::EmbeddingResponse;

    fn path(&self) -> String {
        "/v1/embeddings".to_string()
    }

    fn method(&self) -> reqwest::Method {
        reqwest::Method::POST
    }

    fn headers(&self) -> crate::error::Result<reqwest::header::HeaderMap> {
        // Default headers
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        // Authorization
        default_headers.insert(
            "Authorization",
            format!("Bearer {}", self.settings.api_key.clone())
                .parse()
                .unwrap(),
        );

        merge_headers(
            default_headers,
            self.settings.headers.as_ref(),
            self.embedding_options.extra_headers.as_ref(),
        )
    }

    fn query_params(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }

    fn body(&self) -> crate::error::Result<reqwest::Body> {
        merge_body(
            &self.embedding_options,
            self.settings.body.as_ref(),
            self.embedding_options.extra_body.as_ref(),
        )
    }
}
