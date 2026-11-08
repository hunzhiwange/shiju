//! State for selecting exact model deployments returned by the gateway.

use super::list::Scope;
use vw_gateway_client::vw_api_types::{
    id::ModelId,
    model_catalog::{
        ModelCatalogFillPreviewResponse, ModelCatalogIdentity, ModelCatalogPreset,
        ModelCatalogProject, ModelCatalogReadResponse, ScopedModelCatalogIdentity,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogContext {
    pub scope: Scope,
    pub project: ModelCatalogProject,
    pub access_generation: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) enum FillState {
    #[default]
    Idle,
    Loading(u64),
    Preview(Box<ModelCatalogFillPreviewResponse>),
    Committing(u64),
    Notice(&'static str),
}

#[derive(Clone, Debug)]
pub(crate) struct CatalogForm {
    pub context: CatalogContext,
    pub provider_id: String,
    pub query: String,
    pub items: Vec<ModelCatalogPreset>,
    pub selected: Vec<ScopedModelCatalogIdentity>,
    pub endpoint: String,
    pub manual_id: String,
    pub edit_id: Option<ModelId>,
    pub alias: String,
    pub alias_dirty: bool,
    pub default_override: Option<bool>,
    pub catalog: Option<ModelCatalogReadResponse>,
    pub catalog_loading: bool,
    pub loading: bool,
    pub searched: bool,
    pub saving: bool,
    pub notice: Option<String>,
    pub fill: FillState,
    pub fill_silent: bool,
}

impl CatalogForm {
    pub fn new(context: CatalogContext, provider_id: String) -> Self {
        Self {
            context,
            provider_id,
            query: String::new(),
            items: vec![],
            selected: vec![],
            endpoint: String::new(),
            manual_id: String::new(),
            edit_id: None,
            alias: String::new(),
            alias_dirty: false,
            default_override: None,
            catalog: None,
            catalog_loading: false,
            loading: false,
            searched: false,
            saving: false,
            notice: None,
            fill: FillState::Idle,
            fill_silent: false,
        }
    }

    pub fn finish_search(
        &mut self,
        current: &CatalogContext,
        items: Vec<ModelCatalogPreset>,
    ) -> bool {
        if &self.context != current {
            return false;
        }
        self.items = items.into_iter().filter(|row| row.app_id == current.scope.app).collect();
        self.loading = false;
        self.searched = true;
        true
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        if self.endpoint.trim() != endpoint.trim() {
            self.selected.clear();
            self.clear_model_draft();
            self.catalog = None;
            self.notice = None;
        }
        self.endpoint = endpoint;
    }

    pub fn edit_model(&mut self, model_id: ModelId) {
        let Some(catalog) = &self.catalog else { return };
        let Some(entry) = catalog.items.iter().find(|entry| entry.identity.model_id == model_id)
        else {
            return;
        };
        let alias = entry.display_alias.clone().unwrap_or_default();
        self.clear_model_draft();
        self.alias = alias;
        self.edit_id = Some(model_id);
    }

    pub fn clear_model_draft(&mut self) {
        self.manual_id.clear();
        self.edit_id = None;
        self.alias.clear();
        self.alias_dirty = false;
        self.default_override = None;
        self.fill = FillState::Idle;
        self.fill_silent = false;
    }

    pub fn apply_catalog(&mut self, catalog: ModelCatalogReadResponse) {
        if !self.alias_dirty
            && let Some(id) = &self.edit_id
            && let Some(entry) = catalog.items.iter().find(|row| &row.identity.model_id == id)
        {
            self.alias = entry.display_alias.clone().unwrap_or_default();
        }
        self.catalog = Some(catalog);
    }

    pub fn make_default(&self) -> bool {
        self.default_override.unwrap_or_else(|| {
            self.edit_id.as_ref().is_some_and(|id| {
                self.catalog.as_ref().and_then(|catalog| catalog.default_model_id.as_ref())
                    == Some(id)
            })
        })
    }

    pub fn busy(&self) -> bool {
        self.saving || self.catalog_loading
    }

    pub fn deployment(&self) -> ModelCatalogIdentity {
        ModelCatalogIdentity {
            target_host_id: self.context.scope.host.clone(),
            app_id: self.context.scope.app,
            provider_id: self.provider_id.clone(),
            endpoint: self.endpoint.trim().to_owned(),
        }
    }

    pub fn toggle(&mut self, preset: usize, model: usize, selected: bool) {
        let Some(row) = self.items.get(preset) else { return };
        let deployment = self.deployment();
        if row.endpoint.as_deref() != Some(deployment.endpoint.as_str())
            || deployment.endpoint.is_empty()
        {
            return;
        }
        let Some(model_id) = row.model_ids.get(model) else { return };
        let identity = ScopedModelCatalogIdentity {
            scope: self.context.scope.context.tenant.clone(),
            deployment,
            model_id: model_id.clone(),
        };
        if selected {
            if !self.selected.contains(&identity) {
                self.selected.push(identity);
            }
        } else {
            self.selected.retain(|item| item != &identity);
        }
    }

    pub fn empty_label(&self) -> &'static str {
        if self.query.trim().is_empty() { "目录为空" } else { "未找到匹配的预设" }
    }
}
