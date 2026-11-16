// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::git_panel::types::ConventionalCommitType;
#[test]
fn conventional_commit_types_round_trip_every_prefix_and_display_in_declared_order() {
    let expected = [
        (ConventionalCommitType::Feat, "feat"),
        (ConventionalCommitType::Fix, "fix"),
        (ConventionalCommitType::Docs, "docs"),
        (ConventionalCommitType::Style, "style"),
        (ConventionalCommitType::Refactor, "refactor"),
        (ConventionalCommitType::Perf, "perf"),
        (ConventionalCommitType::Test, "test"),
        (ConventionalCommitType::Build, "build"),
        (ConventionalCommitType::Ci, "ci"),
        (ConventionalCommitType::Chore, "chore"),
        (ConventionalCommitType::Revert, "revert"),
        (ConventionalCommitType::Init, "init"),
        (ConventionalCommitType::Config, "config"),
        (ConventionalCommitType::Release, "release"),
        (ConventionalCommitType::Deploy, "deploy"),
        (ConventionalCommitType::Merge, "merge"),
        (ConventionalCommitType::Wip, "wip"),
        (ConventionalCommitType::Typo, "typo"),
        (ConventionalCommitType::Locale, "locale"),
    ];

    assert_eq!(ConventionalCommitType::all(), expected.map(|(kind, _)| kind));
    for (kind, prefix) in expected {
        assert_eq!(kind.as_str(), prefix);
        assert_eq!(kind.to_string(), prefix);
        assert_eq!(ConventionalCommitType::from_prefix(prefix), Some(kind));
    }
    assert_eq!(ConventionalCommitType::from_prefix("unknown"), None);
    assert_eq!(ConventionalCommitType::from_prefix("FEAT"), None);
    assert_eq!(ConventionalCommitType::from_prefix(" feat"), None);
}
}

mod behavior_1 {
use crate::git_panel::types::GitWorktreeOption;
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
}

mod behavior_2 {
use crate::git_panel::diff_view::{GitDiffLineRange, GitDiffContextMenuState};
#[test]
fn workspace_structs_keep_selected_git_and_find_state_0() {
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


}
}
