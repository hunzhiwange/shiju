//! 升级入口、目标范围、互斥与迟到结果的回归验证。

use super::*;
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use crate::app::state::ToastKind;
use upgrades::{FunctionUpgradeReport, main_selections};

fn app_with_skills() -> App {
    let mut app = App::new().0;
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("test-secret").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    app.expert_hub.account_id = app.account.active_account().map(|account| account.id().to_owned());
    app.expert_hub.installed = LoadState::Loaded(vec![InstalledSkill {
        skill_id: 7,
        slug: "example".into(),
        name: "示例功能".into(),
        description: String::new(),
        installed_version: "1.0.0".into(),
        current_version: "2.0.0".into(),
        format: "skill".into(),
        upgrade_available: true,
    }]);
    app
}

#[test]
fn upgrade_entry_opens_installed_skills_and_closes_detail() {
    let mut app = app_with_skills();
    app.expert_hub.install_guide.open = true;
    app.expert_hub.skill_tab = SkillTab::Install;
    let _ = update(&mut app, SkillHubMessage::OpenInstalledShijuSkills);
    assert_eq!(app.screen, Screen::ShijuSkills);
    assert_eq!(app.expert_hub.skill_tab, SkillTab::Installed);
    assert!(!app.expert_hub.install_guide.open);
    assert!(app.expert_hub.detail.is_none());
    assert!(matches!(app.expert_hub.installed, LoadState::Loading { .. }));
}
