use super::presentation::*;
#[derive(Debug, Clone)]
pub enum ProjectMessage {
CurrentProjectMenuToggled(String),
OpenProjectSessionPressed(String, String),
ProjectEditOpened(String),
ProjectLoadSessions(String),
ProjectToolsMenuClosed,
ProjectToolsMenuToggled(String),
RecentHovered(Option<String>),
RecentRemovePressed(String),
RecentRevealPressed(String),
SessionMenuClose,
SessionRightClicked(String, f32, f32),
SwitchWorkSpacePressed(String),
}
#[derive(Debug, Clone)]
pub enum ViewMessage {
OpenApps,
OpenKnowledge,
OpenSystemSettingsTab(SystemTab),
OpenWorkflowTool,
ProjectFileNewProject,
ProjectFileNewSession,
ToggleWebLinksMenu,
WorkProjectHome,
WorkSidebarFilterClosed,
WorkSidebarFilterReset,
WorkSidebarFilterScopeSelected(WorkSidebarFilterScope),
WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus),
WorkSidebarFilterTimeSelected(WorkSidebarFilterTime),
WorkSidebarFilterToggled,
WorkSidebarLoadMorePressed,
WorkSidebarMoreClosed,
WorkSidebarMoreOpened,
WorkSidebarProjectNavHovered(bool),
WorkSidebarSpaceTasksLoadMorePressed(String),
WorkSidebarTabSelected(WorkSidebarTab),
WorkSidebarToggled,
WorkSpaceToggled(String),
}
#[derive(Debug, Clone)]
pub enum SkillHubMessage {
Open,
OpenShijuSkills,
SelectSection(ExpertHubSection),
}
#[derive(Debug, Clone)]
pub enum ShijuAppsMessage {
Open,
}
#[derive(Debug, Clone)]
pub enum SearchMessage {
Open,
}
#[derive(Debug, Clone)]
pub enum TaskBoardMessage {
ToggleBoard,
}
#[derive(Debug, Clone)]
pub enum SystemTab {
Agents,
Cron,
Projects,
}
#[derive(Debug, Clone)]
pub enum ExpertHubSection {
Experts,
Knowledge,
}
#[derive(Debug, Clone)]
pub enum Message { Batch(Vec<Message>), Project(ProjectMessage), View(ViewMessage), #[cfg(not(target_arch = "wasm32"))] SkillHub(SkillHubMessage), #[cfg(not(target_arch = "wasm32"))] ShijuApps(ShijuAppsMessage), Search(SearchMessage), TaskBoard(TaskBoardMessage), Session(super::super::super::components::sessions_panel::Message) }
