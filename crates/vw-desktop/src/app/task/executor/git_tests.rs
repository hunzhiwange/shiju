#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("git_tests"));
}

fn run_git_test(cwd: &std::path::Path, args: &[&str]) {
    let output = vw_shared::shell::git_std_command()
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command should start");
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_repo() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo dir should be created");
    run_git_test(&repo, &["init", "--initial-branch=main"]);
    run_git_test(&repo, &["config", "user.name", "VibeWindow Test"]);
    run_git_test(&repo, &["config", "user.email", "test@vibewindow.local"]);
    std::fs::write(repo.join("README.md"), "seed\n").expect("seed file should be written");
    run_git_test(&repo, &["add", "."]);
    run_git_test(&repo, &["commit", "-m", "init"]);
    (temp, repo)
}

#[test]
fn git_command_status_accessors_return_stored_values() {
    let status = super::GitCommandStatus { success: true, code: Some(7) };

    assert!(status.success());
    assert_eq!(status.code(), Some(7));
}

#[test]
fn git_command_output_converts_from_gateway_response() {
    let response = vw_gateway_client::vw_api_types::git::GitCommandResponse {
        success: false,
        code: Some(128),
        stdout: "out".to_string(),
        stderr: "err".to_string(),
    };

    let output = super::GitCommandOutput::from(response);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(128));
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, b"err");
}

#[test]
fn benign_abort_failures_match_known_git_messages() {
    let merge = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(128) },
        stdout: Vec::new(),
        stderr: b"fatal: There is no merge to abort".to_vec(),
    };
    assert!(super::is_benign_abort_failure(&["merge", "--abort"], &merge));

    let rebase = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(1) },
        stdout: Vec::new(),
        stderr: b"No rebase in progress?".to_vec(),
    };
    assert!(super::is_benign_abort_failure(&["rebase", "--abort"], &rebase));

    let cherry_pick = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(128) },
        stdout: Vec::new(),
        stderr: b"cherry-pick is not in progress".to_vec(),
    };
    assert!(super::is_benign_abort_failure(&["cherry-pick", "--abort"], &cherry_pick));

    let unknown = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(2) },
        stdout: Vec::new(),
        stderr: b"fatal".to_vec(),
    };
    assert!(!super::is_benign_abort_failure(&["merge", "--abort"], &unknown));
    assert!(!super::is_benign_abort_failure(&["status"], &merge));
}

#[test]
fn benign_abort_failures_accept_localized_messages_and_reject_unrelated_failures() {
    for (args, stderr) in [
        (["merge", "--abort"], "fatal: 没有要终止的合并"),
        (["rebase", "--abort"], "fatal: 没有正在进行的变基"),
        (["cherry-pick", "--abort"], "错误：拣选或还原操作并未进行"),
    ] {
        let output = super::GitCommandOutput {
            status: super::GitCommandStatus { success: false, code: Some(128) },
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        };

        assert!(super::is_benign_abort_failure(&args, &output));
    }

    let unrelated = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(128) },
        stdout: Vec::new(),
        stderr: b"fatal: repository is corrupt".to_vec(),
    };
    let missing_code = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: None },
        stdout: Vec::new(),
        stderr: b"There is no merge to abort".to_vec(),
    };

    assert!(!super::is_benign_abort_failure(&["merge", "--abort"], &unrelated));
    assert!(!super::is_benign_abort_failure(&["merge", "--abort"], &missing_code));
}

#[test]
fn git_maintenance_timeout_uses_state_constant() {
    assert_eq!(
        super::git_maintenance_timeout(),
        std::time::Duration::from_secs(super::GIT_MAINTENANCE_COMMAND_TIMEOUT_SECS)
    );
}

#[test]
fn run_git_reads_branch_and_repo_root_from_temp_repo() {
    let (_temp, repo) = create_repo();
    let repo_path = repo.to_string_lossy().to_string();

    let branch = super::git_branch_name(&repo_path);
    let root = super::git_repo_root(&repo_path);

    assert_eq!(branch.as_deref(), Some("main"));
    let expected_root = std::fs::canonicalize(&repo_path).expect("repo path should canonicalize");
    assert_eq!(root.as_deref(), Some(expected_root.to_string_lossy().as_ref()));
}

#[test]
fn git_branch_and_repo_root_reject_non_repository_directory() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let path = temp.path().to_string_lossy();

    assert_eq!(super::git_branch_name(path.as_ref()), None);
    assert_eq!(super::git_repo_root(path.as_ref()), None);
}

#[test]
fn run_git_local_reports_missing_working_directory() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let missing = temp.path().join("missing");

    let error = super::run_git_local(missing.to_string_lossy().as_ref(), &["status"])
        .expect_err("missing working directory should fail before git starts");

    assert!(error.contains("本地 git 执行失败"));
    assert!(error.contains("status"));
}

