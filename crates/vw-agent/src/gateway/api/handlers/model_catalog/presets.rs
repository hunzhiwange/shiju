//! Project the compiled preset data into a public, credential-free search result.

use std::collections::HashSet;
use vw_agent_apps::models::{
    preset_catalog::*,
    preset_claude::ClaudePreset,
    preset_kimi_dsh::KimiDshCatalog,
    preset_search::{PresetMatch, PresetSearchEntry, search_presets},
};
use vw_api_types::{
    agent_apps::AppId, app_providers::universal::ClaudeModels, model_catalog::ModelCatalogPreset,
};

struct PresetFields<'a> {
    key: &'a str,
    endpoint: Option<&'a str>,
    model_ids: Vec<&'a str>,
}

type Entry<'a> = PresetSearchEntry<'a, PresetFields<'a>>;

pub(super) fn search(app: AppId, query: &str) -> Vec<ModelCatalogPreset> {
    let entries = entries(app);
    search_presets(&entries, query, |_| None)
        .into_iter()
        .flat_map(|hit| {
            hit.group.versions.into_iter().enumerate().filter_map(move |(index, row)| {
                if let PresetMatch::Versions(indices) = &hit.matched
                    && !indices.contains(&index)
                {
                    return None;
                }
                let mut seen = HashSet::new();
                Some(ModelCatalogPreset {
                    app_id: app,
                    key: row.preset.key.into(),
                    name: row.name.into(),
                    name_key: row.name_key.map(str::to_owned),
                    website_url: row.website_url.map(str::to_owned),
                    family: row.family.map(str::to_owned),
                    endpoint: row.preset.endpoint.map(str::to_owned),
                    model_ids: row
                        .preset
                        .model_ids
                        .iter()
                        .copied()
                        .filter(|id| seen.insert(*id))
                        .map(Into::into)
                        .collect(),
                })
            })
        })
        .collect()
}

// Most native preset families share display fields but use different keys and model shapes.
fn entries(app: AppId) -> Vec<Entry<'static>> {
    macro_rules! row {
        ($row:ident, $key:expr, $endpoint:expr, $models:expr) => {
            PresetSearchEntry {
                preset: PresetFields { key: $key, endpoint: $endpoint, model_ids: $models },
                name: &$row.name,
                name_key: $row.name_key.as_deref(),
                website_url: Some(&$row.website_url),
                family: $row.family.as_deref(),
                plan: $row.plan_key,
                region: $row.region_key,
            }
        };
    }

    let mut entries: Vec<_> = match app {
        AppId::Claude | AppId::ClaudeDesktop => claude_presets(app)
            .filter(|preset| !preset.provider().hidden)
            .map(|preset| {
                let row = preset.provider();
                let models = match preset {
                    ClaudePreset::Code { models, small_fast_model, subagent_model, .. } => {
                        let mut ids = claude_models(models);
                        ids.extend(
                            [small_fast_model.as_deref(), subagent_model.as_deref()]
                                .into_iter()
                                .flatten(),
                        );
                        ids
                    }
                    ClaudePreset::Desktop { routes, .. } => {
                        routes.iter().map(|route| route.target.model.as_str()).collect()
                    }
                };
                row!(row, &row.name, row.base_url.as_deref(), models)
            })
            .collect(),
        AppId::Codex => codex_presets(app)
            .filter(|row| !row.hidden)
            .map(|row| {
                let models = row
                    .default_model
                    .as_deref()
                    .into_iter()
                    .chain(row.model_catalog.iter().map(|model| model.model.as_str()))
                    .collect();
                row!(row, &row.name, row.base_url.as_deref(), models)
            })
            .collect(),
        AppId::Gemini | AppId::GrokBuild => gemini_grok_presets(app)
            .map(|row| {
                row!(
                    row,
                    &row.name,
                    row.base_url.as_deref(),
                    row.model.as_deref().into_iter().collect()
                )
            })
            .collect(),
        AppId::OpenCode | AppId::OpenClaw => opencode_openclaw_presets(app)
            .map(|row| {
                row!(
                    row,
                    &row.name,
                    row.base_url.as_deref(),
                    row.models.iter().map(|model| model.id.as_str()).collect()
                )
            })
            .collect(),
        AppId::Hermes | AppId::Mcode => hermes_mcode_presets(app)
            .map(|row| {
                row!(
                    row,
                    row.provider_key.as_deref().unwrap_or(&row.name),
                    Some(&row.base_url),
                    row.models.iter().map(|model| model.id.as_str()).collect()
                )
            })
            .collect(),
        AppId::Kimi | AppId::Dsh => kimi_dsh_presets(app)
            .map(|row| {
                let (endpoint, models) = match &row.catalog {
                    KimiDshCatalog::Kimi(data) => {
                        (&data.provider.base_url, data.models.keys().map(String::as_str).collect())
                    }
                    KimiDshCatalog::Dsh(data) => {
                        (&data.options.base_url, data.models.keys().map(String::as_str).collect())
                    }
                };
                row!(row, &row.provider_key, Some(endpoint), models)
            })
            .collect(),
        AppId::Pi => pi_presets(app)
            .map(|row| {
                row!(
                    row,
                    &row.provider_key,
                    Some(&row.settings_config.base_url),
                    row.settings_config.models.iter().map(|model| model.id.as_str()).collect()
                )
            })
            .collect(),
        AppId::Omp => omp_presets(app)
            .map(|row| PresetSearchEntry {
                preset: PresetFields {
                    key: row.provider_key,
                    endpoint: Some(row.settings_config.base_url),
                    model_ids: row.settings_config.models.iter().map(|model| model.id).collect(),
                },
                name: row.name,
                name_key: row.name_key,
                website_url: Some(row.website_url),
                family: row.family,
                plan: row.plan_key,
                region: row.region_key,
            })
            .collect(),
        AppId::Agy => Vec::new(),
    };
    entries.extend(universal_presets().iter().filter_map(|row| {
        let model_ids = match app {
            AppId::Claude if row.default_apps.claude => {
                row.default_models.claude.as_ref().map_or_else(Vec::new, claude_models)
            }
            AppId::Codex if row.default_apps.codex => row
                .default_models
                .codex
                .as_ref()
                .and_then(|models| models.model.as_deref())
                .into_iter()
                .collect(),
            AppId::Gemini if row.default_apps.gemini => row
                .default_models
                .gemini
                .as_ref()
                .and_then(|models| models.model.as_deref())
                .into_iter()
                .collect(),
            _ => return None,
        };
        Some(PresetSearchEntry {
            preset: PresetFields { key: &row.provider_type, endpoint: None, model_ids },
            name: &row.name,
            name_key: None,
            website_url: row.website_url.as_deref(),
            family: None,
            plan: None,
            region: None,
        })
    }));
    entries
}

fn claude_models(models: &ClaudeModels) -> Vec<&str> {
    [&models.model, &models.haiku_model, &models.sonnet_model, &models.opus_model]
        .into_iter()
        .filter_map(|model| model.as_deref())
        .collect()
}
