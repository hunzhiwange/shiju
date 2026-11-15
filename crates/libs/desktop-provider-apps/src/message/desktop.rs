use crate::Scope;
use vw_api_app_provider_profiles::claude_desktop::*;
use vw_api_app_provider_profiles::desktop_mapping::*;
#[derive(Clone)]
pub enum Event {
    Open(Option<String>),
    Cancel,
    Id(String),
    Name(String),
    BaseUrl(String),
    Key(String),
    Official(bool),
    Apply(bool),
    Proxy(bool),
    Model(String, String),
    Label(String, String),
    OneM(String, bool),
    Remove(String),
    NewRoute(String),
    AddRoute,
    Defaults,
    SaveMapping,
    SaveDirect,
    Import,
    Activate,
    Loaded {
        scope: Scope,
        generation: u64,
        result: Result<DesktopFormView, String>,
    },
    DefaultsLoaded {
        scope: Scope,
        generation: u64,
        result: Result<Vec<DesktopDefaultRoute>, String>,
    },
    Finished {
        scope: Scope,
        generation: u64,
        result: Result<String, String>,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopEvent([redacted])")
    }
}
pub fn message(value: Event) -> crate::Message { crate::Message::Desktop(value) }
