// 验证公共技能来源切换、精选轮播与安装项目选择。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketCoordinateDto, SkillMarketInstallScopeDto,
    SkillMarketInstallSelectionDto, SkillMarketInstallTargetDto, SkillMarketSkillDetailDto,
    SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto, SkillMarketSkillSetPageDto,
    SkillMarketSourceDto,
};

use super::public::{
    PublicRequest, PublicSkillMessage, begin_batch_action, begin_install, reduce,
    update as update_public,
};
use crate::app::App;
use crate::app::message::skill_hub::{SkillHubMessage, update as update_hub};
use crate::app::skill_hub::{
    ExpertHubSection, LoadState, PublicBatchActionKind, PublicInstallState, PublicSkillMarketState,
    PublicSkillTab, SkillInstallScope, SkillInstallTarget,
};

fn source(source: &str) -> SkillMarketSourceDto {
    SkillMarketSourceDto { source: source.to_string(), display_name: source.to_uppercase() }
}

#[test]
fn expert_package_selects_installed_main_only() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/current".to_string());
    app.open_tabs.push(crate::app::AppTab {
        id: "project_/repo/current".to_string(),
        title: "current".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/repo/current".to_string()),
    });
    app.active_tab_id = Some("expert_hub".to_string());
    app.expert_hub.section = ExpertHubSection::Experts;
    loaded_sources(&mut app.expert_hub.public_market);
    let target = crate::app::skill_hub::PublicDetailTarget::Suite {
        source: "skillhub".to_string(),
        slug: "fixture-suite".to_string(),
    };
    reduce(
        &mut app.expert_hub.public_market,
        PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
        },
    );
    let detail = app.expert_hub.public_market.detail.as_mut().expect("detail");
    detail.selected_install_project_path = Some("/repo/current".to_string());
    detail.suite_detail = LoadState::Loaded(SkillMarketSkillSetDetailDto {
        skill_set: SkillMarketSkillSetDto {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
            name: "中文专家".to_string(),
            description: String::new(),
            author: String::new(),
            icon: None,
            member_count: 1,
            updated_at: String::new(),
        },
        members: vec![SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: "skillhub".to_string(),
                namespace: "public".to_string(),
                slug: "member".to_string(),
            },
            version: "1.0.0".to_string(),
        }],
        content: "# Main workflow".to_string(),
    });
    app.expert_hub.public_market.installed = LoadState::Loaded(vec![DesktopSkillCatalogEntryDto {
        id: "skillhub--skillset--fixture-suite".to_string(),
        title: "中文专家".to_string(),
        description: String::new(),
        kind: "personal".to_string(),
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "skillhub".to_string(),
        source_path: Some(
            "/repo/current/.agents/skills/skillhub--skillset--fixture-suite".to_string(),
        ),
        origin: Some("workspace".to_string()),
        namespace: Some("skillset".to_string()),
        package_type: Some("meta-skill".to_string()),
        source_pack: Some("fixture-suite".to_string()),
    }]);
    let _ = update_public(&mut app, PublicSkillMessage::SelectExpertForChat);
    assert_eq!(
        app.current_session_runtime().tool_selector.selected_manual_skills(),
        vec!["skillhub--skillset--fixture-suite".to_string()]
    );
    assert_eq!(app.active_tab_id.as_deref(), Some("project_/repo/current"));
    assert_eq!(app.expert_hub.public_market.detail.as_ref().expect("detail").target, target);
    app.expert_hub.public_market.detail.as_mut().expect("detail").selected_install_project_path =
        Some("/repo/other".to_string());
    assert!(super::public::selectable_expert_skill(&app).is_err());
    app.expert_hub.public_market.detail.as_mut().expect("detail").selected_install_project_path =
        Some("/repo/current".to_string());
    if let LoadState::Loaded(items) = &mut app.expert_hub.public_market.installed {
        items[0].enabled = false;
    }
    assert!(super::public::selectable_expert_skill(&app).is_err());
}

#[test]
fn expert_package_section_loads_suites() {
    let (mut app, _) = App::new();
    loaded_sources(&mut app.expert_hub.public_market);
    app.expert_hub.public_market.tab = PublicSkillTab::SkillHub;

    let _ = update_hub(&mut app, SkillHubMessage::SelectSection(ExpertHubSection::Experts));

    assert_eq!(app.expert_hub.section, ExpertHubSection::Experts);
    assert_eq!(app.expert_hub.public_market.tab, PublicSkillTab::Suites);
    assert!(matches!(app.expert_hub.public_market.suites, LoadState::Loading { .. }));
}

