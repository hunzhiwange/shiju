#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::super::*;
    use crate::app::Shell;
    use git2::{IndexAddOption, Repository, Signature};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    const SOURCE: &str = include_str!("git.rs");

    fn source_declares_symbol(name: &str) -> bool {
        let needles = [
            format!("fn {name}"),
            format!("pub fn {name}"),
            format!("struct {name}"),
            format!("pub struct {name}"),
            format!("enum {name}"),
            format!("pub enum {name}"),
            format!("type {name}"),
            format!("pub type {name}"),
            format!("const {name}"),
            format!("pub const {name}"),
            format!("static {name}"),
            format!("pub static {name}"),
            format!("impl {name}"),
        ];

        needles.iter().any(|needle| SOURCE.contains(needle))
    }

    fn repo_path(dir: &TempDir) -> &str {
        dir.path().to_str().expect("temp path should be utf-8")
    }

    fn write_file(repo_dir: &Path, file: &str, content: &str) {
        let path = repo_dir.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(path, content).expect("write test file");
    }

    fn init_repo() -> (TempDir, Repository) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let repo = Repository::init(dir.path()).expect("init git repo");
        {
            let mut config = repo.config().expect("open git config");
            config.set_str("user.name", "Vibe Window").expect("set user name");
            config.set_str("user.email", "vibe@example.test").expect("set user email");
        }
        (dir, repo)
    }

    fn commit_all(repo: &Repository, message: &str) {
        let mut index = repo.index().expect("open index");
        index.add_all(["*"], IndexAddOption::DEFAULT, None).expect("add all files");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = Signature::now("Vibe Window", "vibe@example.test").expect("signature");
        let parents = if let Ok(head) = repo.head() {
            vec![head.peel_to_commit().expect("head commit")]
        } else {
            Vec::new()
        };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).expect("commit");
    }

    fn index_contains(repo: &Repository, file: &str, content: &str) -> bool {
        let repo = Repository::open(repo.workdir().expect("repo should have workdir"))
            .expect("reopen test repo");
        let index = repo.index().expect("open index");
        let oid = index.get_path(Path::new(file), 0).expect("index entry").id;
        let blob = repo.find_blob(oid).expect("find indexed blob");
        std::str::from_utf8(blob.content()).expect("utf-8 blob") == content
    }

    #[test]
    fn git_tests_keeps_planned_coverage_targets() {
        for name in [
            "DIFF_CONTEXT",
            "git_stage_file",
            "sh_quote",
            "git_commit",
            "git_commit_with_body",
            "git_log",
            "git_discard_file",
            "git_diff_for_file",
            "get_file_content_pair",
            "git_discard_hunk",
            "git_stage_hunk",
            "git_stage_line_insert",
            "git_stage_line_delete",
            "git_revert_line_delete",
            "git_revert_line_restore",
            "run_shell_command",
        ] {
            assert!(
                source_declares_symbol(name),
                "expected source to declare coverage target {name}"
            );
        }
    }

    #[test]
    fn git_stage_file_quotes_paths_and_stages_new_file() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "quote's file.txt", "fresh\n");

        git_stage_file(repo_path(&dir), "quote's file.txt", Shell::Bash).expect("stage file");

        assert!(index_contains(&repo, "quote's file.txt", "fresh\n"));
    }

    #[test]
    fn git_commit_variants_create_log_entries_with_quoted_messages() {
        let (dir, _) = init_repo();
        write_file(dir.path(), "a.txt", "one\n");
        git_stage_file(repo_path(&dir), "a.txt", Shell::Bash).expect("stage file");

        git_commit(repo_path(&dir), "feat: it's quoted", Shell::Bash).expect("commit");
        write_file(dir.path(), "a.txt", "one\ntwo\n");
        git_stage_file(repo_path(&dir), "a.txt", Shell::Bash).expect("stage update");
        git_commit_with_body(
            repo_path(&dir),
            "docs: add body",
            "body keeps it's quotes",
            Shell::Bash,
        )
        .expect("commit with body");

        let log = git_log(repo_path(&dir), 2);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].1, "docs: add body");
        assert_eq!(log[1].1, "feat: it's quoted");
    }

    #[test]
    fn git_commit_with_empty_body_uses_summary_only() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "a.txt", "one\n");
        git_stage_file(repo_path(&dir), "a.txt", Shell::Zsh).expect("stage file");

        git_commit_with_body(repo_path(&dir), "summary only", "", Shell::Zsh)
            .expect("commit with empty body");

        let head = repo.head().expect("head").peel_to_commit().expect("head commit");
        assert_eq!(head.message(), Ok("summary only"));
        assert_eq!(head.parent_count(), 0);
    }

    #[test]
    fn git_commit_variants_reject_invalid_repository() {
        let dir = tempfile::tempdir().expect("create temp dir");

        assert!(git_commit(repo_path(&dir), "summary", Shell::Bash).is_err());
        assert!(git_commit_with_body(repo_path(&dir), "summary", "body", Shell::Bash).is_err());
    }

    #[test]
    fn git_log_returns_empty_for_repo_without_commits() {
        let (dir, _) = init_repo();

        assert!(git_log(repo_path(&dir), 3).is_empty());
    }

    #[test]
    fn git_log_respects_limit_and_preserves_pipe_in_subject() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "a.txt", "one\n");
        commit_all(&repo, "first|subject");
        write_file(dir.path(), "a.txt", "one\ntwo\n");
        commit_all(&repo, "second|subject");

        let log = git_log(repo_path(&dir), 1);

        assert_eq!(log.len(), 1);
        assert_eq!(log[0].1, "second|subject");
    }

    #[test]
    fn git_diff_for_file_reports_modify_create_delete_and_none() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "tracked.txt", "old\nsame\n");
        write_file(dir.path(), "delete.txt", "gone\n");
        commit_all(&repo, "initial");

        assert!(git_diff_for_file(repo_path(&dir), "tracked.txt").is_none());

        write_file(dir.path(), "tracked.txt", "new\nsame\n");
        let changed = git_diff_for_file(repo_path(&dir), "tracked.txt").expect("changed diff");
        assert!(changed.contains("-old"));
        assert!(changed.contains("+new"));

        write_file(dir.path(), "created.txt", "fresh\n");
        let created = git_diff_for_file(repo_path(&dir), "created.txt").expect("created diff");
        assert!(created.contains("+fresh"));

        fs::remove_file(dir.path().join("delete.txt")).expect("delete file");
        let deleted = git_diff_for_file(repo_path(&dir), "delete.txt").expect("deleted diff");
        assert!(deleted.contains("-gone"));
    }

    #[test]
    fn git_diff_for_file_returns_none_for_invalid_repository_and_missing_file() {
        let invalid = tempfile::tempdir().expect("create temp dir");
        assert!(git_diff_for_file(repo_path(&invalid), "missing.txt").is_none());

        let (dir, repo) = init_repo();
        write_file(dir.path(), "tracked.txt", "tracked\n");
        commit_all(&repo, "initial");

        assert!(git_diff_for_file(repo_path(&dir), "missing.txt").is_none());
    }

    #[test]
    fn get_file_content_pair_reads_head_and_worktree_for_existing_and_new_files() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "tracked.txt", "old\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "tracked.txt", "new\n");
        write_file(dir.path(), "new.txt", "fresh\n");

        assert_eq!(
            get_file_content_pair(repo_path(&dir), "tracked.txt"),
            Some(("old\n".to_string(), "new\n".to_string()))
        );
        assert_eq!(
            get_file_content_pair(repo_path(&dir), "new.txt"),
            Some((String::new(), "fresh\n".to_string()))
        );
    }

    #[test]
    fn get_file_content_pair_reads_deleted_file_and_rejects_invalid_repository() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "deleted.txt", "old\n");
        commit_all(&repo, "initial");
        fs::remove_file(dir.path().join("deleted.txt")).expect("delete worktree file");

        assert_eq!(
            get_file_content_pair(repo_path(&dir), "deleted.txt"),
            Some(("old\n".to_string(), String::new()))
        );

        let invalid = tempfile::tempdir().expect("create temp dir");
        assert_eq!(get_file_content_pair(repo_path(&invalid), "missing.txt"), None);
    }

    #[test]
    fn git_discard_file_restores_modified_file_and_removes_new_files() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "tracked.txt", "clean\n");
        commit_all(&repo, "initial");

        write_file(dir.path(), "tracked.txt", "dirty\n");
        git_discard_file(repo_path(&dir), "tracked.txt").expect("discard tracked file");
        assert_eq!(
            fs::read_to_string(dir.path().join("tracked.txt")).expect("read tracked"),
            "clean\n"
        );

        write_file(dir.path(), "untracked.txt", "temp\n");
        git_discard_file(repo_path(&dir), "untracked.txt").expect("discard untracked");
        assert!(!dir.path().join("untracked.txt").exists());

        write_file(dir.path(), "indexed.txt", "temp\n");
        git_stage_file(repo_path(&dir), "indexed.txt", Shell::Bash).expect("stage indexed");
        git_discard_file(repo_path(&dir), "indexed.txt").expect("discard indexed");
        assert!(!dir.path().join("indexed.txt").exists());
        assert!(repo.index().expect("open index").get_path(Path::new("indexed.txt"), 0).is_none());
    }

    #[test]
    fn git_discard_file_errors_for_invalid_repository() {
        let dir = tempfile::tempdir().expect("create temp dir");

        let err = git_discard_file(repo_path(&dir), "missing.txt").expect_err("invalid repo");

        assert!(err.contains("repository") || err.contains("not found"));
    }

    #[test]
    fn git_stage_hunk_stages_selected_change_and_rejects_bad_index() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\ntwo changed\nthree\n");

        assert_eq!(
            git_stage_hunk(repo_path(&dir), "lines.txt", 99, Shell::Bash),
            Err("bad hunk index".to_string())
        );
        git_stage_hunk(repo_path(&dir), "lines.txt", 0, Shell::Bash).expect("stage hunk");

        assert!(index_contains(&repo, "lines.txt", "one\ntwo changed\nthree\n"));
    }

    #[test]
    fn git_stage_hunk_stages_only_selected_change_and_preserves_worktree() {
        let (dir, repo) = init_repo();
        let old = "top\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nbottom\n";
        let changed = "TOP\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nBOTTOM\n";
        write_file(dir.path(), "lines.txt", old);
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", changed);

        git_stage_hunk(repo_path(&dir), "lines.txt", 1, Shell::Bash).expect("stage second hunk");

        assert!(index_contains(&repo, "lines.txt", "top\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\nBOTTOM\n"));
        assert_eq!(
            fs::read_to_string(dir.path().join("lines.txt")).expect("read worktree"),
            changed
        );
    }

    #[test]
    fn git_stage_hunk_applies_insert_and_delete_operations() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "zero\none\nthree\nfour\n");

        git_stage_hunk(repo_path(&dir), "lines.txt", 0, Shell::Bash).expect("stage mixed hunk");

        assert!(index_contains(&repo, "lines.txt", "zero\none\nthree\nfour\n"));
    }

    #[test]
    fn git_discard_hunk_restores_selected_change_and_rejects_bad_index() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\ntwo changed\nthree\n");

        assert_eq!(
            git_discard_hunk(repo_path(&dir), "lines.txt", 99, Shell::Bash),
            Err("bad hunk index".to_string())
        );
        git_discard_hunk(repo_path(&dir), "lines.txt", 0, Shell::Bash).expect("discard hunk");

        assert_eq!(
            fs::read_to_string(dir.path().join("lines.txt")).expect("read lines"),
            "one\ntwo\nthree\n"
        );
    }

    #[test]
    fn git_discard_hunk_handles_new_and_deleted_files() {
        let (new_dir, new_repo) = init_repo();
        write_file(new_dir.path(), "tracked.txt", "base\n");
        commit_all(&new_repo, "initial");
        write_file(new_dir.path(), "created.txt", "fresh\n");

        git_discard_hunk(repo_path(&new_dir), "created.txt", 0, Shell::Bash)
            .expect("discard new-file hunk");
        assert!(!new_dir.path().join("created.txt").exists());

        let (deleted_dir, deleted_repo) = init_repo();
        write_file(deleted_dir.path(), "deleted.txt", "gone\n");
        commit_all(&deleted_repo, "initial");
        fs::remove_file(deleted_dir.path().join("deleted.txt")).expect("delete tracked file");

        git_discard_hunk(repo_path(&deleted_dir), "deleted.txt", 0, Shell::Bash)
            .expect("discard deleted-file hunk");
        assert_eq!(
            fs::read_to_string(deleted_dir.path().join("deleted.txt"))
                .expect("read restored deleted file"),
            "gone\n"
        );
    }

    #[test]
    fn git_stage_line_insert_stages_inserted_line_and_errors_without_anchor() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");

        git_stage_line_insert(repo_path(&dir), "lines.txt", 1, Shell::Bash).expect("stage insert");
        assert!(index_contains(&repo, "lines.txt", "one\ntwo\nthree\n"));

        let err = git_stage_line_insert(repo_path(&dir), "lines.txt", 50, Shell::Bash)
            .expect_err("missing insertion anchor");
        assert_eq!(err, "Cannot locate insertion anchor");
    }

    #[test]
    fn git_stage_line_insert_handles_start_and_end_boundaries() {
        let (start_dir, start_repo) = init_repo();
        write_file(start_dir.path(), "lines.txt", "middle\n");
        commit_all(&start_repo, "initial");
        write_file(start_dir.path(), "lines.txt", "start\nmiddle\n");

        git_stage_line_insert(repo_path(&start_dir), "lines.txt", 0, Shell::Bash)
            .expect("stage first line");
        assert!(index_contains(&start_repo, "lines.txt", "start\nmiddle\n"));

        let (end_dir, end_repo) = init_repo();
        write_file(end_dir.path(), "lines.txt", "middle\n");
        commit_all(&end_repo, "initial");
        write_file(end_dir.path(), "lines.txt", "middle\nend\n");

        git_stage_line_insert(repo_path(&end_dir), "lines.txt", 1, Shell::Bash)
            .expect("stage last line");
        assert!(index_contains(&end_repo, "lines.txt", "middle\nend\n"));
    }

    #[test]
    fn git_stage_line_delete_stages_deleted_line_and_rejects_out_of_bounds_index() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\nthree\n");

        assert_eq!(
            git_stage_line_delete(repo_path(&dir), "lines.txt", 99, Shell::Bash),
            Err("Old line index out of bounds".to_string())
        );
        git_stage_line_delete(repo_path(&dir), "lines.txt", 1, Shell::Bash).expect("stage delete");

        assert!(index_contains(&repo, "lines.txt", "one\nthree\n"));
    }

    #[test]
    fn git_stage_line_delete_handles_first_and_last_line_boundaries() {
        let (first_dir, first_repo) = init_repo();
        write_file(first_dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&first_repo, "initial");
        write_file(first_dir.path(), "lines.txt", "two\nthree\n");

        git_stage_line_delete(repo_path(&first_dir), "lines.txt", 0, Shell::Bash)
            .expect("stage first-line deletion");
        assert!(index_contains(&first_repo, "lines.txt", "two\nthree\n"));

        let (last_dir, last_repo) = init_repo();
        write_file(last_dir.path(), "lines.txt", "one\ntwo\nthree");
        commit_all(&last_repo, "initial");
        write_file(last_dir.path(), "lines.txt", "one\ntwo");

        git_stage_line_delete(repo_path(&last_dir), "lines.txt", 2, Shell::Bash)
            .expect("stage last-line deletion");
        assert!(index_contains(&last_repo, "lines.txt", "one\ntwo"));
    }

    #[test]
    fn hunk_and_line_operations_report_read_errors_for_invalid_repositories() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = repo_path(&dir);

        assert_eq!(
            git_discard_hunk(path, "missing.txt", 0, Shell::Bash),
            Err("Failed to read file content".to_string())
        );
        assert_eq!(
            git_stage_hunk(path, "missing.txt", 0, Shell::Bash),
            Err("Failed to read file content".to_string())
        );
        assert_eq!(
            git_stage_line_insert(path, "missing.txt", 0, Shell::Bash),
            Err("Failed to read file".to_string())
        );
        assert_eq!(
            git_stage_line_delete(path, "missing.txt", 0, Shell::Bash),
            Err("Failed to read file".to_string())
        );
        assert_eq!(
            git_revert_line_restore(path, "missing.txt", 0, 0),
            Err("Failed to read file".to_string())
        );
    }

    #[test]
    fn lines_to_content_matches_reference_line_ending() {
        assert_eq!(
            lines_to_content(vec!["one".to_string(), "two".to_string()], "reference\n"),
            "one\ntwo\n"
        );
        assert_eq!(
            lines_to_content(vec!["one".to_string(), "two".to_string()], "reference"),
            "one\ntwo"
        );
        assert_eq!(lines_to_content(Vec::new(), "reference\n"), "");
    }

    #[test]
    fn write_index_content_updates_only_index_and_rejects_missing_entry() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "tracked.txt", "worktree\n");
        commit_all(&repo, "initial");

        write_index_content(repo_path(&dir), "tracked.txt", "staged\n".to_string())
            .expect("write index content");

        assert!(index_contains(&repo, "tracked.txt", "staged\n"));
        assert_eq!(
            fs::read_to_string(dir.path().join("tracked.txt")).expect("read worktree"),
            "worktree\n"
        );
        assert_eq!(
            write_index_content(repo_path(&dir), "missing.txt", "missing\n".to_string()),
            Err("index entry not found".to_string())
        );
    }

    #[test]
    fn git_revert_line_delete_removes_selected_worktree_line() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");

        assert_eq!(
            git_revert_line_delete(repo_path(&dir), "lines.txt", 99),
            Err("Line index out of bounds".to_string())
        );
        git_revert_line_delete(repo_path(&dir), "lines.txt", 1).expect("remove inserted line");

        assert_eq!(
            fs::read_to_string(dir.path().join("lines.txt")).expect("read lines"),
            "one\nthree\n"
        );
    }

    #[test]
    fn git_revert_line_delete_handles_only_line_and_missing_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_file(dir.path(), "only.txt", "only");

        git_revert_line_delete(repo_path(&dir), "only.txt", 0).expect("remove only line");

        assert_eq!(fs::read_to_string(dir.path().join("only.txt")).expect("read file"), "");
        assert!(git_revert_line_delete(repo_path(&dir), "missing.txt", 0).is_err());
    }

    #[test]
    fn git_revert_line_restore_inserts_head_line_or_appends_when_out_of_range() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\nthree\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "one\nthree\n");

        assert_eq!(
            git_revert_line_restore(repo_path(&dir), "lines.txt", 0, 99),
            Err("Old line index out of bounds".to_string())
        );
        git_revert_line_restore(repo_path(&dir), "lines.txt", 1, 1).expect("restore middle line");
        assert_eq!(
            fs::read_to_string(dir.path().join("lines.txt")).expect("read lines"),
            "one\ntwo\nthree\n"
        );

        git_revert_line_restore(repo_path(&dir), "lines.txt", 99, 0).expect("append old line");
        assert_eq!(
            fs::read_to_string(dir.path().join("lines.txt")).expect("read lines"),
            "one\ntwo\nthree\none\n"
        );
    }

    #[test]
    fn git_revert_line_restore_inserts_into_empty_worktree() {
        let (dir, repo) = init_repo();
        write_file(dir.path(), "lines.txt", "one\ntwo\n");
        commit_all(&repo, "initial");
        write_file(dir.path(), "lines.txt", "");

        git_revert_line_restore(repo_path(&dir), "lines.txt", 0, 1)
            .expect("restore into empty file");

        assert_eq!(fs::read_to_string(dir.path().join("lines.txt")).expect("read lines"), "two\n");
    }

    #[test]
    fn run_shell_command_returns_stdout_and_stderr_and_normalizes_line_endings() {
        let out = run_shell_command(
            None,
            "printf 'out\\r\\n'; printf 'err\\r\\n' >&2".to_string(),
            Shell::Bash,
        );

        assert!(out.contains("out\n"));
        assert!(out.contains("err\n"));
        assert!(!out.contains('\r'));
    }

    #[test]
    fn run_shell_command_uses_requested_working_directory_and_zsh() {
        let dir = tempfile::tempdir().expect("create temp dir");
        write_file(dir.path(), "marker.txt", "marker\n");

        let out = run_shell_command(
            Some(repo_path(&dir).to_string()),
            "test -f marker.txt && printf 'cwd-ok'".to_string(),
            Shell::Zsh,
        );

        assert!(out.contains("cwd-ok"));
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::super::*;
    use crate::app::Shell;

    #[test]
    fn wasm_git_operations_return_explicit_unsupported_results() {
        assert_eq!(git_discard_file("/repo", "file.txt"), Err("Not supported on Web".to_string()));
        assert_eq!(git_diff_for_file("/repo", "file.txt"), None);
        assert_eq!(
            git_discard_hunk("/repo", "file.txt", 0, Shell::Bash),
            Err("Not supported on Web".to_string())
        );
        assert_eq!(
            git_revert_line_delete("/repo", "file.txt", 0),
            Err("Not supported on Web".to_string())
        );
        assert_eq!(
            git_revert_line_restore("/repo", "file.txt", 0, 0),
            Err("Not supported on Web".to_string())
        );
    }
}
