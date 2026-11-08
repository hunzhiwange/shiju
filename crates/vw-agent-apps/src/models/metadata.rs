//! Offline model metadata lookup with deployment-specific fields and vendor fallback.

use super::{
    identity::{MetadataEndpointKey, find_metadata_model},
    pi_thinking::PiThinkingLevelMap,
    preset_catalog::{
        codex_presets, hermes_mcode_presets, kimi_dsh_presets, opencode_openclaw_presets,
        pi_presets,
    },
    preset_kimi_dsh::KimiDshCatalog,
    preset_pi::{PiModelInput, PiPresetCompatValue, PiPresetProtocol},
    vendor_fallback,
};
use std::{collections::BTreeMap, time::UNIX_EPOCH};
use vw_api_types::{
    agent_apps::AppId,
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState, ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance,
        ModelMetadataSource, ModelTokenCost,
    },
};
use vw_provider_resolver::models::{Model, ModelsSnapshot, ReasoningOption};

#[derive(Debug, PartialEq)]
pub struct ResolvedModelMetadata {
    pub metadata: ModelMetadataDto,
    pub pi_thinking: Option<PiThinkingMetadata>,
}

/// The preset's native Pi mapping and the wire settings it was reviewed against.
#[derive(Debug, PartialEq, Eq)]
pub struct PiThinkingMetadata {
    pub levels: PiThinkingLevelMap,
    pub protocol: PiPresetProtocol,
    pub endpoint: &'static str,
    pub compat: Option<BTreeMap<String, PiPresetCompatValue>>,
}

/// Fill from a matching preset, then from the same endpoint in the resolver snapshot.
/// An unambiguous original vendor can fill remaining fields other than price.
/// Endpoint and model normalization here are only for metadata lookup.
pub fn resolve_model_metadata(
    app: AppId,
    endpoint: &str,
    model_id: &str,
    snapshot: Option<&ModelsSnapshot>,
) -> Option<ResolvedModelMetadata> {
    let id = model_id.trim();
    if id.is_empty() {
        return None;
    }
    let key = MetadataEndpointKey::parse(endpoint)?;
    let mut result =
        ResolvedModelMetadata { metadata: ModelMetadataDto::default(), pi_thinking: None };
    if let Some(preset) = preset_metadata(app, &key, id) {
        fill_missing(&mut result.metadata, preset.metadata, ModelMetadataSource::Preset, None);
        result.pi_thinking = preset.pi_thinking;
    }
    if let Some(snapshot) = snapshot {
        let fetched_at = snapshot.fetched_at.and_then(|time| {
            let millis = time.duration_since(UNIX_EPOCH).ok()?.as_millis();
            i64::try_from(millis).ok().map(TimestampMs)
        });
        if let Some(model) = provider_model(snapshot, &key, id) {
            fill_missing(
                &mut result.metadata,
                models_dev_metadata(model),
                ModelMetadataSource::ModelsDev,
                fetched_at,
            );
        }
        if let Some(model) = vendor_fallback::find(snapshot, id) {
            let mut borrowed = models_dev_metadata(model);
            borrowed.cost = None;
            borrowed.context_window = borrowed
                .context_window
                .map(|window| vendor_fallback::cap_gpt_context_window(id, window));
            fill_missing(
                &mut result.metadata,
                borrowed,
                ModelMetadataSource::ModelsDev,
                fetched_at,
            );
        }
    }
    (!result.metadata.provenance.is_empty() || result.pi_thinking.is_some()).then_some(result)
}