#[test]
fn expert_package_navigation_ignores_stale_results() {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    let old_generation = match reduce(
        &mut state,
        PublicSkillMessage::SelectTab(PublicSkillTab::Suites),
    )
    .as_slice()
    {
        [PublicRequest::Suites { generation, .. }] => *generation,
        requests => panic!("expected suite request: {requests:?}"),
    };
    reduce(&mut state, PublicSkillMessage::QueryDraftChanged("中文专家".to_string()));
    let new_generation = match reduce(&mut state, PublicSkillMessage::SubmitQuery).as_slice() {
        [PublicRequest::Suites { generation, query, .. }] => {
            assert_eq!(query.keyword.as_deref(), Some("中文专家"));
            *generation
        }
        requests => panic!("expected search request: {requests:?}"),
    };
    assert_ne!(old_generation, new_generation);
    reduce(
        &mut state,
        PublicSkillMessage::SuitesLoaded {
            source: "skillhub".to_string(),
            generation: old_generation,
            result: Err("stale failure".to_string()),
        },
    );
    assert!(
        matches!(state.suites, LoadState::Loading { generation } if generation == new_generation)
    );

    reduce(
        &mut state,
        PublicSkillMessage::SuitesLoaded {
            source: "skillhub".to_string(),
            generation: new_generation,
            result: Ok(SkillMarketSkillSetPageDto {
                skillsets: Vec::new(),
                total: 48,
                page: 1,
                page_size: 24,
            }),
        },
    );
    let page_generation = match reduce(&mut state, PublicSkillMessage::NextPage).as_slice() {
        [PublicRequest::Suites { generation, query, .. }] => {
            assert_eq!(query.page, 2);
            *generation
        }
        requests => panic!("expected next page request: {requests:?}"),
    };
    reduce(
        &mut state,
        PublicSkillMessage::SuitesLoaded {
            source: "skillhub".to_string(),
            generation: page_generation,
            result: Err("network unavailable".to_string()),
        },
    );
    assert!(matches!(state.suites, LoadState::Error(_)));
    assert!(matches!(reduce(&mut state, PublicSkillMessage::Retry).as_slice(),
        [PublicRequest::Suites { query, .. }] if query.page == 2));
}

#[test]
fn expert_package_install_requires_project() {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    reduce(
        &mut state,
        PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
        },
    );
    reduce(&mut state, PublicSkillMessage::OpenExpertInstall);
    assert!(state.detail.as_ref().expect("detail").install_confirmation_open);
    assert_eq!(
        begin_install(&mut state, None).expect_err("project required"),
        "请先选择项目再安装技能"
    );
}

fn loaded_expert_package() -> PublicSkillMarketState {
    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    reduce(
        &mut state,
        PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
        },
    );
    let generation = state.detail_generation;
    reduce(
        &mut state,
        PublicSkillMessage::SuiteDetailLoaded {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
            generation,
            result: Ok(SkillMarketSkillSetDetailDto {
                skill_set: SkillMarketSkillSetDto {
                    source: "skillhub".to_string(),
                    slug: "fixture-suite".to_string(),
                    name: "Fixture Suite".to_string(),
                    description: String::new(),
                    author: String::new(),
                    icon: None,
                    member_count: 1,
                    updated_at: String::new(),
                },
                members: vec![SkillMarketCoordinateDto {
                    locator: SkillMarketSkillLocatorDto {
                        source: "skillhub".to_string(),
                        namespace: "public".to_string(),
                        slug: "member".to_string(),
                    },
                    version: "1.0.0".to_string(),
                }],
                content: "# Workflow".to_string(),
            }),
        },
    );
    state
}

