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
