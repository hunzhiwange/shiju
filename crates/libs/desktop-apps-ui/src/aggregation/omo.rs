use crate::ProviderScope as Scope;
use vw_api_transport::ProjectId;
use gateway_aggregation_omo::AggregationOmoError;
pub use super::omo_state::{Form, Snapshot, Variant};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub project: ProjectId,
    pub access: u64,
}
#[derive(Clone)]
pub enum Event {
    Open(Variant),
    Close,
    Refresh,
    Provider(String),
    Apply,
    Disable,
    Finished { context: Context, generation: u64, result: Result<Snapshot, AggregationOmoError> },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OmoEvent")
    }
}
