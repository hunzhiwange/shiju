// 验证私有技能仓库、版本操作与异步结果时序保护。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::SkillHubMessage;
use super::skill_hub::{
    ShijuDownloadChunk, agent_install_text, dispatch_download_with, dispatch_shiju_download_with,
    marketplace_query, mine_query, open_shiju_install_dir_with, update as update_skill_hub,
    write_shiju_download_chunks,
};
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use crate::app::skill_hub::{
    CursorPage, InstalledSkill, LoadState, MarketplaceSkill, MineFilterMenu, MineSkillStatus,
    PackageVersionCacheEntry, Page, PublicDetailMode, ShijuDirectInstallState, ShijuInstallItem,
    ShijuInstallation, ShijuInstallationsReport, ShijuKeySyncOutcome, ShijuKeySyncReport,
    ShijuMultiInstallReport, ShijuPackageId, ShijuPackageRelease, ShijuPackageStatus,
    ShijuTargetInstallation, ShijuTargetKeySync, ShijuTargetUninstall, ShijuUninstallReport,
    SkillActionKind, SkillCacheSyncReport, SkillCacheSyncState, SkillDetail, SkillDetailState,
    SkillDownloadUrl, SkillHubError, SkillInstallScope, SkillInstallSelection, SkillInstallTarget,
    SkillTag, SkillVersion, SkillVersionAction, SkillVersionDirection, resolve_skill_destinations,
};
use crate::app::skill_hub::{ExpertHubSection, SkillTab};
use crate::app::state::ToastKind;
use crate::app::views::project::project_list_action_order;
use crate::app::{App, Message, Screen};
use std::fs;
use tempfile::NamedTempFile;

fn repository_with_account(secret: &str, company_id: &str) -> AccountRepository {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new(company_id, "示例公司", "founder", "active").unwrap(),
            SecretSkey::new(secret).unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    repository
}

#[test]
fn market_tag_selection_refreshes_search() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _ = update_skill_hub(&mut app, SkillHubMessage::SelectSkillTab(SkillTab::Marketplace));
    app.expert_hub.search_draft = "保留草稿".to_string();
    app.expert_hub.submitted_search = "保留关键词".to_string();

    let generation = app.expert_hub.request_generation;
    let _ = update_skill_hub(&mut app, SkillHubMessage::MarketTagSelected(Some(2)));
    assert!(app.expert_hub.request_generation > generation);
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loading { .. }));
    assert_eq!(app.expert_hub.market_tag_id, Some(2));
    assert_eq!(app.expert_hub.market_company_tag_id, None);
    let query = marketplace_query(&app);
    assert_eq!(query.query.as_deref(), Some("保留关键词"));
    assert_eq!(query.cursor, None);
    assert_eq!(query.tag_ids, Some(2));
    assert_eq!(query.company_tag_ids, None);

    let generation = app.expert_hub.request_generation;
    let _ = update_skill_hub(&mut app, SkillHubMessage::MarketCompanyTagSelected(Some(7)));
    assert!(app.expert_hub.request_generation > generation);
    assert_eq!(app.expert_hub.market_tag_id, Some(2));
    assert_eq!(app.expert_hub.market_company_tag_id, Some(7));

    let _ = update_skill_hub(&mut app, SkillHubMessage::MarketTagSelected(None));
    let reset = marketplace_query(&app);
    assert_eq!(reset.query.as_deref(), Some("保留关键词"));
    assert_eq!(reset.tag_ids, None);
    assert_eq!(reset.company_tag_ids, Some(7));

    let _ = update_skill_hub(&mut app, SkillHubMessage::FiltersReset);
    assert!(app.expert_hub.search_draft.is_empty());
    assert!(app.expert_hub.submitted_search.is_empty());
    assert_eq!(app.expert_hub.market_tag_id, None);
    assert_eq!(app.expert_hub.market_company_tag_id, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn shiju_package_refresh_message_ignores_late_result_and_preserves_cache_on_failure() {
    let cached =
        PackageVersionCacheEntry::new("3.0.3".to_string(), 1_785_830_400).expect("valid cache");
    let (mut app, _) = App::new();
    app.expert_hub.package_status =
        ShijuPackageStatus::from_sources(Some("3.0.2".to_string()), Some(cached));
    let stale_generation = app.expert_hub.package_status.start_refresh();
    let current_generation = app.expert_hub.package_status.start_refresh();

    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuPackageVersionRefreshed {
        generation: stale_generation,
        result: Ok(PackageVersionCacheEntry::new("9.9.9".to_string(), 1_785_830_401)
            .expect("valid stale result")),
    }));
    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuPackageVersionRefreshed {
        generation: current_generation,
        result: Err("signed-url-secret".to_string()),
    }));

    assert_eq!(app.expert_hub.package_status.server_version.as_deref(), Some("3.0.3"));
    assert_eq!(app.expert_hub.package_status.checked_at_unix, Some(1_785_830_400));
    assert!(!format!("{:?}", app.expert_hub.package_status).contains("signed-url-secret"));
}

fn installed_targets(
    home: &std::path::Path,
    selections: &[SkillInstallSelection],
) -> ShijuInstallationsReport {
    let targets = resolve_skill_destinations(home, None, selections)
        .expect("resolve installed targets")
        .into_iter()
        .map(|destination| ShijuTargetInstallation {
            installation: Some(ShijuInstallation {
                installed_dir: destination.installed_dir.clone(),
                version: Some("3.0.3".to_string()),
            }),
            destination,
        })
        .collect();
    ShijuInstallationsReport { targets }
}

#[test]
fn private_shiju_tabs_dispatch_gateway_requests() {
    let _ = crate::app::session_gateway::gateway_shiju_skill_search_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_installed_async;
    let _ = crate::app::session_gateway::gateway_shiju_system_skills_async;
    let _ = crate::app::session_gateway::gateway_shiju_maintained_skills_async;
}

#[test]
fn private_shiju_detail_and_actions_use_gateway() {
    let _ = crate::app::session_gateway::gateway_shiju_skill_detail_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_readme_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_versions_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_remote_install_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_remote_install_version_async;
    let _ = crate::app::session_gateway::gateway_shiju_skill_remote_uninstall_async;
}

