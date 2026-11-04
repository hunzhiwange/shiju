//! Native delivery boundary. Every call is scoped through the official caller's
//! project rebinding and the original native permission/process lifecycle.
use super::*;
use crate::workflow::video_edit::{apply::*, assets::*, planning::*, preview::*, transcript::*};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub struct ShijuVideoProjectCloud<'a> {
    pub(super) provider: &'a ShijuCloudVideoToolProvider,
    pub(super) transcript: &'a VideoTranscript,
    pub(super) plan: &'a VideoEditPlan,
    pub(super) registry: &'a VideoAssetRegistry,
    pub(super) materials: &'a BTreeMap<String, VideoEffectMaterial>,
}
impl ShijuCloudVideoToolProvider {
    pub fn project_cloud<'a>(&'a self, input: &VideoApplyInput<'a>) -> ShijuVideoProjectCloud<'a> {
        ShijuVideoProjectCloud {
            provider: self,
            transcript: input.transcript,
            plan: input.plan,
            registry: input.registry,
            materials: input.materials,
        }
    }
    /// Production entry: derive material identities again before acquiring the writer fence.
    pub async fn deliver_video_edit<'a>(
        &'a self,
        journal: &dyn VideoApplyJournal,
        input: VideoApplyInput<'a>,
        recipes: &BTreeMap<String, VideoAssetRecipe>,
    ) -> Result<VideoEditDelivery> {
        let resolved = self
            .resolve_video_materials(input.project_id, input.plan, recipes, input.registry)
            .await?;
        ensure!(
            resolved == *input.materials,
            "delivery material choices differ from the validated plan/recipes"
        );
        let cloud = self.project_cloud(&input);
        apply_video_edit(&cloud, journal, input).await
    }
    /// Material descriptions must resolve to actual generation recipes or exact template names.
    pub async fn resolve_video_materials(
        &self,
        project: &str,
        plan: &VideoEditPlan,
        recipes: &BTreeMap<String, VideoAssetRecipe>,
        registry: &VideoAssetRegistry,
    ) -> Result<BTreeMap<String, VideoEffectMaterial>> {
        let mut materials = BTreeMap::new();
        for effect in &plan.effects {
            let material = match effect.visual_type {
                VisualType::Keyword => VideoEffectMaterial::Keyword,
                VisualType::BRoll => {
                    ensure!(
                        effect.assets.len() == 1,
                        "multiple B-roll materials require an explicit composition"
                    );
                    let one = VideoEditPlan { effects: vec![effect.clone()], ..plan.clone() };
                    let requests = video_asset_requests(&one, project, recipes)?;
                    ensure!(requests.len() == 1, "ambiguous B-roll recipe");
                    let digest = requests[0].digest.clone();
                    ensure!(registry.references.contains_key(&digest), "B-roll material not ready");
                    VideoEffectMaterial::GeneratedVideo { digest }
                }
                VisualType::MotionGraphic => {
                    ensure!(
                        effect.assets.len() == 1,
                        "one exact motion-graphic template is required"
                    );
                    let name = &effect.assets[0];
                    let v = self
                        .delivery_call(
                            "search_templates",
                            json!({"editorProjectId":project,"query":name}),
                        )
                        .await?;
                    let matches = v["result"]
                        .as_array()
                        .context("invalid template search")?
                        .iter()
                        .filter(|v| v["name"] == *name)
                        .count();
                    ensure!(
                        matches == 1,
                        "motion graphic description is not one exact available template name"
                    );
                    VideoEffectMaterial::MotionGraphic { template_name: name.clone() }
                }
            };
            ensure!(
                materials.insert(effect.id.clone(), material).is_none(),
                "duplicate effect identity"
            );
        }
        Ok(materials)
    }
    pub(super) async fn delivery_call(&self, operation: &str, args: Value) -> Result<Value> {
        ensure!(
            [
                "read_transcript",
                "read_captions",
                "read_project",
                "inspect_asset",
                "import_media",
                "edit_item",
                "manage_transcript",
                "clean_script",
                "edit_captions",
                "edit_track",
                "add_motion_graphic",
                "search_templates",
                "watch_timeline",
                "get_share_link",
                "track_progress",
                "media_probe"
            ]
            .contains(&operation),
            "unsupported delivery operation"
        );
        let schema: Value = serde_json::from_str(
            &self
                .occ_capture(&["schema".into(), operation.into()])
                .await
                .map_err(anyhow::Error::msg)?,
        )?;
        ensure!(schema["name"] == operation, "delivery schema identity changed");
        validate_arguments(&schema["inputSchema"], &args)?;
        let value =
            self.memory_exchange(json!({"kind":"call","operation":operation,"args":args})).await?;
        ensure!(
            value.is_object() && value.get("error").is_none() && value["outcome"] != "rejected",
            "delivery call failed; reconcile before retry"
        );
        Ok(value)
    }
}
fn validate_arguments(schema: &Value, args: &Value) -> Result<()> {
    if let Some(types) = schema["anyOf"].as_array() {
        ensure!(
            types.iter().any(|s| validate_arguments(s, args).is_ok()),
            "delivery schema union changed"
        );
        return Ok(());
    }
    let valid = match schema["type"].as_str() {
        Some("object") => args.is_object(),
        Some("array") => args.is_array(),
        Some("string") => args.is_string(),
        Some("integer") => args.is_i64() || args.is_u64(),
        Some("number") => args.is_number(),
        Some("boolean") => args.is_boolean(),
        _ => false,
    };
    ensure!(valid, "delivery schema type changed");
    if let Some(values) = schema["enum"].as_array() {
        ensure!(values.contains(args), "delivery enum changed");
    }
    if let Some(object) = args.as_object() {
        let Some(props) = schema["properties"].as_object() else { return Ok(()) };
        for (k, v) in object {
            validate_arguments(props.get(k).context("unsupported delivery argument")?, v)?;
        }
        if let Some(required) = schema["required"].as_array() {
            ensure!(
                required.iter().all(|k| k.as_str().is_some_and(|k| object.contains_key(k))),
                "missing required delivery argument"
            );
        }
    }
    // Dynamic item/word objects are validated by the remote operation. Only those
    // opaque object payloads (the official schema has no properties) cross unchanged.
    if let Some(array) = args.as_array() {
        for value in array {
            if schema["items"]["type"] == "object" && schema["items"].get("properties").is_none() {
                ensure!(value.is_object(), "invalid delivery array item");
            } else {
                validate_arguments(&schema["items"], value)?;
            }
        }
    }
    Ok(())
}
pub(super) fn text_field<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key].as_str().filter(|s| !s.is_empty()).context("missing cloud delivery identity")
}
pub(super) fn number(v: &Value, key: &str) -> Result<f64> {
    let n = v[key].as_f64().context("missing cloud media coordinate")?;
    ensure!(n.is_finite() && n >= 0.0, "invalid cloud media coordinate");
    Ok(n)
}
pub(super) fn import_name(reference: &str) -> String {
    format!("video-edit-{}", hex::encode(Sha256::digest(reference.as_bytes())))
}
pub(super) fn track_name(effect: &VideoEditEffect) -> String {
    format!("{} · {}", effect.anchor, effect.id)
}
impl ShijuVideoProjectCloud<'_> {
    pub(super) async fn project(&self, project: &str) -> Result<Value> {
        let mut value =
            self.provider.delivery_call("read_project", json!({"editorProjectId":project})).await?;
        ensure!(
            value["ok"] == true && value["projectId"] == project,
            "delivery project identity mismatch"
        );
        ensure!(
            value["timeline"]["items"].is_array(),
            "archived/incomplete timeline is unsupported; do not deliver"
        );
        let t = &value["timeline"];
        if t["captions"]["enabled"] == true
            || t["captionTracks"]
                .as_array()
                .is_some_and(|cs| cs.iter().any(|c| c["enabled"] == true))
        {
            let c =
                self.provider.delivery_call("read_captions", json!({"projectId":project})).await?;
            value["timeline"]["_delivery_caption_evidence"] = json!({"enabled":c["enabled"],"complete":c["complete"],"archived":c["archived"],"cards":c["cards"]});
        }
        Ok(value)
    }
    pub(super) fn pool_asset<'a>(&self, project: &'a Value, reference: &str) -> Result<&'a str> {
        let assets = project["mediaPool"]["assets"].as_array().context("missing media pool")?;
        let expected = import_name(reference);
        let mut matches = assets.iter().filter(|a| a["name"] == expected && a["offline"] == false);
        let asset = matches.next().context("imported material not independently readable")?;
        ensure!(matches.next().is_none(), "ambiguous imported material");
        text_field(asset, "id")
    }
    pub(super) fn effect_item<'a>(
        &self,
        project: &'a Value,
        effect: &VideoEditEffect,
    ) -> Result<Option<&'a Value>> {
        let tracks = project["timeline"]["tracks"].as_array().context("missing tracks")?;
        let name = track_name(effect);
        let mut matches = tracks.iter().filter(|t| t["name"] == name);
        let Some(track) = matches.next() else { return Ok(None) };
        ensure!(
            matches.next().is_none() && track["hidden"] == false,
            "effect track ambiguous or hidden"
        );
        let id = text_field(track, "id")?;
        let mut items = project["timeline"]["items"]
            .as_array()
            .context("missing items")?
            .iter()
            .filter(|i| i["trackId"] == id);
        let item = items.next();
        ensure!(items.next().is_none(), "multiple items on managed effect track");
        Ok(item)
    }
    pub(super) async fn mutation(&self, op: &str, args: Value) -> Result<VideoWriteStatus> {
        let v = self.provider.delivery_call(op, args).await?;
        if v["code"] == "NO_CHANGE" || v["outcome"] == "no_change" {
            return Ok(VideoWriteStatus::NoChange);
        }
        if v["automaticCaptionAvoidance"]["ok"] == false {
            return Ok(VideoWriteStatus::Failed);
        }
        if op == "clean_script"
            && v["ok"] == true
            && v["previewOnly"] == false
            && v["candidates"].as_array().is_some_and(Vec::is_empty)
            && v["applied"].as_array().is_some_and(Vec::is_empty)
            && v["stats"]["candidates"] == 0
            && v["stats"]["applied"] == 0
        {
            return Ok(VideoWriteStatus::VerifiedNoOp);
        }
        ensure!(
            v["autoEditSession"]["committed"] == true
                && v["autoEditSession"]["status"] == "applied",
            "mutation not confirmed applied"
        );
        Ok(VideoWriteStatus::Applied)
    }
}
#[path = "cloud_delivery_read.rs"]
mod read;
#[path = "cloud_delivery_write.rs"]
mod write;
