pub use desktop_skill_hub::message::{Effect, HubPage, SkillUpdate, ToastKind};
#[derive(Debug, Clone)]
pub enum Message {
    SkillHub(super::skill_hub::SkillHubMessage),
    CopyCode(String),
    ReloadApps,
}