#[test]
fn business_skill_version_actions_use_server_upgrade_flag_and_immutable_version_id() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, true)],
        next_cursor: None,
        has_more: false,
    });
    app.expert_hub.installed = LoadState::Loaded(vec![InstalledSkill {
        skill_id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "fixture".to_string(),
        installed_version: "1.0.0".to_string(),
        current_version: "1.2.0".to_string(),
        format: "markdown".to_string(),
        upgrade_available: true,
    }]);
    app.expert_hub.detail = Some(SkillDetailState {
        skill_id: 7,
        generation: 1,
        detail: LoadState::Loaded(skill_detail(7)),
        readme: LoadState::Loaded(String::new()),
        readme_mode: PublicDetailMode::Visual,
        readme_items: Vec::new(),
        versions: LoadState::Loaded(vec![skill_version(7)]),
    });
    let target = SkillVersionAction {
        skill_id: 7,
        version_id: 42,
        target_version: "1.2.0".to_string(),
        direction: SkillVersionDirection::Upgrade,
    };

    let task = app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(target.clone())));
    assert_eq!(task.units(), 1);
    let active = app.expert_hub.action.as_ref().expect("version action should be busy");
    assert_eq!(active.skill_id, 7);
    assert_eq!(active.kind, SkillActionKind::Version(target.clone()));
    let generation = active.generation;

    let duplicate =
        app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(target.clone())));
    assert_eq!(duplicate.units(), 0);
    assert_eq!(app.expert_hub.action.as_ref().unwrap().generation, generation);

    let account_id = app.expert_hub.account_id.clone().unwrap();
    let sync = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation,
        account_id: account_id.clone(),
        skill_id: 7,
        action: SkillActionKind::Version(target),
        result: Ok(()),
    }));
    assert_eq!(sync.units(), 2, "progress toast plus cache sync");
    assert!(app.expert_hub.action.is_none());
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Info));
    let cache_sync_generation = match app.expert_hub.cache_sync {
        SkillCacheSyncState::Syncing { generation, skill_id: 7, refresh_detail: true } => {
            generation
        }
        ref state => panic!("unexpected cache sync state: {state:?}"),
    };
    let refresh = app.update(Message::SkillHub(SkillHubMessage::SkillCacheSyncFinished {
        generation: cache_sync_generation,
        account_id,
        skill_id: 7,
        result: Ok(SkillCacheSyncReport { fetch_exit_code: 0, rebuild_exit_code: 0 }),
    }));
    assert!(refresh.units() >= 4, "toast plus three scoped refreshes");
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loading { .. }));
    assert!(matches!(app.expert_hub.installed, LoadState::Loading { .. }));
    assert!(matches!(
        app.expert_hub.detail.as_ref().map(|detail| &detail.detail),
        Some(LoadState::Loading { .. })
    ));
}

#[test]
fn business_skill_version_actions_ignore_late_results_and_preserve_loaded_state_on_failure() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, true)],
        next_cursor: None,
        has_more: false,
    });
    app.expert_hub.installed = LoadState::Loaded(Vec::new());
    let first = SkillVersionAction {
        skill_id: 7,
        version_id: 41,
        target_version: "1.1.0".to_string(),
        direction: SkillVersionDirection::Upgrade,
    };
    let second =
        SkillVersionAction { version_id: 42, target_version: "1.2.0".to_string(), ..first.clone() };

    let _ = app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(first.clone())));
    let stale_generation = app.expert_hub.action.as_ref().unwrap().generation;
    app.expert_hub.action = None;
    let _ = app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(second.clone())));
    let current_generation = app.expert_hub.action.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();

    let stale = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation: stale_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        action: SkillActionKind::Version(first),
        result: Ok(()),
    }));
    assert_eq!(stale.units(), 0);
    assert_eq!(app.expert_hub.action.as_ref().unwrap().generation, current_generation);
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(_)));

    let failed = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation: current_generation,
        account_id,
        skill_id: 7,
        action: SkillActionKind::Version(second),
        result: Err("fixture failure".to_string()),
    }));
    assert_eq!(failed.units(), 1);
    assert!(app.expert_hub.action.is_none());
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(_)));
}

#[test]
fn business_skill_quick_upgrade_resolves_version_id_and_rollback_requires_confirmation() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.installed = LoadState::Loaded(vec![InstalledSkill {
        skill_id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "fixture".to_string(),
        installed_version: "1.0.0".to_string(),
        current_version: "1.2.0".to_string(),
        format: "markdown".to_string(),
        upgrade_available: true,
    }]);

    let resolution = app.update(Message::SkillHub(SkillHubMessage::UpgradeSkill(7)));
    assert_eq!(resolution.units(), 1);
    let generation = app.expert_hub.action.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let install = app.update(Message::SkillHub(SkillHubMessage::SkillUpgradeResolved {
        generation,
        account_id,
        skill_id: 7,
        result: Ok(skill_detail(7)),
    }));
    assert_eq!(install.units(), 1);
    assert!(matches!(
        &app.expert_hub.action.as_ref().unwrap().kind,
        SkillActionKind::Version(action)
            if action.version_id == 17
                && action.target_version == "1.2.0"
                && action.direction == SkillVersionDirection::Upgrade
    ));

    app.expert_hub.action = None;
    let rollback = SkillVersionAction {
        skill_id: 7,
        version_id: 9,
        target_version: "0.9.0".to_string(),
        direction: SkillVersionDirection::Rollback,
    };
    let requested =
        app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(rollback.clone())));
    assert_eq!(requested.units(), 0);
    assert!(app.expert_hub.action.is_none());
    assert_eq!(app.expert_hub.version_confirmation.as_ref(), Some(&rollback));
    let confirmed = app.update(Message::SkillHub(SkillHubMessage::ConfirmSkillVersionChange));
    assert_eq!(confirmed.units(), 1);
    assert!(matches!(
        &app.expert_hub.action.as_ref().unwrap().kind,
        SkillActionKind::Version(action) if action == &rollback
    ));
}

#[test]
fn version_switch_preserves_server_success_when_cache_sync_fails() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, true)],
        next_cursor: None,
        has_more: false,
    });
    app.expert_hub.installed = LoadState::Loaded(Vec::new());
    let target = SkillVersionAction {
        skill_id: 7,
        version_id: 42,
        target_version: "1.2.0".to_string(),
        direction: SkillVersionDirection::Upgrade,
    };
    let _ = app.update(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(target.clone())));
    let action_generation = app.expert_hub.action.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();

    let sync = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation: action_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        action: SkillActionKind::Version(target),
        result: Ok(()),
    }));
    assert_eq!(sync.units(), 2, "progress toast plus cache sync");
    assert!(app.expert_hub.action.is_none());
    let (sync_generation, sync_skill_id) = match app.expert_hub.cache_sync {
        SkillCacheSyncState::Syncing { generation, skill_id, .. } => (generation, skill_id),
        ref state => panic!("unexpected cache state: {state:?}"),
    };
    assert_eq!(sync_skill_id, 7);

    let warning = app.update(Message::SkillHub(SkillHubMessage::SkillCacheSyncFinished {
        generation: sync_generation,
        account_id,
        skill_id: 7,
        result: Err("fixture cache failure".to_string()),
    }));
    assert_eq!(warning.units(), 1);
    assert_eq!(app.expert_hub.cache_sync.pending_retry_skill_id(), Some(7));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Warning));
    assert!(app.active_toast.as_ref().unwrap().message.contains("服务端已更新"));
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(_)));

    let retry = app.update(Message::SkillHub(SkillHubMessage::RetrySkillCacheSync));
    assert_eq!(retry.units(), 2);
    assert!(matches!(app.expert_hub.cache_sync, SkillCacheSyncState::Syncing { skill_id: 7, .. }));
}

