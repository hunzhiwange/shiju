use super::*;
#[test]
fn directory_detection_distinguishes_files_directories_and_unknown_paths() {
    let files = vec!["/project/src/main.rs".to_string(), "/project/README.md".to_string()];

    assert!(super::is_directory_path("/project/src", &files));
    assert!(!super::is_directory_path("/project/README.md", &files));
    assert!(super::is_directory_path("/project/empty", &files));
}

#[test]
fn unique_name_preserves_free_name_and_suffixes_collisions() {
    let directory = std::path::Path::new("/project/src");
    let mut existing = std::collections::HashSet::new();

    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main.rs"
    );

    existing.insert("/project/src/main.rs".to_string());
    existing.insert("/project/src/main copy.rs".to_string());
    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main copy 2.rs"
    );

    existing.insert("/project/src/LICENSE".to_string());
    assert_eq!(
        super::unique_name_for_target(directory, "LICENSE", &existing),
        "/project/src/LICENSE copy"
    );
}

#[test]
fn unique_name_falls_back_to_original_after_suffix_space_is_exhausted() {
    let directory = std::path::Path::new("/project/src");
    let mut existing = std::collections::HashSet::from([
        "/project/src/main.rs".to_string(),
        "/project/src/main copy.rs".to_string(),
    ]);
    for suffix in 2..=999 {
        existing.insert(format!("/project/src/main copy {suffix}.rs"));
    }

    assert_eq!(
        super::unique_name_for_target(directory, "main.rs", &existing),
        "/project/src/main.rs"
    );
}

#[test]
fn path_prefix_replacement_updates_exact_and_descendant_paths_only() {
    assert_eq!(
        super::replace_path_prefix("/project/old", "/project/old", "/project/new").as_deref(),
        Some("/project/new")
    );
    assert_eq!(
        super::replace_path_prefix("/project/old/nested/file.rs", "/project/old", "/project/new",)
            .as_deref(),
        Some("/project/new/nested/file.rs")
    );
    assert_eq!(
        super::replace_path_prefix("/project/older/file.rs", "/project/old", "/project/new"),
        None
    );
}

#[test]
fn path_display_name_uses_final_component_or_original_fallback() {
    assert_eq!(super::path_display_name("/project/src/main.rs"), "main.rs");
    assert_eq!(super::path_display_name(""), "");
    assert_eq!(super::path_display_name("/"), "/");
}

#[test]
fn recursive_batches_remove_descendants_without_matching_sibling_prefixes() {
    let paths = ["/project/src/a.txt", "/project/src-old/b.txt", "/project/src", "/project/src"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(top_level_paths(paths), ["/project/src-old/b.txt", "/project/src"]);
}

#[test]
fn paste_reserves_distinct_targets_for_same_named_sources() {
    let files = vec!["/project/a.txt".into(), "/project/z.txt".into(), "/project/dst/a.txt".into()];
    let targets = paste_targets(
        &files,
        "/project/dst",
        &["/project/one/a.txt".into(), "/project/two/a.txt".into()],
    );
    assert_eq!(targets[0].1, "/project/dst/a copy.txt");
    assert_eq!(targets[1].1, "/project/dst/a copy 2.txt");
}
