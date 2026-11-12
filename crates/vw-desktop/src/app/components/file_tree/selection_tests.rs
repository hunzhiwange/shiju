use super::*;

fn visible() -> Vec<String> {
    ["/project/a", "/project/b", "/project/c", "/project/d"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[test]
fn selection_toggles_items_across_separate_modifier_gestures() {
    let mut selection = FileTreeSelection::default();
    let paths = visible();
    selection.select("/project", paths[0].clone(), false, false, &paths);
    selection.select("/project", paths[2].clone(), true, false, &paths);
    assert_eq!(selection.paths, BTreeSet::from([paths[0].clone(), paths[2].clone()]));
    selection.select("/project", paths[1].clone(), true, false, &paths);
    selection.select("/project", paths[2].clone(), true, false, &paths);
    assert_eq!(selection.paths, BTreeSet::from([paths[0].clone(), paths[1].clone()]));
    selection.select("/project", paths[3].clone(), false, false, &paths);
    assert_eq!(selection.paths, BTreeSet::from([paths[3].clone()]));
}

#[test]
fn selection_ranges_keep_anchor_and_support_additive_and_reverse_ranges() {
    let mut selection = FileTreeSelection::default();
    let paths = visible();
    selection.select("/project", paths[1].clone(), false, false, &paths);
    selection.select("/project", paths[3].clone(), false, true, &paths);
    assert_eq!(selection.paths, paths[1..=3].iter().cloned().collect());
    selection.select("/project", paths[0].clone(), false, true, &paths);
    assert_eq!(selection.paths, paths[0..=1].iter().cloned().collect());
    selection.select("/project", paths[3].clone(), true, true, &paths);
    assert_eq!(selection.paths, paths.iter().cloned().collect());
}

#[test]
fn selection_does_not_leak_between_projects_or_use_hidden_range_anchors() {
    let mut selection = FileTreeSelection::default();
    let paths = visible();
    selection.select("/project", paths[0].clone(), false, false, &paths);
    assert!(!selection.contains("/other", &paths[0]));
    selection.select("/project", paths[2].clone(), false, true, &paths[1..]);
    assert_eq!(selection.paths, BTreeSet::from([paths[2].clone()]));
    selection.select("/other", "/other/a".to_string(), true, true, &["/other/a".to_string()]);
    assert_eq!(selection.paths, BTreeSet::from(["/other/a".to_string()]));
}

#[test]
fn visible_paths_follow_tree_order_and_exclude_collapsed_children() {
    let mut app = App::new().0;
    app.project_path = Some("/project".to_string());
    app.set_file_index(
        "/project",
        vec![
            "/project/src/nested/a.rs".to_string(),
            "/project/src/b.rs".to_string(),
            "/project/README.md".to_string(),
        ],
    );
    app.file_tree_expanded.clear();
    app.file_tree_expanded_set.clear();
    assert_eq!(visible_paths(&app, "/project"), ["/project/src", "/project/README.md"]);
    app.toggle_file_tree_dir_expanded("src".to_string());
    assert_eq!(
        visible_paths(&app, "/project"),
        ["/project/src", "/project/src/nested", "/project/src/b.rs", "/project/README.md",]
    );
}

#[test]
fn relocation_updates_selected_descendants_and_anchor_without_matching_sibling_prefix() {
    let mut selection = FileTreeSelection::default();
    let paths = ["/project/src", "/project/src/a.txt", "/project/src-old"].map(str::to_string);
    for index in [2, 0, 1] {
        selection.select("/project", paths[index].clone(), true, false, &paths);
    }
    selection.relocate("/other", "/project/src", "/project/dst/src");
    assert!(selection.contains("/project", &paths[1]));
    selection.relocate("/project", "/project/src", "/project/dst/src");
    assert!(selection.contains("/project", "/project/dst/src/a.txt"));
    assert!(selection.contains("/project", "/project/dst/src"));
    assert!(selection.contains("/project", "/project/src-old"));
    assert_eq!(selection.anchor.as_deref(), Some("/project/dst/src/a.txt"));
}
