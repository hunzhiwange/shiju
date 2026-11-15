use super::*;
#[test]
fn resolve_absolute_gateway_path_handles_empty_dot_and_relative_paths() {
    assert_eq!(resolve_absolute_gateway_path("/project", ""), "/project");
    assert_eq!(resolve_absolute_gateway_path("/project", "."), "/project");
    assert_eq!(resolve_absolute_gateway_path("/project", "src/lib.rs"), "/project/src/lib.rs");
}

#[test]
fn normalize_path_rewrites_separators_and_trims_trailing_slashes() {
    assert_eq!(normalize_path(r"C:\work\project\"), "C:/work/project");
    assert_eq!(normalize_path("/tmp/project///"), "/tmp/project");
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn normalize_path_handles_empty_and_root_only_inputs() {
    assert_eq!(normalize_path(""), "");
    assert_eq!(normalize_path("///"), "");
}

#[test]
fn collect_gateway_file_paths_recurses_directories_and_sorts_by_caller() {
    let tree = FileNodeDto {
        path: ".".to_string(),
        name: ".".to_string(),
        kind: FileNodeKind::Directory,
        size_bytes: None,
        children: Some(vec![
            FileNodeDto {
                path: "src".to_string(),
                name: "src".to_string(),
                kind: FileNodeKind::Directory,
                size_bytes: None,
                children: Some(vec![FileNodeDto {
                    path: "src/lib.rs".to_string(),
                    name: "lib.rs".to_string(),
                    kind: FileNodeKind::File,
                    size_bytes: Some(3),
                    children: None,
                }]),
            },
            FileNodeDto {
                path: "README.md".to_string(),
                name: "README.md".to_string(),
                kind: FileNodeKind::File,
                size_bytes: Some(4),
                children: None,
            },
        ]),
    };
    let mut paths = Vec::new();

    collect_gateway_file_paths("/project", &tree, &mut paths);
    paths.sort();

    assert_eq!(paths, vec!["/project/README.md".to_string(), "/project/src/lib.rs".to_string()]);
}

#[test]
fn collect_gateway_file_paths_ignores_directories_without_loaded_children() {
    let tree = FileNodeDto {
        path: "src".to_string(),
        name: "src".to_string(),
        kind: FileNodeKind::Directory,
        size_bytes: None,
        children: None,
    };
    let mut paths = vec!["already-present".to_string()];

    collect_gateway_file_paths("/project", &tree, &mut paths);

    assert_eq!(paths, vec!["already-present".to_string()]);
}

#[test]
fn map_timestamp_clamps_negative_values_to_zero() {
    assert_eq!(map_timestamp(-1), 0);
    assert_eq!(map_timestamp(42), 42);
}
