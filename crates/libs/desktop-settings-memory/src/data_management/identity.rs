use desktop_provider_apps::AgentAppsContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub context: AgentAppsContext,
    pub project: String,
    pub host: String,
    pub catalog: u64,
}
