use super::preset_search::{
    PresetMatch, PresetPlan, PresetRegion, PresetSearchEntry, group_presets, search_presets,
};
use vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{ModelCatalogIdentity, ScopedModelCatalogIdentity},
};

fn entry<'a>(name: &'a str, id: &'a str) -> PresetSearchEntry<'a, &'a str> {
    PresetSearchEntry {
        preset: id,
        name,
        name_key: None,
        website_url: None,
        family: None,
        plan: None,
        region: None,
    }
}

fn zhipu(
    name: &'static str,
    region: PresetRegion,
    website: &'static str,
    endpoint: &str,
) -> PresetSearchEntry<'static, ScopedModelCatalogIdentity> {
    PresetSearchEntry {
        preset: ScopedModelCatalogIdentity {
            scope: TenantScope::new("platform", "catalog-test").unwrap(),
            deployment: ModelCatalogIdentity {
                target_host_id: "host-a".into(),
                app_id: AppId::Codex,
                provider_id: name.replace(' ', "-"),
                endpoint: endpoint.into(),
            },
            model_id: "Vendor/GLM-5".into(),
        },
        name,
        name_key: None,
        website_url: Some(website),
        family: Some("zhipu"),
        plan: None,
        region: Some(region),
    }
}

fn zhipu_versions() -> [PresetSearchEntry<'static, ScopedModelCatalogIdentity>; 2] {
    [
        zhipu("Zhipu GLM en", PresetRegion::Intl, "https://z.ai/", "https://api.z.ai/api/paas/v4"),
        zhipu(
            "Zhipu GLM",
            PresetRegion::Cn,
            "https://www.bigmodel.cn/",
            "https://open.bigmodel.cn/api/paas/v4",
        ),
    ]
}

#[test]
fn catalog_preset_search_finds_chinese_alias() {
    let entries = zhipu_versions();
    let hits = search_presets(&entries, "智谱", |_| None);
    assert_eq!(hits.len(), 1, "智谱 should find the Zhipu family");
    let hit = &hits[0];
    assert_eq!(hit.group.family.unwrap().id, "zhipu");
    assert_eq!(hit.matched, PresetMatch::All);
    assert_eq!(hit.group.versions.len(), 2);
    assert_eq!(hit.group.versions[0].preset.model_id.as_ref(), "Vendor/GLM-5");
    assert_eq!(
        hit.group.versions[0].preset.deployment.endpoint,
        "https://open.bigmodel.cn/api/paas/v4"
    );
    assert!(std::ptr::eq(hit.group.versions[0], &entries[1]));
    assert!(std::ptr::eq(hit.group.versions[1], &entries[0]));
}

#[test]
fn catalog_preset_search_matches_old_and_company_names() {
    let mut kimi = entry("Kimi", "kimi-payg");
    kimi.family = Some("kimi");
    kimi.plan = Some(PresetPlan::Payg);
    let mut kimi_coding = entry("Kimi For Coding", "kimi-coding");
    kimi_coding.family = Some("kimi");
    kimi_coding.plan = Some(PresetPlan::Coding);
    let entries = [kimi, kimi_coding, entry("Meta Model API", "meta")];
    let company = search_presets(&entries, "  MOONSHOT  ", |_| None);
    assert_eq!(company.len(), 1);
    assert_eq!(company[0].matched, PresetMatch::All);
    assert_eq!(company[0].group.family.unwrap().id, "kimi");
    let old = search_presets(&entries, "facebook muse", |_| None);
    assert_eq!(old.len(), 1);
    assert!(old[0].group.family.is_none());
    assert_eq!(old[0].group.versions[0].preset, "meta");
}

#[test]
fn catalog_preset_search_limits_version_matches_to_the_matching_region() {
    let entries = zhipu_versions();
    let hits = search_presets(&entries, "Z.AI", |_| None);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].matched, PresetMatch::Versions(vec![1]));
    assert_eq!(hits[0].group.versions[1].region, Some(PresetRegion::Intl));
    let translated = search_presets(&entries, "海外", |key| {
        (key == "providerPreset.region.intl").then(|| "海外".to_owned())
    });
    assert_eq!(translated.len(), 1);
    assert_eq!(translated[0].matched, PresetMatch::Versions(vec![1]));
    assert!(search_presets(&entries, "cn", |_| None).is_empty());
    assert!(search_presets(&entries, "paas", |_| None).is_empty());
}

#[test]
fn catalog_preset_search_preserves_single_versions_and_source_row_order() {
    let mut single = entry("OpenCode Go", "go");
    single.family = Some("opencode");
    single.plan = Some(PresetPlan::Coding);
    let entries = [entry("First", "first"), single, entry("Last", "last")];
    let groups = group_presets(&entries);
    assert_eq!(groups.len(), 3);
    assert!(groups[1].family.is_none());
    assert_eq!(groups[1].versions[0].preset, "go");
    assert!(search_presets(&entries, "unknown", |_| None).is_empty());
    let hits = search_presets(&entries, "  ", |_| None);
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].group.versions[0].preset, "first");
    assert_eq!(hits[2].group.versions[0].preset, "last");
}

#[test]
fn catalog_preset_search_orders_listed_plans_then_source_plans_and_regions() {
    use PresetPlan::{AgentPlan, Coding, Payg, TokenPlan};
    use PresetRegion::{Cn, Intl};
    let entries = [
        ("agent-intl", Some(AgentPlan), Some(Intl)),
        ("token-cn", Some(TokenPlan), Some(Cn)),
        ("coding-intl", Some(Coding), Some(Intl)),
        ("payg-intl", Some(Payg), Some(Intl)),
        ("payg-cn", Some(Payg), Some(Cn)),
        ("agent-cn", Some(AgentPlan), Some(Cn)),
        ("unspecified", None, None),
    ]
    .map(|(id, plan, region)| PresetSearchEntry {
        family: Some("kimi"),
        plan,
        region,
        ..entry(id, id)
    });
    let groups = group_presets(&entries);
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0].versions.iter().map(|row| row.preset).collect::<Vec<_>>(),
        [
            "payg-cn",
            "payg-intl",
            "coding-intl",
            "agent-cn",
            "agent-intl",
            "token-cn",
            "unspecified"
        ]
    );
}