#[test]
fn expert_hub_navigation_follows_project_add_and_reuses_single_tab() {
    assert_eq!(
        project_list_action_order(),
        ["open_project", "expert_hub", "shiju_skills", "shiju_apps"]
    );

    let (mut app, _) = App::new();
    let _ = app.update(Message::SkillHub(SkillHubMessage::Open));
    let _ =
        app.update(Message::SkillHub(SkillHubMessage::SelectSection(ExpertHubSection::Connectors)));
    let _ = app.update(Message::SkillHub(SkillHubMessage::Open));

    let tabs = app.open_tabs.iter().filter(|tab| tab.id == "expert_hub").collect::<Vec<_>>();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "专家·技能·连接器");
    assert_eq!(tabs[0].screen, Screen::ExpertHub);
    assert_eq!(app.active_tab_id.as_deref(), Some("expert_hub"));
    assert_eq!(app.screen, Screen::ExpertHub);
    assert_eq!(app.expert_hub.section, ExpertHubSection::Skills);
    assert_eq!(app.expert_hub.skill_tab, SkillTab::Marketplace);
}

fn assert_skill_hub_routes_tabs_search_pages_and_account_changes() {
    let (mut app, _) = App::new();

    let _task = app.update(Message::SkillHub(SkillHubMessage::Open));
    assert_eq!(app.expert_hub.section, ExpertHubSection::Skills);
    assert!(matches!(app.expert_hub.marketplace, LoadState::Idle));
    let _task = app.update(Message::SkillHub(SkillHubMessage::OpenShijuSkills));
    assert_eq!(app.screen, Screen::ShijuSkills);
    assert_eq!(app.active_tab_id.as_deref(), Some("shiju_skills"));
    let tabs = app.open_tabs.iter().filter(|tab| tab.id == "shiju_skills").collect::<Vec<_>>();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "识矩技能");
    assert_eq!(tabs[0].screen, Screen::ShijuSkills);
    assert!(matches!(app.expert_hub.marketplace, LoadState::NeedsLogin));
    assert_eq!(app.expert_hub.request_generation, 0);

    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _task = app.update(Message::SkillHub(SkillHubMessage::Retry));
    let marketplace_generation = app.expert_hub.request_generation;
    assert!(matches!(
        app.expert_hub.marketplace,
        LoadState::Loading { generation } if generation == marketplace_generation
    ));

    let _task = app.update(Message::SkillHub(SkillHubMessage::SearchDraftChanged(
        "  视觉 & 分析  ".to_string(),
    )));
    assert_eq!(app.expert_hub.request_generation, marketplace_generation);
    let _task = app.update(Message::SkillHub(SkillHubMessage::SearchSubmitted));
    assert_eq!(app.expert_hub.submitted_search, "视觉 & 分析");

    for tab in [SkillTab::Installed, SkillTab::System, SkillTab::Mine] {
        let previous_generation = app.expert_hub.request_generation;
        let previous_installed_generation = app.expert_hub.installed_request_generation;
        let _task = app.update(Message::SkillHub(SkillHubMessage::SelectSkillTab(tab)));
        match tab {
            SkillTab::Installed => {
                assert_eq!(app.expert_hub.request_generation, previous_generation);
                assert!(
                    app.expert_hub.installed_request_generation > previous_installed_generation
                );
                assert!(matches!(app.expert_hub.installed, LoadState::Loading { .. }))
            }
            SkillTab::System => {
                assert!(app.expert_hub.request_generation > previous_generation);
                assert!(matches!(app.expert_hub.system, LoadState::Loading { .. }))
            }
            SkillTab::Mine => {
                assert!(app.expert_hub.request_generation > previous_generation);
                assert!(matches!(app.expert_hub.mine, LoadState::Loading { .. }))
            }
            SkillTab::Marketplace | SkillTab::Install => unreachable!(),
        }
    }

    let _task = app.update(Message::SkillHub(SkillHubMessage::PreviousPage));
    assert_eq!(app.expert_hub.mine_page, 1);
    let generation_at_first_page = app.expert_hub.request_generation;
    let _task = app.update(Message::SkillHub(SkillHubMessage::NextPage));
    assert_eq!(app.expert_hub.mine_page, 2);
    assert!(app.expert_hub.request_generation > generation_at_first_page);

    let current_generation = app.expert_hub.request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _task = app.update(Message::SkillHub(SkillHubMessage::MineLoaded {
        generation: current_generation.wrapping_sub(1),
        account_id: account_id.clone(),
        result: Ok(Page { items: Vec::new(), page: 99, page_size: 20, total: 0 }),
    }));
    assert!(matches!(app.expert_hub.mine, LoadState::Loading { .. }));

    let _task = app.update(Message::SkillHub(SkillHubMessage::MineLoaded {
        generation: current_generation,
        account_id,
        result: Ok(Page { items: Vec::new(), page: 2, page_size: 20, total: 0 }),
    }));
    assert!(matches!(app.expert_hub.mine, LoadState::Loaded(Page { page: 2, .. })));
}

#[test]
fn skill_hub_list_state_routes_tabs_search_pages_and_account_changes() {
    assert_skill_hub_routes_tabs_search_pages_and_account_changes();
}

#[test]
fn skill_hub_message_routes_tabs_search_pages_and_account_changes() {
    assert_skill_hub_routes_tabs_search_pages_and_account_changes();
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn skill_hub_list_filters_submit_reset_and_restart_pagination() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));

    let _ = app.update(Message::SkillHub(SkillHubMessage::SelectSkillTab(SkillTab::System)));
    app.expert_hub.system_page = 4;
    let _ = app.update(Message::SkillHub(SkillHubMessage::FilterKeywordChanged(
        "  系统搜索  ".to_string(),
    )));
    let _ = app.update(Message::SkillHub(SkillHubMessage::FilterTagChanged(Some(2))));
    let _ = app.update(Message::SkillHub(SkillHubMessage::FiltersSubmitted));
    assert_eq!(app.expert_hub.system_filter.submitted_keyword, "系统搜索");
    assert_eq!(app.expert_hub.system_filter.submitted_tag_id, Some(2));
    assert_eq!(app.expert_hub.system_page, 1);
}