#[test]
fn git_clean_and_staged_change_detection_follow_repo_state() {
    let (_temp, repo) = create_repo();

    assert_eq!(super::git_is_clean(repo.to_string_lossy().as_ref()), Ok(true));
    assert_eq!(super::git_has_staged_changes(repo.to_string_lossy().as_ref()), Ok(false));

    std::fs::write(repo.join("README.md"), "seed\nchanged\n").expect("file should update");
    assert_eq!(super::git_is_clean(repo.to_string_lossy().as_ref()), Ok(false));
    assert_eq!(super::git_has_staged_changes(repo.to_string_lossy().as_ref()), Ok(false));

    run_git_test(&repo, &["add", "README.md"]);
    assert_eq!(super::git_has_staged_changes(repo.to_string_lossy().as_ref()), Ok(true));
}

#[test]
fn git_state_queries_report_non_repository_failures() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let path = temp.path().to_string_lossy();

    assert_eq!(super::git_is_clean(path.as_ref()), Err("git status 返回失败".to_string()));
    let staged_error = super::git_has_staged_changes(path.as_ref())
        .expect_err("staged query outside a repository should fail");
    assert!(staged_error.contains("git diff --cached --quiet 失败 code="));
}

#[test]
fn run_git_logged_emits_command_stdout_stderr_and_exit() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let output =
        super::run_git_logged(Some(&tx), repo.to_string_lossy().as_ref(), &["status", "--short"])
            .expect("git status should run");

    assert!(output.status.success());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("[GATEWAY GIT]"))
    ));
    assert!(logs.iter().any(|log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("[GATEWAY GIT EXIT] success=true"))));
}

#[test]
fn run_git_logged_emits_non_empty_stdout() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let output = super::run_git_logged(
        Some(&tx),
        repo.to_string_lossy().as_ref(),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    )
    .expect("git rev-parse should run");

    assert!(output.status.success());
    assert!(rx.try_iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value == "[GATEWAY GIT STDOUT] main\n")
    ));
}

#[test]
fn run_git_logged_emits_stderr_and_failed_exit() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let output = super::run_git_logged(
        Some(&tx),
        repo.to_string_lossy().as_ref(),
        &["rev-parse", "--verify", "missing-ref"],
    )
    .expect("git process should start even when the command fails");

    assert!(!output.status.success());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stderr(value) if value.starts_with("[GATEWAY GIT STDERR] "))
    ));
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("[GATEWAY GIT EXIT] success=false"))
    ));
}

#[test]
fn run_git_logged_with_timeout_emits_elapsed_exit_log() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let (output, _elapsed) = super::run_git_logged_with_timeout(
        Some(&tx),
        repo.to_string_lossy().as_ref(),
        &["status", "--short"],
        std::time::Duration::from_secs(5),
    )
    .expect("git status should run");

    assert!(output.status.success());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("timeout=5s"))
    ));
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("elapsed_ms="))
    ));
}

#[test]
fn run_git_logged_with_timeout_emits_stderr_and_failed_exit() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let (output, _elapsed) = super::run_git_logged_with_timeout(
        Some(&tx),
        repo.to_string_lossy().as_ref(),
        &["rev-parse", "--verify", "missing-ref"],
        std::time::Duration::from_secs(5),
    )
    .expect("git process should start even when the command fails");

    assert!(!output.status.success());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stderr(value) if value.starts_with("[GATEWAY GIT STDERR] "))
    ));
    assert!(logs.iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("[GATEWAY GIT EXIT] success=false"))
    ));
}

#[test]
fn run_git_maintenance_logged_returns_command_output() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    let output = super::run_git_maintenance_logged(
        Some(&tx),
        repo.to_string_lossy().as_ref(),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    )
    .expect("maintenance command should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "main");
    assert!(rx.try_iter().any(
        |log| matches!(log, super::TaskLogStream::Stdout(value) if value.contains("timeout="))
    ));
}

#[test]
fn git_output_failure_detail_prefers_stderr() {
    let output = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(128) },
        stdout: b"out".to_vec(),
        stderr: b"err".to_vec(),
    };

    assert_eq!(super::git_output_failure_detail(&output), "code=128 stderr=err");
}

#[test]
fn git_output_failure_detail_handles_stdout_and_missing_exit_code() {
    let stdout_only = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: Some(1) },
        stdout: b"explanation".to_vec(),
        stderr: Vec::new(),
    };
    let no_detail = super::GitCommandOutput {
        status: super::GitCommandStatus { success: false, code: None },
        stdout: Vec::new(),
        stderr: Vec::new(),
    };

    assert_eq!(super::git_output_failure_detail(&stdout_only), "code=1 stdout=explanation");
    assert_eq!(super::git_output_failure_detail(&no_detail), "code=None");
}

