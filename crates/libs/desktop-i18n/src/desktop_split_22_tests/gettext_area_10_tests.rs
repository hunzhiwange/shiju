
#[test]
fn desktop_gettext_area_10_catalog_covers_display_paths() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let literal =
        regex::Regex::new(r#"(?:\.__|\.format|ProjectFeedback::new)\(\s*("(?:\\.|[^"\\])*")"#)
            .unwrap();
    for source in [
        include_str!("../../../desktop-file-tree/src/find_results.rs"),
        include_str!("../../../../vw-desktop/src/app/components/file_tree/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/components/file_tree/tree_list.rs"),
        include_str!("../../../../vw-desktop/src/app/components/search_panel.rs"),
        include_str!("../project.rs"),
        include_str!("../../../../vw-desktop/src/app/message/project/helpers.rs"),
        include_str!("../../../desktop-workspace/src/project_edit_view/general_tab.rs"),
        include_str!("../../../desktop-workspace/src/project_edit_view/launch_tab.rs"),
        include_str!("../../../desktop-workspace/src/project_edit_view/refresh_tab.rs"),
        include_str!("../../../desktop-workspace/src/project_edit_view/scheduling_tab.rs"),
        include_str!("../../../../vw-desktop/src/app/impls/app_view_modals/project_edit/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/views/project/layout/work_sidebar.rs"),
        include_str!("../../../../vw-desktop/src/app/views/project/layout/work_resources.rs"),
        include_str!("../../../../vw-desktop/src/app/views/project/components/projects_list.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "missing English key: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese key: {key}");
        }
    }
}
