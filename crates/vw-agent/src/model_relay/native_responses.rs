//! Resident Responses differences; session validation remains in `responses`.
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::{body::Body, http::StatusCode};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::Arc;

pub(crate) fn is_path(path: &str) -> bool {
    matches!(
        path,
        "/responses"
            | "/v1/responses"
            | "/v1/v1/responses"
            | "/codex/v1/responses"
            | "/grokbuild/v1/responses"
    )
}

pub(crate) fn check(
    body: &Value,
    references: &References,
    scope: &ReferenceScope,
) -> Result<(), StatusCode> {
    if let Some(id) = body.get("previous_response_id").filter(|id| !id.is_null()) {
        references.resolve(scope, id.as_str().ok_or(StatusCode::BAD_REQUEST)?)?;
    }
    if let Some(input) = body["input"].as_array() {
        for item in input {
            if item["type"] == "item_reference" {
                references
                    .check_item(scope, item["id"].as_str().ok_or(StatusCode::BAD_REQUEST)?)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn remember(
    body: &Value,
    references: &References,
    scope: &ReferenceScope,
) -> Result<(), StatusCode> {
    // Preserve upstream identifiers while owning them in the verified scope.
    if let Some(id) = body["id"].as_str()
        && references.resolve(scope, id).is_err()
    {
        references.remember(scope, id, id)?;
    }
    if let Some(output) = body["output"].as_array() {
        for item in output {
            if let Some(id) = item["id"].as_str() {
                references.remember_item(scope, id)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn stream(body: Body, references: Arc<References>, scope: ReferenceScope) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (
            body.into_data_stream(),
            super::sse_framing::Lines::default(),
            String::new(),
            references,
            scope,
            false,
        ),
        |(mut stream, mut lines, mut data, references, scope, done)| async move {
            if done {
                return None;
            }
            let bytes = match stream.next().await {
                Some(Ok(bytes)) => bytes,
                Some(Err(_)) => {
                    return Some((
                        Err(std::io::Error::other("Responses stream failed")),
                        (stream, lines, data, references, scope, true),
                    ));
                }
                None => return None,
            };
            let result = (|| {
                for line in
                    lines.push(&bytes, 4 * 1024 * 1024).map_err(|_| StatusCode::BAD_GATEWAY)?
                {
                    if line.is_empty() {
                        if !data.is_empty() && data.trim() != "[DONE]" {
                            let event: Value =
                                serde_json::from_str(&data).map_err(|_| StatusCode::BAD_GATEWAY)?;
                            if event["response"].is_object() {
                                remember(&event["response"], &references, &scope)?;
                            }
                            if let Some(id) = event["item"]["id"].as_str() {
                                references.remember_item(&scope, id)?;
                            }
                        }
                        data.clear();
                    } else if let Some(value) = line.strip_prefix("data:") {
                        if data.len() + value.len() + 1 > 4 * 1024 * 1024 {
                            return Err(StatusCode::BAD_GATEWAY);
                        }
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(value.strip_prefix(' ').unwrap_or(value));
                    }
                }
                Ok(())
            })();
            let done = result.is_err();
            let chunk = result
                .map(|()| bytes)
                .map_err(|_| std::io::Error::other("Invalid Responses references"));
            Some((chunk, (stream, lines, data, references, scope, done)))
        },
    ))
}
