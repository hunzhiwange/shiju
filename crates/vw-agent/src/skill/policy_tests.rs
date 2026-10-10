//! 验证旧版技能策略的允许、拒绝及默认能力边界。
//! 来源信任先应用阻止列表，再处理可信域和非交互拒绝，避免宽松别名绕过策略。

use super::*;
use crate::app::agent::skill::constants::SKILL_DOWNLOAD_POLICY_FILE;
use std::path::Path;

#[test]
fn normalizes_domain_lists_and_matches_subdomains() {
    let mut domains = vec![
        " HTTPS://GitHub.COM/org/repo ".to_string(),
        "*.example.com:443".to_string(),
        "github.com".to_string(),
        " ".to_string(),
    ];
    normalize_domain_list(&mut domains);

    assert_eq!(domains, vec!["example.com", "github.com"]);
    assert!(host_matches_trusted_domain("api.github.com", "github.com"));
    assert!(!host_matches_trusted_domain("github.io", "github.com"));
}

#[test]
fn domain_entry_normalization_handles_protocols_separators_and_empty_values() {
    assert_eq!(normalize_domain_entry("   "), "");
    assert_eq!(normalize_domain_entry("http://Example.COM/path"), "example.com");
    assert_eq!(normalize_domain_entry(".Example.COM?download=1"), "example.com");
    assert_eq!(normalize_domain_entry("*.Example.COM#section"), "example.com");
}

#[test]
fn domain_matching_rejects_empty_and_suffix_impostor_hosts() {
    assert!(!host_matches_trusted_domain("", "example.com"));
    assert!(!host_matches_trusted_domain("example.com", ""));
    assert!(!host_matches_trusted_domain("notexample.com", "example.com"));
    assert!(host_matches_trusted_domain("HTTPS://API.EXAMPLE.COM/path", "*.example.com"));
}

#[test]
fn any_domain_matching_accepts_exact_or_subdomain_entries() {
    let domains = vec!["example.com".to_string(), "skills.sh".to_string()];

    assert!(host_matches_any_domain("skills.sh", &domains));
    assert!(host_matches_any_domain("cdn.example.com", &domains));
    assert!(!host_matches_any_domain("example.org", &domains));
    assert!(!host_matches_any_domain("example.com", &[]));
}

#[test]
fn policy_path_uses_stable_file_name() {
    assert_eq!(
        download_policy_path(Path::new("/tmp/skills")),
        Path::new("/tmp/skills").join(SKILL_DOWNLOAD_POLICY_FILE)
    );
}

#[test]
fn load_or_init_policy_creates_normalizes_and_preserves_aliases() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills_path = dir.path().join("skills");
    std::fs::create_dir_all(&skills_path).unwrap();

    let policy = load_or_init_skill_download_policy(&skills_path).unwrap();
    assert!(policy.aliases.contains_key("create-task-plans"));
    assert!(download_policy_path(&skills_path).is_file());

    std::fs::write(
        download_policy_path(&skills_path),
        r#"
        version = 1
        trusted_domains = [" HTTPS://GitHub.COM/org/repo ", "*.example.com:443"]
        blocked_domains = [" BAD.example.com/path ", "bad.example.com"]

        [aliases]
        local = "/tmp/local"
        "#,
    )
    .unwrap();

    let policy = load_or_init_skill_download_policy(&skills_path).unwrap();

    assert_eq!(policy.trusted_domains, vec!["example.com", "github.com"]);
    assert_eq!(policy.blocked_domains, vec!["bad.example.com"]);
    assert_eq!(policy.aliases.get("local").map(String::as_str), Some("/tmp/local"));
    assert!(policy.aliases.contains_key("create-task-plans"));
}

#[test]
fn load_or_init_policy_writes_default_policy_when_missing() {
    let dir = tempfile::tempdir().expect("temp dir");

    let policy = load_or_init_skill_download_policy(dir.path()).unwrap();
    let persisted = std::fs::read_to_string(download_policy_path(dir.path())).unwrap();

    assert_eq!(policy.version, 1);
    assert!(policy.aliases.contains_key("create-task-plans"));
    assert!(persisted.contains("[aliases]"));
    assert!(persisted.contains("create-task-plans"));
}

