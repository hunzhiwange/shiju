use crate::Scope;
use vw_api_app_provider_edit::{endpoints::*,request_options::*};
#[derive(Clone)]
pub enum Event {
    Open(String),
    Cancel,
    Refresh,
    Url(String),
    FullUrl(bool),
    Headers(String),
    Overrides(String),
    UserAgentEnabled(bool),
    UserAgent(String),
    Add,
    Remove(String, AppProviderEndpointKind),
    MarkUsed(String, AppProviderEndpointKind),
    SaveOptions,
    Loaded {
        scope: Scope,
        generation: u64,
        result: Result<Vec<AppProviderEndpoint>, String>,
    },
    OptionsLoaded {
        scope: Scope,
        generation: u64,
        saved: bool,
        result: Result<RequestOptionsView, String>,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EndpointEvent([redacted])")
    }
}
pub fn message(value: Event) -> crate::Message { crate::Message::Endpoints(value) }
