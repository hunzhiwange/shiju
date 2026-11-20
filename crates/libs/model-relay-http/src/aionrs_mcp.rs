//! The aionrs sandbox reaches MCP through its existing, authenticated model port.
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};

pub struct Socket {
    pub path: PathBuf,
    pub token: String,
}

const LIMIT: usize = 1024 * 1024;

#[cfg(unix)]
async fn exchange(channel: &Socket, mut request: Value) -> Result<Value, StatusCode> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    request["token"] = json!(channel.token);
    let io = async {
        let mut socket = tokio::net::UnixStream::connect(&channel.path).await?;
        socket.write_all(&serde_json::to_vec(&request)?).await?;
        socket.shutdown().await?;
        let mut response = Vec::new();
        socket.take((LIMIT + 1) as u64).read_to_end(&mut response).await?;
        if response.len() > LIMIT {
            return Err(std::io::Error::other("oversized MCP bridge response"));
        }
        Ok::<Value, std::io::Error>(serde_json::from_slice(&response)?)
    };
    tokio::time::timeout(Duration::from_secs(20), io)
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

#[cfg(not(unix))]
async fn exchange(_: &Socket, _: Value) -> Result<Value, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

pub async fn handle(channel: &Socket, mount: usize, request: Request) -> Response {
    let result = async {
        if request.uri().query().is_some() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let bytes = to_bytes(request.into_body(), LIMIT)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        let body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        let response = exchange(channel, json!({ "mount": mount, "body": body })).await?;
        if response.is_null() {
            return Ok(StatusCode::ACCEPTED.into_response());
        }
        Ok(([(header::CONTENT_TYPE, "application/json")], Body::from(response.to_string()))
            .into_response())
    }
    .await;
    result.unwrap_or_else(IntoResponse::into_response)
}

pub async fn retain_tools(channel: Option<&Socket>, request: &mut Value) -> Result<(), StatusCode> {
    let Some(tools) = request.get_mut("tools").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    // Only the trusted bridge supplies the discovered catalog. The HTTP caller
    // cannot add a name by placing it in its model request.
    let catalog = match channel {
        Some(channel) if channel.path.exists() => {
            exchange(channel, json!({ "catalog": true })).await?
        }
        _ => Value::Null,
    };
    tools.retain(|tool| {
        let Some(name) = tool.pointer("/function/name").and_then(Value::as_str) else {
            return false;
        };
        matches!(name, "Read" | "Write" | "Edit" | "Glob")
            || (name == "Skill" && channel.is_some())
            || catalog
                .as_array()
                .is_some_and(|names| names.iter().any(|item| item.as_str() == Some(name)))
    });
    Ok(())
}
