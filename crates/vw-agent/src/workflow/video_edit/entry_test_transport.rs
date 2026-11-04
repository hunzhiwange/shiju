//! Session-scoped substitution of external model/cloud I/O only. The gateway,
//! entry, tenant store, resource graph, algorithms and journals are never replaced.
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
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::{Arc, LazyLock, Mutex},
};
static SESSIONS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
pub(crate) struct TransportGuard(String);
impl Drop for TransportGuard {
    fn drop(&mut self) {
        SESSIONS.lock().unwrap().remove(&self.0);
    }
}
pub(crate) fn install(session: &str) -> TransportGuard {
    assert!(SESSIONS.lock().unwrap().insert(session.into()));
    TransportGuard(session.into())
}
pub(super) fn get(
    session: &str,
    context: &ToolRuntimeContext,
) -> Option<(Arc<dyn VideoEntryCloud>, Arc<dyn Provider>)> {
    SESSIONS.lock().unwrap().contains(session).then(|| {
        (
            Arc::new(Cloud { context: context.clone(), writes: PROJECTS.clone() })
                as Arc<dyn VideoEntryCloud>,
            Arc::new(Model) as Arc<dyn Provider>,
        )
    })
}
static PROJECTS: LazyLock<Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>> =
    LazyLock::new(Default::default);
struct Cloud {
    context: ToolRuntimeContext,
    writes: Arc<Mutex<BTreeMap<String, Vec<VideoProjectWrite>>>>,
}
#[async_trait::async_trait]
impl VideoPrepareCloud for Cloud {
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
    }
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>> {
        Ok(vec![CloudVideoScope { brand_id: Some("neutral".into()), store_id: None }])
    }
    async fn create_project(&self, run: &str, _: &CloudVideoScope) -> Result<String> {
        Ok(format!("project-{run}"))
    }
    async fn find_project(&self, _: &str, _: &CloudVideoScope) -> Result<Option<String>> {
        Ok(None)
    }
    async fn create_upload(&self, _: &str, _: &VideoSourceIdentity) -> Result<VideoUploadTicket> {
        Ok(VideoUploadTicket {
            upload_id: "upload".into(),
            url: "https://external.invalid/upload".into(),
            headers: Default::default(),
        })
    }
    async fn upload(&self, _: &VideoUploadTicket, source: &VideoSourceIdentity) -> Result<()> {
        anyhow::ensure!(source.path.is_file(), "missing source");
        Ok(())
    }
    async fn finalize(&self, _: &str, _: &str, _: Option<&str>) -> Result<VideoImportResult> {
        Ok(VideoImportResult::Ready("source".into()))
    }
    async fn probe(&self, _: &str, _: &str) -> Result<VideoMediaInfo> {
        Ok(VideoMediaInfo {
            has_video: true,
            width: 640,
            height: 360,
            duration_sec: 20.,
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
        Ok(
            json!({"durationSec":20,"streams":[{"codec_type":"audio"},{"codec_type":"video","fps":30}]}),
        )
    }
    async fn transcribe(&self, _: &str, _: &str, _: &str, _: Option<&str>) -> Result<Value> {
        Ok(
            json!({"status":"succeeded","text":"first second third","engine":"external-asr","segments":[{"start":0,"end":20,"words":[{"text":"first","start":1,"end":2},{"text":"second","start":3,"end":4},{"text":"third","start":5,"end":6}]}]}),
        )
    }
}
#[async_trait::async_trait]
impl VideoAssetCloud for Cloud {
    async fn submit(&self, _: &VideoAssetRequest, _: &str) -> Result<VideoAssetRemoteState> {
        Ok(VideoAssetRemoteState::Succeeded {
            task_id: "generation".into(),
            reference: "gen:external".into(),
        })
    }
    async fn reconcile(&self, _: &VideoAssetJob) -> Result<VideoAssetRemoteState> {
        Ok(VideoAssetRemoteState::Succeeded {
            task_id: "generation".into(),
            reference: "gen:external".into(),
        })
    }
    async fn verify(&self, _: &str, _: &str) -> Result<()> {
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
                    source_end: 20.,
                    timeline_start: 0.,
                    timeline_end: 20.,
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
            duration_sec: if original { 20. } else { 0. },
        })
    }
    async fn write(
        &self,
        project: &str,
        _: &str,
        write: &VideoProjectWrite,
    ) -> Result<VideoWriteStatus> {
        self.0.writes.lock().unwrap().entry(project.into()).or_default().push(write.clone());
        Ok(VideoWriteStatus::Applied)
    }
    async fn reconcile(&self, _: &str, _: &VideoWriteEntry) -> Result<VideoWriteStatus> {
        Ok(VideoWriteStatus::Unknown)
    }
    async fn preview(&self, project: &str, revision: &str) -> Result<VideoPreviewEvidence> {
        Ok(VideoPreviewEvidence {
            project_id: project.into(),
            revision: revision.into(),
            job_id: "external-preview".into(),
            job_status: "succeeded".into(),
            media_playable: true,
            duration_sec: 20.,
        })
    }
    async fn viewer_url(&self, _: &str) -> Result<String> {
        Ok("https://external.invalid/s/test-preview".into())
    }
}
struct Model;
#[async_trait::async_trait]
impl Provider for Model {
    async fn chat_with_system(&self, _: Option<&str>, _: &str, _: &str, _: f64) -> Result<String> {
        anyhow::bail!("unexpected model interface")
    }
    async fn chat(&self, request: ChatRequest<'_>, _: &str, _: f64) -> Result<ChatResponse> {
        let prompt = request.messages.last().unwrap().content.as_str();
        let text = if prompt.contains("VideoChunkPlan") {
            let marker = "片段：";
            let segment: Value = serde_json::from_str(prompt.split_once(marker).unwrap().1)?;
            let id = segment["chunk_id"].as_str().unwrap();
            let effects=[("keyword","first",1,2,"title"),("b_roll","second",3,4,"scene"),("motion_graphic","third",5,6,"Exact template")].iter().enumerate().map(|(i,(kind,anchor,start,end,asset))|json!({"id":format!("effect-{i}"),"chunk_id":id,"word_index":i,"end_word_index":i,"anchor":anchor,"start":start,"end":end,"visual_type":kind,"assets":[asset]})).collect::<Vec<_>>();
            json!({"chunk_id":id,"effects":effects}).to_string()
        } else if prompt.contains("duration_sec") {
            json!({"scene":{"prompt":"neutral scene","kind":"environment","model":"wan3.0-video","duration_sec":5,"ratio":"16:9","resolution":"720P"}}).to_string()
        } else {
            "bounded external brief".into()
        };
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
