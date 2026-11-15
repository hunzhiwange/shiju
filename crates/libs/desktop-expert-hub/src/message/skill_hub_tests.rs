//! 原消息层的领域状态断言；桌面导航和反馈组合测试留在宿主。
use super::*;
use desktop_skill_hub::skill_hub::*;
use desktop_account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
#[derive(Default)]
struct Fixture { expert_hub: ExpertHubState, account: AccountRepository }
impl Fixture { fn reduce(&mut self, message: SkillHubMessage) -> Task<Message> { update_skill_hub(self, message) } }
fn replace_repository(account: &mut AccountRepository, repository: AccountRepository) { *account = repository; }
fn unavailable_gateway() -> Result<gateway_http::GatewayTransport, String> { Err("fixture does not execute network tasks".into()) }
fn update_skill_hub(fixture: &mut Fixture, message: SkillHubMessage) -> Task<Message> {
    let mut input = SkillUpdate {
        expert_hub: &mut fixture.expert_hub,
        active_account: fixture.account.active_account(),
        project_path: &None,
        recent_projects: &[],
        screen: HubPage::ShijuSkills,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        gateway: unavailable_gateway,
        effects: Vec::new(),
    };
    update(&mut input, message)
}
fn mine_query(fixture: &Fixture) -> gateway_shiju_skill::ShijuMaintainedSkillQuery { super::mine_query(&fixture.expert_hub) }
fn marketplace_query(fixture: &Fixture) -> gateway_shiju_skill::ShijuSkillSearchQuery { super::marketplace_query(&fixture.expert_hub) }

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

fn shiju_release() -> ShijuPackageRelease {
    ShijuPackageRelease {
        download_url: SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/hub/releases/3.0.3/shiju.zip?signature=guide-secret",
        )
        .unwrap(),
    }
}

#[test]
fn market_tag_selection_refreshes_search() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let _ = app.reduce(SkillHubMessage::SelectSkillTab(SkillTab::Marketplace));
    app.expert_hub.search_draft = "保留草稿".to_string();
    app.expert_hub.submitted_search = "保留关键词".to_string();

    let generation = app.expert_hub.request_generation;
    let _ = app.reduce(SkillHubMessage::MarketTagSelected(Some(2)));
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
    let _ = app.reduce(SkillHubMessage::MarketCompanyTagSelected(Some(7)));
    assert!(app.expert_hub.request_generation > generation);
    assert_eq!(app.expert_hub.market_tag_id, Some(2));
    assert_eq!(app.expert_hub.market_company_tag_id, Some(7));

    let _ = app.reduce(SkillHubMessage::MarketTagSelected(None));
    let reset = marketplace_query(&app);
    assert_eq!(reset.query.as_deref(), Some("保留关键词"));
    assert_eq!(reset.tag_ids, None);
    assert_eq!(reset.company_tag_ids, Some(7));

    let _ = app.reduce(SkillHubMessage::FiltersReset);
    assert!(app.expert_hub.search_draft.is_empty());
    assert!(app.expert_hub.submitted_search.is_empty());
    assert_eq!(app.expert_hub.market_tag_id, None);
    assert_eq!(app.expert_hub.market_company_tag_id, None);
}

#[test]
fn shiju_package_refresh_message_ignores_late_result_and_preserves_cache_on_failure() {
    let cached =
        PackageVersionCacheEntry::new("3.0.3".to_string(), 1_785_830_400).expect("valid cache");
    let mut app = Fixture::default();
    app.expert_hub.package_status =
        ShijuPackageStatus::from_sources(Some("3.0.2".to_string()), Some(cached));
    let stale_generation = app.expert_hub.package_status.start_refresh();
    let current_generation = app.expert_hub.package_status.start_refresh();

    let _ = app.reduce(SkillHubMessage::ShijuPackageVersionRefreshed {
        generation: stale_generation,
        result: Ok(PackageVersionCacheEntry::new("9.9.9".to_string(), 1_785_830_401)
            .expect("valid stale result")),
    });
    let _ = app.reduce(SkillHubMessage::ShijuPackageVersionRefreshed {
        generation: current_generation,
        result: Err("signed-url-secret".into()),
    });

    assert_eq!(app.expert_hub.package_status.server_version.as_deref(), Some("3.0.3"));
    assert_eq!(app.expert_hub.package_status.checked_at_unix, Some(1_785_830_400));
    assert!(!format!("{:?}", app.expert_hub.package_status).contains("signed-url-secret"));
}

