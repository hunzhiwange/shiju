//! Serial video stages and durable asset lease reconciliation.
use crate::{apply::*, assets::*, manifest::*, planning::*, prepare::*, preview::*, segments::*, transcript::*, budget, merge, types::VideoEditRequest, VideoProjectJournal, ShijuCloudVideoToolProvider};
use agent_database::GlobalDatabase;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
#[derive(Default)]
struct State {
    prepared: Option<PreparedCloudVideo>,
    transcript: Option<VideoTranscript>,
    segments: Vec<VideoSegment>,
    plan: Option<VideoEditPlan>,
    recipes: BTreeMap<String, VideoAssetRecipe>,
    registry: Option<VideoAssetRegistry>,
    delivery: Option<VideoEditDelivery>,
    stage: usize,
}
pub struct ShijuVideoEntryProvider {
    cloud: Arc<dyn VideoEntryCloud>,
    database: GlobalDatabase,
    video: VideoEditRequest,
    snapshot: workflow_core::templates::TemplateSnapshot,
    model: String,
    budget: budget::VideoEditRunBudget,
    state: Mutex<State>,
}
fn requirements(video: &VideoEditRequest) -> VideoPlanRequirements {
    VideoPlanRequirements {
        instructions: video
            .requirements
            .clone()
            .unwrap_or_else(|| "保留原意和原声，精剪、字幕与三类视觉效果".into()),
        required_types: vec![VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic],
        max_effects_per_word: 1,
    }
}

