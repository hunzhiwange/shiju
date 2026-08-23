//! 工作区状态测试，验证 Git 工作树选项展示与搜索选择状态的数据保持。
//! 测试夹具确保选中的仓库、分支和查找上下文不会在结构转换中丢失。

use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("workspace_tests"));
}

#[test]
fn git_worktree_option_displays_label() {
    let option = GitWorktreeOption {
        directory: "/work/tree".to_string(),
        label: "feature branch".to_string(),
        branch: Some("feature/test".to_string()),
    };

    assert_eq!(option.to_string(), "feature branch");
    assert_eq!(format!("{option}"), "feature branch");
    assert_eq!(option.directory, "/work/tree");
    assert_eq!(option.branch.as_deref(), Some("feature/test"));
}

#[test]
fn workspace_structs_keep_selected_git_and_find_state() {
    let selected = GitDiffSelectedLine {
        file: "src/lib.rs".to_string(),
        line: 12,
        is_old: false,
        text: "let value = 1;".to_string(),
    };
    assert_eq!(selected.file, "src/lib.rs");
    assert_eq!(selected.line, 12);
    assert!(!selected.is_old);
    assert_eq!(selected.text, "let value = 1;");

    let range =
        GitDiffLineRange { file: "src/lib.rs".to_string(), start: 10, end: 12, is_old: true };
    assert_eq!(range.file, "src/lib.rs");
    assert_eq!(range.start, 10);
    assert_eq!(range.end, 12);
    assert!(range.is_old);

    let context = GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 11,
        is_old: false,
        x: 4.0,
        y: 8.0,
    };
    assert_eq!(context.file, "src/lib.rs");
    assert_eq!(context.line, 11);
    assert_eq!(context.x, 4.0);
    assert_eq!(context.y, 8.0);

    let menu = GitDiffFileMenuState { file: "src/lib.rs".to_string() };
    assert_eq!(menu.file, "src/lib.rs");

    let clipboard =
        FileTreeClipboard { mode: FileTreeClipboardMode::Copy, src_path: "/tmp/a.txt".to_string() };
    assert!(matches!(clipboard.mode, FileTreeClipboardMode::Copy));
    assert_eq!(clipboard.src_path, "/tmp/a.txt");

    let find_match = FindInFolderMatch {
        path: "src/lib.rs".to_string(),
        line: 3,
        column: 5,
        preview: "hello world".to_string(),
        match_len: 5,
    };
    assert_eq!(find_match.path, "src/lib.rs");
    assert_eq!(find_match.line, 3);
    assert_eq!(find_match.column, 5);
    assert_eq!(find_match.preview, "hello world");
    assert_eq!(find_match.match_len, 5);
}
