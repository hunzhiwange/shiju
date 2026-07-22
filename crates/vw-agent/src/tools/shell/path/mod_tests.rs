use std::path::{Path, PathBuf};

use crate::tools::shell::ast::ParsedCommand;
use crate::tools::shell::path::{PathCheckResult, check_path_constraints, extract_redirect_paths};

#[test]
fn extract_redirect_paths_skips_fd_duplicates_heredocs_and_dev_null() {
    let cmd = crate::tools::shell::ast::parse_command("echo hi > out.txt 2>&1 <<EOF >/dev/null");

    assert_eq!(extract_redirect_paths(&cmd), vec![PathBuf::from("out.txt")]);
}

#[test]
fn fallback_candidate_paths_use_shell_words_tokens() {
    let cmd = ParsedCommand::Fallback {
        raw: "cat README.md".into(),
        tokens: vec!["cat".into(), "README.md".into()],
    };

    assert_eq!(super::extract_candidate_paths(&cmd), vec!["README.md"]);
}

#[test]
fn tilde_user_paths_are_blocked_before_resolution() {
    let cmd = crate::tools::shell::ast::parse_command("cat ~alice/.ssh/config");
    let workspace = PathBuf::from("/workspace");

    assert_eq!(
        check_path_constraints(&cmd, &workspace, &[]),
        PathCheckResult::Blocked {
            path: PathBuf::from("~alice/.ssh/config"),
            reason: "tilde-user paths are not allowed".into(),
        }
    );
}

#[test]
fn resolve_path_expands_workspace_relative_segments() {
    assert_eq!(
        super::resolve_path("./src/../Cargo.toml", Path::new("/workspace/project")),
        Some(PathBuf::from("/workspace/project/Cargo.toml"))
    );
}

#[test]
fn allows_all_paths_only_when_root_is_in_allowlist() {
    assert!(super::allows_all_paths(&[PathBuf::from("/")]));
    assert!(!super::allows_all_paths(&[PathBuf::from("/workspace")]));
}

#[test]
fn home_ssh_directory_is_considered_dangerous() {
    assert!(super::is_dangerous_path(Path::new("/home/alice/.ssh/id_rsa")));
}

#[test]
fn current_user_ssh_directory_is_considered_dangerous() {
    let home = directories::BaseDirs::new().expect("home directory");

    assert!(super::is_dangerous_path(&home.home_dir().join(".ssh/config")));
}

#[test]
fn fallback_without_tokens_has_no_candidate_paths() {
    let cmd = ParsedCommand::Fallback { raw: String::new(), tokens: Vec::new() };

    assert!(super::extract_candidate_paths(&cmd).is_empty());
}

#[test]
fn standard_stream_marker_does_not_block_path_constraints() {
    let cmd =
        ParsedCommand::Fallback { raw: "cat -".into(), tokens: vec!["cat".into(), "-".into()] };

    assert_eq!(
        check_path_constraints(&cmd, Path::new("/workspace"), &[]),
        PathCheckResult::Allowed
    );
}

#[test]
fn redirect_paths_are_empty_for_fallback_commands() {
    let cmd = ParsedCommand::Fallback {
        raw: "echo hi > out.txt".into(),
        tokens: vec!["echo".into(), "hi".into(), ">".into(), "out.txt".into()],
    };

    assert!(extract_redirect_paths(&cmd).is_empty());
}

#[test]
fn search_paths_include_flag_files_and_positionals_after_pattern() {
    let args = vec![
        "--file=patterns.txt".into(),
        "needle".into(),
        "--".into(),
        "-literal-path".into(),
        "src".into(),
    ];

    assert_eq!(super::extract_search_paths(&args), vec!["patterns.txt", "-literal-path", "src"]);
}

#[test]
fn search_without_a_path_ignores_pattern_and_options() {
    let args = vec!["--line-number".into(), "needle".into()];

    assert!(super::extract_search_paths(&args).is_empty());
}

#[test]
fn git_paths_support_attached_and_equals_flag_values() {
    let args = vec!["-C/worktree".into(), "--git-dir=/metadata".into(), "status".into()];

    assert_eq!(super::extract_git_paths(&args), vec!["/worktree", "/metadata"]);
}

