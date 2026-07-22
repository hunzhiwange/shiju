//! 路径提取测试，覆盖常见只读命令和路径约束检查的组合。

use std::path::PathBuf;

use crate::tools::shell::ast::parse_command;
use crate::tools::shell::path::{PathCheckResult, check_path_constraints};

use super::extractors;

#[test]
fn extracts_ls_paths() {
    let args = vec!["-la".into(), "/tmp".into(), "/var".into()];
    assert_eq!(extractors::extract_paths("ls", &args), vec!["/tmp", "/var"]);
}

#[test]
fn extracts_grep_paths() {
    let cmd = parse_command("grep -f patterns.txt word ./src");
    let workspace = PathBuf::from("/workspace");
    assert_eq!(super::extract_candidate_paths(&cmd), vec!["patterns.txt", "./src"]);
    assert_eq!(check_path_constraints(&cmd, &workspace, &[]), PathCheckResult::Allowed);
}

#[test]
fn extracts_cp_paths() {
    let args = vec!["-r".into(), "src/".into(), "dst/".into()];
    assert_eq!(extractors::extract_paths("cp", &args), vec!["src/", "dst/"]);
}

#[test]
fn extracts_git_paths() {
    let cmd = parse_command("git -C /other status");
    assert_eq!(super::extract_candidate_paths(&cmd), vec!["/other"]);
}

#[test]
fn respects_double_dash() {
    let args = vec!["--".into(), "-file".into()];
    assert_eq!(extractors::extract_paths("cat", &args), vec!["-file"]);
}

#[test]
fn ignores_echo_arguments() {
    let args = vec!["hello".into()];
    assert!(extractors::extract_paths("echo", &args).is_empty());
}

#[test]
fn flag_paths_support_separate_equals_and_attached_values() {
    let args = vec![
        "-o".into(),
        "result.txt".into(),
        "--temporary-directory=tmp".into(),
        "-Tcache".into(),
        "input.txt".into(),
    ];

    assert_eq!(
        extractors::extract_paths("sort", &args),
        vec!["result.txt", "tmp", "cache", "input.txt"]
    );
}

#[test]
fn exact_path_flag_without_value_is_ignored_safely() {
    let args = vec!["-d".into()];

    assert!(extractors::extract_paths("unzip", &args).is_empty());
}

#[test]
fn fixed_path_positions_ignore_extra_positionals() {
    let args = vec!["source".into(), "destination".into(), "extra".into()];

    assert_eq!(extractors::extract_paths("cp", &args), vec!["source", "destination"]);
}

#[test]
fn strips_wrappers_before_extracting() {
    let cmd = parse_command("timeout 10 ls /tmp");
    assert_eq!(super::extract_candidate_paths(&cmd), vec!["/tmp"]);
}

#[test]
fn extracts_redirect_paths() {
    let cmd = parse_command("echo hi > out.txt");
    assert_eq!(super::extract_candidate_paths(&cmd), vec!["out.txt"]);
}

#[test]
fn blocks_dangerous_paths() {
    let cmd = parse_command("cat /etc/passwd");
    let workspace = PathBuf::from("/workspace");
    assert!(matches!(
        check_path_constraints(&cmd, &workspace, &[]),
        PathCheckResult::Blocked { path, .. } if path == PathBuf::from("/etc/passwd")
    ));
}

#[test]
fn allowed_root_slash_permits_dangerous_paths() {
    let cmd = parse_command("cat /etc/passwd");
    let workspace = PathBuf::from("/workspace");
    let allowed = vec![PathBuf::from("/")];
    assert_eq!(check_path_constraints(&cmd, &workspace, &allowed), PathCheckResult::Allowed);
}

#[test]
fn blocks_paths_outside_workspace() {
    let cmd = parse_command("ls /root");
    let workspace = PathBuf::from("/workspace");
    assert!(matches!(
        check_path_constraints(&cmd, &workspace, &[]),
        PathCheckResult::Blocked { path, .. } if path == PathBuf::from("/root")
    ));
}

#[test]
fn allows_paths_inside_allowed_roots() {
    let cmd = parse_command("ls /allowed_root/file");
    let workspace = PathBuf::from("/workspace");
    let allowed = vec![PathBuf::from("/allowed_root")];
    assert_eq!(check_path_constraints(&cmd, &workspace, &allowed), PathCheckResult::Allowed);
}