#[test]
fn build_review_diff_context_uses_parent_commit_for_normal_commit() {
    let (_temp, repo) = create_repo();
    std::fs::write(repo.join("README.md"), "seed\nsecond\n").expect("file should update");
    run_git_test(&repo, &["add", "README.md"]);
    run_git_test(&repo, &["commit", "-m", "second"]);

    let (source, target, commit1, commit2, diff) =
        super::build_review_diff_context(repo.to_string_lossy().as_ref(), None, Some("main"))
            .expect("diff context should build");

    assert_eq!(source, "HEAD");
    assert_eq!(target, "main");
    assert_ne!(commit1, commit2);
    assert!(diff.contains("+second"));
}

#[test]
fn build_review_diff_context_trims_refs_and_defaults_blank_refs() {
    let (_temp, repo) = create_repo();
    let repo_path = repo.to_string_lossy();

    let (source, target, _, _, _) =
        super::build_review_diff_context(repo_path.as_ref(), Some("  main  "), Some("  HEAD  "))
            .expect("trimmed refs should resolve");
    assert_eq!(source, "main");
    assert_eq!(target, "HEAD");

    let (source, target, _, _, _) =
        super::build_review_diff_context(repo_path.as_ref(), Some(" \t "), Some(""))
            .expect("blank refs should use HEAD");
    assert_eq!(source, "HEAD");
    assert_eq!(target, "HEAD");
}

#[test]
fn build_review_diff_context_uses_empty_tree_for_root_commit() {
    let (_temp, repo) = create_repo();
    let root_commit =
        super::run_git(repo.to_string_lossy().as_ref(), &["rev-list", "--max-parents=0", "HEAD"])
            .expect("root commit should resolve");
    let root_commit = String::from_utf8_lossy(&root_commit.stdout).trim().to_string();

    let (_source, _target, commit1, commit2, diff) = super::build_review_diff_context(
        repo.to_string_lossy().as_ref(),
        Some(&root_commit),
        Some("main"),
    )
    .expect("root diff context should build");

    assert_eq!(commit1, "4b825dc642cb6eb9a060e54bf8d69288fbee4904");
    assert_eq!(commit2, root_commit);
    assert!(diff.contains("README.md"));
}

#[test]
fn build_review_diff_context_reports_bad_source_ref() {
    let (_temp, repo) = create_repo();

    let error = super::build_review_diff_context(
        repo.to_string_lossy().as_ref(),
        Some("missing-branch"),
        None,
    )
    .expect_err("bad source should fail");

    assert!(error.contains("git rev-parse 失败 source=missing-branch"));
}

#[test]
fn abort_git_in_progress_states_ignores_benign_empty_repo_state() {
    let (_temp, repo) = create_repo();
    let (tx, rx) = std::sync::mpsc::channel();

    super::abort_git_in_progress_states(repo.to_string_lossy().as_ref(), Some(&tx));

    assert!(rx.try_iter().all(|log| match log {
        super::TaskLogStream::Stdout(value) | super::TaskLogStream::Stderr(value) => {
            !value.contains("abort_failed")
        }
        _ => true,
    }));
}

#[test]
fn abort_git_in_progress_states_aborts_active_merge_and_logs_result() {
    let (_temp, repo) = create_repo();
    run_git_test(&repo, &["checkout", "-b", "feature"]);
    std::fs::write(repo.join("README.md"), "feature\n").expect("feature content");
    run_git_test(&repo, &["add", "README.md"]);
    run_git_test(&repo, &["commit", "-m", "feature"]);
    run_git_test(&repo, &["checkout", "main"]);
    std::fs::write(repo.join("README.md"), "main\n").expect("main content");
    run_git_test(&repo, &["add", "README.md"]);
    run_git_test(&repo, &["commit", "-m", "main"]);

    let merge = vw_shared::shell::git_std_command()
        .current_dir(&repo)
        .args(["merge", "feature"])
        .output()
        .expect("conflicting merge should start");
    assert!(!merge.status.success());
    assert!(repo.join(".git/MERGE_HEAD").exists());

    let (tx, rx) = std::sync::mpsc::channel();
    super::abort_git_in_progress_states(repo.to_string_lossy().as_ref(), Some(&tx));

    assert!(!repo.join(".git/MERGE_HEAD").exists());
    assert!(rx.try_iter().any(|log| {
        matches!(log, super::TaskLogStream::Stdout(value) if value.contains("merge --abort result=aborted"))
    }));
}
