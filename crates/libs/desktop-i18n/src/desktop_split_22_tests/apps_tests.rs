
#[test]
fn desktop_gettext_area_14_all_registered_templates_have_translations() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"(?:\.__|\.format|SkillHubError::localized|SkillCacheSyncError::(?:new|labeled)|Message::(?:new|nested))\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    let mut count = 0;
    for source in [
        include_str!("../../../../vw-desktop/src/app/message/shiju_apps.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub/connectors.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub/public.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub.rs"),
        include_str!("../../../desktop-app-launch/src/shiju_apps/launcher.rs"),
        include_str!("../../../../vw-desktop/src/app/shiju_apps/mod.rs"),
        include_str!("../../../desktop-app-launch/src/shiju_apps/state.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/cache_sync.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/client.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/connectors.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/installer.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/models.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/native.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/package_version_cache.rs"),
        include_str!("../../../../vw-desktop/src/app/skill_hub/state.rs"),
        include_str!("../../../../vw-desktop/src/app/views/apps/tiles.rs"),
        include_str!("../../../../vw-desktop/src/app/views/apps/ui.rs"),
        include_str!("../../../../vw-desktop/src/app/views/apps.rs"),
        include_str!("../../../desktop-expert-hub/src/views/connectors.rs"),
        include_str!("../../../desktop-expert-hub/src/views/detail.rs"),
        include_str!("../../../desktop-expert-hub/src/views/experts.rs"),
        include_str!("../../../desktop-expert-hub/src/views/install_guide.rs"),
        include_str!("../../../desktop-expert-hub/src/views/installed_skills.rs"),
        include_str!("../../../desktop-expert-hub/src/views/list.rs"),
        include_str!("../../../../vw-desktop/src/app/views/expert_hub/mod.rs"),
        include_str!("../../../desktop-expert-hub/src/views/public_skill_detail.rs"),
        include_str!("../../../desktop-expert-hub/src/views/public_skills.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/api_key_binding.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/detail.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/mine.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/mod.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/store.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/styles.rs"),
        include_str!("../../../desktop-expert-hub/src/views/styles.rs"),
        include_str!("../../../../vw-desktop/src/app/views/home/common.rs"),
        include_str!("../../../desktop-navigation/src/views/home/header.rs"),
        include_str!("../../../../vw-desktop/src/app/views/home/mod.rs"),
        include_str!("../../../desktop-navigation/src/views/home/recent.rs"),
        include_str!("../../../../vw-desktop/src/app/views/knowledge.rs"),
        include_str!("../../../../vw-desktop/src/app/views/mod.rs"),
        include_str!("../../../desktop-usage/src/components.rs"),
        include_str!("../../../../vw-desktop/src/app/views/usage/data.rs"),
        include_str!("../../../../vw-desktop/src/app/views/usage/layout.rs"),
        include_str!("../../../../vw-desktop/src/app/views/usage/mod.rs"),
        include_str!("../../../desktop-usage/src/steps.rs"),
        include_str!("../../../desktop-usage/src/styles.rs"),
        include_str!("../../../desktop-usage/src/utils.rs"),
        include_str!("../../../../vw-desktop/src/apps/mod.rs"),
    ] {
        for capture in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            if !key.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
                continue;
            }
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
            count += 1;
        }
    }
    assert!(count > 500);
}
