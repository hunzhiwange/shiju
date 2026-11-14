//! Pure price selection over the shared models.dev catalog; no network or cache ownership.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use vw_provider_resolver::models::{Model, Provider};

use super::pricing::ModelPricingInfo;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelsDevEntry {
    pub key: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub normalized_id: String,
    pub model_name: String,
    pub release_date: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsDevSelectionConfig {
    pub include_common_models: bool,
    pub selected_model_keys: Vec<String>,
    pub excluded_common_model_keys: Vec<String>,
}

fn is_text_pricing_model(id: &str, model: &Model) -> bool {
    if model.status.as_deref().is_some_and(|status| status.eq_ignore_ascii_case("deprecated")) {
        return false;
    }
    if let Some(modalities) = &model.modalities {
        let output = &modalities.output;
        if !output.is_empty()
            && (!output.iter().any(|m| m.eq_ignore_ascii_case("text"))
                || output.iter().any(|m| {
                    ["audio", "image", "video"].iter().any(|blocked| m.eq_ignore_ascii_case(blocked))
                }))
        {
            return false;
        }
    }
    let name = format!("{id} {}", model.name).to_lowercase();
    !["audio", "deprecated", "embedding", "image", "moderation", "realtime", "transcribe", "tts", "video"]
        .iter().any(|marker| name.contains(marker))
}

pub fn normalize_models_dev_model_id(id: &str) -> String {
    let leaf = id.rsplit('/').next().unwrap_or(id);
    let normalized = leaf.split(':').next().unwrap_or_default().trim().replace('@', "-").to_lowercase();
    normalized.strip_suffix("[1m]").unwrap_or(&normalized).trim().to_owned()
}

/// Flatten the existing shared snapshot's providers. Stable key ordering breaks ties
/// because the shared catalog uses HashMap and cannot retain JSON insertion order.
pub fn flatten_models(providers: &HashMap<String, Provider>) -> Vec<ModelsDevEntry> {
    let mut entries = Vec::new();
    let mut providers: Vec<_> = providers.iter().collect();
    providers.sort_by(|a, b| a.0.cmp(b.0));
    for (provider_id, provider) in providers {
        let mut models: Vec<_> = provider.models.iter().collect();
        models.sort_by(|a, b| a.0.cmp(b.0));
        for (model_id, model) in models {
            if !is_text_pricing_model(model_id, model) {
                continue;
            }
            let Some(cost) = &model.cost else { continue };
            if cost.input.is_none() && cost.output.is_none() {
                continue;
            }
            let normalized_id = normalize_models_dev_model_id(model_id);
            if normalized_id.is_empty() {
                continue;
            }
            entries.push(ModelsDevEntry {
                key: format!("{provider_id}/{model_id}"),
                provider_id: provider_id.clone(),
                provider_name: if provider.name.is_empty() { provider_id.clone() } else { provider.name.clone() },
                model_id: model_id.clone(),
                normalized_id,
                model_name: if model.name.is_empty() { model_id.clone() } else { model.name.clone() },
                release_date: model.release_date.clone(),
                input: cost.input.unwrap_or(0.0),
                output: cost.output.unwrap_or(0.0),
                cache_read: cost.cache_read.unwrap_or(0.0),
                cache_write: cost.cache_write.unwrap_or(0.0),
            });
        }
    }
    entries.sort_by(|a, b| b.release_date.cmp(&a.release_date).then_with(|| a.model_name.cmp(&b.model_name)));
    entries
}

/// Canonical providers only, at most six entries per family in candidate order.
pub fn get_common_model_keys(entries: &[ModelsDevEntry]) -> HashSet<String> {
    let mut keys = HashSet::new();
    let rules: [(&str, &[&str]); 11] = [
        ("anthropic", &["claude-"]),
        ("openai", &["gpt-", "o1-", "o3-", "o4-"]),
        ("google", &["gemini-"]),
        ("xai", &["grok-"]),
        ("deepseek", &["deepseek-"]),
        ("alibaba", &["qwen"]),
        ("xiaomi", &["mimo-"]),
        ("longcat", &["longcat-"]),
        ("moonshotai", &["kimi-"]),
        ("minimax-cn", &["minimax-m"]),
        ("zai", &["glm-"]),
    ];
    for (provider, prefixes) in rules {
        for entry in entries.iter().filter(|entry| {
            entry.provider_id == provider
                && prefixes.iter().any(|prefix| entry.model_id.to_lowercase().starts_with(*prefix))
        }).take(6) {
            keys.insert(entry.key.clone());
        }
    }
    keys
}

pub fn resolve_models_dev_selection(
    entries: &[ModelsDevEntry],
    config: &ModelsDevSelectionConfig,
) -> Vec<ModelsDevEntry> {
    let common = if config.include_common_models { get_common_model_keys(entries) } else { HashSet::new() };
    let explicit: HashSet<_> = config.selected_model_keys.iter().collect();
    let excluded: HashSet<_> = config.excluded_common_model_keys.iter().collect();
    entries.iter().filter(|entry| {
        explicit.contains(&entry.key) || (common.contains(&entry.key) && !excluded.contains(&entry.key))
    }).cloned().collect()
}

/// Decimal strings accepted by the pricing store, without exponent notation.
pub fn format_price(value: f64) -> String {
    if !value.is_finite() || value <= 0.0 || value >= 1e12 {
        return "0".into();
    }
    let fixed = format!("{value:.6}");
    fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The first selected candidate for each normalized identity supplies its price.
pub fn to_model_pricing(entries: &[ModelsDevEntry]) -> Vec<ModelPricingInfo> {
    let mut seen = HashSet::new();
    entries.iter().filter(|entry| seen.insert(&entry.normalized_id)).map(|entry| ModelPricingInfo {
        model_id: entry.normalized_id.clone().into(),
        display_name: entry.model_name.clone(),
        input_cost_per_million: format_price(entry.input),
        output_cost_per_million: format_price(entry.output),
        cache_read_cost_per_million: format_price(entry.cache_read),
        cache_creation_cost_per_million: format_price(entry.cache_write),
    }).collect()
}