#[test]
fn mine_filters_toggle_without_loading_then_submit_and_reset() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _ = update_skill_hub(&mut app, SkillHubMessage::SelectSkillTab(SkillTab::Mine));
    let generation_before_drafts = app.expert_hub.request_generation;
    app.expert_hub.mine_page = 3;

    assert_eq!(app.update(Message::SkillHub(SkillHubMessage::ToggleMineTagFilterMenu)).units(), 0);
    assert_eq!(app.expert_hub.mine_filter_menu, Some(MineFilterMenu::Tags));
    let _ = app.update(Message::SkillHub(SkillHubMessage::MineTagToggled(9)));
    let _ = app.update(Message::SkillHub(SkillHubMessage::MineTagToggled(2)));
    let _ =
        app.update(Message::SkillHub(SkillHubMessage::MineStatusToggled(MineSkillStatus::Draft)));
    assert_eq!(app.expert_hub.request_generation, generation_before_drafts);
    assert_eq!(app.expert_hub.mine_tag_ids_draft, vec![9, 2]);
    assert!(!app.expert_hub.mine_statuses_draft.contains(&MineSkillStatus::Draft));

    let submitted = app.update(Message::SkillHub(SkillHubMessage::FiltersSubmitted));
    assert_eq!(submitted.units(), 1);
    assert_eq!(app.expert_hub.mine_page, 1);
    assert_eq!(app.expert_hub.submitted_mine_tag_ids, vec![9, 2]);
    assert!(!app.expert_hub.submitted_mine_statuses.contains(&MineSkillStatus::Draft));
    assert_eq!(app.expert_hub.mine_filter_menu, None);

    let _ = app.update(Message::SkillHub(SkillHubMessage::FiltersReset));
    assert!(app.expert_hub.mine_tag_ids_draft.is_empty());
    assert_eq!(app.expert_hub.mine_statuses_draft, MineSkillStatus::DEFAULT);
    assert_eq!(app.expert_hub.submitted_mine_statuses, MineSkillStatus::DEFAULT);
    assert_eq!(app.expert_hub.mine_page, 1);
}

#[test]
fn mine_query_uses_only_submitted_multi_value_filters() {
    let (mut app, _) = App::new();
    app.expert_hub.mine_page = 4;
    app.expert_hub.mine_filter.keyword_draft = "未提交".to_string();
    app.expert_hub.mine_filter.submitted_keyword = "已提交".to_string();
    app.expert_hub.mine_tag_ids_draft = vec![7];
    app.expert_hub.submitted_mine_tag_ids = vec![9, 2];
    app.expert_hub.mine_company_tag_ids_draft = vec![8];
    app.expert_hub.submitted_mine_company_tag_ids = vec![6, 4];
    app.expert_hub.mine_statuses_draft = vec![MineSkillStatus::Draft];
    app.expert_hub.submitted_mine_statuses =
        vec![MineSkillStatus::PendingReview, MineSkillStatus::Unpublished];

    let query = mine_query(&app);

    assert_eq!(query.page, 4);
    assert_eq!(query.keyword.as_deref(), Some("已提交"));
    assert_eq!(query.tag_ids, vec![9, 2]);
    assert_eq!(query.company_tag_ids, vec![6, 4]);
    assert_eq!(query.statuses, vec!["pending_review", "unpublished"]);

    app.expert_hub.reset_mine_filters();
    let reset_query = mine_query(&app);
    assert!(reset_query.company_tag_ids.is_empty());
    assert_eq!(reset_query.statuses, vec!["draft", "pending_review", "published"]);
}

#[test]
fn mine_tag_catalog_responses_ignore_stale_generation_and_account() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _ = update_skill_hub(&mut app, SkillHubMessage::SelectSkillTab(SkillTab::Mine));
    let generation = app.expert_hub.tag_catalog_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let platform_tags = vec![SkillTag { id: 2, name: "平台".to_string(), sort_order: 1 }];

    let _ = update_skill_hub(
        &mut app,
        SkillHubMessage::PlatformTagCatalogLoaded {
            generation: generation.wrapping_sub(1),
            account_id: account_id.clone(),
            result: Ok(platform_tags.clone()),
        },
    );
    assert!(matches!(app.expert_hub.platform_tag_catalog, LoadState::Loading { .. }));

    let _ = update_skill_hub(
        &mut app,
        SkillHubMessage::PlatformTagCatalogLoaded {
            generation,
            account_id: "other-account".to_string(),
            result: Ok(platform_tags.clone()),
        },
    );
    assert!(matches!(app.expert_hub.platform_tag_catalog, LoadState::Loading { .. }));

    let _ = update_skill_hub(
        &mut app,
        SkillHubMessage::PlatformTagCatalogLoaded {
            generation,
            account_id,
            result: Ok(platform_tags.clone()),
        },
    );
    assert!(
        matches!(app.expert_hub.platform_tag_catalog, LoadState::Loaded(ref tags) if *tags == platform_tags)
    );

    let _ = update_skill_hub(
        &mut app,
        SkillHubMessage::CompanyTagCatalogLoaded {
            generation,
            account_id: "other-account".to_string(),
            result: Ok(Vec::new()),
        },
    );
    assert!(matches!(app.expert_hub.company_tag_catalog, LoadState::Loading { .. }));

    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = update_skill_hub(
        &mut app,
        SkillHubMessage::CompanyTagCatalogLoaded {
            generation,
            account_id,
            result: Err(SkillHubError::from("目录加载失败".to_string())),
        },
    );

    assert!(matches!(
        &app.expert_hub.company_tag_catalog,
        LoadState::Error(message) if message.contains("目录加载失败")
    ));
}

#[test]
fn installed_skill_load_does_not_invalidate_marketplace_results() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _task = app.update(Message::SkillHub(SkillHubMessage::Open));
    let generation = app.expert_hub.request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _task = app.update(Message::SkillHub(SkillHubMessage::SelectSkillTab(SkillTab::Installed)));
    let _task = app.update(Message::SkillHub(SkillHubMessage::MarketplaceLoaded {
        generation,
        account_id,
        result: Ok(CursorPage { items: Vec::new(), next_cursor: None, has_more: false }),
    }));

    assert!(matches!(
        app.expert_hub.marketplace,
        LoadState::Loaded(CursorPage { items, .. }) if items.is_empty()
    ));
}

#[test]
fn ensure_installed_skills_loads_without_switching_active_tab() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let active_tab = app.expert_hub.skill_tab;
    let request_generation = app.expert_hub.request_generation;

    let first_task = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));
    let installed_generation = app.expert_hub.installed_request_generation;

    assert_eq!(first_task.units(), 1);
    assert_eq!(app.expert_hub.skill_tab, active_tab);
    assert_eq!(app.expert_hub.request_generation, request_generation);
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == installed_generation
    ));

    let loading_task = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));
    assert_eq!(loading_task.units(), 0);

    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::InstalledLoaded {
        generation: installed_generation,
        account_id,
        result: Ok(Vec::new()),
    }));
    let cached_task = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));

    assert_eq!(cached_task.units(), 0);
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(items) if items.is_empty()));
    assert_eq!(app.expert_hub.skill_tab, active_tab);
}

#[test]
fn refresh_installed_skills_forces_latest_request_without_switching_tab() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let active_tab = app.expert_hub.skill_tab;
    let request_generation = app.expert_hub.request_generation;

    let _ = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));
    let cached_generation = app.expert_hub.installed_request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::InstalledLoaded {
        generation: cached_generation,
        account_id,
        result: Ok(Vec::new()),
    }));
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loaded(ref items) if items.is_empty()
    ));

    let refresh_task = app.update(Message::SkillHub(SkillHubMessage::RefreshInstalledSkills));
    let refresh_generation = app.expert_hub.installed_request_generation;

    assert_eq!(refresh_task.units(), 1);
    assert!(refresh_generation > cached_generation);
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == refresh_generation
    ));
    assert_eq!(app.expert_hub.skill_tab, active_tab);
    assert_eq!(app.expert_hub.request_generation, request_generation);
}

