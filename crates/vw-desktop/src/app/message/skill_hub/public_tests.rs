use gateway_skill_market::{SkillMarketCoordinateDto, SkillMarketSkillLocatorDto, SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto, SkillMarketSourceDto};
// 验证公共技能来源切换、精选轮播与安装项目选择。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use gateway_desktop_settings::{DesktopSkillCatalogEntryDto};


use super::public::{PublicRequest, PublicSkillMessage, reduce, update as update_public};
use crate::app::App;
use crate::app::message::skill_hub::{SkillHubMessage, update as update_hub};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicSkillMarketState, PublicSkillTab};

fn source(source: &str) -> SkillMarketSourceDto {
    SkillMarketSourceDto { source: source.to_string(), display_name: source.to_uppercase() }
}

#[test]
fn expert_package_selects_installed_main_only() {
    let (mut app, _) = App::new();
    app.navigation_tabs.project_path = Some("/repo/current".to_string());
    app.navigation_tabs.open_tabs.push(crate::app::AppTab {
        id: "project_/repo/current".to_string(),
        title: "current".to_string(),
        screen: crate::app::Screen::Project,
        project_path: Some("/repo/current".to_string()),
    });
    app.navigation_tabs.active_tab_id = Some("expert_hub".to_string());
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
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("project_/repo/current"));
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
fn expert_package_install_reopens_with_project_catalog_refresh() {
    let (mut app, _) = App::new();
    app.navigation_tabs.project_path = Some("/project/current".to_string());
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
