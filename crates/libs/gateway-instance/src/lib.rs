//! 实例管理路由模块
//!
//! `/vcs` 分支读取走 git2，不启动系统 Git。仍依赖 Git CLI 的范围外能力：
//! `/v1/git/command`、普通 worktree fetch/submodule、skill clone/pull、snapshot 等。

#![cfg(not(target_arch = "wasm32"))]

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

use agent_bus as bus;
use gateway_types::ApiError;
pub mod host;
use host::{InstanceHost, InstanceQuery};
use agent_paths as global;
use agent_project::instance;

pub fn router<S, H: InstanceHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/path", get(path_get::<H>))
        .route("/vcs", get(vcs_get::<H>))
        .route("/event", get(instance_event_sse::<H>))
        .route("/instance/dispose", post(instance_dispose::<H>))
}

#[derive(Debug, Serialize)]
pub struct PathInfo {
    pub home: String,
    pub task_worktrees_root: String,
    pub state: String,
    pub config: String,
    pub worktree: String,
    pub directory: String,
}

pub async fn path_get<H: InstanceHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<PathInfo>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            let paths = global::paths();
            Ok(PathInfo {
                home: paths.home.to_string_lossy().to_string(),
                task_worktrees_root: config_runtime_paths::task_worktrees_dir(
                    directories::UserDirs::new()
                        .map(|dirs| dirs.home_dir().to_path_buf())
                        .unwrap_or_else(|| std::path::PathBuf::from(instance::worktree())),
                )
                .to_string_lossy()
                .to_string(),
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
pub struct VcsInfo {
    pub branch: String,
}

pub fn branch_for_worktree(worktree: &str) -> String {
    if worktree.trim().is_empty() {
        return String::new();
    }
    git_discovery::git_repository::current_branch(std::path::Path::new(worktree)).unwrap_or_default()
}

pub async fn vcs_get<H: InstanceHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<VcsInfo>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let result = H::with_instance(dir, move || {
        Box::pin(async move {
            let worktree = instance::worktree();
            Ok(VcsInfo { branch: branch_for_worktree(&worktree) })
        })
    })
    .await?;
    Ok(Json(result))
}

pub async fn instance_event_sse<H: InstanceHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    let sse = H::with_instance(dir, move || {
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

pub async fn instance_dispose<H: InstanceHost>(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let dir = H::resolve_directory(&query, &headers);
    H::clear_approval_manager_for_directory(&dir);
    H::with_instance(dir, move || {
        Box::pin(async move {
            instance::dispose().await;
            Ok(())
        })
    })
    .await?;
    Ok(Json(true))
}
