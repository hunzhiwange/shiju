use vw_api_app_provider_edit::{catalog::ProviderCatalog,presentation::AppProviderPresentationEntry};
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub catalog: ProviderCatalog,
    pub presentation: Vec<AppProviderPresentationEntry>,
}

#[derive(Clone, Debug)]
pub struct Editor {
    pub id: String,
    pub sort: String,
    pub category: String,
    pub notes: String,
    pub saving: bool,
}

use crate::Scope;
use vw_api_app_provider_edit::presentation::AppProviderPresentation;
/// Provider-owned lifecycle data with concrete cross-domain slots supplied by the host.
/// The host clears its usage-card UI before beginning a new list request.
#[derive(Clone, Debug)]
pub struct ListState<Leave, Failover, Aggregation, Subagent, Pi, Diagnostics, Omp, Omo, Usage> {
    pub leave: Leave,
    pub failover: Failover,
    pub scope: Option<Scope>,
    pub generation: u64,
    pub query: String,
    pub loading: bool,
    pub tray_switch: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub editor: Option<Editor>,
    pub form: Option<super::edit::EditForm>,
    pub icons: Option<super::icons::Form>,
    pub universal: Option<super::universal::Form>,
    pub snippets: Option<super::snippets::Form>,
    pub env: Option<super::env::Form>,
    pub mode: Option<super::mode::Form>,
    pub aggregation: Option<Aggregation>,
    pub codex_subagent: Option<Subagent>,
    pub pi_directory: Option<Pi>,
    pub paths: Option<super::paths::Form>,
    pub preview: Option<super::preview::Form>,
    pub endpoint_speed: Option<Diagnostics>,
    pub endpoints: Option<super::endpoints::Form>,
    pub model_catalog: Option<super::model_catalog::CatalogForm>,
    pub omp_directory: Option<Omp>,
    pub desktop: Option<super::desktop::Form>,
    pub omo: Option<Omo>,
    pub new_draft:
        Option<vw_api_app_provider_base::create::AppProviderCreate>,
    pub error: Option<String>,
    pub usage: Usage,
}
impl<Leave, Failover, Aggregation, Subagent, Pi, Diagnostics, Omp, Omo, Usage> ListState<Leave, Failover, Aggregation, Subagent, Pi, Diagnostics, Omp, Omo, Usage> {
    pub fn begin(&mut self, scope: Scope) -> u64 {
        if self.scope.as_ref() != Some(&scope) {
            self.new_draft = None;
            self.query.clear();
        }
        self.generation = self.generation.wrapping_add(1);
        self.scope = Some(scope);
        self.snapshot = None;
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
        self.tray_switch = None;
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

impl<Leave, Failover, Aggregation, Subagent, Pi, Diagnostics, Omp, Omo, Usage> Default for ListState<Leave, Failover, Aggregation, Subagent, Pi, Diagnostics, Omp, Omo, Usage>
where Leave: Default, Failover: Default, Usage: Default {
    fn default() -> Self { Self {
        leave: Default::default(),
        failover: Default::default(),
        scope: Default::default(),
        generation: Default::default(),
        query: Default::default(),
        loading: Default::default(),
        tray_switch: Default::default(),
        snapshot: Default::default(),
        editor: Default::default(),
        form: Default::default(),
        icons: Default::default(),
        universal: Default::default(),
        snippets: Default::default(),
        env: Default::default(),
        mode: Default::default(),
        aggregation: Default::default(),
        codex_subagent: Default::default(),
        pi_directory: Default::default(),
        paths: Default::default(),
        preview: Default::default(),
        endpoint_speed: Default::default(),
        endpoints: Default::default(),
        model_catalog: Default::default(),
        omp_directory: Default::default(),
        desktop: Default::default(),
        omo: Default::default(),
        new_draft: Default::default(),
        error: Default::default(),
        usage: Default::default(),
    } }
}
