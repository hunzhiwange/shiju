// 覆盖文件遍历、目录树与文本搜索的过滤和限制规则。

use super::{FilesInput, SearchInput, files, is_hidden_path, match_globs, search, tree};
use glob::Pattern;
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
fn files_respects_hidden_flag_and_globs() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("visible.rs"), "fn main() {}").expect("write visible");
    fs::write(temp.path().join(".hidden.rs"), "fn hidden() {}").expect("write hidden");

    let result = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: Some(vec!["*.rs".to_string()]),
        hidden: Some(false),
        follow: Some(false),
        max_depth: None,
    })
    .expect("files should work");

    assert_eq!(result, vec!["visible.rs".to_string()]);
    assert!(is_hidden_path(".hidden.rs"));
    let globs = [Pattern::new("*.rs").expect("valid glob")];
    assert!(match_globs(&globs, "visible.rs"));
    assert!(!match_globs(&globs, "visible.txt"));
    assert!(!is_hidden_path("src/lib.rs"));
}

#[test]
fn path_helpers_handle_dot_segments_and_empty_glob_sets() {
    assert!(!is_hidden_path("."));
    assert!(!is_hidden_path("src/./lib.rs"));
    assert!(is_hidden_path("src/.config/settings.toml"));
    assert!(!match_globs(&[], "src/lib.rs"));
}

#[test]
fn files_defaults_include_hidden_and_skip_ignored_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join(".git/objects")).expect("create git");
    fs::create_dir_all(temp.path().join("node_modules/pkg")).expect("create node_modules");
    fs::create_dir_all(temp.path().join("target/debug")).expect("create target");
    fs::write(temp.path().join("visible.txt"), "visible").expect("write visible");
    fs::write(temp.path().join(".hidden.txt"), "hidden").expect("write hidden");
    fs::write(temp.path().join(".git/config"), "git").expect("write git");
    fs::write(temp.path().join("node_modules/pkg/index.js"), "pkg").expect("write pkg");
    fs::write(temp.path().join("target/debug/app"), "bin").expect("write target");

    let result = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: None,
        hidden: None,
        follow: None,
        max_depth: None,
    })
    .expect("files should work");

    assert_eq!(result, vec![".hidden.txt".to_string(), "visible.txt".to_string()]);
}

#[test]
fn files_ignores_invalid_globs_and_honors_max_depth() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src/nested")).expect("create nested");
    fs::write(temp.path().join("root.rs"), "root").expect("write root");
    fs::write(temp.path().join("src/lib.rs"), "lib").expect("write lib");
    fs::write(temp.path().join("src/nested/deep.rs"), "deep").expect("write deep");

    let result = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: Some(vec!["[".to_string()]),
        hidden: Some(true),
        follow: Some(false),
        max_depth: Some(2),
    })
    .expect("files should work");

    assert_eq!(result, vec!["root.rs".to_string(), "src/lib.rs".to_string()]);
}

#[test]
fn files_keeps_valid_globs_when_other_patterns_are_invalid() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("lib.rs"), "lib").expect("write rust");
    fs::write(temp.path().join("readme.md"), "docs").expect("write markdown");
    fs::write(temp.path().join("notes.txt"), "notes").expect("write text");

    let result = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: Some(vec!["[".to_string(), "*.rs".to_string(), "*.md".to_string()]),
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })
    .expect("files should work");

    assert_eq!(result, vec!["lib.rs".to_string(), "readme.md".to_string()]);
}

#[test]
fn files_skips_walk_errors_for_a_missing_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");

    let result = files(FilesInput {
        cwd: missing,
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })
    .expect("walk errors should be skipped");

    assert!(result.is_empty());
}

#[cfg(unix)]
#[test]
fn files_follow_flag_controls_symlinked_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("real.txt"), "real").expect("write real");
    symlink(temp.path().join("real.txt"), temp.path().join("link.txt")).expect("symlink");

    let without_follow = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })
    .expect("files should work");
    let with_follow = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: None,
        hidden: Some(true),
        follow: Some(true),
        max_depth: None,
    })
    .expect("files should work");

    assert_eq!(without_follow, vec!["real.txt".to_string()]);
    assert_eq!(with_follow, vec!["link.txt".to_string(), "real.txt".to_string()]);
}

