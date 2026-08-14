use super::*;
use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketSkillLocatorDto, SkillMarketSort, SkillMarketSourceDto,
};

#[test]
fn expert_hub_state_defaults_to_skills_marketplace() {
    let state = ExpertHubState::default();

    assert_eq!(state.section, ExpertHubSection::Skills);
    assert_eq!(state.skill_tab, SkillTab::Marketplace);
    assert_eq!(state.request_generation, 0);
    assert_eq!(state.installed_request_generation, 0);
    assert!(matches!(state.marketplace, LoadState::Idle));
    assert!(matches!(state.installed, LoadState::Idle));
    assert!(matches!(state.system, LoadState::Idle));
    assert!(matches!(state.company, LoadState::Idle));
    assert!(matches!(state.mine, LoadState::Idle));
    assert_eq!(state.shiju_apps, crate::app::shiju_apps::ShijuAppsState::default());
}

#[test]
fn shiju_package_status_ignores_late_refresh_results() {
    let cached =
        PackageVersionCacheEntry::new("3.0.3".to_string(), 1_785_830_400).expect("valid cache");
    let mut status = ShijuPackageStatus::from_sources(Some("3.0.2".to_string()), Some(cached));
    let stale_generation = status.start_refresh();
    let current_generation = status.start_refresh();
    let stale =
        PackageVersionCacheEntry::new("9.9.9".to_string(), 1_785_830_401).expect("valid result");

    assert!(!status.finish_refresh(stale_generation, Ok(stale)));
    assert_eq!(status.server_version.as_deref(), Some("3.0.3"));
    assert!(matches!(
        status.refresh,
        ShijuPackageRefreshState::Refreshing { generation }
            if generation == current_generation
    ));

    assert!(status.finish_refresh(current_generation, Err("network-secret".to_string())));
    assert_eq!(status.server_version.as_deref(), Some("3.0.3"));
    assert_eq!(status.checked_at_unix, Some(1_785_830_400));
    assert!(matches!(status.refresh, ShijuPackageRefreshState::Failed));
    assert!(!format!("{status:?}").contains("network-secret"));
}

