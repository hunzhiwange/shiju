//! 验证文件引用规范化、行号剥离以及项目根目录内外的路径解析。
//! 引号、文件 URL、裸行号与 Windows 分隔符用于覆盖展示引用的兼容输入。

use super::path::{
    normalize_file_reference_to_path, normalize_file_url_to_path, relative_to_project_root,
    resolve_path,
};
use crate::app::App;

#[test]
fn normalize_file_references_strip_wrappers_and_line_suffixes() {
    assert_eq!(normalize_file_url_to_path("file:///tmp/a.rs"), "tmp/a.rs");
    assert_eq!(
        normalize_file_reference_to_path("[main](file:///tmp/a.rs#L10)"),
        Some("tmp/a.rs".to_string())
    );
    assert_eq!(
        normalize_file_reference_to_path("`'/tmp/demo.rs#line-12'`"),
        Some("/tmp/demo.rs".to_string())
    );
    assert_eq!(normalize_file_reference_to_path(""), None);
}

#[test]
fn normalize_file_url_handles_double_slash_prefix() {
    assert_eq!(normalize_file_url_to_path("file://path"), "path");
    assert_eq!(normalize_file_url_to_path("plain/path"), "plain/path");
}

#[test]
fn normalize_file_reference_strips_quote_wrappers() {
    assert_eq!(normalize_file_reference_to_path("`/tmp/foo.rs`"), Some("/tmp/foo.rs".to_string()));
    assert_eq!(
        normalize_file_reference_to_path("\"/tmp/foo.rs\""),
        Some("/tmp/foo.rs".to_string())
    );
    assert_eq!(normalize_file_reference_to_path("'/tmp/foo.rs'"), Some("/tmp/foo.rs".to_string()));
}

#[test]
fn normalize_file_reference_returns_none_after_stripping_quotes() {
    assert_eq!(normalize_file_reference_to_path("''"), None);
    assert_eq!(normalize_file_reference_to_path("\"\""), None);
    assert_eq!(normalize_file_reference_to_path("``"), None);
}

#[test]
fn normalize_file_reference_returns_none_for_empty_normalized() {
    assert_eq!(normalize_file_reference_to_path("file:///"), None);
    assert_eq!(normalize_file_reference_to_path("file://"), None);
}

#[test]
fn normalize_file_reference_handles_bare_line_suffix() {
    assert_eq!(
        normalize_file_reference_to_path("/path/file.rs#line-42"),
        Some("/path/file.rs".to_string())
    );
}

#[test]
fn resolve_path_returns_none_when_reference_normalizes_to_none() {
    let app = App::new().0;
    assert_eq!(resolve_path(&app, ""), None);
    assert_eq!(resolve_path(&app, "''"), None);
    assert_eq!(resolve_path(&app, "file:///"), None);
}

#[test]
fn resolve_path_falls_back_to_current_dir_when_no_project_root() {
    let app = App::new().0;
    let result = resolve_path(&app, "some/file.rs");
    assert!(result.is_some());
    assert!(result.unwrap().ends_with("some/file.rs"));
}

#[test]
fn relative_to_project_root_returns_none_when_no_root() {
    let app = App::new().0;
    assert_eq!(relative_to_project_root(&app, "/tmp/file.rs"), None);
}

#[test]
fn relative_to_project_root_returns_none_when_path_outside_root() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());
    assert_eq!(relative_to_project_root(&app, "/other/file.rs"), None);
}

#[test]
fn resolve_and_relativize_paths_against_project_root() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());

    assert_eq!(resolve_path(&app, "src/main.rs").as_deref(), Some("/tmp/project/src/main.rs"));
    assert_eq!(
        relative_to_project_root(&app, "/tmp/project/src/main.rs").as_deref(),
        Some("src/main.rs")
    );
    assert_eq!(resolve_path(&app, "/tmp/absolute.rs").as_deref(), Some("/tmp/absolute.rs"));
}
