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
    pub management: Option<Management>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}

#[derive(Clone)]
pub enum Action {
    Create,
    Rename(ProjectProfileId, u64),
    Resnapshot(ProjectProfileId, u64),
    Delete(ProjectProfileId, u64),
    Clear,
}
impl Action {
    pub fn needs_name(&self) -> bool { matches!(self, Self::Create | Self::Rename(..)) }
    pub fn title(&self) -> &'static str {
        match self {
            Self::Create => "创建组合", Self::Rename(..) => "改名",
            Self::Resnapshot(..) => "重新拍摄当前组", Self::Delete(..) => "确认删除组合",
            Self::Clear => "清除当前标记",
        }
    }
}
#[derive(Clone)]
pub(crate) struct Management {
    pub scope: vw_gateway_client::vw_api_types::project_profiles::ProjectProfileScope,
    pub action: Action,
    pub name: String,
    pub busy: bool,
}
