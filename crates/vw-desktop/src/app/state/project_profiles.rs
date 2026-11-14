//! Read state for the three independent application groups.
use vw_gateway_client::vw_api_types::project_profiles::{ProjectProfile, ProjectProfileId, ProjectProfilePerApp};
#[derive(Clone, Default)]
pub(crate) struct Group {
    pub profiles: Vec<ProjectProfile>,
    pub current: Option<ProjectProfileId>,
    pub query: String,
    pub loading: bool,
    pub loaded: bool,
    pub error: bool,
}
#[derive(Clone, Default)]
pub(crate) struct State {
    pub groups: ProjectProfilePerApp<Group>,
    pub generation: u64,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
