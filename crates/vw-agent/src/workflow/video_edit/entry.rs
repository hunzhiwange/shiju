//! Native slash entry. State belongs to one immutable tenant/template run.
use super::{
    apply::*, assets::*, manifest::*, planning::*, prepare::*, preview::*, segments::*,
    transcript::*, *,
};
use crate::{
    global_database::GlobalDatabase, skills::Skill, tools::ToolRuntimeContext, workflow::*,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

pub(crate) fn command_query(query: &str) -> String {
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

pub(crate) async fn build(
    request: &crate::session::processor::Request,
    context: &ToolRuntimeContext,
    skills: &[Skill],
) -> Result<(WorkflowRuntime, vw_api_types::workflow::WorkflowRunRequest)> {
    let query = command_query(&request.query);
    let video = prepare_video_edit_request(&query, context)?.context("not a video command")?;
    ensure!(
        request.options.get("acp_test").and_then(Value::as_bool) != Some(true),
        "剪辑仅支持原生会话"
    );
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let snapshot = crate::workflow::templates::VideoEditTemplateSeed::production()?
        .load(database.clone(), video.tenant_scope.clone())
        .await?;
    let cancel = request.abort_token.clone().unwrap_or_default();
    let budget = budget::VideoEditRunBudget::new(usize::from(snapshot.options.max_parallel()))
        .map_err(anyhow::Error::msg)?
        .with_cancellation(cancel.clone());
    #[cfg(test)]
    let injected = super::entry_test_transport::get(&request.session, context);
    #[cfg(not(test))]
    let injected: Option<(Arc<dyn VideoEntryCloud>, Arc<dyn crate::providers::Provider>)> = None;
    let (cloud, provider) = if let Some(external) = injected {
        external
    } else {
        let cloud = ShijuCloudVideoToolProvider::discover(
            context.clone(),
            skills,
            request.options.get("allowed_tools").cloned(),
        )
        .await
        .map_err(anyhow::Error::msg)?;
        (
            Arc::new(cloud) as Arc<dyn VideoEntryCloud>,
            Arc::new(crate::providers::session_llm::SessionLlmProvider::new())
                as Arc<dyn crate::providers::Provider>,
        )
    };
    let model =
        request.model.clone().filter(|m| !m.trim().is_empty()).context("缺少已解析的原生模型")?;
    let yaml = snapshot.record.workflow_yaml.clone();
    let tools = ShijuVideoEntryProvider {
        cloud,
        database,
        video,
        snapshot,
        model: model.clone(),
        budget: budget.clone(),
        state: Mutex::new(State::default()),
    };
    let runtime = WorkflowRuntime {
        video_edit_budget: budget,
        provider,
        model,
        temperature: 0.2,
        tool_provider: Some(Arc::new(tools)),
        agent_provider: None,
        knowledge_provider: None,
        document_extractor: None,
        pause_store: None,
    };
    Ok((
        runtime,
        vw_api_types::workflow::WorkflowRunRequest {
            workflow_yaml: Some(yaml),
            query: Some(query),
            max_steps: 10_000,
            ..Default::default()
        },
    ))
}

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
struct ShijuVideoEntryProvider {
    cloud: Arc<dyn VideoEntryCloud>,
    database: GlobalDatabase,
    video: VideoEditRequest,
    snapshot: crate::workflow::templates::TemplateSnapshot,
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

#[async_trait::async_trait]
impl WorkflowToolProvider for ShijuVideoEntryProvider {
    async fn call(
        &self,
        request: WorkflowToolRequest,
    ) -> std::result::Result<WorkflowToolResult, String> {
        self.execute(request).await.map_err(|e| format!("云端剪辑失败: {e:#}"))
    }
}
impl ShijuVideoEntryProvider {
    async fn execute(&self, request: WorkflowToolRequest) -> Result<WorkflowToolResult> {
        ensure!(
            request.provider == "shiju_video"
                && request.action.is_none()
                && request.credential_id.is_none(),
            "invalid video provider capability"
        );
        let mut s = self.state.lock().await;
        let stages = ["prepare", "segments", "merge", "assets", "apply", "preview"];
        ensure!(
            stages.get(s.stage) == Some(&request.tool_name.as_str()),
            "video stage out of order"
        );
        ensure!(!self.budget.is_cancelled(), "cancelled");
        let result = match request.tool_name.as_str() {
            "prepare" => {
                ensure!(request.inputs.is_empty(), "prepare takes only native context");
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
                ensure!(request.inputs.is_empty(), "segments takes no overrides");
                s.segments = segment_video_transcript(
                    s.transcript.as_ref().context("missing transcript")?,
                    VideoSegmentOptions::default(),
                )?;
                json!(s.segments)
            }
            "merge" => {
                ensure!(request.inputs.len() == 1, "unexpected merge arguments");
                let values = request
                    .inputs
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
                ensure!(request.inputs.len() == 1, "unexpected asset arguments");
                s.recipes = serde_json::from_str(
                    request
                        .inputs
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
                let (store, run) = crate::global_database::run_isolated(move || {
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
                ensure!(request.inputs.is_empty(), "apply takes only validated state");
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
                let journal = crate::global_database::run_isolated(move || {
                    VideoProjectJournal::new(database, tenant)
                })
                .await??;
                let hash = video_plan_sha256(plan)?;
                let cancel = self
                    .cloud_context()
                    .tool_use_context()
                    .abort_token()
                    .cloned()
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
                ensure!(request.inputs.is_empty(), "preview takes only independent evidence");
                let delivery = s.delivery.as_ref().context("missing verified delivery")?;
                super::preview::validate_preview(&delivery.timeline, &delivery.preview)?;
                super::preview::validate_viewer_url(&delivery.viewer_url)?;
                json!({"viewer_url":delivery.viewer_url,"template_sha256":delivery.template_sha256,"plan_sha256":delivery.plan_sha256})
            }
            _ => anyhow::bail!("unsupported stage"),
        };
        s.stage += 1;
        let text = if request.tool_name == "preview" {
            format!(
                "云端剪辑已完成并核验预览：{}",
                result["viewer_url"].as_str().context("missing viewer")?
            )
        } else {
            serde_json::to_string(&result)?
        };
        Ok(WorkflowToolResult {
            result: result.clone(),
            json: Some(result),
            text: Some(text),
            files: vec![],
        })
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
        let (store, run) = crate::global_database::run_isolated(move || {
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
    fn cloud_context(&self) -> ToolRuntimeContext {
        VideoPrepareCloud::context(self.cloud.as_ref()).clone()
    }
}

/// Replace only the external transport. Parsing, tenant storage, algorithms,
/// workflow scheduling, write journal and preview validation remain production code.
#[async_trait::async_trait]
pub(super) trait VideoEntryCloud:
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
        if crate::global_database::run_sync_database(move || {
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
