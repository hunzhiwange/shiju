//! 实例管理路由模块

use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::app::agent::bus;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::approval_state;
use crate::app::agent::gateway::instance::{InstanceQuery, resolve_directory, with_instance};
use crate::app::agent::global;
use crate::app::agent::project::instance;
use crate::app::agent::shell::git_std_command;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/path", get(path_get))
        .route("/vcs", get(vcs_get))
        .route("/event", get(instance_event_sse))
        .route("/instance/dispose", post(instance_dispose))
}

#[derive(Debug, Serialize)]
struct PathInfo {
    home: String,
    state: String,
    config: String,
    worktree: String,
    directory: String,
}

async fn path_get(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<PathInfo>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let paths = global::paths();
            Ok(PathInfo {
                home: paths.home.to_string_lossy().to_string(),
                state: paths.state.to_string_lossy().to_string(),
                config: paths.config.to_string_lossy().to_string(),
                worktree: instance::worktree(),
                directory: instance::directory(),
            })
        })
    })
    .await?;
    Ok(Json(result))
}

#[derive(Debug, Serialize)]
struct VcsInfo {
    branch: String,
}

fn branch_for_worktree(worktree: &str) -> String {
    if worktree.trim().is_empty() {
        return String::new();
    }
    git_std_command()
        .current_dir(worktree)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

async fn vcs_get(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<VcsInfo>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result = with_instance(dir, move || {
        Box::pin(async move {
            let worktree = instance::worktree();
            Ok(VcsInfo { branch: branch_for_worktree(&worktree) })
        })
    })
    .await?;
    Ok(Json(result))
}

async fn instance_event_sse(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let sse = with_instance(dir, move || {
        Box::pin(async move {
            let (tx, rx) = mpsc::channel::<Event>(512);
            let filter_dir = instance::directory();
            let tx_sub = tx.clone();
            let filter_dir2 = filter_dir.clone();
            let unsub = bus::global_subscribe(move |evt| {
                if !evt.directory.as_deref().map_or(true, |d| d == filter_dir2) {
                    return;
                }
                let data = serde_json::to_string(&evt.payload).unwrap_or("{}".to_string());
                let _ = tx_sub.try_send(Event::default().data(data));
            });

            let tx_task = tx.clone();
            tokio::spawn(async move {
                let connected = serde_json::json!({ "type": "server.connected", "properties": {} });
                let _ = tx_task
                    .try_send(Event::default().data(serde_json::to_string(&connected).unwrap()));
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    let hb = serde_json::json!({ "type": "server.heartbeat", "properties": {} });
                    if tx_task
                        .send(Event::default().data(serde_json::to_string(&hb).unwrap()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                unsub();
            });

            Ok(Sse::new(ReceiverStream::new(rx).map(Ok)).keep_alive(KeepAlive::new()))
        })
    })
    .await?;

    Ok(sse)
}

async fn instance_dispose(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    approval_state::clear_approval_manager_for_directory(&dir);
    with_instance(dir, move || {
        Box::pin(async move {
            instance::dispose().await;
            Ok(())
        })
    })
    .await?;
    Ok(Json(true))
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod instance_tests;