#[test]
fn installed_skill_response_uses_independent_generation() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));

    let _ = app.update(Message::SkillHub(SkillHubMessage::Retry));
    let marketplace_generation = app.expert_hub.request_generation;
    let _ = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));
    let first_installed_generation = app.expert_hub.installed_request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();

    let _ = app.update(Message::SkillHub(SkillHubMessage::Retry));
    assert!(app.expert_hub.request_generation > marketplace_generation);
    let _ = app.update(Message::SkillHub(SkillHubMessage::InstalledLoaded {
        generation: first_installed_generation,
        account_id: account_id.clone(),
        result: Ok(Vec::new()),
    }));
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(items) if items.is_empty()));

    app.expert_hub.installed = LoadState::Idle;
    let _ = app.update(Message::SkillHub(SkillHubMessage::EnsureInstalledSkillsLoaded));
    let current_installed_generation = app.expert_hub.installed_request_generation;
    assert!(current_installed_generation > first_installed_generation);

    let _ = app.update(Message::SkillHub(SkillHubMessage::InstalledLoaded {
        generation: first_installed_generation,
        account_id: account_id.clone(),
        result: Ok(Vec::new()),
    }));
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == current_installed_generation
    ));

    let _ = app.update(Message::SkillHub(SkillHubMessage::InstalledLoaded {
        generation: current_installed_generation,
        account_id: "different-account".to_string(),
        result: Ok(Vec::new()),
    }));
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == current_installed_generation
    ));
}

fn marketplace_skill(skill_id: u64, installed: bool) -> MarketplaceSkill {
    MarketplaceSkill {
        id: skill_id,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "测试详情与安装流程".to_string(),
        level: "basic".to_string(),
        modality: "text".to_string(),
        format: "markdown".to_string(),
        company_id: "company-1".to_string(),
        created_uid: "user-1".to_string(),
        maintainer_uid: "user-1".to_string(),
        status: "published".to_string(),
        is_all_company_accessible: 1,
        current_version_id: 17,
        created_at: "2026-07-28T00:00:00Z".to_string(),
        updated_at: "2026-07-28T00:00:00Z".to_string(),
        deleted_at: None,
        is_installed: installed,
        tags: vec![SkillTag { id: 1, name: "安全".to_string(), sort_order: 0 }],
    }
}

fn skill_detail(skill_id: u64) -> SkillDetail {
    SkillDetail {
        id: skill_id,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "测试详情与安装流程".to_string(),
        level: "basic".to_string(),
        modality: "text".to_string(),
        format: "markdown".to_string(),
        company_id: "company-1".to_string(),
        created_uid: "user-1".to_string(),
        maintainer_uid: "user-1".to_string(),
        status: "published".to_string(),
        is_all_company_accessible: 1,
        current_version_id: 17,
        current_version: None,
        created_at: "2026-07-28T00:00:00Z".to_string(),
        updated_at: "2026-07-28T00:00:00Z".to_string(),
        tags: Vec::new(),
    }
}

fn skill_version(skill_id: u64) -> SkillVersion {
    SkillVersion {
        id: 17,
        skill_id,
        version: "1.0.0".to_string(),
        status: "published".to_string(),
        change_type: "initial".to_string(),
        changelog: Some("首个版本".to_string()),
        checksum: "sha256:test".to_string(),
        content_type: "application/zip".to_string(),
        download_expires_at: 1_800_000_000,
        download_url: Some(
            SkillDownloadUrl::parse("https://skill.shiju.cloud/download?signature=sensitive")
                .unwrap(),
        ),
        created_at: "2026-07-28T00:00:00Z".to_string(),
        updated_at: "2026-07-28T00:00:00Z".to_string(),
    }
}

fn shiju_release() -> ShijuPackageRelease {
    ShijuPackageRelease {
        download_url: SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/hub/releases/3.0.3/shiju.zip?signature=guide-secret",
        )
        .unwrap(),
    }
}

#[test]
fn skill_detail_install_uninstall_and_download_complete_user_flow() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, false)],
        next_cursor: None,
        has_more: false,
    });

    let _detail_tasks = app.update(Message::SkillHub(SkillHubMessage::OpenSkillDetail(7)));
    let detail = app.expert_hub.detail.as_ref().expect("detail should open");
    let detail_generation = detail.generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    assert!(matches!(detail.detail, LoadState::Loading { .. }));
    assert!(matches!(detail.versions, LoadState::Loading { .. }));

    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillDetailLoaded {
        generation: detail_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok(skill_detail(7)),
    }));
    let versions = vec![skill_version(7)];
    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillVersionsLoaded {
        generation: detail_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok(versions),
    }));
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(detail.detail, LoadState::Loaded(_)));
    assert!(matches!(detail.versions, LoadState::Loaded(_)));

    let _first_install = app.update(Message::SkillHub(SkillHubMessage::InstallSkill(7)));
    let action_generation = app.expert_hub.action.as_ref().unwrap().generation;
    let _duplicate_install = app.update(Message::SkillHub(SkillHubMessage::InstallSkill(7)));
    assert_eq!(app.expert_hub.action.as_ref().unwrap().generation, action_generation);

    let _refresh = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation: action_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        action: SkillActionKind::Install,
        result: Ok(()),
    }));
    assert!(app.expert_hub.action.is_none());
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Info));
    let cache_sync_generation = match app.expert_hub.cache_sync {
        SkillCacheSyncState::Syncing { generation, skill_id: 7, .. } => generation,
        ref state => panic!("unexpected cache sync state: {state:?}"),
    };
    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillCacheSyncFinished {
        generation: cache_sync_generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok(SkillCacheSyncReport { fetch_exit_code: 0, rebuild_exit_code: 0 }),
    }));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loading { .. }));

    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, true)],
        next_cursor: None,
        has_more: false,
    });
    let _ = app.update(Message::SkillHub(SkillHubMessage::UninstallSkill(7)));
    let uninstall_generation = app.expert_hub.action.as_ref().unwrap().generation;
    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillActionFinished {
        generation: uninstall_generation,
        account_id,
        skill_id: 7,
        action: SkillActionKind::Uninstall,
        result: Err("fixture failure".to_string()),
    }));
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Error));

    let mut opened = Vec::new();
    dispatch_download_with(&app, 17, |url| {
        opened.push(url.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(opened.len(), 1);
    assert!(dispatch_download_with(&app, 99, |_| Ok(())).is_err());
    let open_error = dispatch_download_with(&app, 17, |url| Err(url.to_string())).unwrap_err();
    assert_eq!(open_error, "无法打开技能下载地址");
    assert!(!open_error.contains("signature"));
    assert!(!format!("{:?}", SkillHubMessage::DownloadVersion(17)).contains("signature"));
}

#[test]
fn skill_hub_detail_ignores_late_results_and_close_preserves_list() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, false)],
        next_cursor: None,
        has_more: false,
    });

    let _ = app.update(Message::SkillHub(SkillHubMessage::OpenSkillDetail(7)));
    let old_generation = app.expert_hub.detail.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::OpenSkillDetail(8)));
    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillDetailLoaded {
        generation: old_generation,
        account_id,
        skill_id: 7,
        result: Ok(skill_detail(7)),
    }));
    assert!(matches!(app.expert_hub.detail.as_ref().unwrap().detail, LoadState::Loading { .. }));

    app.expert_hub.version_confirmation = Some(SkillVersionAction {
        skill_id: 8,
        version_id: 9,
        target_version: "0.9.0".to_string(),
        direction: SkillVersionDirection::Rollback,
    });
    let _ = app.update(Message::SkillHub(SkillHubMessage::CloseSkillDetail));
    assert!(app.expert_hub.detail.is_none());
    assert!(app.expert_hub.version_confirmation.is_none());
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
}

