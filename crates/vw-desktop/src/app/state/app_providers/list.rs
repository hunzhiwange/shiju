use crate::app::state::AgentAppsContext;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        catalog::ProviderCatalog,
        presentation::{AppProviderPresentation, AppProviderPresentationEntry},
    },
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub host: String,
    pub app: AppId,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub catalog: ProviderCatalog,
    pub presentation: Vec<AppProviderPresentationEntry>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct ListState {
    pub scope: Option<Scope>,
    pub generation: u64,
    pub loading: bool,
    pub snapshot: Option<Snapshot>,
    pub editor: Option<Editor>,
    pub form: Option<super::edit::EditForm>,
    pub endpoints: Option<super::endpoints::Form>,
    pub desktop: Option<super::desktop::Form>,
    pub new_draft:
        Option<vw_gateway_client::vw_api_types::app_providers::create::AppProviderCreate>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct Editor {
    pub id: String,
    pub sort: String,
    pub category: String,
    pub notes: String,
    pub saving: bool,
}
impl ListState {
    pub fn begin(&mut self, scope: Scope) -> u64 {
        if self.scope.as_ref() != Some(&scope) {
            self.new_draft = None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.scope = Some(scope);
        self.snapshot = None;
        self.editor = None;
        self.form = None;
        self.desktop = None;
        self.endpoints = None;
        self.loading = true;
        self.error = None;
        self.generation
    }
    pub fn finish(
        &mut self,
        scope: &Scope,
        generation: u64,
        result: Result<Snapshot, String>,
    ) -> bool {
        if self.scope.as_ref() != Some(scope) || self.generation != generation || !self.loading {
            return false;
        }
        self.loading = false;
        match result {
            Ok(value) => self.snapshot = Some(value),
            Err(error) => self.error = Some(error),
        };
        true
    }
    pub fn presentation(&self, id: &str) -> AppProviderPresentation {
        self.snapshot
            .as_ref()
            .and_then(|s| s.presentation.iter().find(|e| e.id == id))
            .map(|e| e.presentation.clone())
            .unwrap_or_default()
    }
}
