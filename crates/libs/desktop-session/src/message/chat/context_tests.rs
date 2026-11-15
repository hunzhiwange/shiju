#[test]
fn mention_display_path_makes_project_paths_relative() {
    let project_path = Some("/workspace");

    assert_eq!(super::mention_display_path(project_path, "/workspace/src/main.rs"), "src/main.rs");
    assert_eq!(super::mention_display_path(project_path, "/workspace"), "");
}

#[test]
fn mention_display_path_preserves_outside_path_and_normalizes_separators() {
    let project_path = Some("/workspace");

    assert_eq!(super::mention_display_path(project_path, r"C:\repo\src\main.rs"), "C:/repo/src/main.rs");
}

#[test]
fn mention_display_path_without_project_normalizes_separators() {
    let project_path = None;

    assert_eq!(super::mention_display_path(project_path, r"src\nested\lib.rs"), "src/nested/lib.rs");
}
