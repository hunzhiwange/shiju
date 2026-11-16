
#[test]
fn desktop_gettext_area_15_display_literals_have_both_catalog_entries() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let calls = regex::Regex::new(r#"__\(\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    let mut count = 0;
    for source in [
        include_str!("../../../../vw-desktop/src/app/components/about_modal.rs"),
        include_str!("../../../../vw-desktop/src/app/components/install_cli_modal.rs"),
        include_str!("../../../../vw-desktop/src/app/components/notification.rs"),
        include_str!("../../../../vw-desktop/src/app/components/toast.rs"),
        include_str!("../../../../vw-desktop/src/app/impls/app_view_modals/account_details.rs"),
        include_str!("../../../../vw-desktop/src/app/impls/app_view_modals/permission_modal.rs"),
        include_str!("../../../../vw-desktop/src/app/impls/app_view_modals/question_modal.rs"),
        include_str!("../../../../vw-desktop/src/app/impls/app_view_modals/rename.rs"),
        include_str!("../../../../vw-desktop/src/app/views/apps/modals.rs"),
        include_str!("../../../../vw-desktop/src/app/views/usage/raw_modal.rs"),
    ] {
        for capture in calls.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "Missing English: {key}");
            assert!(tw.contains_key(&key), "Missing Traditional Chinese: {key}");
            count += 1;
        }
    }
    assert!(count > 70);
}
