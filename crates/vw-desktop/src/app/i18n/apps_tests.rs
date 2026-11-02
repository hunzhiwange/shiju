use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_14_all_registered_templates_have_translations() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"(?:\.__|\.format|SkillHubError::localized|SkillCacheSyncError::(?:new|labeled)|Message::(?:new|nested))\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    let mut count = 0;
    for source in [
        include_str!("../message/shiju_apps.rs"),
        include_str!("../message/skill_hub/connectors.rs"),
        include_str!("../message/skill_hub/public.rs"),
        include_str!("../message/skill_hub.rs"),
        include_str!("../shiju_apps/launcher.rs"),
        include_str!("../shiju_apps/mod.rs"),
        include_str!("../shiju_apps/state.rs"),
        include_str!("../skill_hub/cache_sync.rs"),
        include_str!("../skill_hub/client.rs"),
        include_str!("../skill_hub/connectors.rs"),
        include_str!("../skill_hub/installer.rs"),
        include_str!("../skill_hub/mod.rs"),
        include_str!("../skill_hub/models.rs"),
        include_str!("../skill_hub/native.rs"),
        include_str!("../skill_hub/package_version_cache.rs"),
        include_str!("../skill_hub/state.rs"),
        include_str!("../views/apps/tiles.rs"),
        include_str!("../views/apps/ui.rs"),
        include_str!("../views/apps.rs"),
        include_str!("../views/expert_hub/connectors.rs"),
        include_str!("../views/expert_hub/detail.rs"),
        include_str!("../views/expert_hub/experts.rs"),
        include_str!("../views/expert_hub/install_guide.rs"),
        include_str!("../views/expert_hub/installed_skills.rs"),
        include_str!("../views/expert_hub/list.rs"),
        include_str!("../views/expert_hub/mod.rs"),
        include_str!("../views/expert_hub/public_skill_detail.rs"),
        include_str!("../views/expert_hub/public_skills.rs"),
        include_str!("../views/expert_hub/shiju_apps/api_key_binding.rs"),
        include_str!("../views/expert_hub/shiju_apps/detail.rs"),
        include_str!("../views/expert_hub/shiju_apps/mine.rs"),
        include_str!("../views/expert_hub/shiju_apps/mod.rs"),
        include_str!("../views/expert_hub/shiju_apps/store.rs"),
        include_str!("../views/expert_hub/shiju_apps/styles.rs"),
        include_str!("../views/expert_hub/styles.rs"),
        include_str!("../views/home/common.rs"),
        include_str!("../views/home/header.rs"),
        include_str!("../views/home/mod.rs"),
        include_str!("../views/home/recent.rs"),
        include_str!("../views/knowledge.rs"),
        include_str!("../views/mod.rs"),
        include_str!("../views/usage/components.rs"),
        include_str!("../views/usage/data.rs"),
        include_str!("../views/usage/layout.rs"),
        include_str!("../views/usage/mod.rs"),
        include_str!("../views/usage/steps.rs"),
        include_str!("../views/usage/styles.rs"),
        include_str!("../views/usage/utils.rs"),
        include_str!("../../apps/mod.rs"),
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

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_14_hub_title_updates_after_locale_change() {
    let (mut app, _) = crate::app::App::new();
    crate::app::message::skill_hub::open_expert_hub_tab(&mut app);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "专家·技能·连接器"),
        (DesktopLocale::TraditionalChinese, "專家·技能·連接器"),
        (DesktopLocale::English, "Experts · Skills · Connectors"),
    ] {
        app.desktop_locale = locale;
        super::apps::refresh_hub_titles(&mut app);
        let tab = app
            .open_tabs
            .iter()
            .find(|tab| tab.id == crate::app::message::skill_hub::EXPERT_HUB_TAB_ID)
            .unwrap();
        assert_eq!(tab.title, expected);
    }
}
