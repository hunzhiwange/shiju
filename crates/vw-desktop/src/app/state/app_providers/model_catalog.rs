//! State for selecting exact model deployments returned by the gateway.

use super::list::Scope;
use vw_gateway_client::vw_api_types::model_catalog::{
    ModelCatalogIdentity, ModelCatalogPreset, ModelCatalogProject, ScopedModelCatalogIdentity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CatalogContext {
    pub scope: Scope,
    pub project: ModelCatalogProject,
    pub access_generation: u64,
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
    pub loading: bool,
    pub searched: bool,
    pub saving: bool,
    pub notice: Option<String>,
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
            loading: false,
            searched: false,
            saving: false,
            notice: None,
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
        if self.endpoint != endpoint {
            self.endpoint = endpoint;
            self.selected.clear();
        }
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
