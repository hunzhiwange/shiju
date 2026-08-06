use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketCoordinateDto, SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto, SkillMarketSkillDetailDto,
    SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto, SkillMarketSourceDto,
};

use super::public::{
    PublicRequest, PublicSkillMessage, begin_batch_action, begin_install, reduce,
    update as update_public,
};
use crate::app::App;
use crate::app::skill_hub::{
    LoadState, PublicBatchActionKind, PublicInstallState, PublicSkillMarketState,
    SkillInstallScope, SkillInstallTarget,
};

fn source(source: &str) -> SkillMarketSourceDto {
    SkillMarketSourceDto { source: source.to_string(), display_name: source.to_uppercase() }
}

fn skill(source: &str, slug: &str) -> SkillMarketSkillDto {
    SkillMarketSkillDto {
        coordinate: SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: source.to_string(),
                namespace: "public".to_string(),
                slug: slug.to_string(),
            },
            version: "1.0.0".to_string(),
        },
        name: slug.to_string(),
        description: String::new(),
        author: String::new(),
        category: String::new(),
        tags: Vec::new(),
        icon: None,
        downloads: 0,
        installs: 0,
        rating: 0.0,
        updated_at: String::new(),
    }
}

fn loaded_sources(state: &mut PublicSkillMarketState) {
    let requests = reduce(state, PublicSkillMessage::LoadSources);
    let generation = match requests.as_slice() {
        [PublicRequest::Sources { generation }] => *generation,
        other => panic!("unexpected source request: {other:?}"),
    };
    reduce(
        state,
        PublicSkillMessage::SourcesLoaded {
            generation,
            result: Ok(vec![source("skillhub"), source("second")]),
        },
    );
}

#[test]
fn public_skill_tabs_ignore_stale_cross_source_results() {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    let old_generation = state.recommended_generation;

    reduce(&mut state, PublicSkillMessage::SelectSource("second".to_string()));
    reduce(
        &mut state,
        PublicSkillMessage::RecommendedLoaded {
            source: "skillhub".to_string(),
            generation: old_generation,
            result: Ok(vec![skill("skillhub", "stale")]),
        },
    );

    assert_eq!(state.current_source.as_deref(), Some("second"));
    assert!(!matches!(
        state.recommended,
        LoadState::Loaded(ref skills) if skills.iter().any(|skill| skill.coordinate.locator.slug == "stale")
    ));
}

#[test]
fn public_skill_detail_can_switch_install_project_without_switching_workspace() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/current".to_string());
    app.recent_projects = vec!["/repo/other".to_string(), "/repo/current".to_string()];
    loaded_sources(&mut app.expert_hub.public_market);
    let locator = SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "safe-skill".to_string(),
    };

    let _ = update_public(&mut app, PublicSkillMessage::OpenSkillDetail(locator));
    assert_eq!(
        app.expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.as_deref()),
        Some("/repo/current")
    );

    let previous_generation = app.expert_hub.public_market.installed_generation;
    let _ = update_public(
        &mut app,
        PublicSkillMessage::SelectDetailInstallProject("/repo/other".to_string()),
    );

    assert_eq!(app.project_path.as_deref(), Some("/repo/current"));
    assert_eq!(
        app.expert_hub
            .public_market
            .detail
            .as_ref()
            .and_then(|detail| detail.selected_install_project_path.as_deref()),
        Some("/repo/other")
    );
    assert!(app.expert_hub.public_market.installed_generation > previous_generation);
}

#[test]
fn featured_rotation_wraps_without_gateway_reload() {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    let generation = state.recommended_generation;
    reduce(
        &mut state,
        PublicSkillMessage::RecommendedLoaded {
            source: "skillhub".to_string(),
            generation,
            result: Ok((0..7).map(|index| skill("skillhub", &format!("skill-{index}"))).collect()),
        },
    );

    let before_generation = state.recommended_generation;
    for _ in 0..7 {
        assert!(reduce(&mut state, PublicSkillMessage::RotateFeatured).is_empty());
    }

    assert_eq!(state.recommended_generation, before_generation);
    assert_eq!(state.featured_offset, 0);
}