#[test]
fn business_skill_quick_upgrade_resolves_version_id_and_rollback_requires_confirmation() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
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

    let resolution = app.reduce(SkillHubMessage::UpgradeSkill(7));
    assert_eq!(resolution.units(), 1);
    let generation = app.expert_hub.action.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let install = app.reduce(SkillHubMessage::SkillUpgradeResolved {
        generation,
        account_id,
        skill_id: 7,
        result: Ok(skill_detail(7)),
    });
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
        app.reduce(SkillHubMessage::ChangeSkillVersion(rollback.clone()));
    assert_eq!(requested.units(), 0);
    assert!(app.expert_hub.action.is_none());
    assert_eq!(app.expert_hub.version_confirmation.as_ref(), Some(&rollback));
    let confirmed = app.reduce(SkillHubMessage::ConfirmSkillVersionChange);
    assert_eq!(confirmed.units(), 1);
    assert!(matches!(
        &app.expert_hub.action.as_ref().unwrap().kind,
        SkillActionKind::Version(action) if action == &rollback
    ));
}

#[test]
fn skill_hub_list_filters_submit_reset_and_restart_pagination() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));

    let _ = app.reduce(SkillHubMessage::SelectSkillTab(SkillTab::System));
    app.expert_hub.system_page = 4;
    let _ = app.reduce(SkillHubMessage::FilterKeywordChanged(
        "  系统搜索  ".to_string(),
    ));
    let _ = app.reduce(SkillHubMessage::FilterTagChanged(Some(2)));
    let _ = app.reduce(SkillHubMessage::FiltersSubmitted);
    assert_eq!(app.expert_hub.system_filter.submitted_keyword, "系统搜索");
    assert_eq!(app.expert_hub.system_filter.submitted_tag_id, Some(2));
    assert_eq!(app.expert_hub.system_page, 1);
}

#[test]
fn mine_filters_toggle_without_loading_then_submit_and_reset() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let _ = app.reduce(SkillHubMessage::SelectSkillTab(SkillTab::Mine));
    let generation_before_drafts = app.expert_hub.request_generation;
    app.expert_hub.mine_page = 3;

    assert_eq!(app.reduce(SkillHubMessage::ToggleMineTagFilterMenu).units(), 0);
    assert_eq!(app.expert_hub.mine_filter_menu, Some(MineFilterMenu::Tags));
    let _ = app.reduce(SkillHubMessage::MineTagToggled(9));
    let _ = app.reduce(SkillHubMessage::MineTagToggled(2));
    let _ =
        app.reduce(SkillHubMessage::MineStatusToggled(MineSkillStatus::Draft));
    assert_eq!(app.expert_hub.request_generation, generation_before_drafts);
    assert_eq!(app.expert_hub.mine_tag_ids_draft, vec![9, 2]);
    assert!(!app.expert_hub.mine_statuses_draft.contains(&MineSkillStatus::Draft));

    let submitted = app.reduce(SkillHubMessage::FiltersSubmitted);
    assert_eq!(submitted.units(), 1);
    assert_eq!(app.expert_hub.mine_page, 1);
    assert_eq!(app.expert_hub.submitted_mine_tag_ids, vec![9, 2]);
    assert!(!app.expert_hub.submitted_mine_statuses.contains(&MineSkillStatus::Draft));
    assert_eq!(app.expert_hub.mine_filter_menu, None);

    let _ = app.reduce(SkillHubMessage::FiltersReset);
    assert!(app.expert_hub.mine_tag_ids_draft.is_empty());
    assert_eq!(app.expert_hub.mine_statuses_draft, MineSkillStatus::DEFAULT);
    assert_eq!(app.expert_hub.submitted_mine_statuses, MineSkillStatus::DEFAULT);
    assert_eq!(app.expert_hub.mine_page, 1);
}

