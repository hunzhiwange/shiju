//! 领域状态重导出与桌面跨域账号清理。
pub(crate) use desktop_skill_hub::skill_hub::*;

impl crate::app::App {
    pub(crate) fn clear_expert_hub_account(&mut self, account_id: Option<String>) {
        self.expert_hub.clear_for_account(account_id.clone());
        self.shiju_apps.clear_for_account(account_id);
    }
}
