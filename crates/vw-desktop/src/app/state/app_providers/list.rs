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
    pub leave: super::leave::LeaveState,
    pub failover: crate::app::message::agent_apps_high_availability::State,
    pub scope: Option<Scope>,
    pub generation: u64,
    pub query: String,
    pub loading: bool,
    pub snapshot: Option<Snapshot>,
    pub editor: Option<Editor>,
    pub form: Option<super::edit::EditForm>,
    pub icons: Option<super::icons::Form>,
    pub universal: Option<super::universal::Form>,
    pub snippets: Option<super::snippets::Form>,
    pub env: Option<super::env::Form>,
    pub mode: Option<super::mode::Form>,
    pub aggregation: Option<crate::app::message::settings::aggregation_list::Form>,
    pub codex_subagent: Option<crate::app::message::settings::aggregation_subagent::Form>,
    pub pi_directory: Option<crate::app::message::settings::aggregation_pi::Form>,
    pub paths: Option<super::paths::Form>,
    pub preview: Option<super::preview::Form>,
    pub endpoint_speed: Option<crate::app::message::agent_apps_diagnostics::endpoint_state::State>,
    pub endpoints: Option<super::endpoints::Form>,
    pub model_catalog: Option<super::model_catalog::CatalogForm>,
    pub omp_directory: Option<crate::app::message::settings::aggregation_omp::Form>,
    pub desktop: Option<super::desktop::Form>,
    pub omo: Option<crate::app::state::aggregation_omo::Form>,
    pub new_draft:
        Option<vw_gateway_client::vw_api_types::app_providers::create::AppProviderCreate>,
    pub error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub usage: crate::app::components::apps_usage_cards::Cards,
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
            self.query.clear();
        }
        self.generation = self.generation.wrapping_add(1);
        self.scope = Some(scope);
        self.snapshot = None;
        #[cfg(not(target_arch = "wasm32"))]
        self.usage.clear();
        self.editor = None;
        self.form = None;
        self.omp_directory = None;
        self.desktop = None;
        self.omo = None;
        self.endpoint_speed = None;
        self.endpoints = None;
        self.model_catalog = None;
        self.icons = None;
        self.universal = None;
        self.snippets = None;
        self.preview = None;
        self.env = None;
        self.mode = None;
        self.aggregation = None;
        self.paths = None;
        self.pi_directory = None;
        self.codex_subagent = None;
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
