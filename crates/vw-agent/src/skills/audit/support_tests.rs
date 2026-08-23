//! 验证技能审计辅助函数的路径、文本和敏感信息处理。

use super::*;
use std::fs;

#[test]
fn extension_helpers_are_case_insensitive() {
    assert!(is_markdown_file(Path::new("Guide.MD")));
    assert!(is_markdown_file(Path::new("Guide.markdown")));
    assert!(is_toml_file(Path::new("SKILL.TOML")));
    assert!(has_script_suffix("install.PS1"));
}

#[test]
fn unsupported_script_detects_shell_shebang() {
    let dir = tempfile::tempdir().expect("temp dir");
    let script = dir.path().join("run");
    fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();

    assert!(is_unsupported_script_file(&script));
    assert!(!is_unsupported_script_file(&dir.path().join("missing")));
}

#[test]
fn link_text_helpers_strip_and_classify_targets() {
    assert_eq!(strip_query_and_fragment("docs/readme.md?x=1#top"), "docs/readme.md");
    assert_eq!(url_scheme("https://example.com"), Some("https"));
    assert_eq!(url_scheme(":missing"), None);
    assert_eq!(url_scheme("git:"), None);
    assert_eq!(url_scheme("bad scheme://example.com"), None);
    assert!(looks_like_absolute_path("/etc/passwd"));
    assert!(looks_like_absolute_path("C:/Windows/System32"));
    assert!(looks_like_absolute_path("~/secret"));
    assert!(!looks_like_absolute_path("docs/readme.md"));
    assert!(has_markdown_suffix("README.Markdown"));
    assert!(!has_markdown_suffix("README.txt"));
}

#[test]
fn relative_display_prefers_root_relative_paths() {
    let root = Path::new("/tmp/skill");
    assert_eq!(relative_display(root, root), ".");
    assert_eq!(relative_display(root, &root.join("SKILL.md")), "SKILL.md");
}

#[test]
fn helper_boundaries_reject_lookalike_paths_and_schemes() {
    assert_eq!(relative_display(Path::new("root"), Path::new("elsewhere/a.md")), "elsewhere/a.md");
    assert!(!is_markdown_file(Path::new("guide.md.bak")));
    assert!(!is_toml_file(Path::new("SKILL.yaml")));
    assert!(!has_script_suffix("scripts/run.rs"));
    assert_eq!(strip_query_and_fragment("docs/readme.md"), "docs/readme.md");
    assert_eq!(url_scheme("docs/readme.md"), None);
    assert_eq!(url_scheme("git+ssh:repository"), Some("git+ssh"));
    assert!(looks_like_absolute_path(r"C:\Windows\System32"));
    assert!(!has_markdown_suffix("README.md.bak"));
}

#[test]
fn unsupported_script_allows_non_shell_shebang() {
    let dir = tempfile::tempdir().expect("temp dir");
    let script = dir.path().join("python_runner");
    fs::write(&script, "#!/usr/bin/env python3\nprint('safe')\n").unwrap();

    assert!(!is_unsupported_script_file(&script));
}
