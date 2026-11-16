//! Catalog observation completion validation belongs to the settings domain.
use super::catalog::{self, CatalogRow};
use desktop_apps_ui::AgentAppsContext;
use vw_api_app_identity::{AppId, AppReadResponse};

pub struct ObservationInput<'a> {
    pub context: &'a Option<AgentAppsContext>,
    pub host: &'a str,
    pub generation: u64,
    pub loading: &'a mut bool,
    pub rows: &'a mut [CatalogRow],
}

pub fn finish_observation(
    state: ObservationInput<'_>,
    current: Option<&AgentAppsContext>,
    context: AgentAppsContext,
    host: String,
    generation: u64,
    id: AppId,
    result: Result<AppReadResponse, gateway_http::AgentAppsReadError>,
) -> (bool, Option<String>) {
    if current != Some(&context)
        || state.context.as_ref() != Some(&context)
        || state.host != host
        || state.generation != generation
        || !*state.loading
    {
        return (false, None);
    }
    *state.loading = false;
    let error = result.as_ref().err().map(|error| catalog::error_label(error).to_owned());
    if let Some(row) = state.rows.iter_mut().find(|row| row.entry.id == id) {
        row.observation = result;
    } else {
        return (true, None);
    }
    if let Some(error) = error {
        return (true, Some(error));
    }
    (true, None)
}

/// Validate a local-auth completion before the host invalidates cross-domain access.
pub fn accepts_local_credentials(
    current: Option<&AgentAppsContext>,
    context: &AgentAppsContext,
    generation: u64,
    reply_generation: u64,
    loading: bool,
) -> bool {
    current == Some(context) && generation == reply_generation && loading
}

/// Local catalog fields borrowed independently from cross-domain panels.
pub struct CatalogInput<'a> {
    pub context: &'a mut Option<AgentAppsContext>,
    pub connection_open: &'a mut bool,
    pub host: &'a mut String,
    pub token: &'a mut String,
    pub query: &'a mut String,
    pub generation: &'a mut u64,
    pub loading: &'a mut bool,
    pub rows: &'a mut Vec<CatalogRow>,
    pub loaded_at: &'a mut Option<web_time::Instant>,
    pub error: &'a mut Option<String>,
    pub expanded: &'a mut Option<AppId>,
    pub hovered: &'a mut Option<AppId>,
    pub drag: &'a mut super::drag::DragState,
    pub lifecycle: &'a mut super::lifecycle::LifecycleState,
    pub preferences: &'a mut super::preferences::PreferencesState,
    pub launch: &'a mut super::launch::LaunchState,
}
impl CatalogInput<'_> {
    pub fn query(&mut self, query: String) {
        *self.query = query;
        *self.drag = Default::default();
    }
    pub fn credentials(&mut self, host: Option<String>, token: Option<String>) {
        self.lifecycle.invalidate();
        self.preferences.invalidate();
        self.launch.invalidate();
        if let Some(host) = host {
            *self.host = host;
        }
        if let Some(token) = token {
            *self.token = token;
        }
        *self.drag = Default::default();
        *self.expanded = None;
        *self.generation = self.generation.wrapping_add(1);
        *self.loading = false;
        self.rows.clear();
        *self.loaded_at = None;
        *self.error = None;
    }
}

impl CatalogInput<'_> {
    pub fn begin(&mut self, value: AgentAppsContext) -> u64 {
        self.preferences.invalidate();
        self.launch.invalidate();
        *self.generation = self.generation.wrapping_add(1);
        *self.context = Some(value);
        *self.loading = true;
        self.rows.clear();
        *self.loaded_at = None;
        *self.drag = Default::default();
        *self.expanded = None;
        *self.hovered = None;
        *self.error = None;
        *self.generation
    }
    pub fn finish(
        &mut self,
        value: &AgentAppsContext,
        generation: u64,
        result: Result<Vec<CatalogRow>, String>,
    ) -> bool {
        if self.context.as_ref() != Some(value) || *self.generation != generation || !*self.loading
        {
            return false;
        }
        *self.loading = false;
        match result {
            Ok(rows) => {
                *self.loaded_at =
                    rows.iter().all(|row| row.observation.is_ok()).then(web_time::Instant::now);
                *self.rows = rows;
            }
            Err(error) => *self.error = Some(error),
        }
        true
    }
}

