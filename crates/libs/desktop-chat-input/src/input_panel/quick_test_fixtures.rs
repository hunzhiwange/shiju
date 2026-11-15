use super::*;
#[derive(PartialEq)]
pub enum WorkspaceMode { Work, Code }
pub struct Popovers { pub quick_message_group: Option<QuickMessageGroup>, pub recommended_prompt: bool, pub shiju_skill: bool }
pub struct Entry { pub key: String, pub recommended_prompts: Vec<String> }
pub struct Agents { pub entries: Vec<Entry> }
pub struct Runtime { pub tool_selector: desktop_chat_tools::state::SessionToolSelectorState }
pub struct Fixture {
 pub expert_hub: desktop_skill_hub::skill_hub::ExpertHubState,
 pub workspace_mode: WorkspaceMode, pub project_is_git_repo: bool,
 pub quick_messages: Vec<QuickMessageConfig>, pub input_popovers: Popovers,
 pub recent_projects: Vec<String>, pub recent_projects_edits: Vec<String>,
 pub project_path: Option<String>, pub work_space_picker_open: bool,
 pub agents_settings: Agents, pub runtime: Runtime,
}
impl Fixture {
 pub fn new() -> (Self, ()) { (Self {
 expert_hub: Default::default(), workspace_mode: WorkspaceMode::Code, project_is_git_repo: false,
 quick_messages: config_ui::AppSystemSettingsConfig::default().quick_messages,
 input_popovers: Popovers { quick_message_group: None, recommended_prompt: false, shiju_skill: false },
 recent_projects: vec![], recent_projects_edits: vec![], project_path: None, work_space_picker_open: false,
 agents_settings: Agents { entries: vec![Entry { key: "main".into(), recommended_prompts: vec![] }] },
 runtime: Runtime { tool_selector: Default::default() },
 }, ()) }
 pub fn current_session_runtime_mut(&mut self) -> &mut Runtime { &mut self.runtime }
 pub fn input(&self) -> QuickInput<'_> { QuickInput {
 i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
 quick_messages: &self.quick_messages, quick_message_group: self.input_popovers.quick_message_group,
 recommended_prompt: self.input_popovers.recommended_prompt,
 recommended_prompts: Some(&self.agents_settings.entries[0].recommended_prompts),
 show_workspace: self.workspace_mode == WorkspaceMode::Work, project_is_git_repo: self.project_is_git_repo,
 recent_projects: &self.recent_projects, recent_projects_edits: &self.recent_projects_edits,
 project_path: self.project_path.as_deref(), work_space_picker_open: self.work_space_picker_open,
 shiju_skill: self.input_popovers.shiju_skill, installed: &self.expert_hub.installed,
 package_status: &self.expert_hub.package_status, tool_selector: Some(&self.runtime.tool_selector),
 } }
}
