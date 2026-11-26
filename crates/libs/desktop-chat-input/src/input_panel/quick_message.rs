#[derive(Debug, Clone)]
pub enum Message {
 QuickMessageSelected(usize), QuickMessageGroupChanged(Option<config_ui::QuickMessageGroup>),
 EmployeeCommandMenuChanged(bool),
 EmployeeCommandSelected { input: &'static str, needs_argument: bool },
 #[cfg(not(target_arch = "wasm32"))]
 MediaShortcutMenuChanged(bool),
 #[cfg(not(target_arch = "wasm32"))]
 MediaPromptSelected(String),
 #[cfg(not(target_arch = "wasm32"))]
 OpenMediaModelPicker(vw_api_media::MediaOperation),
 #[cfg(not(target_arch = "wasm32"))]
 BackFromMediaModelPicker,
 #[cfg(not(target_arch = "wasm32"))]
 MediaModelPicked(Option<vw_api_media::MediaModelSelection>),
 #[cfg(not(target_arch = "wasm32"))]
 SelectVideoEditorSkill,
 #[cfg(not(target_arch = "wasm32"))]
 SelectAppDevelopmentSkill,
 #[cfg(not(target_arch = "wasm32"))]
 SelectShijuSkill(u64),
 #[cfg(not(target_arch = "wasm32"))]
 RefreshInstalledSkills,
 #[cfg(not(target_arch = "wasm32"))]
 OpenInstalledShijuSkills,
 #[cfg(not(target_arch = "wasm32"))]
 ToggleShijuSkillMenu,
 #[cfg(not(target_arch = "wasm32"))]
 PrepareNativeShijuSkill, AgentRecommendedPromptSelected(usize), RecommendedPromptMenuChanged(bool),
 WorkSpacePickerClosed, WorkSpacePickerToggled, SwitchWorkSpacePressed(String), OpenFolderPressed,
 Batch(Vec<Message>),
}
