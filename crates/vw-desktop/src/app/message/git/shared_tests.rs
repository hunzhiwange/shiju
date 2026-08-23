// 验证 Git 上下文路径解析、瞬态界面清理与差异加载任务。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::state::{
    ConventionalCommitType, GitDiffContextMenuState, GitDiffLineRange, GitDiffSelectedLine,
};
use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashSet;

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };

    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }

    Vec::new()
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn git_context_path_prefers_trimmed_worktree_and_falls_back_to_project() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/project/root".to_string());
    app.selected_git_worktree_directory = Some("  /project/worktree  ".to_string());

    assert_eq!(
        super::shared::git_context_path_for_app(&app),
        Some("/project/worktree".to_string())
    );

    app.selected_git_worktree_directory = Some(" \n\t ".to_string());

    assert_eq!(super::shared::git_context_path_for_app(&app), Some("/project/root".to_string()));
}

#[test]
fn git_context_path_returns_none_without_any_context() {
    let (mut app, _task) = crate::app::App::new();
    app.selected_git_worktree_directory = None;
    app.active_session_id = None;
    app.sessions.clear();
    app.project_path = None;

    assert_eq!(super::shared::git_context_path_for_app(&app), None);
}

#[test]
fn git_context_path_uses_trimmed_active_session_directory() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-1".to_string());
    app.sessions = vec![vw_shared::session::info::Info {
        id: "session-1".to_string(),
        slug: "session-1".to_string(),
        project_id: "project-1".to_string(),
        directory: "  /session/root  ".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 2,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }];
    app.project_path = Some("/project/root".to_string());

    assert_eq!(super::shared::git_context_path_for_app(&app).as_deref(), Some("/session/root"));
}

#[test]
fn dismiss_preview_transient_ui_clears_git_file_menu() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_file_menu =
        Some(crate::app::state::GitDiffFileMenuState { file: "src/lib.rs".to_string() });

    super::shared::dismiss_preview_transient_ui(&mut app);

    assert!(app.git_diff_file_menu.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_git_panel_data_task_emits_all_refresh_messages() {
    let messages = collect_task_messages(super::shared::refresh_git_panel_data_task()).await;

    assert_eq!(messages.len(), 3);
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshWorktreeOptions)
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshChangedFiles)
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Git(super::GitMessage::RefreshDiffFileMetas)
    )));
}

#[test]
fn take_diff_content_task_rejects_loaded_loading_and_missing_repo_states() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_contents.insert("loaded.rs".to_string(), (String::new(), String::new()));
    app.git_diff_contents_loading.insert("loading.rs".to_string());
    app.project_path = None;

    assert!(super::shared::take_diff_content_task(&mut app, "loaded.rs".to_string()).is_none());
    assert!(super::shared::take_diff_content_task(&mut app, "loading.rs".to_string()).is_none());
    assert!(super::shared::take_diff_content_task(&mut app, "missing.rs".to_string()).is_none());
}

#[test]
fn load_missing_diff_contents_without_repo_does_not_mark_files_loading() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;

    let _task = super::shared::load_missing_diff_contents(
        &mut app,
        ["a.rs".to_string(), "b.rs".to_string()],
    );

    assert!(app.git_diff_contents_loading.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn git_repo_path_rejects_explicit_non_repository_worktree() {
    let (mut app, _task) = crate::app::App::new();
    app.selected_git_worktree_directory = Some("/path/that/does/not/exist".to_string());
    app.project_path = Some("/fallback/project".to_string());

    assert_eq!(super::shared::git_repo_path_for_app(&app), None);
}

#[test]
fn refresh_after_repo_mutation_clears_stale_loading_guards() {
    let (mut app, _task) = crate::app::App::new();

    app.git_changed_files_loading = true;
    app.git_changed_files_repo_path = Some("/tmp/repo".to_string());
    app.git_diff_file_metas_loading = true;
    app.git_diff_file_metas_repo_path = Some("/tmp/repo".to_string());
    app.git_diff_contents_loading.insert("src/main.rs".to_string());

    let _task = super::shared::refresh_git_panel_data_after_repo_mutation_task(&mut app);

    assert!(!app.git_changed_files_loading);
    assert_eq!(app.git_changed_files_repo_path, None);
    assert!(!app.git_diff_file_metas_loading);
    assert_eq!(app.git_diff_file_metas_repo_path, None);
    assert!(app.git_diff_contents_loading.is_empty());
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn build_selected_commit_request_formats_conventional_subject_and_body() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_scope = "desktop".to_string();
    app.git_commit_message = " refresh git state ".to_string();
    app.git_commit_description = " body line ".to_string();
    app.staged_files_selected = vec!["src/main.rs".to_string()];
    app.staged_hunks_selected = vec![("src/lib.rs".to_string(), 2)];
    app.staged_lines_selected = vec![("src/lib.rs".to_string(), 4)];
    app.staged_old_lines_selected = vec![("src/lib.rs".to_string(), 3)];

    let request = super::shared::build_selected_commit_request(&app).expect("message is valid");

    assert_eq!(request.message, "fix(desktop): refresh git state\n\nbody line");
    assert!(!request.stage_all);
    assert_eq!(request.selected_files, vec!["src/main.rs"]);
    assert_eq!(request.selected_hunks, vec![("src/lib.rs".to_string(), 2)]);
    assert_eq!(request.selected_lines, vec![("src/lib.rs".to_string(), 4)]);
    assert_eq!(request.selected_old_lines, vec![("src/lib.rs".to_string(), 3)]);
}

#[test]
fn build_all_commit_request_enables_atomic_stage_all() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_message = "ship everything".to_string();

    let request = super::shared::build_all_commit_request(&app).expect("message is valid");

    assert!(request.stage_all);
    assert_eq!(request.message, "feat: ship everything");
}