#[test]
fn public_markdown_disables_fenced_code_highlighting_without_losing_code() {
    let markdown = concat!(
        "```rust\n",
        "// 中文注释\n",
        "```\n",
        "  ~~~~bash extra\n",
        "# 中文脚本\n",
        "  ~~~~\n",
    );
    let normalized =
        concat!("```\n", "// 中文注释\n", "```\n", "  ~~~~\n", "# 中文脚本\n", "  ~~~~\n",);

    let items = parse_public_markdown(markdown);
    let code_blocks = items
        .iter()
        .filter_map(|item| match item {
            iced::widget::markdown::Item::CodeBlock { language, code, .. } => {
                Some((language, code))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(super::state::markdown_without_code_highlighting(markdown), normalized);
    assert_eq!(super::state::markdown_without_code_highlighting("    ```text\n"), "    ```text\n");
    assert_eq!(code_blocks.len(), 2);
    assert!(code_blocks.iter().all(|(language, _)| language.is_none()));
    assert_eq!(code_blocks[0].1, "// 中文注释\n");
    assert_eq!(code_blocks[1].1, "# 中文脚本\n");
}

#[test]
fn expert_hub_state_clears_account_scoped_results_and_pages() {
    let mut state = ExpertHubState {
        account_id: Some("old-account".to_string()),
        search_draft: "draft".to_string(),
        submitted_search: "submitted".to_string(),
        system_filter: SkillListFilter {
            keyword_draft: "system".to_string(),
            submitted_keyword: "system".to_string(),
            tag_id_draft: Some(2),
            submitted_tag_id: Some(2),
        },
        company_status_draft: Some("draft".to_string()),
        submitted_company_status: Some("draft".to_string()),
        system_page: 4,
        company_page: 3,
        mine_page: 2,
        ..ExpertHubState::default()
    };
    state.marketplace =
        LoadState::Loaded(CursorPage { items: Vec::new(), next_cursor: None, has_more: false });

    state.clear_for_account(Some("new-account".to_string()));

    assert_eq!(state.account_id.as_deref(), Some("new-account"));
    assert!(state.search_draft.is_empty());
    assert!(state.submitted_search.is_empty());
    assert_eq!(state.system_filter, SkillListFilter::default());
    assert_eq!(state.company_status_draft, None);
    assert_eq!(state.submitted_company_status, None);
    assert_eq!((state.system_page, state.company_page, state.mine_page), (1, 1, 1));
    assert!(matches!(state.marketplace, LoadState::Idle));
    assert!(state.detail.is_none());
    assert!(state.action.is_none());
    assert_eq!(state.shiju_apps.account_id.as_deref(), Some("new-account"));
    assert!(matches!(state.shiju_apps.store, crate::app::shiju_apps::ShijuAppsLoadState::Idle));
}

#[test]
fn expert_hub_state_account_clear_invalidates_installed_requests() {
    let mut state = ExpertHubState::default();
    let stale_generation = state.next_installed_request_generation();
    state.installed = LoadState::Loading { generation: stale_generation };

    state.clear_for_account(Some("new-account".to_string()));

    assert!(state.installed_request_generation > stale_generation);
    assert!(matches!(state.installed, LoadState::Idle));
}

#[test]
fn expert_hub_state_deduplicates_skill_actions_until_completion() {
    let mut state = ExpertHubState::default();

    let generation =
        state.start_action(7, SkillActionKind::Install).expect("first action should start");
    assert!(state.start_action(7, SkillActionKind::Install).is_none());
    assert_eq!(state.action.as_ref().unwrap().generation, generation);
    assert_eq!(state.action.as_ref().unwrap().skill_id, 7);
}

#[test]
fn expert_hub_state_tracks_version_target_and_rollback_confirmation() {
    let mut state = ExpertHubState::default();
    let target = SkillVersionAction {
        skill_id: 7,
        version_id: 42,
        target_version: "1.2.0".to_string(),
        direction: SkillVersionDirection::Rollback,
    };

    assert!(state.request_version_action(target.clone()).is_none());
    assert_eq!(state.version_confirmation.as_ref(), Some(&target));
    assert!(state.start_action(7, SkillActionKind::Uninstall).is_none());
    let generation = state.confirm_version_action().expect("confirmed rollback should start");

    let action = state.action.as_ref().expect("version action should be busy");
    assert_eq!(action.generation, generation);
    assert_eq!(action.skill_id, 7);
    assert_eq!(action.kind, SkillActionKind::Version(target));
    assert!(state.confirm_version_action().is_none());
}

#[test]
fn expert_hub_state_serializes_cache_sync_and_keeps_retry_target() {
    let mut state = ExpertHubState::default();

    let generation = state.start_cache_sync(7, true).expect("first sync should start");
    assert!(state.start_cache_sync(8, false).is_none());
    assert!(!state.finish_cache_sync(generation.wrapping_sub(1), 7, false));
    assert!(state.finish_cache_sync(generation, 7, false));
    assert_eq!(state.cache_sync.pending_retry_skill_id(), Some(7));

    let retry_generation = state.start_cache_sync(7, true).expect("pending sync should retry");
    assert!(retry_generation > generation);
    assert!(state.start_action(7, SkillActionKind::Install).is_none());
}

#[test]
fn install_guide_state_resets_requests_without_leaking_signed_urls() {
    let mut state = ShijuInstallGuideState::default();
    state.open = true;
    state.current_package = Some(ShijuPackageId::DarwinArm64);
    state.agent_package = LoadState::Loaded(ShijuPackageRelease {
        download_url: SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/shiju.zip?signature=sensitive",
        )
        .unwrap(),
    });
    state.direct_install = ShijuDirectInstallState::Fetching {
        generation: 7,
        package_id: ShijuPackageId::DarwinArm64,
    };
    state.manual_download =
        Some(ShijuManualDownloadState { generation: 8, package_id: ShijuPackageId::LinuxAmd64 });

    state.reset_requests();

    assert!(state.open);
    assert!(matches!(state.agent_package, LoadState::Idle));
    assert!(matches!(state.direct_install, ShijuDirectInstallState::Idle));
    assert!(state.manual_download.is_none());
    assert!(!format!("{state:?}").contains("sensitive"));
}

#[test]
fn install_guide_defaults_to_global_shared_agents_target() {
    let state = ShijuInstallGuideState::default();

    assert_eq!(state.install_scope, SkillInstallScope::Global);
    assert_eq!(state.selected_targets, vec![SkillInstallTarget::AgentSkills]);
    assert_eq!(
        state.selected_install_selections(),
        vec![SkillInstallSelection {
            target: SkillInstallTarget::AgentSkills,
            scope: SkillInstallScope::Global,
        }]
    );
}

#[test]
fn install_guide_deduplicates_uninstall_and_preserves_local_status_on_reset() {
    let home = tempfile::tempdir().expect("home fixture");
    let selection = SkillInstallSelection {
        target: SkillInstallTarget::AgentSkills,
        scope: SkillInstallScope::Global,
    };
    let destination = resolve_skill_destinations(home.path(), None, &[selection])
        .expect("resolve fixture")
        .remove(0);
    let mut state = ShijuInstallGuideState::default();
    state.set_installations(ShijuInstallationsReport {
        targets: vec![ShijuTargetInstallation {
            installation: Some(ShijuInstallation {
                installed_dir: destination.installed_dir.clone(),
                version: Some("3.0.3".to_string()),
            }),
            destination,
        }],
    });

    let generation = state.start_uninstall().expect("start uninstall");
    assert!(state.start_uninstall().is_none());
    assert!(matches!(
        state.direct_install,
        ShijuDirectInstallState::Uninstalling {
            generation: active,
        } if active == generation
    ));

    state.reset_requests();
    assert!(state.is_installed);
    assert_eq!(state.installed_version.as_deref(), Some("3.0.3"));
    assert!(!state.uninstall_confirmation_open);
}

#[test]
fn market_state_consumes_gateway_contract_and_switches_sources() {
    let mut state = PublicSkillMarketState::default();
    state.sources = LoadState::Loaded(vec![
        SkillMarketSourceDto {
            source: "skillhub".to_string(),
            display_name: "SkillHub".to_string(),
        },
        SkillMarketSourceDto { source: "second".to_string(), display_name: "Second".to_string() },
    ]);
    state.current_source = Some("skillhub".to_string());
    state.query.keyword_draft = "rust".to_string();
    state.query.submitted_keyword = "old".to_string();
    state.query.category = Some("development".to_string());
    state.query.sort = SkillMarketSort::Downloads;
    state.query.page = 4;
    let generations = state.generations();

    assert!(state.select_source("second"));

    assert_eq!(state.current_source.as_deref(), Some("second"));
    assert_eq!(state.query.page, 1);
    assert!(state.query.keyword_draft.is_empty());
    assert!(state.query.submitted_keyword.is_empty());
    assert_eq!(state.query.category, None);
    assert_eq!(state.query.sort, SkillMarketSort::Score);
    assert!(matches!(state.recommended, LoadState::Idle));
    assert!(matches!(state.categories, LoadState::Idle));
    assert!(matches!(state.market, LoadState::Idle));
    assert!(matches!(state.suites, LoadState::Idle));
    assert!(state.generations().iter().zip(generations).all(|(after, before)| *after > before));
}

#[test]
fn market_detail_switches_visual_and_markdown_without_losing_coordinate() {
    let mut state = PublicSkillMarketState::default();
    state.sources = LoadState::Loaded(vec![SkillMarketSourceDto {
        source: "skillhub".to_string(),
        display_name: "SkillHub".to_string(),
    }]);
    state.current_source = Some("skillhub".to_string());
    let locator = SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "safe-skill".to_string(),
    };

    let generation = state.open_skill_detail(locator.clone());
    state.detail.as_mut().expect("detail").mode = PublicDetailMode::Markdown;

    assert_eq!(state.detail_generation, generation);
    assert_eq!(state.detail.as_ref().expect("detail").target, PublicDetailTarget::Skill(locator));
    assert_eq!(state.detail.as_ref().expect("detail").mode, PublicDetailMode::Markdown);
    assert_eq!(state.detail.as_ref().expect("detail").install_scope, SkillInstallScope::Global);

    state.sources = LoadState::Loaded(vec![
        SkillMarketSourceDto {
            source: "skillhub".to_string(),
            display_name: "SkillHub".to_string(),
        },
        SkillMarketSourceDto { source: "second".to_string(), display_name: "Second".to_string() },
    ]);
    assert!(state.select_source("second"));
    assert!(state.detail.is_none());
    assert!(state.detail_generation > generation);
}

fn installed_item(id: &str) -> DesktopSkillCatalogEntryDto {
    DesktopSkillCatalogEntryDto {
        id: id.to_string(),
        title: id.to_string(),
        description: String::new(),
        kind: "personal".to_string(),
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "skillhub".to_string(),
        source_path: Some(format!("/project/{id}")),
        origin: Some("workspace".to_string()),
        namespace: Some("public".to_string()),
        package_type: Some("skill".to_string()),
        source_pack: None,
    }
}

#[test]
fn installed_skills_batch_selection_and_actions_are_scoped() {
    let mut state = PublicSkillMarketState::default();
    state.installed = LoadState::Loaded(vec![installed_item("first"), installed_item("second")]);

    assert!(state.set_installed_selected("first", true));
    assert!(!state.set_installed_selected("missing", true));
    state.select_all_installed();
    assert_eq!(
        state.installed_selection,
        ["first".to_string(), "second".to_string()].into_iter().collect()
    );

    let action = state.start_batch_action(PublicBatchActionKind::Disable).expect("start batch");
    assert_eq!(action.skill_ids, vec!["first", "second"]);
    assert!(state.start_batch_action(PublicBatchActionKind::Enable).is_none());

    state.batch_action = None;
    state.clear_installed_selection();
    assert!(state.installed_selection.is_empty());
}
