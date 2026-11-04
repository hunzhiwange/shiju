//! Controlled external model/cloud receipts. No entry, scheduler or algorithm replacement.
use super::{
    apply::*, assets::*, entry::VideoEntryCloud, manifest::*, planning::*, prepare::*, preview::*,
    transcript::*,
};
use crate::{
    providers::{ChatRequest, ChatResponse, Provider},
    tools::ToolRuntimeContext,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fault {
    None,
    Quota,
    InvalidChunk,
    AcceptedTimeout,
    FakePreview,
}
pub(super) struct Scenario {
    fixture: Value,
    cloud_receipt: Value,
    pub fault: Fault,
    pub capture: Mutex<Vec<(String, String)>>,
    model_gate: FinishOrder,
    asset_gate: FinishOrder,
    pub active: AtomicUsize,
    pub peak: AtomicUsize,
    pub writing: AtomicUsize,
    pub write_peak: AtomicUsize,
}
impl Scenario {
    pub fn new(duration: u32, fault: Fault) -> Arc<Self> {
        let fixture = match duration {
            300 => include_str!("../../../testdata/workflow/video_edit/synthetic_300.json"),
            600 => include_str!("../../../testdata/workflow/video_edit/synthetic_600.json"),
            _ => panic!("unsupported test duration"),
        };
        Arc::new(Self {
            model_gate: FinishOrder::new(),
            asset_gate: FinishOrder::new(),
            fixture: serde_json::from_str(fixture).unwrap(),
            cloud_receipt: serde_json::from_str(include_str!(
                "../../../testdata/workflow/video_edit/synthetic_cloud_receipt.json"
            ))
            .unwrap(),
            fault,
            capture: Default::default(),
            active: Default::default(),
            peak: Default::default(),
            writing: Default::default(),
            write_peak: Default::default(),
        })
    }
    fn fixture(&self) -> &Value {
        &self.fixture
    }
    pub fn duration(&self) -> f64 {
        self.fixture["duration_sec"].as_f64().unwrap()
    }
    pub fn record(&self, kind: &str, id: &str) -> usize {
        let mut capture = self.capture.lock().unwrap();
        let order = capture.iter().filter(|(k, _)| k == kind).count();
        capture.push((kind.into(), id.into()));
        order
    }
    async fn leaf(&self, order: usize, model: bool, id: &str) {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        let gate = if model { &self.model_gate } else { &self.asset_gate };
        if order != 99 {
            tokio::time::timeout(std::time::Duration::from_secs(15), gate.wait(order))
                .await
                .expect("five external operations must overlap");
            self.record(if model { "model.done" } else { "asset.done" }, id);
        }
        if model || self.fault != Fault::AcceptedTimeout {
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
        if order != 99 {
            gate.complete(order);
        }
    }
}

struct FinishOrder {
    barrier: tokio::sync::Barrier,
    next: AtomicUsize,
    changed: tokio::sync::Notify,
}
impl FinishOrder {
    fn new() -> Self {
        Self {
            barrier: tokio::sync::Barrier::new(5),
            next: AtomicUsize::new(4),
            changed: Default::default(),
        }
    }
    async fn wait(&self, order: usize) {
        if order < 5 {
            self.barrier.wait().await;
        }
        loop {
            let changed = self.changed.notified();
            let next = self.next.load(Ordering::SeqCst);
            if next == if order < 5 { order } else { usize::MAX } {
                break;
            }
            changed.await;
        }
    }
    fn complete(&self, order: usize) {
        if order < 5 {
            self.next.store(order.checked_sub(1).unwrap_or(usize::MAX), Ordering::SeqCst);
            self.changed.notify_waiters();
        }
    }
}

pub(super) struct Cloud {
    pub scenario: Arc<Scenario>,
    pub context: ToolRuntimeContext,
    pub writes: Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>,
}
#[async_trait::async_trait]
impl VideoPrepareCloud for Cloud {
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
    }
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>> {
        self.scenario.record("scope.list", "authorized fixture scope");
        Ok(vec![CloudVideoScope { brand_id: Some("neutral".into()), store_id: None }])
    }
    async fn create_project(&self, run: &str, _: &CloudVideoScope) -> Result<String> {
        self.scenario.record("project.create", run);
        Ok(format!("project-{run}"))
    }
    async fn find_project(&self, _: &str, _: &CloudVideoScope) -> Result<Option<String>> {
        self.scenario.record("project.lookup", "reconciliation");
        Ok(None)
    }
    async fn create_upload(&self, _: &str, _: &VideoSourceIdentity) -> Result<VideoUploadTicket> {
        self.scenario.record("upload.create", "synthetic source");
        Ok(VideoUploadTicket {
            upload_id: "upload".into(),
            url: "https://external.invalid/upload".into(),
            headers: Default::default(),
        })
    }
    async fn upload(&self, _: &VideoUploadTicket, source: &VideoSourceIdentity) -> Result<()> {
        self.scenario.record("upload.put", &source.path.to_string_lossy());
        anyhow::ensure!(source.path.is_file(), "missing source");
        Ok(())
    }
    async fn finalize(&self, _: &str, _: &str, _: Option<&str>) -> Result<VideoImportResult> {
        self.scenario.record("upload.finalize", "source");
        Ok(VideoImportResult::Ready("source".into()))
    }
    async fn probe(&self, _: &str, _: &str) -> Result<VideoMediaInfo> {
        self.scenario.record("media.probe", "source");
        Ok(VideoMediaInfo {
            has_video: true,
            width: 640,
            height: 360,
            duration_sec: self.scenario.duration(),
            has_audio: true,
        })
    }
}
#[async_trait::async_trait]
impl VideoTranscriptCloud for Cloud {
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
    }
    async fn probe_transcript_media(&self, _: &str, _: &str) -> Result<Value> {
        self.scenario.record("transcript.probe", "source");
        Ok(self.scenario.fixture()["probe"].clone())
    }
    async fn transcribe(&self, _: &str, _: &str, _: &str, _: Option<&str>) -> Result<Value> {
        self.scenario.record("transcript.submit", "source");
        Ok(self.scenario.fixture()["receipt"].clone())
    }
}
#[async_trait::async_trait]
impl VideoAssetCloud for Cloud {
    async fn submit(
        &self,
        request: &VideoAssetRequest,
        key: &str,
    ) -> Result<VideoAssetRemoteState> {
        let order = self.scenario.record("asset.submit", key);
        self.scenario.leaf(order, false, key).await;
        if self.scenario.fault == Fault::AcceptedTimeout {
            anyhow::bail!("synthetic transport timed out after acceptance");
        }
        Ok(VideoAssetRemoteState::Succeeded {
            task_id: key.into(),
            reference: format!("gen:{}", request.digest),
        })
    }
    async fn reconcile(&self, job: &VideoAssetJob) -> Result<VideoAssetRemoteState> {
        self.scenario.record("asset.reconcile", &job.idempotency_key);
        if self.scenario.fault == Fault::AcceptedTimeout {
            self.scenario.active.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(VideoAssetRemoteState::Succeeded {
            task_id: job.idempotency_key.clone(),
            reference: format!("gen:{}", job.request.digest),
        })
    }
    async fn verify(&self, _: &str, _: &str) -> Result<()> {
        self.scenario.record("asset.verify", "generated reference");
        Ok(())
    }
}
#[async_trait::async_trait]
impl VideoEntryCloud for Cloud {
    fn versions(&self) -> (&str, &str) {
        ("external-fixture", "external-fixture")
    }
    async fn resolve_video_materials(
        &self,
        project: &str,
        plan: &VideoEditPlan,
        recipes: &BTreeMap<String, VideoAssetRecipe>,
        registry: &VideoAssetRegistry,
    ) -> Result<BTreeMap<String, VideoEffectMaterial>> {
        let mut result = BTreeMap::new();
        for e in &plan.effects {
            let value = match e.visual_type {
                VisualType::Keyword => VideoEffectMaterial::Keyword,
                VisualType::MotionGraphic => {
                    VideoEffectMaterial::MotionGraphic { template_name: e.assets[0].clone() }
                }
                VisualType::BRoll => {
                    let p = VideoEditPlan { effects: vec![e.clone()], ..plan.clone() };
                    let request = video_asset_requests(&p, project, recipes)?.remove(0);
                    anyhow::ensure!(
                        registry.references.contains_key(&request.digest),
                        "asset not ready"
                    );
                    VideoEffectMaterial::GeneratedVideo { digest: request.digest }
                }
            };
            result.insert(e.id.clone(), value);
        }
        Ok(result)
    }
    fn project_cloud<'a>(&'a self, _: &VideoApplyInput<'a>) -> Box<dyn VideoProjectCloud + 'a> {
        Box::new(RemoteProject(self))
    }
}
struct RemoteProject<'a>(&'a Cloud);
#[async_trait::async_trait]
impl VideoProjectCloud for RemoteProject<'_> {
    async fn read_timeline(&self, project: &str) -> Result<VideoTimelineEvidence> {
        self.0.scenario.record("timeline.read", project);
        let all = self.0.writes.lock().unwrap();
        let writes = all.get(project).cloned().unwrap_or_default();
        let original = writes.iter().any(|w| matches!(w, VideoProjectWrite::Original { .. }));
        Ok(VideoTimelineEvidence {
            project_id: project.into(),
            revision: "revision".into(),
            source_asset_id: if original { "source" } else { "" }.into(),
            windows: if original {
                vec![VideoSourceWindow {
                    source_start: 0.,
                    source_end: self.0.scenario.duration(),
                    timeline_start: 0.,
                    timeline_end: self.0.scenario.duration(),
                }]
            } else {
                vec![]
            },
            captions_present: writes.iter().any(|w| matches!(w, VideoProjectWrite::Captions)),
            effect_ids: writes
                .iter()
                .filter_map(|w| {
                    if let VideoProjectWrite::Effect { effect, .. } = w {
                        Some(effect.id.clone())
                    } else {
                        None
                    }
                })
                .collect::<BTreeSet<_>>(),
            video_readable: original,
            original_audio_present: original,
            duration_sec: if original { self.0.scenario.duration() } else { 0. },
        })
    }
    async fn write(
        &self,
        project: &str,
        _: &str,
        write: &VideoProjectWrite,
    ) -> Result<VideoWriteStatus> {
        let active = self.0.scenario.writing.fetch_add(1, Ordering::SeqCst) + 1;
        self.0.scenario.write_peak.fetch_max(active, Ordering::SeqCst);
        self.0.scenario.record("project.write", &format!("{project}:{write:?}"));
        tokio::task::yield_now().await;
        self.0.writes.lock().unwrap().entry(project.into()).or_default().push(write.clone());
        self.0.scenario.writing.fetch_sub(1, Ordering::SeqCst);
        Ok(VideoWriteStatus::Applied)
    }
    async fn reconcile(&self, _: &str, _: &VideoWriteEntry) -> Result<VideoWriteStatus> {
        Ok(VideoWriteStatus::Unknown)
    }
    async fn preview(&self, project: &str, revision: &str) -> Result<VideoPreviewEvidence> {
        self.0.scenario.record("preview.read", project);
        Ok(VideoPreviewEvidence {
            project_id: project.into(),
            revision: revision.into(),
            job_id: self.0.scenario.cloud_receipt["job_id"].as_str().unwrap().into(),
            job_status: self.0.scenario.cloud_receipt["job_status"].as_str().unwrap().into(),
            media_playable: self.0.scenario.fault != Fault::FakePreview,
            duration_sec: self.0.scenario.duration(),
        })
    }
    async fn viewer_url(&self, _: &str) -> Result<String> {
        self.0.scenario.record("viewer.read", "synthetic viewer");
        Ok(self.0.scenario.cloud_receipt["viewer_url"].as_str().unwrap().into())
    }
}

