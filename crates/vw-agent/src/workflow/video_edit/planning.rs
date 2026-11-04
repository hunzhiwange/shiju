//! Typed, source-time video planning contract.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualType {
    Keyword,
    BRoll,
    MotionGraphic,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoEditEffect {
    pub id: String,
    pub chunk_id: String,
    pub word_index: u64,
    pub end_word_index: u64,
    pub anchor: String,
    pub start: f64,
    pub end: f64,
    pub visual_type: VisualType,
    pub assets: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoChunkPlan {
    pub chunk_id: String,
    pub effects: Vec<VideoEditEffect>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoEditPlan {
    pub asset_id: String,
    pub duration_sec: f64,
    pub effects: Vec<VideoEditEffect>,
}
/// Global business constraints; types are required across the whole video only.
#[derive(Debug, Clone)]
pub struct VideoPlanRequirements {
    pub instructions: String,
    pub required_types: Vec<VisualType>,
    pub max_effects_per_word: usize,
}

/// Explicit byte budget is conservative for model token limits (UTF-8 bytes >= tokens).
/// The caller supplies a bound supported by the selected model; no text is truncated.
#[derive(Debug, Clone)]
pub struct VideoPlanningContext {
    pub brief: String,
    pub max_prompt_bytes: usize,
    /// Editable template's model object, or None to use the resolved runtime model.
    pub model: Option<serde_json::Value>,
}

/// Run the production iteration/LLM/VariablePool path, then validate atomically.
/// This function has no cloud write or asset submission capability.
pub async fn plan_video_edit(
    runtime: crate::workflow::WorkflowRuntime,
    transcript: &super::transcript::VideoTranscript,
    segments: &[super::segments::VideoSegment],
    requirements: &VideoPlanRequirements,
    context: &VideoPlanningContext,
) -> anyhow::Result<VideoEditPlan> {
    use anyhow::{Context, ensure};
    use serde_json::json;
    use std::collections::BTreeMap;
    use vw_api_types::workflow::{WorkflowRunRequest, WorkflowRunStatus};
    super::merge::validate_segments(transcript, segments)?;
    ensure!(
        !requirements.instructions.trim().is_empty() && !context.brief.trim().is_empty(),
        "missing global requirements or bounded brief"
    );
    ensure!(
        context.max_prompt_bytes > 0 && context.max_prompt_bytes <= 128 * 1024,
        "unsupported model context budget"
    );
    let contract = "Return only a raw JSON VideoChunkPlan object: {\"chunk_id\":\"supplied chunk_id\",\"effects\":[{\"id\":\"stable effect ID\",\"chunk_id\":\"owning chunk_id\",\"word_index\":0,\"end_word_index\":0,\"anchor\":\"exact first word text\",\"start\":0.0,\"end\":1.0,\"visual_type\":\"keyword|b_roll|motion_graphic\",\"assets\":[\"nonempty material description\"]}]}. Use original absolute word times and stable word indices. Publish effects only for your core words; context is read-only. The three visual types are global requirements, do not force all into every chunk. No prose or Markdown.";
    let mut items = Vec::new();
    for segment in segments {
        let prompt = serde_json::to_string(
            &json!({"contract":contract,"requirements":requirements.instructions,"required_types":requirements.required_types,"max_effects_per_word":requirements.max_effects_per_word,"brief":context.brief,"segment":segment}),
        )?;
        ensure!(
            prompt.len() <= context.max_prompt_bytes,
            "chunk {} exceeds model context bound: {} > {} bytes",
            segment.chunk_id,
            prompt.len(),
            context.max_prompt_bytes
        );
        items.push(json!({"chunk_id":segment.chunk_id,"prompt":prompt}));
    }
    let mut model = json!({"type":"llm","prompt_template":[{"role":"user","text":"{{#planning.item.prompt#}}"}]});
    if let Some(value) = &context.model {
        model["model"] = value.clone();
    }
    let graph = json!({"workflow":{"graph":{"nodes":[
        {"id":"start","data":{"type":"start","variables":[{"variable":"items"}]}},
        {"id":"planning","data":{"type":"iteration","parallel":true,"max_parallel":runtime.video_edit_budget.limit(),"input_selector":["start","items"],"output_selector":["model","text"],"graph":{"nodes":[{"id":"model","data":model}],"edges":[]}}}
    ],"edges":[{"source":"start","target":"planning"}]}}});
    let response = crate::workflow::run_workflow(
        runtime,
        WorkflowRunRequest {
            workflow_yaml: Some(serde_json::to_string(&graph)?),
            inputs: BTreeMap::from([("items".into(), json!(items))]),
            max_steps: u32::try_from(
                segments.len().checked_mul(4).context("too many segments")?.max(10),
            )?,
            ..Default::default()
        },
    )
    .await
    .map_err(anyhow::Error::msg)?;
    ensure!(
        response.status == WorkflowRunStatus::Succeeded,
        "segment planning failed: {}",
        response.error.as_deref().unwrap_or("no successful result")
    );
    let output = response
        .nodes
        .iter()
        .find(|n| n.node_id == "planning")
        .context("missing planning iteration output")?;
    let values =
        output.outputs["output"].as_array().context("invalid planning iteration output")?;
    ensure!(values.len() == segments.len(), "incomplete planning output");
    let mut plans = Vec::new();
    for (segment, value) in segments.iter().zip(values) {
        let plan: VideoChunkPlan =
            serde_json::from_str(value.as_str().context("model plan must be JSON text")?)
                .with_context(|| format!("chunk {}: invalid model plan", segment.chunk_id))?;
        ensure!(
            plan.chunk_id == segment.chunk_id,
            "chunk {}: response identity mismatch",
            segment.chunk_id
        );
        plans.push(plan);
    }
    super::merge::merge_video_edit_plans(transcript, segments, plans, requirements)
}
