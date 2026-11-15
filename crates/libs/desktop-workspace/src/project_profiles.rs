//! Read state for the three independent application groups.
use vw_api_project_profiles::{ProjectProfile, ProjectProfileId, ProjectProfilePerApp};
#[derive(Clone, Default)]
pub struct Group {
    pub profiles: Vec<ProjectProfile>,
    pub current: Option<ProjectProfileId>,
    pub query: String,
    pub loading: bool,
    pub loaded: bool,
    pub error: bool,
    /// Historical group outcome; never confirms an in-flight application.
    pub stored_result: Option<vw_api_project_profiles::ProjectProfileApplyResult>,
    pub result_error: bool,
}
#[derive(Clone)]
pub struct State<Identity> {
    pub groups: ProjectProfilePerApp<Group>,
    pub generation: u64,
    pub management: Option<Management>,
    pub application: Option<Application<Identity>>,
}
impl<Identity> Default for State<Identity> {
    fn default() -> Self {
        Self { groups: Default::default(), generation: 0, management: None, application: None }
    }
}
impl<Identity> State<Identity> {
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
pub struct Management {
    pub scope: vw_api_project_profiles::ProjectProfileScope,
    pub action: Action,
    pub name: String,
    pub busy: bool,
}

/// A submitted write is never replayed from this modal, even after transport failure.
#[derive(Clone)]
pub struct Application<Identity> {
    pub scope: vw_api_project_profiles::ProjectProfileScope,
    pub owner: Identity,
    pub operation_id: Option<String>,
    pub cancelling: bool,
    pub id: ProjectProfileId,
    pub version: u64,
    pub name: String,
    pub loading: bool,
    pub submitted: bool,
    pub preview: Option<Preview>,
    pub result: Option<vw_api_project_profiles::ProjectProfileApplyResult>,
    pub error: Option<&'static str>,
}
/// Fail closed until the executor supplies this complete, structured preview.
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub group: vw_api_project_profiles::ProjectProfileScope,
    pub captured: bool,
    pub items: Vec<PreviewItem>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewItem {
    pub category: vw_api_project_profiles::ProjectProfileCategory,
    pub reference_id: String,
    pub action: PreviewAction,
}
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewAction { Select, Enable, Disable, Missing }