#[test]
fn market_detail_installs_skill_or_suite_through_gateway() {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    let locator = SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "safe-skill".to_string(),
    };
    let requests = reduce(&mut state, PublicSkillMessage::OpenSkillDetail(locator.clone()));
    assert!(matches!(
        requests.as_slice(),
        [
            PublicRequest::SkillDetail { locator: detail, .. },
            PublicRequest::SkillFile { locator: file, .. }
        ] if detail == &locator && file == &locator
    ));
    let generation = state.detail_generation;
    let coordinate =
        SkillMarketCoordinateDto { locator: locator.clone(), version: "1.0.0".to_string() };
    reduce(
        &mut state,
        PublicSkillMessage::SkillDetailLoaded {
            locator: locator.clone(),
            generation,
            result: Ok(SkillMarketSkillDetailDto {
                skill: skill("skillhub", "safe-skill"),
                license: None,
                homepage: None,
                repository: None,
                requires_api_key: false,
                security: None,
            }),
        },
    );
    reduce(
        &mut state,
        PublicSkillMessage::SkillFileLoaded {
            locator: locator.clone(),
            generation,
            result: Ok(SkillMarketSkillFileDto {
                coordinate: coordinate.clone(),
                content: "# Safe".to_string(),
            }),
        },
    );
    assert!(matches!(
        begin_install(&mut state, None).expect("global skill install"),
        PublicRequest::InstallSkill {
            project_path,
            coordinate: requested,
            installations,
            ..
        } if project_path.is_empty()
            && requested == coordinate
            && installations == vec![SkillMarketInstallSelectionDto {
                target: SkillMarketInstallTargetDto::VibeWindow,
                scope: SkillMarketInstallScopeDto::Global,
            }]
    ));
    state.detail.as_mut().expect("skill detail").install = PublicInstallState::Idle;
    reduce(&mut state, PublicSkillMessage::SelectDetailInstallScope(SkillInstallScope::Project));
    reduce(&mut state, PublicSkillMessage::ToggleDetailInstallTarget(SkillInstallTarget::Codex));
    reduce(
        &mut state,
        PublicSkillMessage::ToggleDetailInstallTarget(SkillInstallTarget::VibeWindow),
    );
    assert!(matches!(
        begin_install(&mut state, Some("/project")).expect("project Codex install"),
        PublicRequest::InstallSkill {
            project_path,
            installations,
            ..
        } if project_path == "/project"
            && installations == vec![SkillMarketInstallSelectionDto {
                target: SkillMarketInstallTargetDto::Codex,
                scope: SkillMarketInstallScopeDto::Project,
            }]
    ));

    reduce(&mut state, PublicSkillMessage::CloseDetail);
    let suite_requests = reduce(
        &mut state,
        PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "safe-suite".to_string(),
        },
    );
    assert!(matches!(
        suite_requests.as_slice(),
        [PublicRequest::SuiteDetail { source, slug, .. }]
            if source == "skillhub" && slug == "safe-suite"
    ));
    let generation = state.detail_generation;
    reduce(
        &mut state,
        PublicSkillMessage::SuiteDetailLoaded {
            source: "skillhub".to_string(),
            slug: "safe-suite".to_string(),
            generation,
            result: Ok(SkillMarketSkillSetDetailDto {
                skill_set: SkillMarketSkillSetDto {
                    source: "skillhub".to_string(),
                    slug: "safe-suite".to_string(),
                    name: "Safe Suite".to_string(),
                    description: String::new(),
                    author: String::new(),
                    icon: None,
                    member_count: 0,
                    updated_at: String::new(),
                },
                members: Vec::new(),
                content: "# Safe Suite".to_string(),
            }),
        },
    );
    assert!(matches!(
        begin_install(&mut state, Some("/project")).expect("suite install"),
        PublicRequest::InstallSuite {
            project_path,
            source,
            slug,
            ..
        } if project_path == "/project" && source == "skillhub" && slug == "safe-suite"
    ));

    assert_eq!(
        begin_install(&mut state, None).expect_err("project is required"),
        "请先打开项目再安装技能"
    );
}

fn installed(id: &str) -> DesktopSkillCatalogEntryDto {
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
fn batch_delete_requires_confirmation_and_reports_partial_failure() {
    let mut state = PublicSkillMarketState::default();
    state.installed = LoadState::Loaded(vec![installed("first"), installed("second")]);
    state.select_all_installed();

    assert!(reduce(&mut state, PublicSkillMessage::RequestBatchDelete).is_empty());
    assert!(state.delete_confirmation_open);
    let requests = begin_batch_action(&mut state, Some("/project"), PublicBatchActionKind::Delete)
        .expect("confirmed delete");
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| matches!(request, PublicRequest::DeleteSkill { .. })));
    let generation = state.batch_action.as_ref().expect("batch action").generation;

    reduce(
        &mut state,
        PublicSkillMessage::BatchItemFinished {
            generation,
            project_path: "/project".to_string(),
            skill_id: "first".to_string(),
            result: Ok(()),
        },
    );
    let refresh = reduce(
        &mut state,
        PublicSkillMessage::BatchItemFinished {
            generation,
            project_path: "/project".to_string(),
            skill_id: "second".to_string(),
            result: Err("fixture failure".to_string()),
        },
    );

    assert!(matches!(
        refresh.as_slice(),
        [PublicRequest::RefreshInstalled { project_path, .. }]
            if project_path == "/project"
    ));
    assert_eq!(state.installed_selection, ["second".to_string()].into_iter().collect());
    assert_eq!(
        state.last_batch_summary,
        Some(crate::app::skill_hub::PublicBatchSummary { succeeded: 1, failed: 1 })
    );
}
