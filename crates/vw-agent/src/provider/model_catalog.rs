//! Assemble an explicitly bound native catalog into the agent's resolver views.

use super::provider::{
    ApiInfo, CapabilityIO, Model, ModelCost, ModelCostCache, ModelLimit, State,
    default_model_capabilities,
};
use crate::config::{Config, Error};
use std::collections::{BTreeMap, HashMap};
use vw_api_types::model_catalog::{
    ModelCapabilityState, ModelCatalogEntry, ModelCatalogReadResponse,
};
use vw_shared::provider::reasoning::ReasoningOption;
use vw_shared::provider::state::{DEFAULT_MODEL_CONTEXT_LIMIT, DEFAULT_MODEL_OUTPUT_LIMIT};

pub(super) struct DeploymentCatalog {
    adapter: String,
    catalog: ModelCatalogReadResponse,
}

pub(super) type DeploymentCatalogs = BTreeMap<String, DeploymentCatalog>;

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn read(config: &Config) -> Result<DeploymentCatalogs, Error> {
    use crate::{
        app_providers::AppProviderStore,
        global_database::{self, GlobalDatabase},
        session::ui_store::current_tenant_scope,
    };
    use serde_json::Value;
    use vw_api_types::model_catalog::ModelCatalogIdentity;

    let mut bindings = Vec::new();
    for (provider_id, provider) in &config.providers {
        let Some(binding) = provider.get("model_catalog") else { continue };
        vw_agent_apps::providers::validate_identity(provider_id).map_err(Error::Invalid)?;
        let deployment: ModelCatalogIdentity = serde_json::from_value(binding.clone())
            .map_err(|_| Error::Invalid("invalid runtime model catalog binding".into()))?;
        if matches!(provider_id.as_str(), "shiju" | "acp")
            || provider.get("api").and_then(Value::as_str) != Some(deployment.endpoint.as_str())
        {
            return Err(Error::Invalid(
                "runtime model catalog requires the same configured endpoint on a custom provider"
                    .into(),
            ));
        }
        let adapter = provider.get("adapter").and_then(Value::as_str).unwrap_or_default();
        if !matches!(adapter, "openai" | "openai-compatible" | "openai-responses" | "anthropic") {
            return Err(Error::Invalid("unsupported runtime model catalog adapter".into()));
        }
        bindings.push((provider_id.clone(), adapter.to_string(), deployment));
    }
    if bindings.is_empty() {
        return Ok(BTreeMap::new());
    }

    let tenant = current_tenant_scope();
    global_database::run_isolated(move || {
        let database = GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| Error::Invalid("application provider storage unavailable".into()))?;
        bindings
            .into_iter()
            .map(|(id, adapter, deployment)| {
                // Read tenant-owned metadata only. Runtime routing and authentication
                // remain on the configured runtime provider.
                let record = AppProviderStore::preview_record(
                    &database,
                    &tenant,
                    deployment.app_id,
                    &deployment.provider_id,
                )
                .map_err(Error::Invalid)?;
                let catalog = vw_agent_apps::models::edits::read(&tenant, &deployment, &record)
                    .map_err(Error::Invalid)?;
                Ok((id, DeploymentCatalog { adapter, catalog }))
            })
            .collect()
    })
    .await
    .map_err(|_| Error::Invalid("application provider storage unavailable".into()))?
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn read(_: &Config) -> Result<DeploymentCatalogs, Error> {
    Ok(BTreeMap::new())
}

pub(super) fn revisions(catalogs: &DeploymentCatalogs) -> impl Iterator<Item = (&str, &str)> {
    catalogs.iter().map(|(id, entry)| (id.as_str(), entry.catalog.revision.as_str()))
}

pub(super) fn project_runtime(
    state: &mut State,
    catalogs: &DeploymentCatalogs,
) -> HashMap<String, String> {
    let mut defaults = HashMap::new();
    for (id, deployment) in catalogs {
        let provider = state.providers.get_mut(id).expect("bound provider is configured");
        provider.models = deployment
            .catalog
            .items
            .iter()
            .map(|row| {
                let model = project_model(id, &deployment.adapter, row);
                (model.id.clone(), model)
            })
            .collect();
        if let Some(default) = deployment.catalog.default_model_id.as_ref()
            && provider.models.contains_key(default.as_ref())
        {
            defaults.insert(id.clone(), default.as_ref().to_owned());
        }
    }
    defaults
}

pub(super) fn project_aliases(state: &mut State, catalogs: &DeploymentCatalogs) {
    for (id, deployment) in catalogs {
        let provider = state.providers.get_mut(id).expect("bound provider is configured");
        for row in &deployment.catalog.items {
            if let Some(alias) = row.display_alias.as_ref() {
                let model = provider
                    .models
                    .get_mut(row.identity.model_id.as_ref())
                    .expect("runtime projection contains saved row");
                model.name.clone_from(alias);
            }
        }
    }
}

fn project_model(provider_id: &str, adapter: &str, row: &ModelCatalogEntry) -> Model {
    let id = row.identity.model_id.as_ref();
    let metadata = &row.metadata;
    let declared =
        vw_agent_apps::models::capabilities::resolve_model_capabilities(id, None, metadata, false);
    let mut capabilities = default_model_capabilities();
    capabilities.toolcall = declared.tool_calls == ModelCapabilityState::Supported;
    capabilities.reasoning = declared.reasoning == ModelCapabilityState::Supported;
    capabilities.reasoning_options = Some(
        declared
            .reasoning_efforts
            .filter(|_| capabilities.reasoning)
            .map(|values| vec![ReasoningOption::Effort { values: values.to_vec() }])
            .unwrap_or_default(),
    );
    if let Some(input) = &metadata.input_modalities {
        capabilities.input = modalities(input);
        capabilities.attachment = capabilities.input.image
            || capabilities.input.audio
            || capabilities.input.video
            || capabilities.input.pdf;
    }
    if let Some(output) = &metadata.output_modalities {
        capabilities.output = modalities(output);
    }
    let cost = metadata.cost.as_ref();
    let context = metadata.context_window.unwrap_or_else(|| {
        DEFAULT_MODEL_CONTEXT_LIMIT
            .max(metadata.max_output_tokens.unwrap_or_default().saturating_add(1))
    });
    let output = metadata
        .max_output_tokens
        .unwrap_or_else(|| DEFAULT_MODEL_OUTPUT_LIMIT.min(context.saturating_sub(1)));
    let limit = ModelLimit { context, input: Some(context.saturating_sub(output)), output };
    Model {
        id: id.into(),
        provider_id: provider_id.into(),
        api: ApiInfo {
            id: id.into(),
            url: row.identity.deployment.endpoint.clone(),
            adapter: adapter.into(),
        },
        // Keep display aliases out of runtime name-based routing heuristics.
        name: id.into(),
        family: None,
        capabilities,
        cost: ModelCost {
            input: cost.map_or(0.0, |cost| cost.input),
            output: cost.map_or(0.0, |cost| cost.output),
            cache: ModelCostCache {
                read: cost.and_then(|cost| cost.cache_read).unwrap_or_default(),
                write: cost.and_then(|cost| cost.cache_write).unwrap_or_default(),
            },
            experimental_over_200k: None,
        },
        limit,
        status: "active".into(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: String::new(),
        variants: HashMap::new(),
    }
}

fn modalities(values: &[String]) -> CapabilityIO {
    let contains = |name| values.iter().any(|value| value == name);
    CapabilityIO {
        text: contains("text"),
        audio: contains("audio"),
        image: contains("image"),
        video: contains("video"),
        pdf: contains("pdf"),
    }
}