#[test]
fn tree_lists_directories_and_reports_truncation() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src/bin")).expect("create src");
    fs::create_dir_all(temp.path().join("docs")).expect("create docs");
    fs::create_dir_all(temp.path().join("examples/basic")).expect("create examples");
    fs::create_dir_all(temp.path().join(".vibewindow/cache")).expect("create vibewindow");
    fs::write(temp.path().join("top.txt"), "top").expect("write top");
    fs::write(temp.path().join("src/lib.rs"), "lib").expect("write lib");
    fs::write(temp.path().join("src/bin/main.rs"), "main").expect("write main");
    fs::write(temp.path().join("docs/readme.md"), "docs").expect("write docs");
    fs::write(temp.path().join("examples/basic/main.rs"), "example").expect("write example");
    fs::write(temp.path().join(".vibewindow/cache/state.json"), "{}").expect("write state");

    let full = tree(temp.path(), None).expect("tree should work");
    assert_eq!(full, "docs\nexamples\nexamples/basic\nsrc");

    let truncated = tree(temp.path(), Some(2)).expect("tree should work");
    assert_eq!(truncated, "docs\nexamples\n[2 truncated]");
}

#[test]
fn tree_handles_no_directories_and_zero_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("top.txt"), "top").expect("write top");
    assert_eq!(tree(temp.path(), None).expect("tree should work"), "");

    fs::create_dir_all(temp.path().join("src/nested")).expect("create nested");
    fs::write(temp.path().join("src/nested/lib.rs"), "lib").expect("write nested");

    assert_eq!(tree(temp.path(), Some(0)).expect("tree should work"), "[2 truncated]");
}

#[test]
fn files_filters_nested_hidden_paths_and_can_walk_file_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src/.secret")).expect("create secret");
    fs::write(temp.path().join("src/.secret/config.txt"), "secret").expect("write secret");
    fs::write(temp.path().join("src/public.txt"), "public").expect("write public");

    let visible = files(FilesInput {
        cwd: temp.path().to_path_buf(),
        glob: Some(vec!["src/*.txt".to_string()]),
        hidden: Some(false),
        follow: Some(false),
        max_depth: None,
    })
    .expect("files should work");
    assert_eq!(visible, vec!["src/public.txt".to_string()]);

    let file_root = temp.path().join("src/public.txt");
    let single_file = files(FilesInput {
        cwd: file_root,
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })
    .expect("files should work");
    assert_eq!(single_file, vec!["".to_string()]);
}

#[test]
fn search_returns_match_metadata_and_respects_glob_and_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("a.txt"), "alpha\nneedle one\nneedle two\n").expect("write a");
    fs::write(temp.path().join("b.md"), "needle markdown\n").expect("write b");

    let result = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "needle".to_string(),
        glob: Some(vec!["*.txt".to_string()]),
        limit: Some(1),
        follow: Some(false),
    })
    .expect("search should work");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "a.txt");
    assert_eq!(result[0].line_number, 2);
    assert_eq!(result[0].line, "needle one");
    assert_eq!(result[0].start, 0);
    assert_eq!(result[0].end, 6);
}

#[test]
fn search_skips_non_utf8_files_handles_zero_limit_and_reports_regex_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("binary.txt"), [0xff, 0xfe]).expect("write binary");
    fs::write(temp.path().join("text.txt"), "first\nneedle\n").expect("write text");

    let invalid = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "[".to_string(),
        glob: None,
        limit: None,
        follow: Some(false),
    })
    .expect_err("invalid regex should fail");
    assert_eq!(invalid.kind(), std::io::ErrorKind::Other);

    let no_results = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "needle".to_string(),
        glob: None,
        limit: Some(0),
        follow: Some(false),
    })
    .expect("search should work");
    assert!(no_results.is_empty());

    let result = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "needle".to_string(),
        glob: None,
        limit: None,
        follow: Some(false),
    })
    .expect("search should work");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "text.txt");
}

#[test]
fn search_uses_default_limit_and_records_match_offsets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut content = String::new();
    for i in 0..120 {
        content.push_str(&format!("prefix needle {i}\n"));
    }
    fs::write(temp.path().join("many.txt"), content).expect("write many");

    let result = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "needle \\d+".to_string(),
        glob: None,
        limit: None,
        follow: Some(false),
    })
    .expect("search should work");

    assert_eq!(result.len(), 100);
    assert_eq!(result[0].line_number, 1);
    assert_eq!(result[0].start, 7);
    assert_eq!(result[0].end, 15);
    assert_eq!(result[99].line_number, 100);
}

#[test]
fn search_records_only_the_first_match_per_line_with_utf8_byte_offsets() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("unicode.txt"), "é needle needle\nno match\n你needle\n")
        .expect("write unicode");

    let result = search(SearchInput {
        cwd: temp.path().to_path_buf(),
        pattern: "needle".to_string(),
        glob: None,
        limit: None,
        follow: Some(false),
    })
    .expect("search should work");

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].line_number, 1);
    assert_eq!(result[0].line, "é needle needle");
    assert_eq!(result[0].start, 3);
    assert_eq!(result[0].end, 9);
    assert_eq!(result[1].line_number, 3);
    assert_eq!(result[1].start, 3);
    assert_eq!(result[1].end, 9);
}