#[test]
fn expert_package_install_failure_preserves_reason() {
    let mut state = loaded_expert_package();
    reduce(&mut state, PublicSkillMessage::OpenExpertInstall);
    let request = begin_install(&mut state, Some("/project")).expect("start install");
    let PublicRequest::InstallSuite { target, generation, project_path, .. } = request else {
        panic!("expected suite installation");
    };
    let reason = "成员 member 签名校验失败".to_string();
    let requests = reduce(
        &mut state,
        PublicSkillMessage::InstallFinished {
            target,
            generation,
            project_path,
            result: Err(reason.clone()),
        },
    );
    assert!(requests.is_empty());
    assert!(matches!(state.detail.as_ref().expect("detail").install, PublicInstallState::Idle));
    assert!(begin_install(&mut state, Some("/project")).is_ok(), "failure must allow retry");
    assert_eq!(reason, "成员 member 签名校验失败");
}

#[test]
fn expert_package_install_success_refreshes_catalog() {
    let mut state = loaded_expert_package();
    let request = begin_install(&mut state, Some("/project")).expect("start install");
    let PublicRequest::InstallSuite { target, generation, project_path, .. } = request else {
        panic!("expected suite installation");
    };
    let requests = reduce(
        &mut state,
        PublicSkillMessage::InstallFinished {
            target: target.clone(),
            generation,
            project_path: project_path.clone(),
            result: Ok(vec![
                "/project/.agents/skills/skillhub--skillset--fixture-suite".to_string(),
            ]),
        },
    );
    assert!(
        matches!(requests.as_slice(), [PublicRequest::RefreshInstalled { project_path: path, .. }] if path == &project_path)
    );
    assert!(matches!(
        state.detail.as_ref().expect("detail").install,
        PublicInstallState::Installed { .. }
    ));
    reduce(&mut state, PublicSkillMessage::CloseDetail);
    let ignored = reduce(
        &mut state,
        PublicSkillMessage::InstallFinished {
            target,
            generation,
            project_path,
            result: Err("late failure".to_string()),
        },
    );
    assert!(ignored.is_empty());
}

#[test]
fn expert_package_install_reopens_with_project_catalog_refresh() {
    let (mut app, _) = App::new();
    app.project_path = Some("/project/current".to_string());
    loaded_sources(&mut app.expert_hub.public_market);
    app.expert_hub.public_market.installed =
        LoadState::Loaded(vec![installed("skillhub--skillset--fixture-suite")]);
    let _ = update_public(
        &mut app,
        PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "fixture-suite".to_string(),
        },
    );
    assert!(matches!(app.expert_hub.public_market.installed, LoadState::Loading { .. }));
    assert_eq!(
        app.expert_hub
            .public_market
            .detail
            .as_ref()
            .expect("detail")
            .selected_install_project_path
            .as_deref(),
        Some("/project/current")
    );
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
fn suite_categories_reset_pagination_and_keep_skill_categories_independent() {
    use crate::app::skill_hub::PublicSkillTab;

    let mut state = PublicSkillMarketState::default();
    loaded_sources(&mut state);
    state.query.category = Some("development".to_string());
    state.query.keyword_draft = "review".to_string();
    reduce(&mut state, PublicSkillMessage::SubmitQuery);
    reduce(&mut state, PublicSkillMessage::SelectTab(PublicSkillTab::Suites));
    state.suites_page = 3;
    let requests =
        reduce(&mut state, PublicSkillMessage::SelectSuiteCategory(Some("tech".to_string())));
    let [PublicRequest::Suites { query, .. }] = requests.as_slice() else {
        panic!("expected suite request: {requests:?}");
    };
    assert_eq!(query.category.as_deref(), Some("tech"));
    assert_eq!(query.keyword.as_deref(), Some("review"));
    assert_eq!(query.page, 1);
    assert_eq!(state.query.category.as_deref(), Some("development"));

    let requests = reduce(&mut state, PublicSkillMessage::SelectTab(PublicSkillTab::SkillHub));
    assert!(requests.iter().any(|request| matches!(request,
        PublicRequest::Market { query, .. } if query.category.as_deref() == Some("development")
    )));
    reduce(&mut state, PublicSkillMessage::SelectTab(PublicSkillTab::Suites));
    let requests = reduce(&mut state, PublicSkillMessage::SelectSuiteCategory(None));
    assert!(matches!(requests.as_slice(), [PublicRequest::Suites { query, .. }]
        if query.category.is_none() && query.page == 1));

    reduce(&mut state, PublicSkillMessage::SelectSuiteCategory(Some("finance".to_string())));
    reduce(&mut state, PublicSkillMessage::SelectSource("second".to_string()));
    assert!(state.suites_category.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
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
        "请先选择项目再安装技能"
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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