impl ShijuVideoEntryProvider {
    pub fn new(cloud: Arc<dyn VideoEntryCloud>, database: GlobalDatabase, video: VideoEditRequest, snapshot: workflow_core::templates::TemplateSnapshot, model: String, budget: budget::VideoEditRunBudget) -> Self {
        Self { cloud, database, video, snapshot, model, budget, state: Mutex::new(State::default()) }
    }
    pub async fn execute(&self, provider: &str, tool_name: &str, action: Option<&str>, credential_id: Option<&str>, inputs: &BTreeMap<String, Value>) -> Result<(Value, String)> {
        ensure!(
            provider == "shiju_video"
                && action.is_none()
                && credential_id.is_none(),
            "invalid video provider capability"
        );
        let mut s = self.state.lock().await;
        let stages = ["prepare", "segments", "merge", "assets", "apply", "preview"];
        ensure!(
            stages.get(s.stage) == Some(&tool_name),
            "video stage out of order"
        );
        ensure!(!self.budget.is_cancelled(), "cancelled");
        let result = match tool_name {
            "prepare" => {
                ensure!(inputs.is_empty(), "prepare takes only native context");
                let prepared = loop {
                    match prepare_cloud_video(
                        self.database.clone(),
                        self.cloud.as_ref(),
                        &self.video,
                        self.snapshot.record.workflow_yaml.as_bytes(),
                        &self.model,
                        None,
                    )
                    .await
                    {
                        Ok(prepared) => break prepared,
                        Err(e)
                            if e.to_string()
                                == "import still running; resume the same upload/job" =>
                        {
                            self.poll_delay().await?
                        }
                        Err(e) => return Err(e),
                    }
                };
                self.reconcile_before_planning(&prepared).await?;
                let transcript = loop {
                    match transcribe_cloud_video(
                        self.database.clone(),
                        self.cloud.as_ref(),
                        &prepared,
                    )
                    .await
                    {
                        Ok(transcript) => break transcript,
                        Err(e)
                            if e.to_string()
                                == "transcription pending; resume the same cloud task" =>
                        {
                            self.poll_delay().await?
                        }
                        Err(e) => return Err(e),
                    }
                };
                let result = json!({"transcript":transcript,"requirements":requirements(&self.video).instructions,
                    "template_sha256":self.snapshot.digest,"model":self.model,"skill_versions":self.cloud.versions()});
                s.prepared = Some(prepared);
                s.transcript = Some(transcript);
                result
            }
            "segments" => {
                ensure!(inputs.is_empty(), "segments takes no overrides");
                s.segments = segment_video_transcript(
                    s.transcript.as_ref().context("missing transcript")?,
                    VideoSegmentOptions::default(),
                )?;
                json!(s.segments)
            }
            "merge" => {
                ensure!(inputs.len() == 1, "unexpected merge arguments");
                let values = inputs
                    .get("plans")
                    .and_then(Value::as_array)
                    .context("missing chunk plans")?;
                let plans = values
                    .iter()
                    .map(|v| {
                        serde_json::from_str::<VideoChunkPlan>(
                            v.as_str().context("plan must be raw JSON text")?,
                        )
                        .map_err(anyhow::Error::from)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let plan = merge::merge_video_edit_plans(
                    s.transcript.as_ref().context("missing transcript")?,
                    &s.segments,
                    plans,
                    &requirements(&self.video),
                )?;
                let result = json!(plan);
                s.plan = Some(plan);
                result
            }
            "assets" => {
                ensure!(inputs.len() == 1, "unexpected asset arguments");
                s.recipes = serde_json::from_str(
                    inputs
                        .get("recipes")
                        .and_then(Value::as_str)
                        .context("missing explicit recipes")?,
                )?;
                let prepared = s.prepared.as_ref().context("missing preparation")?;
                let project = prepared.manifest.project_id.as_deref().context("missing project")?;
                let plan = s.plan.as_ref().context("missing validated plan")?;
                let broll = VideoEditPlan {
                    effects: plan
                        .effects
                        .iter()
                        .filter(|e| e.visual_type == VisualType::BRoll)
                        .cloned()
                        .collect(),
                    ..plan.clone()
                };
                let requests = video_asset_requests(&broll, project, &s.recipes)?;
                let database = self.database.clone();
                let candidate = prepared.manifest.clone();
                let (store, run) = agent_database::run_isolated(move || {
                    let store = VideoEditManifestStore::new(database)?;
                    let run = store.claim(
                        candidate,
                        &uuid::Uuid::new_v4().to_string(),
                        super::prepare::now_ms(),
                    )?;
                    Ok::<_, anyhow::Error>((store, run))
                })
                .await??;
                let mut lease =
                    AssetLease { store: store.clone(), run, budget: self.budget.clone() };
                let result = async {
                    loop {
                        if let Some(registry) = prepare_video_assets(
                            self.cloud.as_ref(),
                            &store,
                            &mut lease.run,
                            &requests,
                            &self.budget,
                        )
                        .await?
                        {
                            return Ok::<_, anyhow::Error>(registry);
                        }
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {},
                            _ = self.budget.cancelled() => {
                                cancel_video_assets(&store, &mut lease.run, &self.budget).await?;
                                anyhow::bail!("cancelled");
                            }
                        }
                    }
                }
                .await;
                let registry = result?;
                let result = json!(registry.progress);
                s.registry = Some(registry);
                result
            }
            "apply" => {
                ensure!(inputs.is_empty(), "apply takes only validated state");
                let project = s
                    .prepared
                    .as_ref()
                    .and_then(|p| p.manifest.project_id.as_deref())
                    .context("missing project")?;
                let plan = s.plan.as_ref().context("missing plan")?;
                let registry = s.registry.as_ref().context("missing ready assets")?;
                let materials =
                    self.cloud.resolve_video_materials(project, plan, &s.recipes, registry).await?;
                let database = self.database.clone();
                let tenant = self.video.tenant_scope.clone();
                let journal = agent_database::run_isolated(move || {
                    VideoProjectJournal::new(database, tenant)
                })
                .await??;
                let hash = video_plan_sha256(plan)?;
                let cancel = self
                    .cloud_context()
                    .abort_token()
                    .unwrap_or_default();
                let req = requirements(&self.video);
                let input = VideoApplyInput {
                    project_id: project,
                    template_sha256: &self.snapshot.digest,
                    plan_sha256: &hash,
                    plan,
                    transcript: s.transcript.as_ref().context("missing transcript")?,
                    segments: &s.segments,
                    requirements: &req,
                    template_snapshot: &self.snapshot.record.workflow_yaml,
                    registry,
                    materials: &materials,
                    cancel: &cancel,
                };
                let project_cloud = self.cloud.project_cloud(&input);
                let delivery = apply_video_edit(project_cloud.as_ref(), &journal, input).await?;
                drop(project_cloud);
                let result = json!({"project_id":delivery.project_id,"template_sha256":delivery.template_sha256,"plan_sha256":delivery.plan_sha256});
                s.delivery = Some(delivery);
                result
            }
            "preview" => {
                ensure!(inputs.is_empty(), "preview takes only independent evidence");
                let delivery = s.delivery.as_ref().context("missing verified delivery")?;
                super::preview::validate_preview(&delivery.timeline, &delivery.preview)?;
                super::preview::validate_viewer_url(&delivery.viewer_url)?;
                json!({"viewer_url":delivery.viewer_url,"template_sha256":delivery.template_sha256,"plan_sha256":delivery.plan_sha256})
            }
            _ => anyhow::bail!("unsupported stage"),
        };
        s.stage += 1;
        let text = if tool_name == "preview" {
            format!(
                "云端剪辑已完成并核验预览：{}",
                result["viewer_url"].as_str().context("missing viewer")?
            )
        } else {
            serde_json::to_string(&result)?
        };
        Ok((result, text))
    }
    async fn reconcile_before_planning(&self, prepared: &PreparedCloudVideo) -> Result<()> {
        if !prepared.manifest.asset_cancelled
            && prepared.manifest.asset_jobs.iter().all(|job| job.state.is_terminal())
        {
            return Ok(());
        }
        // A fresh runtime must settle old accepted work before acquiring model permits.
        // In particular, a cancelled run may reconcile, but must never plan or dispatch again.
        let database = self.database.clone();
        let candidate = prepared.manifest.clone();
        let (store, run) = agent_database::run_isolated(move || {
            let store = VideoEditManifestStore::new(database)?;
            let run = store.claim(
                candidate,
                &uuid::Uuid::new_v4().to_string(),
                super::prepare::now_ms(),
            )?;
            Ok::<_, anyhow::Error>((store, run))
        })
        .await??;
        let mut lease = AssetLease { store: store.clone(), run, budget: self.budget.clone() };
        let requests =
            lease.run.asset_jobs.iter().map(|job| job.request.clone()).collect::<Vec<_>>();
        loop {
            if prepare_video_assets(
                self.cloud.as_ref(),
                &store,
                &mut lease.run,
                &requests,
                &self.budget,
            )
            .await?
            .is_some()
            {
                return Ok(());
            }
            self.poll_delay().await?;
        }
    }
    async fn poll_delay(&self) -> Result<()> {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => Ok(()),
            _ = self.budget.cancelled() => anyhow::bail!("cancelled"),
        }
    }
    fn cloud_context(&self) -> crate::context::VideoContextHandle {
        VideoPrepareCloud::context(self.cloud.as_ref())
    }
}

