//! 验证命令白名单匹配及参数和重定向中的禁用路径检测。

use super::*;

#[test]
fn allowlist_matches_exact_base_and_wildcard() {
    assert!(is_allowlist_entry_match("git", "/usr/bin/git", "git"));
    assert!(is_allowlist_entry_match("*", "/usr/bin/rm", "rm"));
    assert!(!is_allowlist_entry_match("cargo", "/usr/bin/git", "git"));
}

#[test]
fn allowlist_path_entries_expand_home_and_quotes() {
    let home = std::env::var("HOME").expect("HOME should be set for path expansion tests");
    let executable = format!("{home}/bin/tool");

    assert!(is_allowlist_entry_match("\"~/bin/tool\"", &executable, "tool"));
    assert!(!is_allowlist_entry_match("", &executable, "tool"));
    assert!(!is_allowlist_entry_match("~/bin/other", &executable, "tool"));
}

#[test]
fn allowlist_trims_whitespace_and_wrapping_quotes_before_matching() {
    assert!(is_allowlist_entry_match("  'git'  ", "/usr/bin/git", "git"));
    assert!(is_allowlist_entry_match("\"*\"", "/usr/bin/rm", "rm"));
    assert!(!is_allowlist_entry_match("  ", "/usr/bin/git", "git"));
}

#[test]
fn allowlist_path_entries_require_the_exact_executable_path() {
    assert!(is_allowlist_entry_match("/opt/tools/git", "/opt/tools/git", "git"));
    assert!(!is_allowlist_entry_match("/opt/tools/git", "/usr/bin/git", "git"));
}

#[test]
fn forbidden_path_arguments_include_flags_and_redirects() {
    let denied = |path: &str| !path.contains("secret") && !path.contains("outside");

    assert_eq!(
        find_forbidden_path_argument("cat ./ok.txt ./secret.txt", denied),
        Some("./secret.txt".to_string())
    );
    assert_eq!(
        find_forbidden_path_argument("grep needle --path=../outside/file", denied),
        Some("../outside/file".to_string())
    );
    assert_eq!(
        find_forbidden_path_argument("cmd -o./secret-output", denied),
        Some("./secret-output".to_string())
    );
    assert_eq!(
        find_forbidden_path_argument("echo hi >./secret.log", denied),
        Some("./secret.log".to_string())
    );
}

#[test]
fn forbidden_path_arguments_ignore_urls_empty_values_and_safe_paths() {
    fn always_deny(_: &str) -> bool {
        false
    }

    assert_eq!(find_forbidden_path_argument("curl https://example.com/a/b", always_deny), None);
    assert_eq!(find_forbidden_path_argument("echo plain words", always_deny), None);
    assert_eq!(
        find_forbidden_path_argument("cat ./allowed.txt", |path| path == "./allowed.txt"),
        None
    );
}

#[test]
fn forbidden_path_arguments_skip_empty_and_assignment_only_segments() {
    assert_eq!(find_forbidden_path_argument("", |_| false), None);
    assert_eq!(find_forbidden_path_argument("MODE=safe ; OTHER=value", |_| false), None);
}

#[test]
fn forbidden_path_arguments_check_redirect_in_executable_position() {
    assert_eq!(
        find_forbidden_path_argument(">./blocked.log", |_| false),
        Some("./blocked.log".to_string())
    );
    assert_eq!(find_forbidden_path_argument(">https://example.com/log", |_| false), None);
}

#[test]
fn forbidden_path_arguments_scan_environment_prefixed_command_segments() {
    let command = "MODE=safe cat ./allowed.txt && cp ../blocked ./later";

    assert_eq!(
        find_forbidden_path_argument(command, |path| path == "./allowed.txt"),
        Some("../blocked".to_string())
    );
}

#[test]
fn forbidden_path_arguments_strip_quotes_and_skip_empty_tokens() {
    assert_eq!(
        find_forbidden_path_argument("cat \"\" '../blocked.txt'", |_| false),
        Some("../blocked.txt".to_string())
    );
}

#[test]
fn forbidden_path_arguments_continue_after_allowed_option_paths() {
    let command = "tool --output=./allowed.log -o./allowed.tmp ../blocked";

    assert_eq!(
        find_forbidden_path_argument(command, |path| path.starts_with("./allowed")),
        Some("../blocked".to_string())
    );
}

#[test]
fn forbidden_path_arguments_ignore_switches_and_file_descriptor_redirects() {
    assert_eq!(find_forbidden_path_argument("tool --verbose -q 2>&1", |_| false), None);
}

#[test]
fn forbidden_path_arguments_continue_after_allowed_redirects() {
    assert_eq!(
        find_forbidden_path_argument("echo >./allowed.log ../blocked", |path| {
            path == "./allowed.log"
        }),
        Some("../blocked".to_string())
    );
}

#[test]
fn forbidden_path_arguments_continue_after_allowed_executable_redirect() {
    assert_eq!(
        find_forbidden_path_argument(">./allowed.log ../blocked", |path| path == "./allowed.log"),
        Some("../blocked".to_string())
    );
    assert_eq!(
        find_forbidden_path_argument(">https://example.com/log ../blocked", |_| false),
        Some("../blocked".to_string())
    );
}

#[test]
fn forbidden_path_arguments_ignore_empty_and_url_option_values() {
    assert_eq!(
        find_forbidden_path_argument("tool --output='' --source=https://example.com/a/b", |_| {
            false
        }),
        None
    );
}

#[test]
fn forbidden_path_arguments_continue_after_allowed_plain_paths_and_url_tokens() {
    assert_eq!(
        find_forbidden_path_argument("tool ./allowed https://example.com/a/b ../blocked", |path| {
            path == "./allowed"
        }),
        Some("../blocked".to_string())
    );
}
