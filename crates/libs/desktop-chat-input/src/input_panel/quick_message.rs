#[derive(Debug, Clone)]
pub enum Message {
 QuickMessageSelected(usize), QuickMessageGroupChanged(Option<config_ui::QuickMessageGroup>),
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
