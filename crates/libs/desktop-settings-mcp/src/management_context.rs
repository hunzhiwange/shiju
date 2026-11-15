use desktop_provider_apps::AgentAppsContext;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub endpoint: AgentAppsContext,
    pub host: String,
    pub credentials_generation: u64,
}