#[test]
fn command_candidate_dispatch_uses_specialized_extractors() {
    let cases = [
        ("jq . input.json", vec!["input.json"]),
        ("sed s/old/new/ input.txt", vec!["input.txt"]),
        ("find src -name '*.rs'", vec!["src"]),
        ("chmod 755 src", vec!["src"]),
    ];

    for (raw, expected) in cases {
        let cmd = crate::tools::shell::ast::parse_command(raw);
        assert_eq!(super::extract_candidate_paths(&cmd), expected, "command: {raw}");
    }
}

#[test]
fn jq_paths_include_filter_file_and_input_file() {
    let args = vec!["-f".into(), "filter.jq".into(), ".".into(), "input.json".into()];

    assert_eq!(super::extract_jq_paths(&args), vec!["filter.jq", "input.json"]);
}

#[test]
fn sed_paths_include_script_file_and_edit_target() {
    let args = vec![
        "-e".into(),
        "s/old/new/".into(),
        "-f".into(),
        "rules.sed".into(),
        "-i".into(),
        String::new(),
        "input.txt".into(),
    ];

    assert_eq!(super::extract_sed_paths(&args), vec!["rules.sed", "input.txt"]);
}

#[test]
fn sed_paths_skip_inline_edit_and_expression_options() {
    let args = vec![
        "-i.bak".into(),
        "-E".into(),
        "s/old/new/".into(),
        "first.txt".into(),
        "--".into(),
        "second.txt".into(),
    ];

    assert_eq!(super::extract_sed_paths(&args), vec!["first.txt", "second.txt"]);
}

#[test]
fn find_paths_stop_at_the_first_expression_token() {
    let args = vec![
        "--".into(),
        "first".into(),
        "second".into(),
        "!".into(),
        "-name".into(),
        "*.rs".into(),
    ];

    assert_eq!(super::extract_find_paths(&args), vec!["first", "second"]);
}

#[test]
fn mode_target_paths_skip_mode_and_later_options() {
    let args = vec!["755".into(), "-R".into(), "src".into(), "tests".into()];

    assert_eq!(super::extract_mode_target_paths(&args), vec!["src", "tests"]);
}

#[test]
fn flag_value_parser_leaves_index_when_exact_flag_has_no_value() {
    let args = vec!["--file".into()];
    let mut index = 0;

    assert_eq!(super::take_flag_value(&args, &mut index, &["--file"]), None);
    assert_eq!(index, 0);
}

#[test]
fn base_command_name_strips_parent_directories() {
    assert_eq!(super::base_command_name("/usr/local/bin/rg"), "rg");
    assert_eq!(super::base_command_name("rg"), "rg");
}

#[test]
fn resolve_path_ignores_empty_and_standard_stream_markers() {
    assert_eq!(super::resolve_path("", Path::new("/workspace")), None);
    assert_eq!(super::resolve_path("-", Path::new("/workspace")), None);
}

#[test]
fn normalize_path_collapses_absolute_parent_and_current_segments() {
    assert_eq!(
        super::normalize_path(Path::new("/workspace/./src/../Cargo.toml")),
        PathBuf::from("/workspace/Cargo.toml")
    );
}

#[test]
fn path_allowance_respects_component_boundaries_and_normalized_roots() {
    let workspace = Path::new("/workspace/project/./");
    let allowed_roots = vec![PathBuf::from("/shared/root/../allowed")];

    assert!(super::is_path_allowed(
        Path::new("/shared/allowed/file.txt"),
        workspace,
        &allowed_roots
    ));
    assert!(!super::is_path_allowed(
        Path::new("/workspace/project-sibling/file.txt"),
        workspace,
        &allowed_roots
    ));
}

#[test]
fn outside_non_dangerous_path_reports_directory_boundary_reason() {
    let cmd = crate::tools::shell::ast::parse_command("cat /outside/data.txt");

    assert_eq!(
        check_path_constraints(&cmd, Path::new("/workspace"), &[]),
        PathCheckResult::Blocked {
            path: PathBuf::from("/outside/data.txt"),
            reason: "Path is outside allowed directories".into(),
        }
    );
}

#[test]
fn dangerous_path_matching_does_not_use_partial_file_names() {
    assert!(!super::is_dangerous_path(Path::new("/etc/passwd.backup")));
    assert!(!super::is_dangerous_path(Path::new("/workspace/.ssh/id_rsa")));
}