/// Replace only the external transport. Parsing, tenant storage, algorithms,
/// workflow scheduling, write journal and preview validation remain production code.
#[async_trait::async_trait]
pub trait VideoEntryCloud:
    VideoPrepareCloud + VideoTranscriptCloud + VideoAssetCloud
{
    fn versions(&self) -> (&str, &str);
    async fn resolve_video_materials(
        &self,
        project: &str,
        plan: &VideoEditPlan,
        recipes: &BTreeMap<String, VideoAssetRecipe>,
        registry: &VideoAssetRegistry,
    ) -> Result<BTreeMap<String, VideoEffectMaterial>>;
    fn project_cloud<'a>(&'a self, input: &VideoApplyInput<'a>) -> Box<dyn VideoProjectCloud + 'a>;
}
#[async_trait::async_trait]
impl VideoEntryCloud for ShijuCloudVideoToolProvider {
    fn versions(&self) -> (&str, &str) {
        self.versions()
    }
    async fn resolve_video_materials(
        &self,
        project: &str,
        plan: &VideoEditPlan,
        recipes: &BTreeMap<String, VideoAssetRecipe>,
        registry: &VideoAssetRegistry,
    ) -> Result<BTreeMap<String, VideoEffectMaterial>> {
        self.resolve_video_materials(project, plan, recipes, registry).await
    }
    fn project_cloud<'a>(&'a self, input: &VideoApplyInput<'a>) -> Box<dyn VideoProjectCloud + 'a> {
        Box::new(self.project_cloud(input))
    }
}