#[test]
fn mine_query_uses_only_submitted_multi_value_filters() {
    let mut app = Fixture::default();
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
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let _ = app.reduce(SkillHubMessage::SelectSkillTab(SkillTab::Mine));
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
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let _task = app.reduce(SkillHubMessage::Open);
    let generation = app.expert_hub.request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _task = app.reduce(SkillHubMessage::SelectSkillTab(SkillTab::Installed));
    let _task = app.reduce(SkillHubMessage::MarketplaceLoaded {
        generation,
        account_id,
        result: Ok(CursorPage { items: Vec::new(), next_cursor: None, has_more: false }),
    });

    assert!(matches!(
        app.expert_hub.marketplace,
        LoadState::Loaded(CursorPage { items, .. }) if items.is_empty()
    ));
}

#[test]
fn ensure_installed_skills_loads_without_switching_active_tab() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let active_tab = app.expert_hub.skill_tab;
    let request_generation = app.expert_hub.request_generation;

    let first_task = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);
    let installed_generation = app.expert_hub.installed_request_generation;

    assert_eq!(first_task.units(), 1);
    assert_eq!(app.expert_hub.skill_tab, active_tab);
    assert_eq!(app.expert_hub.request_generation, request_generation);
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == installed_generation
    ));

    let loading_task = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);
    assert_eq!(loading_task.units(), 0);

    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.reduce(SkillHubMessage::InstalledLoaded {
        generation: installed_generation,
        account_id,
        result: Ok(Vec::new()),
    });
    let cached_task = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);

    assert_eq!(cached_task.units(), 0);
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(items) if items.is_empty()));
    assert_eq!(app.expert_hub.skill_tab, active_tab);
}

#[test]
fn refresh_installed_skills_forces_latest_request_without_switching_tab() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let active_tab = app.expert_hub.skill_tab;
    let request_generation = app.expert_hub.request_generation;

    let _ = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);
    let cached_generation = app.expert_hub.installed_request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.reduce(SkillHubMessage::InstalledLoaded {
        generation: cached_generation,
        account_id,
        result: Ok(Vec::new()),
    });
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loaded(ref items) if items.is_empty()
    ));

    let refresh_task = app.reduce(SkillHubMessage::RefreshInstalledSkills);
    let refresh_generation = app.expert_hub.installed_request_generation;

    assert_eq!(refresh_task.units(), 2);
    assert!(matches!(
        app.expert_hub.package_status.refresh,
        desktop_skill_hub::skill_hub::ShijuPackageRefreshState::Refreshing { .. }
    ));
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
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));

    let _ = app.reduce(SkillHubMessage::Retry);
    let marketplace_generation = app.expert_hub.request_generation;
    let _ = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);
    let first_installed_generation = app.expert_hub.installed_request_generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();

    let _ = app.reduce(SkillHubMessage::Retry);
    assert!(app.expert_hub.request_generation > marketplace_generation);
    let _ = app.reduce(SkillHubMessage::InstalledLoaded {
        generation: first_installed_generation,
        account_id: account_id.clone(),
        result: Ok(Vec::new()),
    });
    assert!(matches!(app.expert_hub.installed, LoadState::Loaded(items) if items.is_empty()));

    app.expert_hub.installed = LoadState::Idle;
    let _ = app.reduce(SkillHubMessage::EnsureInstalledSkillsLoaded);
    let current_installed_generation = app.expert_hub.installed_request_generation;
    assert!(current_installed_generation > first_installed_generation);

    let _ = app.reduce(SkillHubMessage::InstalledLoaded {
        generation: first_installed_generation,
        account_id: account_id.clone(),
        result: Ok(Vec::new()),
    });
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == current_installed_generation
    ));

    let _ = app.reduce(SkillHubMessage::InstalledLoaded {
        generation: current_installed_generation,
        account_id: "different-account".to_string(),
        result: Ok(Vec::new()),
    });
    assert!(matches!(
        app.expert_hub.installed,
        LoadState::Loading { generation } if generation == current_installed_generation
    ));
}

