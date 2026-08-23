//! 验证技能目录和清单相关常量保持稳定。

use super::*;

#[test]
fn defaults_are_stable_and_include_builtin_aliases() {
    let aliases = default_preloaded_skill_aliases();
    assert_eq!(default_policy_version(), 1);
    assert_eq!(default_version(), "0.1.0");
    assert_eq!(
        aliases.get("create-task-plans").map(String::as_str),
        Some("bundled:create-task-plans")
    );
    assert_eq!(aliases.get("do-task-plans").map(String::as_str), Some("bundled:do-task-plans"));
    assert_eq!(aliases.get("rustcodegraph").map(String::as_str), Some("bundled:rustcodegraph"));
    assert_eq!(BUILTIN_PRELOADED_SKILLS.len(), DEFAULT_PRELOADED_SKILL_SOURCES.len());
}

#[test]
fn repository_policy_and_sync_constants_are_stable() {
    assert_eq!(OPEN_SKILLS_REPO_URL, "https://github.com/besoeasy/open-skills");
    assert_eq!(OPEN_SKILLS_SYNC_MARKER, ".vibewindow-open-skills-sync");
    assert_eq!(OPEN_SKILLS_SYNC_INTERVAL_SECS, 60 * 60 * 24 * 7);
    assert_eq!(SKILL_DOWNLOAD_POLICY_FILE, ".download-policy.toml");
    assert_eq!(SKILLS_SH_HOST, "skills.sh");
}

#[test]
fn builtin_preloaded_skills_have_embedded_markdown_and_sources() {
    for builtin in BUILTIN_PRELOADED_SKILLS {
        assert!(!builtin.dir_name.trim().is_empty());
        assert!(!builtin.source_url.trim().is_empty());
        assert!(builtin.markdown.contains("#") || builtin.markdown.contains("name:"));
    }
}
