//! Read-only projection of Pi thinking settings into a native model draft.

use super::{
    metadata::{PiThinkingMetadata, ResolvedModelMetadata},
    pi_thinking::{PiThinkingLevel, PiThinkingLevelMap},
    preset_pi::{PiPresetCompatValue, PiPresetProtocol},
};
use crate::providers::validation;
use reqwest::Url;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::pi::PiProfile,
    model_catalog::{
        ModelCapabilityState, ModelMetadataField, ModelMetadataProvenance, ModelMetadataSource,
    },
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PiModelFill {
    pub changes: Map<String, Value>,
    /// Keys to merge into the model's compat object.
    pub missing_compat: Map<String, Value>,
    /// Thinking level maps report their source under `ReasoningEfforts`.
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Suggest a thinking map using metadata resolved for this app, model ID and effective endpoint.
pub fn fill_pi_model(
    app: AppId,
    provider_id: &str,
    provider: &PiProfile,
    model: &Map<String, Value>,
    resolved: &ResolvedModelMetadata,
) -> PiModelFill {
    let reasoning = match model.get("reasoning") {
        Some(value) => value.as_bool() == Some(true),
        None => resolved.metadata.reasoning == ModelCapabilityState::Supported,
    };
    if app != AppId::Pi || !reasoning || model.contains_key("thinkingLevelMap") {
        return PiModelFill::default();
    }
    project_thinking(provider_id, &provider.declaration, model, resolved).unwrap_or_default()
}

fn project_thinking(
    provider_id: &str,
    provider: &Value,
    model: &Map<String, Value>,
    resolved: &ResolvedModelMetadata,
) -> Option<PiModelFill> {
    use ModelMetadataField::ReasoningEfforts;

    let transport = |key: &str| model.get(key).or_else(|| provider.get(key));
    let protocol = PiPresetProtocol::deserialize(transport("api")?).ok()?;
    let endpoint = parse_endpoint(transport("baseUrl")?.as_str()?)?;
    let empty = Map::new();
    let provider_compat = provider.get("compat").map_or(Some(&empty), Value::as_object)?;
    let model_compat = model.get("compat").map_or(Some(&empty), Value::as_object)?;
    let compat = |key: &str| model_compat.get(key).or_else(|| provider_compat.get(key));

    let (levels, missing_compat, source) = if let Some(preset) = &resolved.pi_thinking {
        let missing = preset_compat(preset, protocol, &endpoint, provider_id, compat)?;
        let source =
            ModelMetadataProvenance { source: ModelMetadataSource::Preset, fetched_at: None };
        (preset.levels.clone(), missing, Some(source))
    } else {
        match protocol {
            PiPresetProtocol::OpenaiResponses => {}
            PiPresetProtocol::OpenaiCompletions
                if completions_thinking(&endpoint, provider_id, compat) == ("openai", true) => {}
            _ => return None,
        }
        let levels = thinking_map_from_efforts(resolved.metadata.reasoning_efforts.as_deref()?)?;
        (levels, Map::new(), resolved.metadata.provenance.get(&ReasoningEfforts).cloned())
    };
    Some(PiModelFill {
        changes: Map::from_iter([("thinkingLevelMap".into(), json!(levels))]),
        missing_compat,
        provenance: source.into_iter().map(|source| (ReasoningEfforts, source)).collect(),
    })
}

fn parse_endpoint(raw: &str) -> Option<Url> {
    validation::endpoint(raw).ok()?;
    Url::parse(raw).ok()
}

fn preset_compat<'a>(
    preset: &PiThinkingMetadata,
    protocol: PiPresetProtocol,
    endpoint: &Url,
    provider_id: &str,
    compat: impl Fn(&str) -> Option<&'a Value>,
) -> Option<Map<String, Value>> {
    let expected_endpoint = parse_endpoint(preset.endpoint)?;
    if preset.protocol != protocol
        || endpoint.origin() != expected_endpoint.origin()
        || endpoint.path().trim_end_matches('/') != expected_endpoint.path().trim_end_matches('/')
    {
        return None;
    }

    let mut expected = Map::new();
    let mut missing = Map::new();
    for (key, value) in preset.compat.iter().flatten() {
        let value = match value {
            PiPresetCompatValue::Flag(flag) => Value::Bool(*flag),
            PiPresetCompatValue::Dialect(dialect) => Value::String(dialect.clone()),
        };
        match compat(key) {
            Some(actual) if *actual != value => return None,
            Some(_) => {}
            None => {
                missing.insert(key.clone(), value.clone());
            }
        }
        expected.insert(key.clone(), value);
    }
    if protocol == PiPresetProtocol::OpenaiCompletions
        && completions_thinking(&expected_endpoint, "", |key| expected.get(key))
            != completions_thinking(endpoint, provider_id, |key| {
                compat(key).or_else(|| missing.get(key))
            })
    {
        return None;
    }
    Some(missing)
}

/// Pi's Chat Completions detection uses provider IDs and URL substring checks.
fn completions_thinking<'a>(
    endpoint: &Url,
    provider_id: &str,
    compat: impl Fn(&str) -> Option<&'a Value>,
) -> (&'a str, bool) {
    let url = endpoint.as_str().to_ascii_lowercase();
    let is = |ids: &[&str], hosts: &[&str]| {
        ids.contains(&provider_id) || hosts.iter().any(|host| url.contains(host))
    };
    let zai = is(&["zai", "zai-coding-cn"], &["api.z.ai", "open.bigmodel.cn"]);
    let together = is(&["together"], &["api.together.ai", "api.together.xyz"]);
    let ant_ling = is(&["ant-ling"], &["api.ant-ling.com"]);
    let format = compat("thinkingFormat").and_then(Value::as_str).unwrap_or_else(|| {
        if is(&["deepseek"], &["deepseek.com"]) {
            "deepseek"
        } else if zai {
            "zai"
        } else if together {
            "together"
        } else if ant_ling {
            "ant-ling"
        } else if is(&["openrouter"], &["openrouter.ai"]) {
            "openrouter"
        } else {
            "openai"
        }
    });
    let effort = compat("supportsReasoningEffort").and_then(Value::as_bool).unwrap_or_else(|| {
        !zai && !together
            && !ant_ling
            && !is(&["xai"], &["api.x.ai"])
            && !is(&["moonshotai", "moonshotai-cn"], &["api.moonshot."])
            && !is(&["cloudflare-ai-gateway"], &["gateway.ai.cloudflare.com"])
            && !is(&["nvidia"], &["integrate.api.nvidia.com"])
    });
    (format, effort)
}

fn thinking_map_from_efforts(efforts: &[String]) -> Option<PiThinkingLevelMap> {
    use PiThinkingLevel::*;

    let map: PiThinkingLevelMap = [
        (Off, "none"),
        (Minimal, "minimal"),
        (Low, "low"),
        (Medium, "medium"),
        (High, "high"),
        (Xhigh, "xhigh"),
        (Max, "max"),
    ]
    .into_iter()
    .map(|(level, effort)| {
        (level, efforts.iter().any(|known| known == effort).then(|| effort.into()))
    })
    .collect();
    map.values().any(Option::is_some).then_some(map)
}