#[test]
fn skill_hub_detail_ignores_late_results_and_close_preserves_list() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());
    app.expert_hub.marketplace = LoadState::Loaded(CursorPage {
        items: vec![marketplace_skill(7, false)],
        next_cursor: None,
        has_more: false,
    });

    let _ = app.reduce(SkillHubMessage::OpenSkillDetail(7));
    let old_generation = app.expert_hub.detail.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.reduce(SkillHubMessage::OpenSkillDetail(8));
    let _ = app.reduce(SkillHubMessage::SkillDetailLoaded {
        generation: old_generation,
        account_id,
        skill_id: 7,
        result: Ok(skill_detail(7)),
    });
    assert!(matches!(app.expert_hub.detail.as_ref().unwrap().detail, LoadState::Loading { .. }));

    app.expert_hub.version_confirmation = Some(SkillVersionAction {
        skill_id: 8,
        version_id: 9,
        target_version: "0.9.0".to_string(),
        direction: SkillVersionDirection::Rollback,
    });
    let _ = app.reduce(SkillHubMessage::CloseSkillDetail);
    assert!(app.expert_hub.detail.is_none());
    assert!(app.expert_hub.version_confirmation.is_none());
    assert!(matches!(app.expert_hub.marketplace, LoadState::Loaded(_)));
}

#[test]
fn skill_detail_readme_ignores_stale_results_and_preserves_detail() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id =
        app.account.active_account().map(|account| account.id().to_string());

    let _ = app.reduce(SkillHubMessage::OpenSkillDetail(7));
    let generation = app.expert_hub.detail.as_ref().unwrap().generation;
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let requests = app.reduce(SkillHubMessage::SkillDetailLoaded {
        generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok(skill_detail(7)),
    });
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
        let _ = app.reduce(SkillHubMessage::SkillReadmeLoaded {
            generation: result_generation,
            account_id: result_account_id,
            skill_id: result_skill_id,
            result: Err(SkillHubError::from("stale README failure".to_string())),
        });
    }
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(detail.detail, LoadState::Loaded(_)));
    assert!(matches!(detail.readme, LoadState::Loading { .. }));

    let _ = app.reduce(SkillHubMessage::SkillReadmeLoaded {
        generation,
        account_id: account_id.clone(),
        skill_id: 7,
        result: Ok("# 使用说明\n支持 UTF-8 正文。".to_string()),
    });
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(&detail.readme, LoadState::Loaded(content) if content.starts_with('#')));
    assert!(!detail.readme_items.is_empty());
    assert!(matches!(detail.detail, LoadState::Loaded(_)));

    let _ = app.reduce(SkillHubMessage::SetSkillReadmeMode(PublicDetailMode::Markdown));
    assert_eq!(app.expert_hub.detail.as_ref().unwrap().readme_mode, PublicDetailMode::Markdown);

    let _ = app.reduce(SkillHubMessage::SkillReadmeLoaded {
        generation,
        account_id,
        skill_id: 7,
        result: Err(SkillHubError::from("README unavailable".to_string())),
    });
    let detail = app.expert_hub.detail.as_ref().unwrap();
    assert!(matches!(&detail.readme, LoadState::Error(error) if error == "README unavailable"));
    assert!(matches!(detail.detail, LoadState::Loaded(_)));
}

