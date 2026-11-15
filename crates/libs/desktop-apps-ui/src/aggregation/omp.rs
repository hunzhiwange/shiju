use crate::ProviderScope as Scope;
use vw_api_transport::ProjectId;
use iced::widget::text_editor;
pub use super::omp_core::*;
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Id(String),
    Name(String),
    Declaration(text_editor::Action),
    Save,
    ReviewRemove(String),
    CancelRemove,
    ConfirmRemove,
    Completed {
        scope: Scope,
        project: ProjectId,
        access: u64,
        generation: u64,
        writing: bool,
        result: Result<
            vw_api_app_aggregation::OmpDirectoryState,
            String,
        >,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OmpDirectoryEvent([redacted])")
    }
}