impl CatalogInput<'_> {
    pub fn reset_if_context_changed(&mut self, context: &AgentAppsContext) -> bool {
        if self.context.as_ref() == Some(context) {
            return false;
        }
        self.credentials(None, Some(String::new()));
        *self.hovered = None;
        *self.connection_open = false;
        *self.error = Some("连接已更改，正在重新连接应用".into());
        true
    }
}
#[derive(Default)]
pub struct CatalogState {
    pub rows: Vec<CatalogRow>,
    pub host: String,
    pub token: String,
    pub query: String,
    pub loading: bool,
    pub error: Option<String>,
    pub expanded: Option<vw_api_app_identity::AppId>,
    pub hovered: Option<vw_api_app_identity::AppId>,
    pub connection_open: bool,
    pub preferences: super::preferences::PreferencesState,
    pub launch: super::launch::LaunchState,
    pub lifecycle: super::lifecycle::LifecycleState,
    pub drag: super::drag::DragState,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
    pub loaded_at: Option<web_time::Instant>,
}

impl CatalogState {
    pub fn input(&mut self) -> CatalogInput<'_> {
        CatalogInput {
            context: &mut self.context,
            connection_open: &mut self.connection_open,
            host: &mut self.host,
            token: &mut self.token,
            query: &mut self.query,
            generation: &mut self.generation,
            loading: &mut self.loading,
            rows: &mut self.rows,
            loaded_at: &mut self.loaded_at,
            error: &mut self.error,
            expanded: &mut self.expanded,
            hovered: &mut self.hovered,
            drag: &mut self.drag,
            lifecycle: &mut self.lifecycle,
            preferences: &mut self.preferences,
            launch: &mut self.launch,
        }
    }
    pub fn begin(&mut self, context: AgentAppsContext) -> u64 {
        self.input().begin(context)
    }
    pub fn finish(
        &mut self,
        context: &AgentAppsContext,
        generation: u64,
        result: Result<Vec<CatalogRow>, String>,
    ) -> bool {
        self.input().finish(context, generation, result)
    }
    pub fn reset_if_context_changed(&mut self, context: &AgentAppsContext) -> bool {
        self.input().reset_if_context_changed(context)
    }
    pub fn cache_is_fresh(&self) -> bool {
        self.error.is_none()
            && !self.host.trim().is_empty()
            && !self.token.is_empty()
            && self.loaded_at.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(30))
    }
}
impl std::fmt::Debug for CatalogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatalogState")
            .field("generation", &self.generation)
            .field("loading", &self.loading)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshDecision {
    Skip,
    ConnectLocal,
    OpenConnection,
    Load,
}
impl CatalogInput<'_> {
    pub fn refresh_decision(&mut self, use_cache: bool, local: bool) -> RefreshDecision {
        if use_cache
            && self.error.is_none()
            && !self.host.trim().is_empty()
            && !self.token.is_empty()
            && self.loaded_at.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(30))
        {
            return RefreshDecision::Skip;
        }
        if self.host.trim().is_empty() || self.token.is_empty() {
            if local && cfg!(unix) {
                return RefreshDecision::ConnectLocal;
            }
            *self.connection_open = true;
            return RefreshDecision::OpenConnection;
        }
        if *self.loading {
            return RefreshDecision::Skip;
        }
        *self.connection_open = false;
        RefreshDecision::Load
    }
    pub fn local_ready(
        &mut self,
        current: Option<&AgentAppsContext>,
        context: &AgentAppsContext,
        generation: u64,
        result: Result<local_app_auth::Credentials, String>,
    ) -> Result<bool, String> {
        if !accepts_local_credentials(current, context, *self.generation, generation, *self.loading)
        {
            return Ok(false);
        }
        *self.loading = false;
        match result {
            Ok(credentials) => {
                *self.token = credentials.token;
                *self.host = local_app_auth::HOST_ID.into();
                Ok(true)
            }
            Err(error) => {
                *self.error = Some(error.clone());
                Err(error)
            }
        }
    }
}

pub struct ObservationRequest {
    pub generation: u64,
    pub host: String,
    pub token: String,
}
impl CatalogInput<'_> {
    pub fn begin_observation(
        &mut self,
        context: &AgentAppsContext,
        id: AppId,
    ) -> Result<Option<ObservationRequest>, &'static str> {
        if *self.loading {
            return Ok(None);
        }
        if self.context.as_ref() != Some(context) || self.host.is_empty() || self.token.is_empty() {
            return Err("请先连接应用服务");
        }
        if !self.rows.iter().any(|row| row.entry.id == id) {
            return Ok(None);
        }
        *self.generation = self.generation.wrapping_add(1);
        *self.loading = true;
        Ok(Some(ObservationRequest {
            generation: *self.generation,
            host: self.host.clone(),
            token: self.token.clone(),
        }))
    }
    pub fn toggle_details(&mut self, id: AppId) {
        *self.expanded = (*self.expanded != Some(id)).then_some(id);
    }
    pub fn unavailable(&mut self) -> &'static str {
        *self.loading = false;
        *self.generation = self.generation.wrapping_add(1);
        self.rows.clear();
        let label = "网关不可用，请配置连接后重试";
        *self.error = Some(label.into());
        label
    }
}
