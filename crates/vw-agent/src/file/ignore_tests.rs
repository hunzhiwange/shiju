// 覆盖默认忽略模式、白名单及跨平台路径匹配。

use super::{PATTERNS, matches, split_path};
use glob::Pattern;

// 临时目录夹具同时覆盖正常路径和逃逸路径，确保安全判断不依赖当前工作区。
#[test]
fn split_path_accepts_unix_and_windows_separators() {
    assert_eq!(
        split_path("src\\nested/file.rs").collect::<Vec<_>>(),
        vec!["src", "nested", "file.rs"]
    );
}

#[test]
fn matches_honors_whitelist_before_default_rules() {
    let whitelist = [Pattern::new("node_modules/kept.js").expect("valid glob")];

    assert!(!matches("node_modules/kept.js", None, Some(&whitelist)));
    assert!(matches("node_modules/other.js", None, Some(&whitelist)));
}

#[test]
fn matches_default_folder_names_across_path_styles() {
    assert!(matches("src/node_modules/pkg/index.js", None, None));
    assert!(matches("src\\target\\debug\\app", None, None));
    assert!(matches(".git/config", None, None));
    assert!(!matches("src/lib.rs", None, None));
}

#[test]
fn matches_default_file_globs_for_cache_logs_and_temp_paths() {
    assert!(matches("notes/.DS_Store", None, None));
    assert!(matches("app/logs/output.txt", None, None));
    assert!(matches("app/tmp/file.txt", None, None));
    assert!(matches("app/coverage/lcov.info", None, None));
    assert!(matches("src/main.rs.swp", None, None));
}

#[test]
fn matches_extra_globs_after_whitelist_and_before_default_files() {
    let extra = [Pattern::new("generated/**").expect("valid extra glob")];
    let whitelist = [Pattern::new("generated/keep.rs").expect("valid whitelist glob")];

    assert!(!matches("generated/keep.rs", Some(&extra), Some(&whitelist)));
    assert!(matches("generated/drop.rs", Some(&extra), Some(&whitelist)));
    assert!(matches("logs/output.txt", Some(&extra), Some(&whitelist)));
}

#[test]
fn patterns_exposes_default_folder_and_file_patterns() {
    assert!(PATTERNS.iter().any(|pattern| pattern == "node_modules"));
    assert!(PATTERNS.iter().any(|pattern| pattern == "**/*.log"));
}