#[test]
fn build_selected_commit_request_rejects_blank_message() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_type = None;
    app.git_commit_message = "   ".to_string();
    app.git_commit_description = "\n\t".to_string();

    let error = super::shared::build_selected_commit_request(&app).expect_err("blank is invalid");

    assert_eq!(error, "提交消息不能为空");
}

#[test]
fn build_selected_commit_request_formats_subject_without_scope_or_type() {
    let (mut app, _task) = crate::app::App::new();
    app.git_commit_type = Some(ConventionalCommitType::Feat);
    app.git_commit_scope = "  ".to_string();
    app.git_commit_message = " add tests ".to_string();

    let conventional =
        super::shared::build_selected_commit_request(&app).expect("summary is valid");

    assert_eq!(conventional.message, "feat: add tests");

    app.git_commit_type = None;
    app.git_commit_description = " details ".to_string();

    let plain = super::shared::build_selected_commit_request(&app).expect("body is valid");

    assert_eq!(plain.message, "add tests\n\ndetails");
}

#[test]
fn reset_commit_form_state_clears_selection_and_restores_defaults() {
    let (mut app, _task) = crate::app::App::new();
    app.staged_files_selected = vec!["a.rs".to_string()];
    app.staged_hunks_selected = vec![("a.rs".to_string(), 1)];
    app.staged_lines_selected = vec![("a.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 3)];
    app.expanded_hunks.push(("a.rs".to_string(), 1));
    app.context_expansions.insert(("a.rs".to_string(), 0), (1, 2));
    app.git_commit_message = "msg".to_string();
    app.git_commit_type = Some(ConventionalCommitType::Docs);
    app.git_commit_scope = "scope".to_string();
    app.git_commit_description = "body".to_string();
    app.git_commit_description_editor = iced::widget::text_editor::Content::with_text("body");

    super::shared::reset_commit_form_state(&mut app);

    assert!(app.staged_files_selected.is_empty());
    assert!(app.staged_hunks_selected.is_empty());
    assert!(app.staged_lines_selected.is_empty());
    assert!(app.staged_old_lines_selected.is_empty());
    assert!(app.expanded_files.is_empty());
    assert!(app.expanded_hunks.is_empty());
    assert!(app.context_expansions.is_empty());
    assert!(app.git_commit_message.is_empty());
    assert_eq!(app.git_commit_type, Some(ConventionalCommitType::Feat));
    assert!(app.git_commit_scope.is_empty());
    assert!(app.git_commit_description.is_empty());
    assert!(app.git_commit_description_editor.text().is_empty());
}

#[test]
fn text_too_large_for_code_editor_detects_limits_and_nul() {
    assert!(!super::shared::text_too_large_for_code_editor("small\ntext"));
    assert!(super::shared::text_too_large_for_code_editor(&"a".repeat(1_000_001)));
    assert!(super::shared::text_too_large_for_code_editor(&format!("{}\n", "a".repeat(20_001))));
    assert!(super::shared::text_too_large_for_code_editor("abc\0def"));
}

#[test]
fn text_too_large_for_code_editor_accepts_exact_limits_and_rejects_extra_line() {
    let exact_max_bytes =
        format!("{}{}", ("a".repeat(20_000) + "\n").repeat(49), "a".repeat(19_951));

    assert_eq!(exact_max_bytes.len(), 1_000_000);
    assert!(!super::shared::text_too_large_for_code_editor(&exact_max_bytes));
    assert!(!super::shared::text_too_large_for_code_editor(&"a".repeat(20_000)));
    assert!(!super::shared::text_too_large_for_code_editor(&"x\n".repeat(20_001)));
    assert!(super::shared::text_too_large_for_code_editor(&"x\n".repeat(20_002)));
}

#[test]
fn normalize_range_orders_start_and_end() {
    let range = super::shared::normalize_range(GitDiffLineRange {
        file: "src/lib.rs".to_string(),
        start: 9,
        end: 3,
        is_old: true,
    });

    assert_eq!(range.start, 3);
    assert_eq!(range.end, 9);
    assert!(range.is_old);
}

#[test]
fn normalize_range_preserves_an_ordered_single_line_range() {
    let range = super::shared::normalize_range(GitDiffLineRange {
        file: "src/lib.rs".to_string(),
        start: 4,
        end: 4,
        is_old: false,
    });

    assert_eq!(range.start, 4);
    assert_eq!(range.end, 4);
    assert!(!range.is_old);
}

#[test]
fn selected_lines_from_range_returns_empty_without_diff_content() {
    let (app, _task) = crate::app::App::new();
    let range =
        GitDiffLineRange { file: "missing.rs".to_string(), start: 1, end: 3, is_old: false };

    assert!(super::shared::selected_lines_from_range(&app, &range).is_empty());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn diff_context_target_range_prefers_menu_inside_selected_range() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 8, end: 4, is_old: false });
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 6,
        is_old: false,
        x: 1.0,
        y: 2.0,
    });

    let range = super::shared::diff_context_target_range(&app).expect("range should resolve");

    assert_eq!(range.file, "src/lib.rs");
    assert_eq!(range.start, 4);
    assert_eq!(range.end, 8);
    assert!(!range.is_old);
}