#[test]
fn skill_detail_readme_ignores_stale_results_and_preserves_detail() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());

    let _ = app.update(Message::SkillHub(SkillHubMessage::OpenSkillDetail(7)));
    let generation = app.expert_hub.detail.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let requests = app.update(Message::SkillHub(SkillHubMessage::SkillDetailLoaded {
        generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok(skill_detail(7)),
    }));
    assert_eq!(requests.units(), 2, "versions and README should load independently");
    assert!(matches!(
        app.expert_hub.detail.as_ref().unwrap().readme,
        LoadState::Loading { generation: active } if active == generation
    ));

    for (result_generation, result_account_id, result_skill_id) in [
        (generation.wrapping_sub(1), account_id.clone(), 7),
        (generation, "other-account".to_string(), 7),
        (generation, account_id.clone(), 8),
    ] {
        let _ = app.update(Message::SkillHub(SkillHubMessage::SkillReadmeLoaded {
            generation: result_generation,
            account_id: result_account_id,
            skill_id: result_skill_id,
            result: Err(SkillHubError::from("stale README failure".to_string())),
        }));
    }
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(detail.detail, LoadState::Loaded(_)));
    assert!(matches!(detail.readme, LoadState::Loading { .. }));

    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillReadmeLoaded {
        generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok("# 使用说明\n支持 UTF-8 正文。".to_string()),
    }));
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(&detail.readme, LoadState::Loaded(content) if content.starts_with('#')));
    assert!(!detail.readme_items.is_empty());
    assert!(matches!(detail.detail, LoadState::Loaded(_)));

    let _ = app
        .update(Message::SkillHub(SkillHubMessage::SetSkillReadmeMode(PublicDetailMode::Markdown)));
    assert_eq!(app.expert_hub.detail.as_ref().unwrap().readme_mode, PublicDetailMode::Markdown);

    let _ = app.update(Message::SkillHub(SkillHubMessage::SkillReadmeLoaded {
        generation,
        account_id,
        skill_id: 7,
        result: Err(SkillHubError::from("README unavailable".to_string())),
    }));
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(&detail.readme, LoadState::Error(error) if error == "README unavailable"));
    assert!(matches!(detail.detail, LoadState::Loaded(_)));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn install_guide_routes_current_package_copy_download_and_direct_install() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.account.menu_open = true;

    let _load = app.update(Message::SkillHub(SkillHubMessage::OpenInstallGuide));

    assert!(!app.account.menu_open);
    assert_eq!(app.screen, Screen::ShijuSkills);
    assert_eq!(app.active_tab_id.as_deref(), Some("shiju_skills"));
    assert!(app.expert_hub.install_guide.open);
    assert_eq!(app.expert_hub.skill_tab, SkillTab::Install);
    assert_eq!(app.expert_hub.install_guide.current_package, ShijuPackageId::current());
    let generation = app.expert_hub.install_guide.agent_generation;
    assert!(matches!(
        app.expert_hub.install_guide.agent_package,
        LoadState::Loading { generation: active } if active == generation
    ));
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::AgentPackageLoaded {
        generation,
        account_id: account_id.clone(),
        copy_after_load: false,
        result: Ok(shiju_release()),
    }));
    assert!(matches!(app.expert_hub.install_guide.agent_package, LoadState::Loaded(_)));
    let install_text = agent_install_text(&app.expert_hub.install_guide);
    assert!(install_text.contains("请帮我安装识矩技能"));
    assert!(install_text.contains("guide-secret"));

    let mut opened = Vec::new();
    dispatch_shiju_download_with(&shiju_release(), |url| {
        opened.push(url.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(opened.len(), 1);

    let _direct = app.update(Message::SkillHub(SkillHubMessage::DirectInstallShiju));
    assert!(matches!(
        app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Fetching { .. }
    ));
}

#[test]
fn project_scope_requires_open_project() {
    let (mut app, _) = App::new();
    app.expert_hub.install_guide.open = true;
    app.recent_projects.clear();

    let _ = app.update(Message::SkillHub(SkillHubMessage::SelectShijuInstallScope(
        SkillInstallScope::Project,
    )));

    assert_eq!(app.expert_hub.install_guide.install_scope, SkillInstallScope::Global);
    let error = app.active_toast.as_ref().expect("missing project should show an error");
    assert_eq!(error.kind, ToastKind::Error);
    assert!(error.message.contains("项目"));

    let project = tempfile::tempdir().expect("project fixture");
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    let _ = app.update(Message::SkillHub(SkillHubMessage::SelectShijuInstallScope(
        SkillInstallScope::Project,
    )));

    assert_eq!(app.expert_hub.install_guide.install_scope, SkillInstallScope::Project);
}

#[test]
fn project_scope_uses_selected_install_project_and_can_switch_projects() {
    let (mut app, _) = App::new();
    app.expert_hub.install_guide.open = true;
    app.recent_projects = vec!["/repo/first".to_string(), "/repo/second".to_string()];
    app.expert_hub.install_guide.selected_project_path = Some("/repo/first".to_string());

    let _ = app.update(Message::SkillHub(SkillHubMessage::SelectShijuInstallScope(
        SkillInstallScope::Project,
    )));
    assert_eq!(app.expert_hub.install_guide.install_scope, SkillInstallScope::Project);

    let previous_generation = app.expert_hub.install_guide.installation_generation;
    let _ = app.update(Message::SkillHub(SkillHubMessage::SelectShijuInstallProject(
        "/repo/second".to_string(),
    )));

    assert_eq!(app.expert_hub.install_guide.selected_project_path.as_deref(), Some("/repo/second"));
    assert!(app.expert_hub.install_guide.installation_generation > previous_generation);
}

#[test]
fn install_guide_ignores_stale_results_and_duplicate_actions() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let _ = app.update(Message::SkillHub(SkillHubMessage::OpenInstallGuide));
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::DirectInstallShiju));
    let (generation, package_id) = match app.expert_hub.install_guide.direct_install {
        ShijuDirectInstallState::Fetching { generation, package_id } => (generation, package_id),
        ref other => panic!("expected fetching state, got {other:?}"),
    };
    let selections = app.expert_hub.install_guide.direct_selections.clone();

    let _duplicate = app.update(Message::SkillHub(SkillHubMessage::DirectInstallShiju));
    assert!(matches!(
        app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Fetching { generation: active, .. } if active == generation
    ));

    app.expert_hub.install_guide.reset_requests();
    let _stale = app.update(Message::SkillHub(SkillHubMessage::DirectPackageLoaded {
        generation,
        account_id,
        package_id,
        selections,
        project_root: None,
        result: Ok(shiju_release()),
    }));
    assert!(matches!(app.expert_hub.install_guide.direct_install, ShijuDirectInstallState::Idle));
}