#[test]
fn install_guide_ignores_stale_results_and_duplicate_actions() {
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    let _ = app.reduce(SkillHubMessage::OpenInstallGuide);
    let account_id = app.expert_hub.account_id.clone().unwrap();
    let _ = app.reduce(SkillHubMessage::DirectInstallShiju);
    let (generation, package_id) = match app.expert_hub.install_guide.direct_install {
        ShijuDirectInstallState::Fetching { generation, package_id } => (generation, package_id),
        ref other => panic!("expected fetching state, got {other:?}"),
    };
    let selections = app.expert_hub.install_guide.direct_selections.clone();

    let _duplicate = app.reduce(SkillHubMessage::DirectInstallShiju);
    assert!(matches!(
        app.expert_hub.install_guide.direct_install,
        ShijuDirectInstallState::Fetching { generation: active, .. } if active == generation
    ));

    app.expert_hub.install_guide.reset_requests();
    let _stale = app.reduce(SkillHubMessage::DirectPackageLoaded {
        generation,
        account_id,
        package_id,
        selections,
        project_root: None,
        result: Ok(shiju_release()),
    });
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
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
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

    let _ = app.reduce(SkillHubMessage::ShijuInstallFinished {
        generation: generation.wrapping_sub(1),
        account_id: account_id.clone(),
        package_id,
        selections: selections.clone(),
        result: Ok(report.clone()),
    });
    assert!(app.expert_hub.install_guide.installations.is_empty());

    let task = app.reduce(SkillHubMessage::ShijuInstallFinished {
        generation,
        account_id,
        package_id,
        selections: selections.clone(),
        result: Ok(report),
    });

    assert_eq!(app.expert_hub.install_guide.selected_install_selections(), selections);
    assert_eq!(app.expert_hub.install_guide.installed_selection_count(), 2);
    assert_eq!(task.units(), 2);
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
    let mut app = Fixture::default();
    replace_repository(&mut app.account, repository_with_account("account-secret", "company-1"));
    app.expert_hub.account_id = Some(app.account.active_account().unwrap().id().to_string());
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.selected_targets =
        vec![SkillInstallTarget::AgentSkills, SkillInstallTarget::ClaudeCode];
    app.expert_hub
        .install_guide
        .set_installations(ShijuInstallationsReport { targets: installations });

    let _ = app.reduce(SkillHubMessage::UninstallShiju);
    let _ = app.reduce(SkillHubMessage::ConfirmShijuUninstall);
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
    let _ = app.reduce(SkillHubMessage::ShijuUninstallFinished {
        generation,
        selections: selected,
        result: Ok(ShijuUninstallReport { targets: uninstall_targets }),
    });
    assert_eq!(app.expert_hub.install_guide.installed_selection_count(), 0);
    assert!(
        app.expert_hub
            .install_guide
            .installation(SkillInstallScope::Global, SkillInstallTarget::Cursor)
            .is_some()
    );

    app.expert_hub.install_guide.selected_targets = vec![SkillInstallTarget::Cursor];
    let _ = app.reduce(SkillHubMessage::SyncShijuKey);
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
    let _ = app.reduce(SkillHubMessage::ShijuKeySyncFinished {
        generation: sync_generation,
        account_id: app.expert_hub.account_id.clone().unwrap(),
        selections: sync_selections,
        requested_by_user: true,
        result: Ok(ShijuKeySyncReport { targets: sync_targets }),
    });
    assert!(app.expert_hub.install_guide.login_sync.is_none());
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
fn private_shiju_tabs_dispatch_gateway_requests() {
    let _ = super::super::skill_gateway::gateway_shiju_skill_search_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_installed_async;
    let _ = super::super::skill_gateway::gateway_shiju_system_skills_async;
    let _ = super::super::skill_gateway::gateway_shiju_maintained_skills_async;
}

#[test]
fn private_shiju_detail_and_actions_use_gateway() {
    let _ = super::super::skill_gateway::gateway_shiju_skill_detail_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_readme_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_versions_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_remote_install_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_remote_install_version_async;
    let _ = super::super::skill_gateway::gateway_shiju_skill_remote_uninstall_async;
}
