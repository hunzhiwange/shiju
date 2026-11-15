const SOURCE: &str = include_str!("agents_view.rs");
#[test]
fn agent_catalog_reserves_space_between_content_and_scrollbar() {
    assert!(SOURCE.contains("container(catalog::grid(input.catalog)).padding([0, 7])"));
}

#[test]
fn agent_project_options_put_current_first_and_deduplicate_recent_paths() {
    let current = Some("/repo/current".to_string());
    let recent = vec![
        "/repo/other".to_string(),
        "/repo/current".to_string(),
        "  /repo/third  ".to_string(),
        String::new(),
    ];

    assert_eq!(
        super::agent_project_options(current.as_deref(), &recent),
        vec!["/repo/current".to_string(), "/repo/other".to_string(), "/repo/third".to_string(),]
    );
}
