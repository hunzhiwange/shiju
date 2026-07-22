use super::*;

#[test]
fn state_default_has_no_skills_or_directories() {
    let state = State::default();

    assert!(state.skills.is_empty());
    assert!(state.dirs.is_empty());
}

#[test]
fn split_frontmatter_separates_yaml_header_from_body() {
    let input = "---\nname: test\ndescription: demo\n---\nBody text";
    let (frontmatter, body) = split_frontmatter(input);

    assert!(frontmatter.unwrap().contains("name: test"));
    assert_eq!(body.trim(), "Body text");
}

#[test]
fn split_frontmatter_returns_body_when_missing_header() {
    let (frontmatter, body) = split_frontmatter("Body only");
    assert!(frontmatter.is_none());
    assert_eq!(body, "Body only");
}

#[test]
fn split_frontmatter_requires_closing_marker() {
    let input = "---\nname: missing-close\nBody";
    let (frontmatter, body) = split_frontmatter(input);

    assert!(frontmatter.is_none());
    assert_eq!(body, input);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn split_frontmatter_handles_empty_header_and_requires_marker_newline() {
    let (frontmatter, body) = split_frontmatter("---\n\n---\nBody");
    assert_eq!(frontmatter, Some(""));
    assert_eq!(body, "Body");

    let input = "---name: inline\n---\nBody";
    let (frontmatter, body) = split_frontmatter(input);
    assert!(frontmatter.is_none());
    assert_eq!(body, input);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn split_frontmatter_preserves_body_whitespace() {
    let input = "---\nname: test\n---\n\n Body text \n";
    let (frontmatter, body) = split_frontmatter(input);

    assert_eq!(frontmatter, Some("name: test"));
    assert_eq!(body, "\n Body text \n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn split_frontmatter_rejects_closing_marker_without_trailing_newline() {
    let input = "---\nname: test\n---";
    let (frontmatter, body) = split_frontmatter(input);

    assert!(frontmatter.is_none());
    assert_eq!(body, input);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_path_handles_empty_absolute_and_relative_inputs() {
    assert_eq!(resolve_path("  \n"), PathBuf::new());

    let absolute = std::env::current_dir().unwrap().join("absolute-skill");
    assert_eq!(resolve_path(absolute.to_str().unwrap()), absolute);

    let relative = PathBuf::from("skills/reviewer");
    let project_dir = instance::directory();
    let expected = if project_dir.is_empty() {
        relative.clone()
    } else {
        PathBuf::from(project_dir).join(&relative)
    };
    assert_eq!(resolve_path("  skills/reviewer  "), expected);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_path_expands_home_prefix() {
    assert_eq!(resolve_path("  ~/skills/reviewer  "), global::paths().home.join("skills/reviewer"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn public_skill_queries_load_state_and_return_missing_skill_as_none() {
    let missing_name = format!(
        "__vibewindow_missing_skill_for_unit_test_{}__",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    );

    assert!(get(&missing_name).await.is_none());

    let all_skills = all().await;
    let skill_dirs = dirs().await;

    assert!(all_skills.iter().all(|skill| !skill.name.trim().is_empty()));
    assert!(skill_dirs.iter().all(|dir| !dir.trim().is_empty()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_state_scans_workspace_skills_skips_invalid_files_and_tracks_duplicate_names() {
    let workspace = tempfile::tempdir().expect("workspace dir");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_nanos();
    let skill_name = format!("workspace-skill-{unique_suffix}");
    let primary_dir = workspace.path().join(".vibewindow/skills/primary");
    let duplicate_dir = workspace.path().join("skills/duplicate");
    let invalid_dir = workspace.path().join(".vibewindow/skills/invalid");
    std::fs::create_dir_all(&primary_dir).expect("primary skill dir");
    std::fs::create_dir_all(&duplicate_dir).expect("duplicate skill dir");
    std::fs::create_dir_all(&invalid_dir).expect("invalid skill dir");
    std::fs::write(
        primary_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: first\n---\nFirst body\n"),
    )
    .expect("write primary skill");
    std::fs::write(
        duplicate_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: second\n---\nSecond body\n"),
    )
    .expect("write duplicate skill");
    std::fs::write(invalid_dir.join("SKILL.md"), "---\ndescription: missing name\n---\nBody")
        .expect("write invalid skill");

    let state =
        instance::provide(workspace.path(), None, || Box::pin(async move { load_state().await }))
            .await
            .expect("project context");

    let loaded = state.skills.get(&skill_name).expect("workspace skill loaded");
    assert_eq!(loaded.name, skill_name);
    assert!(matches!(loaded.description.as_str(), "first" | "second"));
    assert_eq!(state.skills.values().filter(|skill| skill.name == loaded.name).count(), 1);
    assert!(state.dirs.iter().any(|dir| dir.ends_with(".vibewindow/skills/primary")));
    assert!(state.dirs.iter().any(|dir| dir.ends_with("skills/duplicate")));
    assert!(!state.dirs.iter().any(|dir| dir.ends_with(".vibewindow/skills/invalid")));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_reads_frontmatter_and_normalizes_newlines() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");
    std::fs::write(&path, "---\r\nname: reviewer\r\ndescription: Review code\r\n---\r\nBody\r\n")
        .unwrap();

    let parsed = parse_skill_md(&path).await.expect("parsed skill");

    assert_eq!(parsed.0, "reviewer");
    assert_eq!(parsed.1, "Review code");
    assert_eq!(parsed.2, "Body");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_normalizes_lone_carriage_returns() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");
    std::fs::write(&path, "---\rname: reviewer\rdescription: Review code\r---\rBody\r").unwrap();

    let parsed = parse_skill_md(&path).await.expect("parsed skill");

    assert_eq!(parsed.0, "reviewer");
    assert_eq!(parsed.1, "Review code");
    assert_eq!(parsed.2, "Body");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_returns_none_when_file_cannot_be_read() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(parse_skill_md(&dir.path().join("missing.md")).await.is_none());
    assert!(parse_skill_md(dir.path()).await.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_rejects_missing_or_blank_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");

    std::fs::write(&path, "---\ndescription: Missing name\n---\nBody").unwrap();
    assert!(parse_skill_md(&path).await.is_none());

    std::fs::write(&path, "---\nname: '   '\n---\nBody").unwrap();
    assert!(parse_skill_md(&path).await.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_rejects_documents_without_a_string_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");

    std::fs::write(&path, "Body without frontmatter").unwrap();
    assert!(parse_skill_md(&path).await.is_none());

    std::fs::write(&path, "---\nplain scalar\n---\nBody").unwrap();
    assert!(parse_skill_md(&path).await.is_none());

    std::fs::write(&path, "---\nname: 42\n---\nBody").unwrap();
    assert!(parse_skill_md(&path).await.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_defaults_description_and_rejects_invalid_yaml() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");

    std::fs::write(&path, "---\nname: helper\n---\n\nBody").unwrap();
    let parsed = parse_skill_md(&path).await.expect("parsed skill");
    assert_eq!(parsed.0, "helper");
    assert_eq!(parsed.1, "");
    assert_eq!(parsed.2, "Body");

    std::fs::write(&path, "---\nname: [unterminated\n---\nBody").unwrap();
    assert!(parse_skill_md(&path).await.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn parse_skill_md_trims_fields_and_defaults_non_string_description() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");
    std::fs::write(
        &path,
        "---\nname: '  reviewer  '\ndescription: '  Review code  '\n---\n  Body text  \n",
    )
    .unwrap();

    let parsed = parse_skill_md(&path).await.expect("parsed skill");

    assert_eq!(parsed.0, "reviewer");
    assert_eq!(parsed.1, "Review code");
    assert_eq!(parsed.2, "Body text");

    std::fs::write(&path, "---\nname: reviewer\ndescription: 42\n---\nBody").unwrap();
    let parsed = parse_skill_md(&path).await.expect("parsed skill");
    assert_eq!(parsed.1, "");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn glob_files_returns_only_regular_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir_all(dir.path().join("nested")).unwrap();
    std::fs::write(dir.path().join("nested/SKILL.md"), "content").unwrap();
    std::fs::create_dir(dir.path().join("nested/dir.md")).unwrap();

    let files = glob_files(dir.path(), "**/*.md");

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().and_then(|name| name.to_str()), Some("SKILL.md"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn glob_files_returns_empty_for_invalid_patterns() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(glob_files(dir.path(), "[invalid").is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn glob_files_returns_empty_for_missing_root() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(glob_files(&dir.path().join("missing"), "**/SKILL.md").is_empty());
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn log_external_scan_accepts_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(std::ffi::OsString::from_vec(b"skills-\xff".to_vec()));

    log_external_scan(&path, "configured");
}
