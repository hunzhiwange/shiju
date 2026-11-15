//! PTY（伪终端）HTTP 和 WebSocket 路由模块

use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::get;
use serde::Deserialize;

use gateway_types::ApiError;
use crate::host::{InstanceQuery, PtyHost};
use agent_pty as pty;

pub fn router<S, H: PtyHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/pty", get(pty_list::<H>).post(pty_create::<H>))
        .route("/pty/{pty_id}", get(pty_get::<H>).put(pty_update::<H>).delete(pty_remove::<H>))
        .route("/pty/{pty_id}/connect", get(pty_connect_ws::<H>))
}

pub async fn pty_list<H: PtyHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<pty::pty::Info>>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result =
        H::with_instance(dir, move || Box::pin(async move { Ok(pty::pty::list().await) })).await?;
    Ok(Json(result))
}

pub async fn pty_create<H: PtyHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(input): Json<pty::pty::CreateInput>,
) -> Result<Json<pty::pty::Info>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            pty::pty::create(input).await.map_err(|e| ApiError::bad_request(e.to_string()))
        })
    })
    .await?;
    Ok(Json(result))
}

pub async fn pty_get<H: PtyHost>(
    Path(pty_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<pty::pty::Info>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            pty::pty::get(&pty_id).await.ok_or_else(|| ApiError::not_found("session not found"))
        })
    })
    .await?;
    Ok(Json(result))
}

pub async fn pty_update<H: PtyHost>(
    Path(pty_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(input): Json<pty::pty::UpdateInput>,
) -> Result<Json<pty::pty::Info>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            let updated = pty::pty::update(&pty_id, input)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            updated.ok_or_else(|| ApiError::not_found("session not found"))
        })
    })
    .await?;
    Ok(Json(result))
}

pub async fn pty_remove<H: PtyHost>(
    Path(pty_id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    H::with_instance(dir, move || {
        Box::pin(async move {
            pty::pty::remove(&pty_id).await.map_err(|e| ApiError::bad_request(e.to_string()))?;
            Ok(())
        })
    })
    .await?;
    Ok(Json(true))
}

#[derive(Debug, Deserialize)]
pub struct PtyConnectQuery {
    pub directory: Option<String>,
    pub cursor: Option<i64>,
}

pub async fn pty_connect_ws<H: PtyHost>(
    Path(pty_id): Path<String>,
    ws: WebSocketUpgrade,
    Query(query): Query<PtyConnectQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let dir = H::resolve_directory(&InstanceQuery { directory: query.directory.clone() }, &headers);
    let cursor = query.cursor.unwrap_or(-1);
    Ok(ws.on_upgrade(move |socket| pty_ws_loop::<H>(socket, dir, pty_id, cursor)))
}

async fn pty_ws_loop<H: PtyHost>(mut socket: WebSocket, directory: String, pty_id: String, cursor: i64) {
    let _ = H::with_instance(directory, move || {
        Box::pin(async move {
            let mut cur = cursor;
            loop {
                tokio::select! {
                    maybe = socket.recv() => {
                        match maybe {
                            None => break,
                            Some(Ok(Message::Text(t))) => {
                                pty::pty::write(&pty_id, t.as_str()).await;
                            }
                            Some(Ok(Message::Binary(b))) => {
                                if let Ok(s) = String::from_utf8(b.to_vec()) {
                                    pty::pty::write(&pty_id, &s).await;
                                }
                            }
                            Some(Ok(Message::Close(_))) => break,
                            Some(Ok(_)) => {}
                            Some(Err(_)) => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        if let Some((data, next)) = pty::pty::read(&pty_id, cur).await {
                            if !data.is_empty() {
                                cur = next as i64;
                                let payload = serde_json::json!({
                                    "type": "data",
                                    "data": data,
                                    "cursor": next
                                });
                                if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
            Ok(())
        })
    })
    .await;
}