#[test]
fn diff_context_target_range_falls_back_to_menu_line() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "other.rs".to_string(), start: 1, end: 3, is_old: false });
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 6,
        is_old: true,
        x: 1.0,
        y: 2.0,
    });

    let range = super::shared::diff_context_target_range(&app).expect("range should resolve");

    assert_eq!(range.file, "src/lib.rs");
    assert_eq!(range.start, 6);
    assert_eq!(range.end, 6);
    assert!(range.is_old);
}

#[test]
fn diff_context_target_range_uses_selection_then_selected_line_then_none() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "range.rs".to_string(), start: 7, end: 3, is_old: false });

    let range = super::shared::diff_context_target_range(&app).expect("range should resolve");
    assert_eq!((range.file.as_str(), range.start, range.end), ("range.rs", 3, 7));

    app.git_diff_selected_range = None;
    app.git_diff_selected_lines = vec![GitDiffSelectedLine {
        file: "line.rs".to_string(),
        line: 9,
        is_old: true,
        text: "removed".to_string(),
    }];

    let line = super::shared::diff_context_target_range(&app).expect("line should resolve");
    assert_eq!((line.file.as_str(), line.start, line.end, line.is_old), ("line.rs", 9, 9, true));

    app.git_diff_selected_lines.clear();
    assert!(super::shared::diff_context_target_range(&app).is_none());
}

#[test]
fn diff_context_target_stage_lines_deduplicates_selected_lines() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 5,
            is_old: false,
            text: "a".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 5,
            is_old: false,
            text: "a".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 2,
            is_old: true,
            text: "b".to_string(),
        },
    ];

    let lines = super::shared::diff_context_target_stage_lines(&app);

    assert_eq!(
        lines,
        vec![("src/lib.rs".to_string(), 2, true), ("src/lib.rs".to_string(), 5, false)]
    );
}

#[test]
fn diff_context_target_stage_lines_uses_range_containing_menu_line() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_range =
        Some(GitDiffLineRange { file: "src/lib.rs".to_string(), start: 5, end: 3, is_old: true });
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 4,
        is_old: true,
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(
        super::shared::diff_context_target_stage_lines(&app),
        vec![
            ("src/lib.rs".to_string(), 3, true),
            ("src/lib.rs".to_string(), 4, true),
            ("src/lib.rs".to_string(), 5, true),
        ]
    );
}