#[test]
fn load_policy_falls_back_to_defaults_for_invalid_toml() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(download_policy_path(dir.path()), "not = [valid toml").unwrap();

    let policy = load_or_init_skill_download_policy(dir.path()).unwrap();

    assert_eq!(policy.version, 1);
    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
    assert!(policy.aliases.contains_key("create-task-plans"));
}

#[test]
fn save_policy_normalizes_without_mutating_input_and_alias_resolution_trims() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skills_path = dir.path();
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["*.Example.com".to_string(), "example.com".to_string()];
    policy.blocked_domains = vec![" HTTPS://BAD.EXAMPLE/path ".to_string()];
    policy.aliases.insert("alias".to_string(), "https://github.com/acme/skill.git".to_string());

    save_skill_download_policy(skills_path, &policy).unwrap();
    assert_eq!(policy.trusted_domains.len(), 2);
    assert_eq!(policy.blocked_domains, vec![" HTTPS://BAD.EXAMPLE/path "]);

    let loaded = load_or_init_skill_download_policy(skills_path).unwrap();
    assert_eq!(loaded.trusted_domains, vec!["example.com"]);
    assert_eq!(loaded.blocked_domains, vec!["bad.example"]);
    assert_eq!(resolve_skill_source_alias(" alias ", &loaded), "https://github.com/acme/skill.git");
    assert_eq!(resolve_skill_source_alias("unknown", &loaded), "unknown");
    assert_eq!(resolve_skill_source_alias(" unknown ", &loaded), " unknown ");
}

#[test]
fn save_policy_reports_write_errors_for_missing_directories() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing_skills_path = dir.path().join("missing").join("skills");

    let error = save_skill_download_policy(&missing_skills_path, &SkillDownloadPolicy::default())
        .expect_err("saving below a missing directory must fail");

    assert!(error.to_string().contains("failed to write"));
    assert!(error.to_string().contains(SKILL_DOWNLOAD_POLICY_FILE));
}

#[test]
fn source_domain_trust_allows_trusted_and_rejects_blocked_domains() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["github.com".to_string()];
    policy.blocked_domains = vec!["evil.example".to_string()];

    ensure_source_domain_trust("https://api.github.com/acme/skill.git", &mut policy, dir.path())
        .unwrap();

    let err =
        ensure_source_domain_trust("https://evil.example/acme/skill.git", &mut policy, dir.path())
            .expect_err("blocked domain must fail");
    assert!(err.to_string().contains("explicitly blocked"));
}

#[test]
fn source_domain_trust_rejects_untrusted_domain_in_non_interactive_mode() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut policy = SkillDownloadPolicy::default();

    let error = ensure_source_domain_trust(
        "https://unknown.example/acme/skill.git",
        &mut policy,
        dir.path(),
    )
    .expect_err("unknown domains must fail without interactive confirmation");

    assert!(error.to_string().contains("untrusted domain 'unknown.example'"));
    assert!(error.to_string().contains(SKILL_DOWNLOAD_POLICY_FILE));
    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
    assert!(!download_policy_path(dir.path()).exists());
}

#[test]
fn source_domain_trust_skips_sources_without_checkable_hosts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut policy = SkillDownloadPolicy::default();

    ensure_source_domain_trust("local-skill-alias", &mut policy, dir.path()).unwrap();
    ensure_source_domain_trust("https:///missing-host", &mut policy, dir.path()).unwrap();

    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
    assert!(!download_policy_path(dir.path()).exists());
}

// 阻止列表具有最高优先级，即使父域同时位于可信列表也不能放行。
#[test]
fn source_domain_trust_gives_blocked_domains_precedence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["example.com".to_string()];
    policy.blocked_domains = vec!["example.com".to_string()];

    let error =
        ensure_source_domain_trust("https://example.com/acme/skill.git", &mut policy, dir.path())
            .expect_err("blocked domains must override trusted domains");

    assert!(error.to_string().contains("explicitly blocked"));
}

#[test]
fn source_domain_trust_checks_repository_derived_from_skills_sh_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["skills.sh".to_string()];
    policy.blocked_domains = vec!["github.com".to_string()];

    let error = ensure_source_domain_trust(
        "https://skills.sh/acme/skills/example",
        &mut policy,
        dir.path(),
    )
    .expect_err("the derived repository domain must also be checked");

    assert!(error.to_string().contains("github.com"));
    assert!(error.to_string().contains("explicitly blocked"));
}