struct AssetLease {
    store: VideoEditManifestStore,
    run: VideoEditRunManifest,
    budget: budget::VideoEditRunBudget,
}
impl Drop for AssetLease {
    fn drop(&mut self) {
        let store = self.store.clone();
        let run = self.run.clone();
        let cancelled = self.budget.is_cancelled();
        if agent_database::run_sync_database(move || {
            if cancelled {
                store.cancel_and_release(&run, super::prepare::now_ms())
            } else {
                store.release(&run)
            }
        })
        .is_err()
        {
            tracing::warn!("video asset lease needs expiry/reconciliation");
        }
    }
}

pub fn command_query(query: &str) -> String {
    let (text, selection) =
        query.split_once("\n\n<session_control_selection>\n").unwrap_or((query, ""));
    let selected =
        selection.split_once("</session_control_selection>").map(|(s, _)| s).unwrap_or("");
    let shiju = selected
        .lines()
        .filter_map(|l| l.strip_prefix("技能："))
        .flat_map(|s| s.split(','))
        .any(|s| s.trim() == "shiju");
    if !text.trim_start().starts_with('/') && shiju {
        format!("/shiju {}", text.trim())
    } else {
        text.trim().to_owned()
    }
}


/// Native request values needed for one video run; no session processor ownership.
pub struct VideoEntryInput<'a> {
    pub query: &'a str,
    pub acp_test: bool,
    pub allowed_tools: Option<Value>,
    pub model: Option<String>,
    pub abort_token: Option<tokio_util::sync::CancellationToken>,
}

pub struct VideoEntryServices {
    pub cloud: Arc<dyn VideoEntryCloud>,
    pub provider: Arc<dyn model_types::Provider>,
}

/// Only application-owned database location and capability-filtered services are injected.
#[async_trait::async_trait]
pub trait VideoEntryHost: Send + Sync {
    fn database(&self) -> Result<GlobalDatabase>;
    async fn services(&self, allowed_tools: Option<Value>) -> Result<VideoEntryServices>;
}

pub struct VideoEntryBuild {
    pub tools: ShijuVideoEntryProvider,
    pub budget: budget::VideoEditRunBudget,
    pub provider: Arc<dyn model_types::Provider>,
    pub model: String,
    pub request: vw_api_workflow::WorkflowRunRequest,
}

pub async fn build_entry(
    input: VideoEntryInput<'_>,
    context: &dyn crate::context::VideoContext,
    host: &dyn VideoEntryHost,
) -> Result<VideoEntryBuild> {
    let query = command_query(input.query);
    let video = crate::command::prepare_video_edit_request(&query, context)?.context("not a video command")?;
    ensure!(!input.acp_test, "剪辑仅支持原生会话");
    let database = host.database()?;
    let snapshot = workflow_core::templates::VideoEditTemplateSeed::production()?
        .load(database.clone(), video.tenant_scope.clone()).await?;
    let cancel = input.abort_token.unwrap_or_default();
    let budget = budget::VideoEditRunBudget::new(usize::from(snapshot.options.max_parallel()))
        .map_err(anyhow::Error::msg)?.with_cancellation(cancel);
    let services = host.services(input.allowed_tools).await?;
    let model = input.model.filter(|m| !m.trim().is_empty()).context("缺少已解析的原生模型")?;
    let yaml = snapshot.record.workflow_yaml.clone();
    let tools = ShijuVideoEntryProvider::new(services.cloud, database, video, snapshot, model.clone(), budget.clone());
    Ok(VideoEntryBuild {
        tools, budget, provider: services.provider, model,
        request: vw_api_workflow::WorkflowRunRequest {
            workflow_yaml: Some(yaml), query: Some(query), max_steps: 10_000,
            ..Default::default()
        },
    })
}
