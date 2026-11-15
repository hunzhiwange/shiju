//! 技能更新所需的借用输入与桌面组合效果。
use desktop_account::StoredAccount;
use desktop_skill_hub::skill_hub::ExpertHubState;
use desktop_i18n::format::{Argument, Message as Feedback};
use iced::Task;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubPage { ExpertHub, ShijuSkills, ShijuApps, Other }
#[derive(Debug, Clone, Copy)]
pub enum ToastKind { Info, Success, Warning, Error }
pub enum Effect {
    SelectExpertForChat(String),
    OpenExpertHub, OpenSkills, CloseAccountMenu,
    ClearAccount(Option<String>),
    EnsurePublic, SelectPublicSuites, EnsureConnectors, EnsureKnowledge,
    ReloadConnectors, ReloadKnowledge,
    Feedback(ToastKind, Feedback, config_ui::DesktopLocale),
}
#[derive(Debug, Clone)]
pub enum Message {
    SkillHub(super::skill_hub::SkillHubMessage),
    CopyCode(String),
    ReloadApps,
}
/// 只借用技能状态及本次更新实际消费的账号、项目和页面信息。
pub struct SkillUpdate<'a> {
    pub expert_hub: &'a mut ExpertHubState,
    pub active_account: Option<&'a StoredAccount>,
    pub project_path: &'a Option<String>,
    pub recent_projects: &'a [String],
    pub screen: HubPage,
    pub has_project_chat: bool,
    pub desktop_locale: config_ui::DesktopLocale,
    pub gateway: crate::GatewayFactory,
    pub effects: Vec<Effect>,
}
impl SkillUpdate<'_> {
    pub fn clear_expert_hub_account(&mut self, id: Option<String>) {
        self.expert_hub.clear_for_account(id.clone());
        self.effects.push(Effect::ClearAccount(id));
    }
    pub fn effect(&mut self, effect: Effect) -> Task<Message> {
        self.effects.push(effect);
        Task::none()
    }
    pub fn feedback(&mut self, kind: ToastKind, message: Feedback) -> Task<Message> {
        self.effect(Effect::Feedback(kind, message, self.desktop_locale))
    }
    pub fn show_localized_toast(&mut self, kind: ToastKind, source: &str, arguments: Vec<Argument>) -> Task<Message> {
        self.effect(Effect::Feedback(kind, Feedback::new(source, arguments), config_ui::DesktopLocale::SimplifiedChinese))
    }
}
