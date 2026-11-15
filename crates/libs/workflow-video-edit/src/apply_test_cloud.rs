use super::{apply::*, preview::*};
use anyhow::Result;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;
#[derive(Default)]
pub(super) struct Remote {
    pub(super) writes: Vec<VideoProjectWrite>,
    pub(super) keys: BTreeSet<String>,
    pub(super) lost_receipt: bool,
    pub(super) no_change: bool,
    pub(super) missing_audio: bool,
    pub(super) pending_preview: bool,
    pub(super) empty_page: bool,
    pub(super) canceller: Option<CancellationToken>,
    pub(super) gate: Option<Arc<Gate>>,
}
#[derive(Default)]
pub(super) struct Gate {
    pub(super) entered: tokio::sync::Notify,
    pub(super) release: tokio::sync::Notify,
}
pub(super) struct Endpoint {
    pub(super) url: String,
    pub(super) state: Arc<Mutex<Remote>>,
    pub(super) task: tokio::task::JoinHandle<()>,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn endpoint(
    State(state): State<Arc<Mutex<Remote>>>,
    Json(v): Json<Value>,
) -> axum::response::Response {
    let (result, gate) = {
        let mut s = state.lock().unwrap();
        let gate = if v["op"] == "write" { s.gate.take() } else { None };
        let result = match v["op"].as_str().unwrap() {
            "write" => {
                let key = v["key"].as_str().unwrap().to_owned();
                assert!(s.keys.insert(key), "remote mutation replayed");
                let write: VideoProjectWrite = serde_json::from_value(v["write"].clone()).unwrap();
                if s.no_change {
                    return Json(json!(VideoWriteStatus::NoChange)).into_response();
                }
                if matches!(write, VideoProjectWrite::Cleanup) {
                    if let Some(cancel) = &s.canceller {
                        cancel.cancel();
                    }
                }
                s.writes.push(write);
                if s.lost_receipt {
                    s.lost_receipt = false;
                    json!(VideoWriteStatus::Unknown)
                } else {
                    json!(VideoWriteStatus::Applied)
                }
            }
            "reconcile" => {
                assert!(s.keys.contains(v["key"].as_str().unwrap()));
                json!(VideoWriteStatus::Applied)
            }
            "timeline" => {
                let original =
                    s.writes.iter().any(|w| matches!(w, VideoProjectWrite::Original { .. }));
                let cleaned = s.writes.iter().any(|w| matches!(w, VideoProjectWrite::Cleanup));
                let windows = if !original {
                    vec![]
                } else if cleaned {
                    vec![
                        VideoSourceWindow {
                            source_start: 3.0,
                            source_end: 8.0,
                            timeline_start: 0.0,
                            timeline_end: 5.0,
                        },
                        VideoSourceWindow {
                            source_start: 10.0,
                            source_end: 15.0,
                            timeline_start: 5.0,
                            timeline_end: 10.0,
                        },
                    ]
                } else {
                    vec![VideoSourceWindow {
                        source_start: 0.0,
                        source_end: 20.0,
                        timeline_start: 0.0,
                        timeline_end: 20.0,
                    }]
                };
                json!(VideoTimelineEvidence {
                    project_id: "project".into(),
                    revision: format!("rev-{}", s.writes.len()),
                    source_asset_id: "source".into(),
                    windows,
                    captions_present: s
                        .writes
                        .iter()
                        .any(|w| matches!(w, VideoProjectWrite::Captions)),
                    effect_ids: s
                        .writes
                        .iter()
                        .filter_map(|w| match w {
                            VideoProjectWrite::Effect { effect, .. } => Some(effect.id.clone()),
                            _ => None,
                        })
                        .collect(),
                    video_readable: original,
                    original_audio_present: original && !s.missing_audio,
                    duration_sec: if cleaned {
                        10.0
                    } else if original {
                        20.0
                    } else {
                        0.0
                    }
                })
            }
            "preview" => json!(VideoPreviewEvidence {
                project_id: "project".into(),
                revision: v["revision"].as_str().unwrap().into(),
                job_id: "preview-job".into(),
                job_status: if s.pending_preview { "running" } else { "succeeded" }.into(),
                media_playable: !s.pending_preview,
                duration_sec: 10.0
            }),
            "viewer" => json!(if s.empty_page { "" } else { "https://example.test/s/preview" }),
            _ => panic!("unsupported operation; final export is never allowed"),
        };
        (result, gate)
    };
    if let Some(gate) = gate {
        gate.entered.notify_one();
        gate.release.notified().await;
    }
    if result == json!(VideoWriteStatus::Unknown) {
        (axum::http::StatusCode::GATEWAY_TIMEOUT, Json(result)).into_response()
    } else {
        Json(result).into_response()
    }
}
impl Endpoint {
    pub(super) async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(Remote::default()));
        let router = Router::new().route("/", post(endpoint)).with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { url, state, task }
    }
    async fn call(&self, value: Value) -> Result<Value> {
        Ok(reqwest::Client::builder()
            .no_proxy()
            .build()?
            .post(&self.url)
            .json(&value)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
#[async_trait::async_trait]
impl VideoProjectCloud for Endpoint {
    async fn read_timeline(&self, _: &str) -> Result<VideoTimelineEvidence> {
        Ok(serde_json::from_value(self.call(json!({"op":"timeline"})).await?)?)
    }
    async fn write(
        &self,
        _: &str,
        key: &str,
        write: &VideoProjectWrite,
    ) -> Result<VideoWriteStatus> {
        Ok(serde_json::from_value(self.call(json!({"op":"write","key":key,"write":write})).await?)?)
    }
    async fn reconcile(&self, _: &str, entry: &VideoWriteEntry) -> Result<VideoWriteStatus> {
        Ok(serde_json::from_value(self.call(json!({"op":"reconcile","key":entry.key})).await?)?)
    }
    async fn preview(&self, _: &str, revision: &str) -> Result<VideoPreviewEvidence> {
        Ok(serde_json::from_value(self.call(json!({"op":"preview","revision":revision})).await?)?)
    }
    async fn viewer_url(&self, _: &str) -> Result<String> {
        Ok(serde_json::from_value(self.call(json!({"op":"viewer"})).await?)?)
    }
}
