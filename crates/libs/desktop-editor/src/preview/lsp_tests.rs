#[test]
fn lsp_workspace_root_prefers_project_containing_preview_file() {
    let project = std::env::current_dir().unwrap().join("nested-project");
    let file = project.join("src").join("main.rs");
    let project_path = project.to_string_lossy();

    let root_uri = super::lsp_root_uri_for_preview_path(Some(&project_path), &file);

    assert_eq!(root_uri, Some(preview_content::path_to_file_uri(&project)));
}

#[test]
fn lsp_workspace_root_keeps_active_project_for_external_definition() {
    let project = std::env::current_dir().unwrap().join("active-project");
    let external_file = std::env::temp_dir().join("dependency").join("lib.rs");
    let project_path = project.to_string_lossy();

    let root_uri = super::lsp_root_uri_for_preview_path(Some(&project_path), &external_file);

    assert_eq!(root_uri, Some(preview_content::path_to_file_uri(&project)));
}

#[test]
fn lsp_workspace_root_uses_file_parent_without_active_project() {
    let file = std::env::current_dir().unwrap().join("standalone").join("main.rs");
    let expected_root = file.parent().unwrap();

    let root_uri = super::lsp_root_uri_for_preview_path(None, &file);

    assert_eq!(root_uri, Some(preview_content::path_to_file_uri(expected_root)));
}

#[test]
fn find_word_start_handles_boundaries_unicode_and_separators() {
    assert_eq!(super::find_word_start("hello_world", 11), 0);
    assert_eq!(super::find_word_start("let value", 9), 4);
    assert_eq!(super::find_word_start("变量_name", 7), 0);
    assert_eq!(super::find_word_start("value", 0), 0);
    assert_eq!(super::find_word_start("value", 99), 99);
}

#[test]
fn file_uri_to_path_decodes_spaces_and_rejects_non_file_uri() {
    assert_eq!(
        super::file_uri_to_path("file:///tmp/my%20file.rs").as_deref(),
        Some("/tmp/my file.rs")
    );
    assert_eq!(super::file_uri_to_path("https://example.com/main.rs"), None);
}
