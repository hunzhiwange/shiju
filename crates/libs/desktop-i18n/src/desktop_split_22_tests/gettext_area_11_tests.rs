
#[test]
fn desktop_gettext_area_11_catalog_covers_display_paths() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let literal = regex::Regex::new(r#"\.__\("([^"]+)"\)"#).unwrap();
    for source in [
        include_str!("../../../../vw-desktop/src/app/components/git_panel/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/left_panel.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/header.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/filter_options.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/diff_view/context_menu.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/diff_view/comment_editor.rs"),
        include_str!("../../../../vw-desktop/src/app/components/git_panel/diff_view/file_view/mod.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            assert!(en.contains_key(&capture[1]), "missing English: {}", &capture[1]);
            assert!(tw.contains_key(&capture[1]), "missing Traditional Chinese: {}", &capture[1]);
        }
    }
}
