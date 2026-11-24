#[test]
fn diff_file_expansion_helpers_keep_order_and_set_in_sync() {
    let mut git = crate::state::State::new(false, false);

    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &git.expanded_files_set,
        "src/lib.rs"
    ));

    crate::message_git::diff::ensure_diff_file_expanded(
        &mut git.expanded_files_set,
        &mut git.expanded_files,
        "src/lib.rs".to_string(),
    );
    crate::message_git::diff::ensure_diff_file_expanded(
        &mut git.expanded_files_set,
        &mut git.expanded_files,
        "src/lib.rs".to_string(),
    );
    assert_eq!(git.expanded_files, ["src/lib.rs"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "src/lib.rs"));

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "src/lib.rs".to_string(),
    );
    assert!(git.expanded_files.is_empty());
    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &git.expanded_files_set,
        "src/lib.rs"
    ));

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "src/main.rs".to_string(),
    );
    crate::message_git::diff::replace_expanded_files(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        vec!["README.md".to_string(), "src/main.rs".to_string()],
    );
    assert_eq!(git.expanded_files, ["README.md", "src/main.rs"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "README.md"));

    crate::message_git::diff::set_single_expanded_file(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
        "Cargo.toml".to_string(),
    );
    assert_eq!(git.expanded_files, ["Cargo.toml"]);
    assert!(crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "Cargo.toml"));
    assert!(!crate::message_git::diff::is_diff_file_expanded(&git.expanded_files_set, "README.md"));

    crate::message_git::diff::clear_expanded_files(
        &mut git.expanded_files,
        &mut git.expanded_files_set,
    );
    assert!(git.expanded_files.is_empty());
    assert!(git.expanded_files_set.is_empty());
}

#[test]
fn expansion_toggles_clear_membership_when_ordered_entries_are_absent() {
    let mut state = crate::state::State::new(false, false);
    state.expanded_files.clear();
    state.expanded_files_set.clear();

    state.expanded_files_set.insert("src/lib.rs".to_string());

    crate::message_git::diff::toggle_diff_file_expanded(
        &mut state.expanded_files,
        &mut state.expanded_files_set,
        "src/lib.rs".to_string(),
    );

    assert!(!crate::message_git::diff::is_diff_file_expanded(
        &state.expanded_files_set,
        "src/lib.rs"
    ));
    assert!(state.expanded_files.is_empty());
}

#[test]
fn desktop_gettext_workbench_visible_copy() {
    use config_ui::DesktopLocale::{English, SimplifiedChinese, TraditionalChinese};
    use crate::git_panel::{repository::history_copy, repository_ops::RepositoryState};
    use desktop_i18n::I18n;

    let mut state = RepositoryState::default();
    let cases = [
        (SimplifiedChinese, "提交历史", "正在加载历史…", "暂无提交历史", "1 个变更文件 · 点击展开差异"),
        (English, "Commit history", "Loading history…", "No commit history", "1 changed files · Click to expand diff"),
        (TraditionalChinese, "提交歷史", "正在載入歷史…", "暫無提交歷史", "1 個變更檔案 · 點選展開差異"),
    ];
    for (locale, title, loading, empty, files) in cases {
        let i18n = I18n::new(locale);
        state.loading = true;
        let copy = history_copy(i18n, &state);
        assert_eq!(copy.title, title);
        assert_eq!(copy.status, Some(loading));
        state.loading = false;
        assert_eq!(history_copy(i18n, &state).status, Some(empty));
        state.detail = Some(vw_api_git::GitHistoryDetailDto {
            message: "用户提交 %s/%n：修复项目".into(),
            files: vec![vw_api_git::GitHistoryFileDto {
                path: "用户项目/功能.rs".into(),
                status: vw_api_git::GitFileStatus::Modified,
                old_content: Some("旧内容 %n".into()),
                new_content: Some("新内容 %s".into()),
                notice: None,
                insertions: 1,
                deletions: 1,
            }],
        });
        state.snapshot.branch = Some("分支/%s".into());
        assert_eq!(history_copy(i18n, &state).changed_files.as_deref(), Some(files));
        assert_eq!(state.detail.as_ref().unwrap().message, "用户提交 %s/%n：修复项目");
        assert_eq!(state.detail.as_ref().unwrap().files[0].path, "用户项目/功能.rs");
        assert_eq!(state.snapshot.branch.as_deref(), Some("分支/%s"));
        state.snapshot.commits.push(vw_api_git::GitHistoryCommitDto {
            sha: "abc123".into(), summary: "用户提交 %s".into(), author: "作者%n".into(), time: 0,
        });
        assert_eq!(history_copy(i18n, &state).status, None);
        assert_eq!(state.snapshot.commits[0].summary, "用户提交 %s");
        state.snapshot.commits.clear();
    }
}