fn preset_metadata(
    app: AppId,
    endpoint: &MetadataEndpointKey,
    id: &str,
) -> Option<ResolvedModelMetadata> {
    let matches = |raw| MetadataEndpointKey::parse(raw).as_ref() == Some(endpoint);
    let metadata = match app {
        AppId::Codex => codex_presets(app).find_map(|preset| {
            if !preset
                .base_url
                .as_deref()
                .into_iter()
                .chain(preset.endpoint_candidates.iter().map(String::as_str))
                .any(matches)
            {
                return None;
            }
            let row = find_metadata_model(
                id,
                preset.model_catalog.iter().map(|r| (r.model.as_str(), r)),
            )?;
            nonempty(ModelMetadataDto {
                context_window: positive(row.context_window),
                reasoning_efforts: strings(row.reasoning_levels.iter().flatten()),
                input_modalities: strings(row.input_modalities.iter().flatten()),
                tool_calls: capability(row.supports_parallel_tool_calls.filter(|value| *value)),
                ..Default::default()
            })
        }),
        AppId::OpenCode | AppId::OpenClaw => {
            opencode_openclaw_presets(app)
                .filter(|preset| preset.base_url.as_deref().is_some_and(matches))
                .find_map(|preset| {
                    let row =
                        find_metadata_model(id, preset.models.iter().map(|r| (r.id.as_str(), r)))?;
                    nonempty(ModelMetadataDto {
                        context_window: positive(row.context_window),
                        max_output_tokens: positive(row.max_output_tokens),
                        input_modalities: strings(row.input_modalities.iter().flatten()),
                        output_modalities: strings(row.output_modalities.iter().flatten()),
                        reasoning: capability(row.reasoning),
                        reasoning_efforts: strings(row.compat.iter().flat_map(|compat| {
                            compat.supported_reasoning_efforts.iter().flatten()
                        })),
                        tool_calls: capability(
                            row.compat.as_ref().and_then(|compat| compat.supports_tools),
                        ),
                        cost: row.cost.clone(),
                        ..Default::default()
                    })
                })
        }
        AppId::Hermes | AppId::Mcode => hermes_mcode_presets(app)
            .filter(|preset| matches(&preset.base_url))
            .find_map(|preset| {
                let row =
                    find_metadata_model(id, preset.models.iter().map(|r| (r.id.as_str(), r)))?;
                nonempty(ModelMetadataDto {
                    context_window: positive(row.context_window),
                    max_output_tokens: positive(row.max_output_tokens),
                    input_modalities: strings(row.input_modalities.iter().flatten()),
                    output_modalities: strings(row.output_modalities.iter().flatten()),
                    reasoning: capability(row.reasoning),
                    reasoning_efforts: strings(row.effort_options.iter().flatten()),
                    ..Default::default()
                })
            }),
        AppId::Pi => {
            return pi_presets(app)
                .map(|preset| &preset.settings_config)
                .filter(|config| matches(&config.base_url))
                .find_map(|config| {
                    let row =
                        find_metadata_model(id, config.models.iter().map(|r| (r.id.as_str(), r)))?;
                    let caps = &row.capabilities;
                    // A normalized lookup can supply generic metadata. The wire mapping is exact-ID only.
                    let pi_thinking = if row.id == id {
                        row.thinking_level_map().map(|levels| PiThinkingMetadata {
                            levels,
                            protocol: config.api,
                            endpoint: &config.base_url,
                            compat: row.compat.clone(),
                        })
                    } else {
                        None
                    };
                    Some(ResolvedModelMetadata {
                        metadata: ModelMetadataDto {
                            context_window: positive(Some(caps.context_window)),
                            max_output_tokens: positive(Some(caps.max_tokens)),
                            reasoning: capability(Some(caps.reasoning)),
                            input_modalities: strings(caps.input.iter().map(|mode| match mode {
                                PiModelInput::Text => "text",
                                PiModelInput::Image => "image",
                            })),
                            ..Default::default()
                        },
                        pi_thinking,
                    })
                });
        }
        AppId::Kimi => kimi_dsh_presets(app).find_map(|preset| {
            let KimiDshCatalog::Kimi(catalog) = &preset.catalog else {
                return None;
            };
            if !matches(&catalog.provider.base_url) {
                return None;
            }
            let row =
                find_metadata_model(id, catalog.models.values().map(|r| (r.model.as_str(), r)))?;
            let has = |value| row.capabilities.iter().any(|capability| capability == value);
            nonempty(ModelMetadataDto {
                context_window: positive(Some(row.max_context_size)),
                max_output_tokens: positive(row.max_output_size),
                reasoning: capability(Some(has("thinking"))),
                tool_calls: capability(Some(has("tool_use"))),
                reasoning_efforts: strings(row.support_efforts.iter().flatten()),
                input_modalities: has("image_in").then(|| vec!["text".into(), "image".into()]),
                ..Default::default()
            })
        }),
        _ => None,
    }?;
    Some(ResolvedModelMetadata { metadata, pi_thinking: None })
}

