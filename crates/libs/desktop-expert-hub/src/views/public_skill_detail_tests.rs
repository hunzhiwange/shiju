//! 公共技能详情测试，覆盖 Markdown 内容及按安装范围生成的平台动作。
//! 项目上下文缺失或安装正忙时，操作可用性必须收敛到安全状态。


use gateway_desktop_settings::DesktopSkillCatalogEntryDto;
use super::public_skill_detail::installed_in_catalog;
use desktop_skill_hub::skill_hub::{LoadState, PublicDetailTarget, PublicSkillMarketState};

#[test]
fn expert_package_install_catalog_requires_matching_origin_and_meta_skill() {
    let target = PublicDetailTarget::Suite {
        source: "skillhub".to_string(),
        slug: "fixture-suite".to_string(),
    };
    let mut state = PublicSkillMarketState::default();
    let mut entry = DesktopSkillCatalogEntryDto {
        id: "skillhub--skillset--fixture-suite".to_string(),
        title: "fixture-suite".to_string(),
        description: String::new(),
        kind: "personal".to_string(),
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "other".to_string(),
        source_path: Some("/project/.agents/skills/fixture-suite".to_string()),
        origin: Some("workspace".to_string()),
        namespace: Some("skillset".to_string()),
        package_type: Some("meta-skill".to_string()),
        source_pack: Some("fixture-suite".to_string()),
    };
    state.installed = LoadState::Loaded(vec![entry.clone()]);
    assert!(!installed_in_catalog(&state, &target));
    entry.source = "skillhub".to_string();
    state.installed = LoadState::Loaded(vec![entry.clone()]);
    assert!(installed_in_catalog(&state, &target));
    entry.package_type = Some("skill".to_string());
    state.installed = LoadState::Loaded(vec![entry]);
    assert!(!installed_in_catalog(&state, &target));
}
