use super::{assets::*, budget::VideoEditRunBudget, manifest::*, planning::*};
use crate::global_database::GlobalDatabase;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
struct Remote {
    jobs: BTreeMap<String, String>,
    states: BTreeMap<String, String>,
    posts: usize,
    active: usize,
    peak: usize,
    lose_receipt: bool,
    refusals: usize,
    unreadable: bool,
}
struct Endpoint {
    url: String,
    state: Arc<Mutex<Remote>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Endpoint {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let state = Arc::new(Mutex::new(Remote::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = state.clone();
        let quit = stop.clone();
        let thread = std::thread::spawn(move || {
            while !quit.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut size = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            size = value.trim().parse().unwrap();
                        }
                    }
                }
                let mut bytes = vec![0; size];
                reader.read_exact(&mut bytes).unwrap();
                let input: Value = serde_json::from_slice(&bytes).unwrap();
                let mut s = shared.lock().unwrap();
                let key = input["key"].as_str().unwrap_or("");
                let refused = input["op"] == "submit" && s.refusals > 0;
                let lose_receipt = input["op"] == "submit" && s.lose_receipt && s.refusals == 0;
                let response = match input["op"].as_str().unwrap() {
                    "submit" => {
                        s.posts += 1;
                        if s.refusals > 0 {
                            s.refusals -= 1;
                            json!({"state":"not_submitted"})
                        } else {
                            assert!(!s.jobs.contains_key(key), "duplicate remote submission");
                            let id = format!("task-{}", s.jobs.len());
                            s.jobs.insert(key.into(), id.clone());
                            s.states.insert(id.clone(), "running".into());
                            s.active += 1;
                            s.peak = s.peak.max(s.active);
                            if s.lose_receipt {
                                s.lose_receipt = false;
                                json!({"state":"unknown"})
                            } else {
                                json!({"state":"running", "task":id})
                            }
                        }
                    }
                    "reconcile" => {
                        let id = input["task"]
                            .as_str()
                            .map(str::to_owned)
                            .or_else(|| s.jobs.get(key).cloned());
                        match id {
                            Some(id) => {
                                json!({"state":s.states[&id],"task":id,"reference":format!("gen:{id}")})
                            }
                            None => json!({"state":"unknown"}),
                        }
                    }
                    "verify" => {
                        let reference = input["reference"].as_str().unwrap();
                        let ready = s
                            .states
                            .get(reference.strip_prefix("gen:").unwrap())
                            .is_some_and(|s| s == "succeeded");
                        json!({"ok":ready && !s.unreadable})
                    }
                    _ => panic!("unexpected operation"),
                };
                if lose_receipt {
                    drop(s);
                    std::thread::sleep(std::time::Duration::from_millis(1100));
                    continue;
                }
                let body = response.to_string();
                let status = if refused { "429 Too Many Requests" } else { "200 OK" };
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        Self { url, state, stop, thread: Some(thread) }
    }
    fn finish(&self, task: &str, failed: bool) {
        let mut s = self.state.lock().unwrap();
        assert_eq!(s.states[task], "running");
        s.states.insert(task.into(), if failed { "failed" } else { "succeeded" }.into());
        s.active -= 1;
    }
    async fn call(&self, value: Value) -> anyhow::Result<Value> {
        Ok(reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .no_proxy()
            .build()?
            .post(&self.url)
            .json(&value)
            .send()
            .await?
            .json()
            .await?)
    }
    fn decode(value: Value) -> VideoAssetRemoteState {
        let id = value["task"].as_str().unwrap_or("").to_owned();
        match value["state"].as_str().unwrap() {
            "running" => VideoAssetRemoteState::Running(id),
            "succeeded" => VideoAssetRemoteState::Succeeded {
                task_id: id,
                reference: value["reference"].as_str().unwrap().into(),
            },
            "failed" => VideoAssetRemoteState::Failed { task_id: id },
            "not_submitted" => VideoAssetRemoteState::NotSubmitted,
            _ => VideoAssetRemoteState::Unknown,
        }
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
#[async_trait::async_trait]
impl VideoAssetCloud for Endpoint {
    async fn submit(
        &self,
        _: &VideoAssetRequest,
        key: &str,
    ) -> anyhow::Result<VideoAssetRemoteState> {
        Ok(Self::decode(self.call(json!({"op":"submit","key":key})).await?))
    }
    async fn reconcile(&self, job: &VideoAssetJob) -> anyhow::Result<VideoAssetRemoteState> {
        Ok(Self::decode(
            self.call(json!({"op":"reconcile","key":job.idempotency_key,"task":job.task_id}))
                .await?,
        ))
    }
    async fn verify(&self, _: &str, reference: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.call(json!({"op":"verify","reference":reference})).await?["ok"] == true,
            "unreadable asset"
        );
        Ok(())
    }
}
pub(super) fn fixture(
    n: usize,
) -> (tempfile::TempDir, VideoEditManifestStore, VideoEditRunManifest, Vec<VideoAssetRequest>) {
    let dir = tempfile::tempdir().unwrap();
    let store =
        VideoEditManifestStore::new(GlobalDatabase::new(dir.path().join("db")).unwrap()).unwrap();
    let mut run = super::manifest_tests::candidate();
    run.project_id = Some("project".into());
    let run = store.claim(run, "asset-test", chrono::Utc::now().timestamp_millis()).unwrap();
    let mut recipes = BTreeMap::new();
    let mut effects = Vec::new();
    for i in 0..n {
        let description = format!("素材{i}");
        recipes.insert(
            description.clone(),
            VideoAssetRecipe {
                prompt: format!("landscape {i}"),
                kind: "environment".into(),
                model: "wan3.0-video".into(),
                duration_sec: 5,
                ratio: "9:16".into(),
                resolution: "720P".into(),
            },
        );
        effects.push(VideoEditEffect {
            id: i.to_string(),
            chunk_id: "chunk".into(),
            word_index: i as u64,
            end_word_index: i as u64,
            anchor: "word".into(),
            start: i as f64,
            end: i as f64 + 1.0,
            visual_type: VisualType::BRoll,
            assets: vec![description.clone(), description],
        });
    }
    let plan = VideoEditPlan { asset_id: "source".into(), duration_sec: n as f64, effects };
    let requests = video_asset_requests(&plan, "project", &recipes).unwrap();
    assert_eq!(requests.len(), n, "duplicate materials must collapse");
    (dir, store, run, requests)
}

#[test]
fn video_edit_asset_jobs_unknown_is_not_ready() {
    assert!(!VideoAssetJobState::Unknown.is_terminal());
}
#[tokio::test]
async fn video_edit_asset_jobs_hold_five_until_reverse_completion() {
    let (_dir, store, mut run, requests) = fixture(7);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::default();
    assert!(
        prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap().is_none()
    );
    assert_eq!(cloud.state.lock().unwrap().posts, 5);
    assert!(
        prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap().is_none()
    );
    assert_eq!(
        cloud.state.lock().unwrap().posts,
        5,
        "sixth POST forbidden while five jobs run remotely"
    );
    cloud.finish("task-4", false);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    assert_eq!(cloud.state.lock().unwrap().posts, 6);
    cloud.finish("task-3", false);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    for id in ["task-6", "task-5", "task-2", "task-1", "task-0"] {
        cloud.finish(id, false);
    }
    let registry =
        prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap().unwrap();
    assert_eq!(registry.references.len(), 7);
    assert_eq!(registry.progress.completed, 7);
    assert_eq!(registry.progress.peak, 5);
    assert_eq!(cloud.state.lock().unwrap().peak, 5);
}
#[tokio::test]
async fn video_edit_asset_jobs_restart_reconciles_lost_receipt_without_post() {
    let (_dir, store, mut run, requests) = fixture(1);
    let cloud = Endpoint::new();
    cloud.state.lock().unwrap().lose_receipt = true;
    prepare_video_assets(&cloud, &store, &mut run, &requests, &VideoEditRunBudget::default())
        .await
        .unwrap();
    assert_eq!(run.asset_jobs[0].state, VideoAssetJobState::Unknown);
    assert!(run.asset_jobs[0].task_id.is_none());
    store.release(&run).unwrap();
    let mut recovered =
        store.claim(run, "restarted", chrono::Utc::now().timestamp_millis()).unwrap();
    cloud.finish("task-0", false);
    let result = prepare_video_assets(
        &cloud,
        &store,
        &mut recovered,
        &requests,
        &VideoEditRunBudget::default(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.references.len(), 1);
    assert_eq!(cloud.state.lock().unwrap().posts, 1);
    assert!(recovered.asset_jobs[0].reconciliation_version >= 2);
}
#[tokio::test]
async fn video_edit_asset_jobs_refusal_retries_share_peak_and_bound_attempts() {
    let (_dir, store, mut run, requests) = fixture(6);
    let cloud = Endpoint::new();
    cloud.state.lock().unwrap().refusals = 1;
    let budget = VideoEditRunBudget::default();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    assert_eq!(cloud.state.lock().unwrap().peak, 5);
    assert_eq!(run.asset_jobs.iter().map(|j| j.attempts).max(), Some(2));
    let (_dir2, store2, mut run2, requests2) = fixture(1);
    let failing = Endpoint::new();
    failing.state.lock().unwrap().refusals = 9;
    let budget2 = VideoEditRunBudget::default();
    for _ in 0..2 {
        prepare_video_assets(&failing, &store2, &mut run2, &requests2, &budget2).await.unwrap();
    }
    assert!(
        prepare_video_assets(&failing, &store2, &mut run2, &requests2, &budget2).await.is_err()
    );
    assert_eq!(run2.asset_jobs[0].attempts, 3);
}
#[tokio::test]
async fn video_edit_asset_jobs_cancel_retains_remote_slots_and_stops_dispatch() {
    let (_dir, store, mut run, requests) = fixture(6);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::default();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    budget.cancel();
    assert!(prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.is_err());
    assert_eq!(cloud.state.lock().unwrap().posts, 5);
    assert_eq!(run.asset_jobs.iter().filter(|j| j.state == VideoAssetJobState::Running).count(), 5);
    assert_eq!(run.asset_jobs.iter().filter(|j| j.state == VideoAssetJobState::Queued).count(), 1);
}
#[tokio::test]
async fn video_edit_asset_jobs_failure_or_unreadable_result_blocks_registry() {
    let (_dir, store, mut run, requests) = fixture(2);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::default();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    cloud.finish("task-0", true);
    cloud.finish("task-1", false);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    cloud.finish("task-2", true);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    cloud.finish("task-3", true);
    assert!(prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.is_err());
    let (_dir2, store2, mut run2, requests2) = fixture(1);
    let cloud2 = Endpoint::new();
    let budget2 = VideoEditRunBudget::default();
    prepare_video_assets(&cloud2, &store2, &mut run2, &requests2, &budget2).await.unwrap();
    cloud2.finish("task-0", false);
    cloud2.state.lock().unwrap().unreadable = true;
    assert!(prepare_video_assets(&cloud2, &store2, &mut run2, &requests2, &budget2).await.is_err());
}
#[tokio::test]
async fn video_edit_asset_jobs_share_model_budget_and_resume_unknown() {
    let (_dir, store, mut run, requests) = fixture(6);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::default();
    let model = budget.acquire().await.unwrap();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    assert_eq!(cloud.state.lock().unwrap().posts, 4);
    drop(model);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    assert_eq!(cloud.state.lock().unwrap().posts, 5);
}

#[tokio::test]
async fn video_edit_asset_jobs_unknown_missing_from_remote_is_not_resubmitted() {
    let (_dir, store, mut run, requests) = fixture(1);
    let cloud = Endpoint::new();
    cloud.state.lock().unwrap().lose_receipt = true;
    let budget = VideoEditRunBudget::new(1).unwrap();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    cloud.state.lock().unwrap().jobs.clear();
    assert!(
        prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap().is_none()
    );
    assert_eq!(cloud.state.lock().unwrap().posts, 1);
    assert!(!budget.try_reserve_asset("other-request").unwrap());
    assert_eq!(run.asset_jobs[0].state, VideoAssetJobState::Unknown);
}

#[tokio::test]
async fn video_edit_asset_jobs_failed_attempt_retries_only_after_terminal_and_keeps_id() {
    let (_dir, store, mut run, requests) = fixture(1);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::new(1).unwrap();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    let first_key = run.asset_jobs[0].idempotency_key.clone();
    cloud.finish("task-0", true);
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    assert_eq!(run.asset_jobs[0].failed_attempts[0].task_id, "task-0");
    assert_eq!(run.asset_jobs[0].failed_attempts[0].idempotency_key, first_key);
    assert_ne!(run.asset_jobs[0].idempotency_key, first_key);
    cloud.finish("task-1", false);
    let registry =
        prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap().unwrap();
    assert_eq!(registry.references.len(), 1);
    assert_eq!(cloud.state.lock().unwrap().peak, 1);
}

#[test]
fn video_edit_asset_jobs_deduplicate_content_parameters_and_scope_without_guessing_recipe() {
    let (_dir, _, _, requests) = fixture(1);
    let recipe = requests[0].recipe.clone();
    let plan = VideoEditPlan {
        asset_id: "source".into(),
        duration_sec: 1.0,
        effects: vec![VideoEditEffect {
            id: "effect".into(),
            chunk_id: "chunk".into(),
            word_index: 0,
            end_word_index: 0,
            anchor: "word".into(),
            start: 0.0,
            end: 1.0,
            visual_type: VisualType::BRoll,
            assets: vec!["a".into(), "b".into()],
        }],
    };
    let mut recipes = BTreeMap::from([("a".into(), recipe.clone()), ("b".into(), recipe)]);
    let a = video_asset_requests(&plan, "project", &recipes).unwrap();
    assert_eq!(a.len(), 1);
    assert_ne!(
        a[0].digest,
        video_asset_requests(&plan, "other-project", &recipes).unwrap()[0].digest
    );
    recipes.get_mut("b").unwrap().duration_sec = 10;
    assert_eq!(video_asset_requests(&plan, "project", &recipes).unwrap().len(), 2);
    recipes.remove("b");
    assert!(video_asset_requests(&plan, "project", &recipes).is_err());
}

#[tokio::test]
async fn video_edit_asset_jobs_restart_does_not_resume_cancelled_dispatch() {
    let (_dir, store, mut run, requests) = fixture(6);
    let cloud = Endpoint::new();
    let budget = VideoEditRunBudget::default();
    prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.unwrap();
    budget.cancel();
    assert!(prepare_video_assets(&cloud, &store, &mut run, &requests, &budget).await.is_err());
    store.release(&run).unwrap();
    let mut recovered =
        store.claim(run, "restart-cancelled", chrono::Utc::now().timestamp_millis()).unwrap();
    cloud.finish("task-0", false);
    let result = prepare_video_assets(
        &cloud,
        &store,
        &mut recovered,
        &requests,
        &VideoEditRunBudget::default(),
    )
    .await;
    assert!(result.is_err(), "restart must retain cancellation, got {result:?}");
    assert_eq!(cloud.state.lock().unwrap().posts, 5);
}