fn provider_model<'a>(
    snapshot: &'a ModelsSnapshot,
    endpoint: &MetadataEndpointKey,
    id: &str,
) -> Option<&'a Model> {
    let mut matches = snapshot
        .providers
        .iter()
        .filter(|(key, provider)| {
            // The resolver overlays Shiju's local data. It did not come from models.dev.
            key.as_str() != "shiju"
                && provider.api.as_deref().and_then(MetadataEndpointKey::parse).as_ref()
                    == Some(endpoint)
        })
        .filter_map(|(_, provider)| {
            provider.models.get(id).or_else(|| {
                // Keep normalized ties stable even though the resolver map is unordered.
                let mut rows: Vec<_> = provider.models.iter().collect();
                rows.sort_unstable_by_key(|(id, _)| *id);
                find_metadata_model(id, rows.into_iter().map(|(id, model)| (id.as_str(), model)))
            })
        });
    let model = matches.next()?;
    // Some metered and subscription deployments share both the endpoint and the model ID.
    matches.next().is_none().then_some(model)
}

fn models_dev_metadata(model: &Model) -> ModelMetadataDto {
    let efforts = model
        .reasoning_options
        .iter()
        .flatten()
        .filter_map(|option| match option {
            ReasoningOption::Effort { values } => Some(values),
            _ => None,
        })
        .flatten();
    ModelMetadataDto {
        context_window: positive(Some(model.limit.context)),
        max_output_tokens: positive(Some(model.limit.output)),
        reasoning: capability(model.reasoning),
        tool_calls: capability(model.tool_call),
        reasoning_efforts: strings(efforts),
        input_modalities: strings(model.modalities.iter().flat_map(|m| &m.input)),
        output_modalities: strings(model.modalities.iter().flat_map(|m| &m.output)),
        cost: model.cost.as_ref().and_then(|cost| {
            Some(ModelTokenCost {
                input: price(cost.input)?,
                output: price(cost.output)?,
                cache_read: price(cost.cache_read),
                cache_write: price(cost.cache_write),
            })
        }),
        ..Default::default()
    }
}

fn nonempty(metadata: ModelMetadataDto) -> Option<ModelMetadataDto> {
    (metadata != ModelMetadataDto::default()).then_some(metadata)
}

fn positive(value: Option<u64>) -> Option<u64> {
    value.filter(|value| *value > 0)
}

fn price(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

fn strings(values: impl IntoIterator<Item = impl AsRef<str>>) -> Option<Vec<String>> {
    let mut list = Vec::new();
    for value in values {
        let value = value.as_ref().trim().to_lowercase();
        if !value.is_empty() && !list.contains(&value) {
            list.push(value);
        }
    }
    (!list.is_empty()).then_some(list)
}

fn capability(value: Option<bool>) -> ModelCapabilityState {
    match value {
        Some(true) => ModelCapabilityState::Supported,
        Some(false) => ModelCapabilityState::Unsupported,
        None => ModelCapabilityState::Unknown,
    }
}

fn fill_missing(
    target: &mut ModelMetadataDto,
    incoming: ModelMetadataDto,
    source: ModelMetadataSource,
    fetched_at: Option<TimestampMs>,
) {
    use ModelMetadataField::*;
    let mut record = |field| {
        target.provenance.insert(field, ModelMetadataProvenance { source, fetched_at });
    };
    macro_rules! fill_option {
        ($name:ident, $field:ident) => {
            if target.$name.is_none() && incoming.$name.is_some() {
                target.$name = incoming.$name;
                record($field);
            }
        };
    }
    fill_option!(context_window, ContextWindow);
    fill_option!(max_output_tokens, MaxOutputTokens);
    fill_option!(reasoning_efforts, ReasoningEfforts);
    fill_option!(input_modalities, InputModalities);
    fill_option!(output_modalities, OutputModalities);
    fill_option!(cost, Cost);
    if target.reasoning == ModelCapabilityState::Unknown
        && incoming.reasoning != ModelCapabilityState::Unknown
    {
        target.reasoning = incoming.reasoning;
        record(Reasoning);
    }
    if target.tool_calls == ModelCapabilityState::Unknown
        && incoming.tool_calls != ModelCapabilityState::Unknown
    {
        target.tool_calls = incoming.tool_calls;
        record(ToolCalls);
    }
}
