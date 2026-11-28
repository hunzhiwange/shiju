use gateway_desktop_settings::{DesktopSkillSyncItem, SkillSyncScope, SkillSyncTarget};
use iced::Point;

#[derive(Clone, Debug)]
pub enum Event {
    Menu(String, Point),
    CloseMenu,
    Open(String),
    Close,
    Scope(SkillSyncScope),
    Select(SkillSyncTarget, bool),
    Submit,
    Completed(u64, Result<Vec<DesktopSkillSyncItem>, String>),
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub menu: Option<(String, Point)>,
    pub form: Option<Form>,
    pub generation: u64,
}

#[derive(Clone, Debug)]
pub struct Form {
    pub skill_id: String,
    pub expected_source_path: Option<String>,
    pub title: String,
    pub project_path: Option<String>,
    pub scope: SkillSyncScope,
    pub targets: Vec<SkillSyncTarget>,
    pub busy: bool,
    pub error: Option<String>,
    pub results: Vec<DesktopSkillSyncItem>,
}