#[test]
fn diff_context_target_stage_lines_filters_menu_side_or_uses_menu_only() {
    let (mut app, _task) = crate::app::App::new();
    app.git_diff_selected_lines = vec![
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 6,
            is_old: false,
            text: "selected".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: false,
            text: "selected".to_string(),
        },
        GitDiffSelectedLine {
            file: "src/lib.rs".to_string(),
            line: 4,
            is_old: true,
            text: "other side".to_string(),
        },
    ];
    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "src/lib.rs".to_string(),
        line: 4,
        is_old: false,
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(
        super::shared::diff_context_target_stage_lines(&app),
        vec![("src/lib.rs".to_string(), 4, false), ("src/lib.rs".to_string(), 6, false)]
    );

    app.git_diff_context_menu = Some(GitDiffContextMenuState {
        file: "other.rs".to_string(),
        line: 2,
        is_old: true,
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(
        super::shared::diff_context_target_stage_lines(&app),
        vec![("other.rs".to_string(), 2, true)]
    );
}

#[test]
fn extend_and_clear_stage_selection_for_lines_are_idempotent() {
    let (mut app, _task) = crate::app::App::new();
    let lines = vec![
        ("src/lib.rs".to_string(), 2, false),
        ("src/lib.rs".to_string(), 2, false),
        ("src/lib.rs".to_string(), 1, true),
    ];

    super::shared::extend_stage_selection_for_lines(&mut app, &lines);

    assert_eq!(app.staged_lines_selected, vec![("src/lib.rs".to_string(), 2)]);
    assert_eq!(app.staged_old_lines_selected, vec![("src/lib.rs".to_string(), 1)]);

    super::shared::clear_stage_selection_for_lines(
        &mut app,
        &[("src/lib.rs".to_string(), 2, false)],
    );

    assert!(app.staged_lines_selected.is_empty());
    assert_eq!(app.staged_old_lines_selected, vec![("src/lib.rs".to_string(), 1)]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn normalize_stage_line_selections_sorts_and_deduplicates_both_sides() {
    let (mut app, _task) = crate::app::App::new();
    app.staged_lines_selected =
        vec![("b.rs".to_string(), 3), ("a.rs".to_string(), 2), ("a.rs".to_string(), 2)];
    app.staged_old_lines_selected = vec![("a.rs".to_string(), 4), ("a.rs".to_string(), 1)];

    super::shared::normalize_stage_line_selections(&mut app);

    assert_eq!(app.staged_lines_selected, vec![("a.rs".to_string(), 2), ("b.rs".to_string(), 3)]);
    assert_eq!(
        app.staged_old_lines_selected,
        vec![("a.rs".to_string(), 1), ("a.rs".to_string(), 4)]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn replace_stage_selection_for_file_lines_replaces_only_target_file() {
    let (mut app, _task) = crate::app::App::new();
    app.staged_lines_selected = vec![("target.rs".to_string(), 1), ("keep.rs".to_string(), 8)];
    app.staged_old_lines_selected = vec![("target.rs".to_string(), 2), ("keep.rs".to_string(), 7)];
    let old_changed = HashSet::from([3, 5]);
    let new_changed = HashSet::from([4, 6]);

    super::shared::replace_stage_selection_for_file_lines(
        &mut app,
        "target.rs",
        &old_changed,
        &new_changed,
    );

    assert!(app.staged_lines_selected.contains(&("keep.rs".to_string(), 8)));
    assert!(app.staged_lines_selected.contains(&("target.rs".to_string(), 4)));
    assert!(app.staged_lines_selected.contains(&("target.rs".to_string(), 6)));
    assert_eq!(app.staged_lines_selected.len(), 3);
    assert!(app.staged_old_lines_selected.contains(&("keep.rs".to_string(), 7)));
    assert!(app.staged_old_lines_selected.contains(&("target.rs".to_string(), 3)));
    assert!(app.staged_old_lines_selected.contains(&("target.rs".to_string(), 5)));
    assert_eq!(app.staged_old_lines_selected.len(), 3);
}

#[cfg(not(target_arch = "wasm32"))]
// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn invert_stage_selection_for_file_lines_toggles_changed_lines_only() {
    let (mut app, _task) = crate::app::App::new();
    app.staged_lines_selected = vec![("target.rs".to_string(), 1), ("keep.rs".to_string(), 8)];
    app.staged_old_lines_selected = vec![("target.rs".to_string(), 2), ("keep.rs".to_string(), 7)];
    let old_changed = HashSet::from([2, 3]);
    let new_changed = HashSet::from([1, 4]);

    super::shared::invert_stage_selection_for_file_lines(
        &mut app,
        "target.rs",
        &old_changed,
        &new_changed,
    );

    assert!(!app.staged_lines_selected.contains(&("target.rs".to_string(), 1)));
    assert!(app.staged_lines_selected.contains(&("target.rs".to_string(), 4)));
    assert!(app.staged_lines_selected.contains(&("keep.rs".to_string(), 8)));
    assert!(!app.staged_old_lines_selected.contains(&("target.rs".to_string(), 2)));
    assert!(app.staged_old_lines_selected.contains(&("target.rs".to_string(), 3)));
    assert!(app.staged_old_lines_selected.contains(&("keep.rs".to_string(), 7)));
}