pub(super) struct Model(pub Arc<Scenario>);
#[async_trait::async_trait]
impl Provider for Model {
    async fn chat_with_system(&self, _: Option<&str>, _: &str, _: &str, _: f64) -> Result<String> {
        anyhow::bail!("unexpected interface")
    }
    async fn chat(&self, request: ChatRequest<'_>, _: &str, _: f64) -> Result<ChatResponse> {
        let prompt = &request.messages.last().unwrap().content;
        self.0.record("model.prompt", prompt);
        if self.0.fault == Fault::Quota {
            anyhow::bail!("synthetic quota exhausted");
        }
        let text = if prompt.contains("VideoChunkPlan") {
            let segment: super::segments::VideoSegment =
                serde_json::from_str(prompt.split_once("片段：").unwrap().1)?;
            let order = self.0.record("model.start", &segment.chunk_id);
            self.0.leaf(order, true, &segment.chunk_id).await;
            let effects=segment.words.iter().filter(|w|w.is_core).take(3).enumerate().map(|(i,w)|{
                let kind=["keyword","b_roll","motion_graphic"][i];
                json!({"id":format!("{}-{i}",segment.chunk_id),"chunk_id":segment.chunk_id,"word_index":w.word_index,"end_word_index":w.word_index,"anchor":if self.0.fault==Fault::InvalidChunk {"WRONG"} else {&w.text},"start":w.start,"end":w.end,"visual_type":kind,"assets":[if i==1 {format!("scene-{}",segment.index)} else {"Exact template".into()}]})
            }).collect::<Vec<_>>();
            json!({"chunk_id":segment.chunk_id,"effects":effects}).to_string()
        } else if prompt.contains("只为方案中 b_roll") {
            self.0.leaf(99, true, "single model request").await;
            let plan: VideoEditPlan = serde_json::from_str(prompt.split_once("方案：").unwrap().1)?;
            let recipes:BTreeMap<_,_>=plan.effects.iter().filter(|e|e.visual_type==VisualType::BRoll).map(|e|(e.assets[0].clone(),json!({"prompt":e.assets[0],"kind":"environment","model":"wan3.0-video","duration_sec":5,"ratio":"16:9","resolution":"720P"}))).collect();
            serde_json::to_string(&recipes)?
        } else {
            self.0.leaf(99, true, "single model request").await;
            "synthetic bounded brief".into()
        };
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
