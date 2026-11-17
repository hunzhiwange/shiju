
#[test]
fn desktop_gettext_area_14_feedback_catalog_covers_message_paths() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(
        r#"show_localized_toast\(\s*crate::app::state::ToastKind::\w+,\s*("(?:[^"\\]|\\.)*")"#,
    )
    .unwrap();
    let mut count = 0;
    for source in [
        include_str!("../../../../vw-desktop/src/app/message/shiju_apps.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub/connectors.rs"),
        include_str!("../../../../vw-desktop/src/app/message/skill_hub/public.rs"),
    ] {
        for captures in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&captures[1]).unwrap();
            assert!(en.contains_key(&key), "English feedback missing: {key}");
            assert!(tw.contains_key(&key), "Traditional feedback missing: {key}");
            count += 1;
        }
    }
    assert!(count > 60);
}

#[test]
fn desktop_gettext_area_14_public_helpers_have_catalog_entries() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"\.__\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    for source in [
        include_str!("../../../desktop-expert-hub/src/views/public_skills.rs"),
        include_str!("../../../desktop-expert-hub/src/views/list.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/store.rs"),
        include_str!("../../../desktop-expert-hub/src/views/shiju_apps/mine.rs"),
    ] {
        for captures in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&captures[1]).unwrap();
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
        }
    }
}