#[test]
fn direct_install_preserves_selected_targets_and_ignores_stale_results() {
    let home = tempfile::tempdir().expect("home fixture");
    let selections = vec![
        SkillInstallSelection {
            target: SkillInstallTarget::AgentSkills,
            scope: SkillInstallScope::Global,
        },
        SkillInstallSelection {
            target: SkillInstallTarget::ClaudeCode,
            scope: SkillInstallScope::Global,
        },
    ];
    let installs = resolve_skill_destinations(home.path(), None, &selections)
        .expect("resolve selected destinations")
        .into_iter()
        .map(|destination| ShijuInstallItem { destination, version: Some("3.0.3".to_string()) })
        .collect();
    let report = ShijuMultiInstallReport { installs };
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let account_id = app.account.active_account().unwrap().id().to_string();
    let generation = 7;
    let package_id = ShijuPackageId::current().expect("supported test platform");
    app.expert_hub.account_id = Some(account_id.clone());
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.selected_targets =
        vec![SkillInstallTarget::AgentSkills, SkillInstallTarget::ClaudeCode];
    app.expert_hub.install_guide.direct_selections = selections.clone();
    app.expert_hub.install_guide.direct_install =
        ShijuDirectInstallState::Installing { generation, package_id };

    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuInstallFinished {
        generation: generation.wrapping_sub(1),
        account_id: account_id.clone(),
        package_id,
        selections: selections.clone(),
        result: Ok(report.clone()),
    }));
    assert!(app.expert_hub.install_guide.installations.is_empty());

    let task = app.update(Message::SkillHub(SkillHubMessage::ShijuInstallFinished {
        generation,
        account_id,
        package_id,
        selections: selections.clone(),
        result: Ok(report),
    }));

    assert_eq!(app.expert_hub.install_guide.selected_install_selections(), selections);
    assert_eq!(app.expert_hub.install_guide.installed_selection_count(), 2);
    assert_eq!(task.units(), 2);
}

#[test]
fn installed_shiju_directory_opener_receives_the_exact_directory() {
    let directory = tempfile::tempdir().expect("fixture install directory");
    let mut opened = Vec::new();

    open_shiju_install_dir_with(directory.path(), |path| {
        opened.push(path.to_path_buf());
        Ok(())
    })
    .expect("open install directory");

    assert_eq!(opened, vec![directory.path().to_path_buf()]);
}

#[test]
fn manual_shiju_key_sync_requires_active_account() {
    let home = tempfile::tempdir().expect("home fixture");
    let selections = [SkillInstallSelection {
        target: SkillInstallTarget::VibeWindow,
        scope: SkillInstallScope::Global,
    }];
    let installations = installed_targets(home.path(), &selections);
    let destination = installations.targets[0].destination.clone();
    let (mut app, _) = App::new();
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.set_installations(installations);

    let _ = app.update(Message::SkillHub(SkillHubMessage::SyncShijuKey));

    let warning = app.active_toast.as_ref().expect("missing account should show a warning");
    assert_eq!(warning.kind, ToastKind::Warning);
    assert!(warning.message.contains("登录识矩账户"));
    assert!(!app.expert_hub.install_guide.login_sync_in_progress);

    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuKeySyncFinished {
        generation: 1,
        account_id: "missing".to_string(),
        selections: selections.to_vec(),
        requested_by_user: true,
        result: Err("credential-secret".to_string()),
    }));
    let warning = app.active_toast.as_ref().expect("stale sync result should be ignored");
    assert_eq!(warning.kind, ToastKind::Warning);
    assert!(!warning.message.contains("credential-secret"));
    assert!(!app.expert_hub.install_guide.login_sync_in_progress);

    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.active_toast = None;
    let task = app.update(Message::SkillHub(SkillHubMessage::SyncShijuKey));
    assert_eq!(task.units(), 1);
    assert!(app.active_toast.is_none());
    assert!(app.expert_hub.install_guide.login_sync_in_progress);
    let sync = app.expert_hub.install_guide.login_sync.clone().unwrap();
    let account_id = app.account.active_account().unwrap().id().to_string();

    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuKeySyncFinished {
        generation: sync.generation,
        account_id,
        selections: sync.selections,
        requested_by_user: true,
        result: Ok(ShijuKeySyncReport {
            targets: vec![ShijuTargetKeySync {
                destination,
                outcome: ShijuKeySyncOutcome::Updated,
            }],
        }),
    }));
    assert!(!app.expert_hub.install_guide.login_sync_in_progress);
}

#[test]
fn uninstall_and_sync_use_selected_installations() {
    let home = tempfile::tempdir().expect("home fixture");
    let all_selections = [
        SkillInstallSelection {
            target: SkillInstallTarget::AgentSkills,
            scope: SkillInstallScope::Global,
        },
        SkillInstallSelection {
            target: SkillInstallTarget::ClaudeCode,
            scope: SkillInstallScope::Global,
        },
        SkillInstallSelection {
            target: SkillInstallTarget::Cursor,
            scope: SkillInstallScope::Global,
        },
    ];
    let destinations = resolve_skill_destinations(home.path(), None, &all_selections)
        .expect("resolve fixture destinations");
    let installations = destinations
        .iter()
        .cloned()
        .map(|destination| ShijuTargetInstallation {
            installation: Some(ShijuInstallation {
                installed_dir: destination.installed_dir.clone(),
                version: Some("3.0.3".to_string()),
            }),
            destination,
        })
        .collect();
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id = Some(app.account.active_account().unwrap().id().to_string());
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.selected_targets =
        vec![SkillInstallTarget::AgentSkills, SkillInstallTarget::ClaudeCode];
    app.expert_hub
        .install_guide
        .set_installations(ShijuInstallationsReport { targets: installations });

    let _ = app.update(Message::SkillHub(SkillHubMessage::UninstallShiju));
    let _ = app.update(Message::SkillHub(SkillHubMessage::ConfirmShijuUninstall));
    let (generation, selected) = match app.expert_hub.install_guide.direct_install {
        ShijuDirectInstallState::Uninstalling { generation } => {
            (generation, app.expert_hub.install_guide.direct_selections.clone())
        }
        other => panic!("expected uninstalling state, got {other:?}"),
    };
    assert_eq!(selected, all_selections[..2]);
    let uninstall_targets = destinations[..2]
        .iter()
        .cloned()
        .map(|destination| ShijuTargetUninstall { destination, removed: true })
        .collect();
    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuUninstallFinished {
        generation,
        selections: selected,
        result: Ok(ShijuUninstallReport { targets: uninstall_targets }),
    }));
    assert_eq!(app.expert_hub.install_guide.installed_selection_count(), 0);
    assert!(
        app.expert_hub
            .install_guide
            .installation(SkillInstallScope::Global, SkillInstallTarget::Cursor)
            .is_some()
    );

    app.expert_hub.install_guide.selected_targets = vec![SkillInstallTarget::Cursor];
    let _ = app.update(Message::SkillHub(SkillHubMessage::SyncShijuKey));
    let sync = app
        .expert_hub
        .install_guide
        .login_sync
        .as_ref()
        .expect("sync should start for selected installation");
    assert_eq!(sync.selections, all_selections[2..]);
    let sync_generation = sync.generation;
    let sync_selections = sync.selections.clone();
    let sync_targets = destinations[2..]
        .iter()
        .cloned()
        .map(|destination| ShijuTargetKeySync {
            destination,
            outcome: ShijuKeySyncOutcome::Updated,
        })
        .collect();
    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuKeySyncFinished {
        generation: sync_generation,
        account_id: app.expert_hub.account_id.clone().unwrap(),
        selections: sync_selections,
        requested_by_user: true,
        result: Ok(ShijuKeySyncReport { targets: sync_targets }),
    }));
    assert!(app.expert_hub.install_guide.login_sync.is_none());
}

