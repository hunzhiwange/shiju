use crate::Scope;
use vw_api_types::agent_apps::CustomIconEntry;
use vw_api_app_provider_edit::icons::AppProviderIcon;
#[derive(Clone)]
pub enum Outcome {
    Refreshed {
        rows: Result<Vec<CustomIconEntry>, String>,
        current: Result<AppProviderIcon, String>,
        deleted: Option<String>,
    },
    Uploaded(CustomIconEntry),
    Renamed(CustomIconEntry),
    Saved(AppProviderIcon),
    FilePicked(Option<(String, String)>),
}

pub const PNG_PREFIX: &str = "data:image/png;base64,";
pub const MAX_ICON_BYTES: usize = 64 * 1024;
pub const SCROLL_ID: &str = "cc04-provider-icons-scroll";

#[derive(Clone)]
pub enum Event {
    Open(String),
    Close,
    Refresh,
    Pick,
    Rename(String),
    Delete(String),
    DraftName(String),
    CancelDraft,
    CommitDraft,
    Select(String),
    Clear,
    Save,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, String> },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderIconEvent([redacted])")
    }
}

pub fn message(value: Event) -> crate::Message { crate::Message::Icons(value) }
