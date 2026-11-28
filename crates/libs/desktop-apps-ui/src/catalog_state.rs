//! Catalog data; cross-domain panels are supplied by the desktop composition.
use crate::AgentAppsContext;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentWorkspaceTab {
    #[default]
    Direct,
    Routing,
    Configuration,
}

#[derive(Clone)]
pub struct AppsCatalogState<
    ExternalSessions,
    ProjectProfiles,
    Prompts,
    WorkspaceFiles,
    DailyMemory,
    HermesMemory,
    OpenclawCatalog,
    OpenclawTools,
    OpenclawEnv,
    OpenclawModel,
    OpenclawDefaults,
    HermesDashboard,
    Launch,
    Preferences,
    Lifecycle,
    Drag,
    ThirdPartySkills,
    ThirdPartyMcpScope,
    Row
> {
    pub external_sessions: ExternalSessions,
    pub project_profiles: ProjectProfiles,
    pub routing: crate::app_routing::lifecycle::State,
    pub prompts: Prompts,
    pub workspace_files: WorkspaceFiles,
    pub daily_memory: DailyMemory,
    pub hermes_memory: HermesMemory,
    pub openclaw_catalog: OpenclawCatalog,
    pub openclaw_tools: OpenclawTools,
    pub openclaw_env: OpenclawEnv,
    pub openclaw_model: OpenclawModel,
    pub openclaw_defaults: OpenclawDefaults,
    pub hermes_dashboard: HermesDashboard,
    pub failover_health: crate::agent_apps_health::State,
    pub launch: Launch,
    pub preferences: Preferences,
    pub lifecycle: Lifecycle,
    pub model_diagnostics: crate::diagnostics::model_state::ModelState,
    pub high_availability: crate::agent_apps_high_availability_policy::State,
    pub agent_workspace_tab: AgentWorkspaceTab,
    pub agent_provider_search_open: bool,
    pub third_party_page: desktop_ui_controls::settings_navigation::ThirdPartyPage,
    pub navigation_generation: u64,
    pub third_party_skills: ThirdPartySkills,
    pub third_party_mcp_scope: ThirdPartyMcpScope,
    pub updates: UpdatesState,
    pub commands: CommandsState,
    pub query: String,
    pub hovered: Option<vw_api_app_identity::AppId>,
    pub pending_tray_app: Option<vw_api_app_identity::AppId>,
    pub expanded: Option<vw_api_app_identity::AppId>,
    pub drag: Drag,
    pub host: String,
    pub token: String,
    pub connection_open: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub rows: Vec<Row>,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
    pub loaded_at: Option<web_time::Instant>,
}
// Credentials must never enter App/Message debug output.
impl<
    ExternalSessions,
    ProjectProfiles,
    Prompts,
    WorkspaceFiles,
    DailyMemory,
    HermesMemory,
    OpenclawCatalog,
    OpenclawTools,
    OpenclawEnv,
    OpenclawModel,
    OpenclawDefaults,
    HermesDashboard,
    Launch,
    Preferences,
    Lifecycle,
    Drag,
    ThirdPartySkills,
    ThirdPartyMcpScope,
    Row
> std::fmt::Debug for AppsCatalogState<
    ExternalSessions,
    ProjectProfiles,
    Prompts,
    WorkspaceFiles,
    DailyMemory,
    HermesMemory,
    OpenclawCatalog,
    OpenclawTools,
    OpenclawEnv,
    OpenclawModel,
    OpenclawDefaults,
    HermesDashboard,
    Launch,
    Preferences,
    Lifecycle,
    Drag,
    ThirdPartySkills,
    ThirdPartyMcpScope,
    Row
> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppsCatalogState").field("loading", &self.loading).finish_non_exhaustive()
    }
}
impl<
    ExternalSessions: Default,
    ProjectProfiles: Default,
    Prompts: Default,
    WorkspaceFiles: Default,
    DailyMemory: Default,
    HermesMemory: Default,
    OpenclawCatalog: Default,
    OpenclawTools: Default,
    OpenclawEnv: Default,
    OpenclawModel: Default,
    OpenclawDefaults: Default,
    HermesDashboard: Default,
    Launch: Default,
    Preferences: Default,
    Lifecycle: Default,
    Drag: Default,
    ThirdPartySkills: Default,
    ThirdPartyMcpScope: Default,
    Row
> Default for AppsCatalogState<
    ExternalSessions,
    ProjectProfiles,
    Prompts,
    WorkspaceFiles,
    DailyMemory,
    HermesMemory,
    OpenclawCatalog,
    OpenclawTools,
    OpenclawEnv,
    OpenclawModel,
    OpenclawDefaults,
    HermesDashboard,
    Launch,
    Preferences,
    Lifecycle,
    Drag,
    ThirdPartySkills,
    ThirdPartyMcpScope,
    Row
> {
    fn default() -> Self { Self {
        external_sessions: Default::default(),
        project_profiles: Default::default(),
        routing: Default::default(),
        prompts: Default::default(),
        workspace_files: Default::default(),
        daily_memory: Default::default(),
        hermes_memory: Default::default(),
        openclaw_catalog: Default::default(),
        openclaw_tools: Default::default(),
        openclaw_env: Default::default(),
        openclaw_model: Default::default(),
        openclaw_defaults: Default::default(),
        hermes_dashboard: Default::default(),
        failover_health: Default::default(),
        launch: Default::default(),
        preferences: Default::default(),
        lifecycle: Default::default(),
        model_diagnostics: Default::default(),
        high_availability: Default::default(),
        agent_workspace_tab: Default::default(),
        agent_provider_search_open: false,
        third_party_page: Default::default(),
        navigation_generation: 0,
        third_party_skills: Default::default(),
        third_party_mcp_scope: Default::default(),
        updates: Default::default(),
        commands: Default::default(),
        query: Default::default(),
        hovered: Default::default(),
        pending_tray_app: Default::default(),
        expanded: Default::default(),
        drag: Default::default(),
        host: "desktop-local".into(),
        token: Default::default(),
        connection_open: Default::default(),
        loading: Default::default(),
        error: Default::default(),
        rows: Default::default(),
        context: Default::default(),
        generation: Default::default(),
        loaded_at: Default::default(),
    } }
}

/// Transient copy feedback uses a monotonically increasing request identity.
#[derive(Clone, Debug, Default)]
pub struct CommandsState {
    /// 1: install, 2: uninstall, 3: installation diagnostics.
    pub panel: Option<u8>,
    pub platform: usize,
    pub method: usize,
    pub inferred: bool,
    pub diagnostics_loading: bool,
    pub diagnostics_error: Option<String>,
    pub diagnostics: Vec<(String, Vec<desktop_i18n::format::Message>)>,
    pub sequence: u64,
    pub copied: std::collections::BTreeMap<usize, u64>,
}
impl CommandsState {
    pub fn invalidate(&mut self) {
        self.panel = None;
        self.clear_feedback();
    }
    pub fn clear_feedback(&mut self) {
        self.sequence = self.sequence.wrapping_add(1);
        self.copied.clear();
    }
}

/// Metadata is separate from authorized installation observations.
#[derive(Clone, Debug, Default)]
pub struct UpdatesState {
    pub checking: bool,
    pub checked_at: Option<std::time::SystemTime>,
    pub error: Option<String>,
    pub results: Vec<UpdateCandidate>,
}
#[derive(Clone, Debug)]
pub struct UpdateCandidate {
    pub installation: vw_api_app_identity::AppInstallation,
    pub latest: Option<String>,
    pub supported: bool,
    pub status: String,
}
