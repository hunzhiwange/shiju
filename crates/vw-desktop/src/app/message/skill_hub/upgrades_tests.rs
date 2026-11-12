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

#[test]
fn main_upgrade_includes_native_and_only_selected_agent_directories() {
    let mut app = app_with_skills();
    app.expert_hub.install_guide.install_scope = SkillInstallScope::Project;
    app.expert_hub.install_guide.selected_targets =
        vec![SkillInstallTarget::Codex, SkillInstallTarget::ClaudeCode];
    assert_eq!(
        main_selections(&app),
        vec![
            SkillInstallSelection {
                target: SkillInstallTarget::VibeWindow,
                scope: SkillInstallScope::Global
            },
            SkillInstallSelection {
                target: SkillInstallTarget::Codex,
                scope: SkillInstallScope::Project
            },
            SkillInstallSelection {
                target: SkillInstallTarget::ClaudeCode,
                scope: SkillInstallScope::Project
            },
        ]
    );
    app.expert_hub.install_guide.selected_targets = vec![SkillInstallTarget::VibeWindow];
    app.expert_hub.install_guide.install_scope = SkillInstallScope::Global;
    assert_eq!(main_selections(&app).len(), 1);
}

#[test]
fn function_upgrade_prevents_duplicates_and_conflicting_operations() {
    let mut app = app_with_skills();
    let _ = update(&mut app, SkillHubMessage::UpgradeAllFunctions);
    let generation = app.expert_hub.function_upgrade.expect("batch started");
    let _ = update(&mut app, SkillHubMessage::UpgradeAllFunctions);
    let _ = update(&mut app, SkillHubMessage::UpgradeSkill(7));
    let _ = update(&mut app, SkillHubMessage::UpgradeMainSkill);
    let _ = update(&mut app, SkillHubMessage::DirectInstallShiju);
    assert_eq!(app.expert_hub.function_upgrade, Some(generation));
    assert!(app.expert_hub.action.is_none());
    assert!(app.expert_hub.main_upgrade.is_none());
    assert!(matches!(app.expert_hub.install_guide.direct_install, ShijuDirectInstallState::Idle));
}

#[test]
fn function_upgrade_keeps_cache_retries_and_ignores_stale_results() {
    let mut app = app_with_skills();
    let _ = update(&mut app, SkillHubMessage::UpgradeAllFunctions);
    let generation = app.expert_hub.function_upgrade.unwrap();
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = update(
        &mut app,
        SkillHubMessage::AllFunctionsUpgraded {
            generation: generation.wrapping_sub(1),
            account_id: account_id.clone(),
            result: Ok(FunctionUpgradeReport::default()),
        },
    );
    assert_eq!(app.expert_hub.function_upgrade, Some(generation));
    let _ = update(
        &mut app,
        SkillHubMessage::AllFunctionsUpgraded {
            generation,
            account_id,
            result: Ok(FunctionUpgradeReport {
                succeeded: 1,
                failed: vec![8],
                cache_pending: vec![7],
            }),
        },
    );
    assert!(app.expert_hub.function_upgrade.is_none());
    assert_eq!(app.expert_hub.function_cache_retry, vec![7]);
    assert_eq!(app.active_toast.as_ref().unwrap().kind, ToastKind::Warning);
    app.expert_hub.installed = LoadState::Loaded(vec![]);
    let _ = update(&mut app, SkillHubMessage::UpgradeAllFunctions);
    assert!(
        app.expert_hub.function_upgrade.is_some(),
        "cache retries work without remote upgrades"
    );
    app.expert_hub.clear_for_account(None);
    assert!(app.expert_hub.function_upgrade.is_none());
    assert!(app.expert_hub.function_cache_retry.is_empty());
}

#[test]
fn main_upgrade_rejects_missing_project_and_accepts_selected_targets() {
    let mut app = app_with_skills();
    app.expert_hub.skill_tab = SkillTab::Installed;
    app.expert_hub.install_guide.install_scope = SkillInstallScope::Project;
    app.expert_hub.install_guide.selected_targets = vec![SkillInstallTarget::Codex];
    app.expert_hub.install_guide.selected_project_path = None;
    let _ = update(&mut app, SkillHubMessage::UpgradeMainSkill);
    assert!(app.expert_hub.main_upgrade.is_none());
    app.expert_hub.install_guide.selected_project_path = Some("/example/project".into());
    let _ = update(&mut app, SkillHubMessage::UpgradeMainSkill);
    if ShijuPackageId::current().is_some() {
        assert!(app.expert_hub.main_upgrade.is_some());
        assert_eq!(app.expert_hub.skill_tab, SkillTab::Installed);
    }
}

#[test]
fn main_upgrade_ignores_stale_package_and_releases_busy_state_on_failure() {
    let mut app = app_with_skills();
    let _ = update(&mut app, SkillHubMessage::UpgradeMainSkill);
    let Some(generation) = app.expert_hub.main_upgrade else { return };
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let selections = main_selections(&app);
    let _ = update(
        &mut app,
        SkillHubMessage::MainUpgradePackageLoaded {
            generation: generation.wrapping_sub(1),
            account_id: account_id.clone(),
            selections,
            project_root: None,
            result: Err("stale package".into()),
        },
    );
    assert_eq!(app.expert_hub.main_upgrade, Some(generation));
    let _ = update(
        &mut app,
        SkillHubMessage::MainUpgradePackageLoaded {
            generation,
            account_id,
            selections: vec![],
            project_root: None,
            result: Err("package unavailable".into()),
        },
    );
    assert!(app.expert_hub.main_upgrade.is_none());
    assert_eq!(app.active_toast.as_ref().unwrap().kind, ToastKind::Error);
}