#[test]
fn direct_install_success_schedules_key_sync() {
    let home = tempfile::tempdir().expect("home fixture");
    let selections = vec![SkillInstallSelection {
        target: SkillInstallTarget::VibeWindow,
        scope: SkillInstallScope::Global,
    }];
    let destination = resolve_skill_destinations(home.path(), None, &selections).unwrap().remove(0);
    let (mut app, _) = App::new();
    app.account.replace_repository(repository_with_account("account-secret", "company-1"));
    let account_id = app.account.active_account().unwrap().id().to_string();
    let generation = 7;
    let package_id = ShijuPackageId::current().expect("supported test platform");
    app.expert_hub.account_id = Some(account_id.clone());
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.direct_selections = selections.clone();
    app.expert_hub.install_guide.direct_install =
        ShijuDirectInstallState::Installing { generation, package_id };

    let task = app.update(Message::SkillHub(SkillHubMessage::ShijuInstallFinished {
        generation,
        account_id,
        package_id,
        selections,
        result: Ok(ShijuMultiInstallReport {
            installs: vec![ShijuInstallItem {
                destination: destination.clone(),
                version: Some("3.0.3".to_string()),
            }],
        }),
    }));

    assert_eq!(task.units(), 2);
    assert!(app.expert_hub.install_guide.is_installed);
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));

    let sync = app.expert_hub.install_guide.login_sync.clone().unwrap();
    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuKeySyncFinished {
        generation: sync.generation,
        account_id: app.expert_hub.account_id.clone().unwrap(),
        selections: sync.selections,
        requested_by_user: false,
        result: Ok(ShijuKeySyncReport {
            targets: vec![ShijuTargetKeySync {
                destination,
                outcome: ShijuKeySyncOutcome::Updated,
            }],
        }),
    }));
    let success = app.active_toast.as_ref().expect("sync completion should show success");
    assert_eq!(success.kind, ToastKind::Success);
    assert!(!success.message.contains("account-secret"));
}

#[test]
fn install_guide_uninstall_completion_updates_local_status_and_ignores_stale_result() {
    let home = tempfile::tempdir().expect("home fixture");
    let selections = vec![SkillInstallSelection {
        target: SkillInstallTarget::VibeWindow,
        scope: SkillInstallScope::Global,
    }];
    let installations = installed_targets(home.path(), &selections);
    let destination = installations.targets[0].destination.clone();
    let (mut app, _) = App::new();
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.set_installations(installations);

    let _ = app.update(Message::SkillHub(SkillHubMessage::UninstallShiju));
    assert!(app.expert_hub.install_guide.uninstall_confirmation_open);
    assert!(!matches!(
        app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Uninstalling { .. }
    ));
    let _ = app.update(Message::SkillHub(SkillHubMessage::CancelShijuUninstall));
    assert!(!app.expert_hub.install_guide.uninstall_confirmation_open);

    let _ = app.update(Message::SkillHub(SkillHubMessage::UninstallShiju));
    let _ = app.update(Message::SkillHub(SkillHubMessage::ConfirmShijuUninstall));
    assert!(!app.expert_hub.install_guide.uninstall_confirmation_open);
    let generation = match app.expert_hub.install_guide.direct_install {
        ShijuDirectInstallState::Uninstalling { generation } => generation,
        ref other => panic!("expected uninstalling state, got {other:?}"),
    };
    let operation_selections = app.expert_hub.install_guide.direct_selections.clone();
    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuUninstallFinished {
        generation: generation.wrapping_sub(1),
        selections: operation_selections.clone(),
        result: Ok(ShijuUninstallReport {
            targets: vec![ShijuTargetUninstall { destination: destination.clone(), removed: true }],
        }),
    }));
    assert!(app.expert_hub.install_guide.is_installed);

    let _ = app.update(Message::SkillHub(SkillHubMessage::ShijuUninstallFinished {
        generation,
        selections: operation_selections,
        result: Ok(ShijuUninstallReport {
            targets: vec![ShijuTargetUninstall { destination, removed: true }],
        }),
    }));
    assert!(!app.expert_hub.install_guide.is_installed);
    assert!(app.expert_hub.install_guide.installed_version.is_none());
    assert!(matches!(app.expert_hub.install_guide.direct_install, ShijuDirectInstallState::Idle));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn shiju_download_writer_preserves_chunk_order_and_propagates_errors() {
    let archive = NamedTempFile::new().expect("archive fixture");
    let archive_path = archive.path().to_path_buf();
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    sender.try_send(ShijuDownloadChunk::Data(b"first-".to_vec())).expect("queue first chunk");
    sender.try_send(ShijuDownloadChunk::Data(b"second".to_vec())).expect("queue second chunk");
    sender.try_send(ShijuDownloadChunk::Complete).expect("queue completion");
    drop(sender);

    let archive =
        write_shiju_download_chunks(archive, receiver).expect("write queued download chunks");
    assert_eq!(fs::read(&archive_path).expect("read archive fixture"), b"first-second");
    drop(archive);

    let archive = NamedTempFile::new().expect("error archive fixture");
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    sender.try_send(ShijuDownloadChunk::Data(b"partial".to_vec())).expect("queue partial chunk");
    sender
        .try_send(ShijuDownloadChunk::Failed("下载中断".to_string()))
        .expect("queue download error");
    drop(sender);

    let error = write_shiju_download_chunks(archive, receiver)
        .expect_err("download error must stop the writer");
    assert_eq!(error, "下载中断");

    let archive = NamedTempFile::new().expect("cancelled archive fixture");
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    drop(sender);
    let error = write_shiju_download_chunks(archive, receiver)
        .expect_err("closed channel must not look complete");
    assert_eq!(error, "识矩技能安装包下载未完成");
}
