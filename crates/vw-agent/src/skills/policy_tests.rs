//! 验证新版技能策略的来源信任、能力限制与拒绝规则。

use super::*;
use crate::app::agent::skills::types::SkillDownloadPolicy;

#[test]
fn normalize_domain_list_sorts_deduplicates_and_strips_url_parts() {
    let mut entries = vec![
        " HTTPS://Example.COM/path ".to_string(),
        "*.example.com".to_string(),
        "docs.example.com:443".to_string(),
        "".to_string(),
    ];
    normalize_domain_list(&mut entries);
    assert_eq!(entries, vec!["docs.example.com", "example.com"]);
}

#[test]
fn normalize_domain_entry_handles_http_leading_dots_and_empty_values() {
    assert_eq!(normalize_domain_entry(" HTTP://.Example.COM?source=test "), "example.com");
    assert_eq!(normalize_domain_entry(" .Docs.Example.COM#section "), "docs.example.com");
    assert_eq!(normalize_domain_entry("   "), "");
}

#[test]
fn trusted_domain_matches_exact_and_subdomains_only() {
    assert!(host_matches_trusted_domain("example.com", "example.com"));
    assert!(host_matches_trusted_domain("docs.example.com", "example.com"));
    assert!(!host_matches_trusted_domain("badexample.com", "example.com"));
}

#[test]
fn trusted_domain_matching_normalizes_both_sides_and_rejects_empty_values() {
    assert!(host_matches_trusted_domain("HTTPS://API.Example.COM/path", "*.example.com"));
    assert!(!host_matches_trusted_domain("", "example.com"));
    assert!(!host_matches_trusted_domain("example.com", "   "));
}

#[test]
fn source_urls_for_trust_check_expands_skills_sh_source() {
    let urls = source_urls_for_trust_check("https://skills.sh/acme/tools/review");
    assert_eq!(
        urls,
        vec![
            "https://skills.sh/acme/tools/review".to_string(),
            "https://github.com/acme/tools.git".to_string(),
        ]
    );
}

#[test]
fn source_urls_for_trust_check_accepts_remote_schemes_and_ignores_local_sources() {
    for source in [
        "https://example.com/skill.git",
        "http://example.com/skill.git",
        "ssh://git@example.com/skill.git",
        "git://example.com/skill.git",
    ] {
        assert_eq!(source_urls_for_trust_check(source), vec![source.to_string()]);
    }
    assert!(source_urls_for_trust_check("/tmp/local-skill").is_empty());
}

#[test]
fn resolve_skill_source_alias_trims_lookup_key() {
    let mut policy = SkillDownloadPolicy::default();
    policy.aliases.insert("demo".into(), "https://example.com/demo.git".into());
    assert_eq!(resolve_skill_source_alias(" demo ", &policy), "https://example.com/demo.git");
    assert_eq!(resolve_skill_source_alias("raw", &policy), "raw");
}

#[test]
fn resolve_unknown_alias_preserves_original_source() {
    let policy = SkillDownloadPolicy::default();

    assert_eq!(resolve_skill_source_alias(" raw-source ", &policy), " raw-source ");
}

#[test]
fn policy_load_initializes_and_persists_defaults_when_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();

    let loaded = load_or_init_skill_download_policy(dir.path()).unwrap();

    assert_eq!(loaded.version, 1);
    assert!(loaded.aliases.contains_key("create-task-plans"));
    let persisted: SkillDownloadPolicy =
        toml::from_str(&std::fs::read_to_string(dir.path().join(".download-policy.toml")).unwrap())
            .unwrap();
    assert_eq!(persisted.version, loaded.version);
    assert_eq!(persisted.aliases, loaded.aliases);
}

#[test]
fn policy_load_save_initializes_defaults_and_normalizes_domains() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = SkillDownloadPolicy {
        version: 7,
        aliases: Default::default(),
        trusted_domains: vec!["HTTPS://Example.COM/path".into()],
        blocked_domains: vec!["*.Bad.COM:443".into(), "bad.com".into()],
    };

    save_skill_download_policy(dir.path(), &policy).unwrap();
    policy.trusted_domains.clear();
    let loaded = load_or_init_skill_download_policy(dir.path()).unwrap();

    assert_eq!(loaded.version, 7);
    assert_eq!(loaded.trusted_domains, vec!["example.com"]);
    assert_eq!(loaded.blocked_domains, vec!["bad.com"]);
    assert!(loaded.aliases.contains_key("create-task-plans"));
}

#[test]
fn policy_load_normalizes_existing_file_and_preserves_custom_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".download-policy.toml");
    std::fs::write(
        &path,
        r#"
version = 9
trusted_domains = [" HTTPS://Docs.Example.COM/path ", "docs.example.com"]
blocked_domains = ["*.Bad.Example:443", ".bad.example"]

[aliases]
custom = "https://example.com/custom.git"
"#,
    )
    .unwrap();

    let loaded = load_or_init_skill_download_policy(dir.path()).unwrap();

    assert_eq!(loaded.version, 9);
    assert_eq!(loaded.trusted_domains, vec!["docs.example.com"]);
    assert_eq!(loaded.blocked_domains, vec!["bad.example"]);
    assert_eq!(
        loaded.aliases.get("custom").map(String::as_str),
        Some("https://example.com/custom.git")
    );
    assert!(loaded.aliases.contains_key("create-task-plans"));

    let persisted: SkillDownloadPolicy =
        toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(persisted.trusted_domains, loaded.trusted_domains);
    assert_eq!(persisted.blocked_domains, loaded.blocked_domains);
    assert_eq!(persisted.aliases, loaded.aliases);
}

#[test]
fn policy_load_reports_read_error_for_directory_policy_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".download-policy.toml")).unwrap();

    let error = load_or_init_skill_download_policy(dir.path()).unwrap_err();

    assert!(error.to_string().contains("failed to read skill download policy"));
    assert!(error.to_string().contains(".download-policy.toml"));
}

#[test]
fn policy_save_reports_write_error_when_skills_directory_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let missing_skills_dir = dir.path().join("missing");

    let error = save_skill_download_policy(&missing_skills_dir, &SkillDownloadPolicy::default())
        .unwrap_err();

    assert!(error.to_string().contains("failed to write"));
    assert!(error.to_string().contains(".download-policy.toml"));
}

#[test]
fn policy_save_normalizes_persisted_copy_without_mutating_input() {
    let dir = tempfile::tempdir().unwrap();
    let policy = SkillDownloadPolicy {
        version: 1,
        aliases: Default::default(),
        trusted_domains: vec!["HTTPS://Example.COM/path".into()],
        blocked_domains: vec!["*.Bad.COM:443".into(), "bad.com".into()],
    };

    save_skill_download_policy(dir.path(), &policy).unwrap();

    assert_eq!(policy.trusted_domains, vec!["HTTPS://Example.COM/path"]);
    assert_eq!(policy.blocked_domains, vec!["*.Bad.COM:443", "bad.com"]);
    let persisted: SkillDownloadPolicy =
        toml::from_str(&std::fs::read_to_string(dir.path().join(".download-policy.toml")).unwrap())
            .unwrap();
    assert_eq!(persisted.trusted_domains, vec!["example.com"]);
    assert_eq!(persisted.blocked_domains, vec!["bad.com"]);
}

#[test]
fn malformed_policy_falls_back_to_default_aliases() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".download-policy.toml"), "not valid = [").unwrap();

    let policy = load_or_init_skill_download_policy(dir.path()).unwrap();

    assert!(policy.aliases.contains_key("create-task-plans"));
    assert!(policy.trusted_domains.is_empty());
}

#[test]
fn extract_link_host_supports_common_remote_source_shapes() {
    assert_eq!(
        extract_link_host("zip:https://User@Example.COM:443/path?x=1").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        extract_link_host("ssh://git@github.com/acme/repo.git").as_deref(),
        Some("github.com")
    );
    assert_eq!(extract_link_host("git://github.com/acme/repo.git").as_deref(), Some("github.com"));
    assert_eq!(extract_link_host("file:///tmp/skill"), None);
}

#[test]
fn extract_link_host_rejects_missing_hosts_and_unsupported_shapes() {
    assert_eq!(extract_link_host("https://:443/path"), None);
    assert_eq!(extract_link_host("zip:https://:443/path"), None);
    assert_eq!(extract_link_host("git@example.com:acme/repo.git"), None);
}

#[test]
fn ensure_source_domain_trust_allows_local_and_trusted_sources() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = SkillDownloadPolicy::default();

    ensure_source_domain_trust("/tmp/local-skill", &mut policy, dir.path()).unwrap();

    policy.trusted_domains = vec!["github.com".into(), "skills.sh".into()];
    ensure_source_domain_trust("https://skills.sh/acme/tools/review", &mut policy, dir.path())
        .unwrap();
}

#[test]
fn ensure_source_domain_trust_skips_remote_source_without_extractable_host() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = SkillDownloadPolicy::default();

    ensure_source_domain_trust_with_interactivity(
        "https:///missing-host",
        &mut policy,
        dir.path(),
        false,
    )
    .unwrap();

    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
    assert!(!download_policy_path(dir.path()).exists());
}

#[test]
fn ensure_source_domain_trust_rejects_blocked_and_unknown_noninteractive_sources() {
    let dir = tempfile::tempdir().unwrap();
    let mut blocked = SkillDownloadPolicy::default();
    blocked.blocked_domains = vec!["example.com".into()];

    let err = ensure_source_domain_trust("https://example.com/skill.git", &mut blocked, dir.path())
        .unwrap_err();
    assert!(err.to_string().contains("explicitly blocked"));

    let mut unknown = SkillDownloadPolicy::default();
    let err = ensure_source_domain_trust_with_interactivity(
        "https://unknown.invalid/skill.git",
        &mut unknown,
        dir.path(),
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("untrusted domain"));
}

#[test]
fn ensure_source_domain_trust_checks_expanded_github_host() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["skills.sh".into()];

    let error = ensure_source_domain_trust_with_interactivity(
        "https://skills.sh/acme/tools/review",
        &mut policy,
        dir.path(),
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("untrusted domain 'github.com'"));
    assert_eq!(policy.trusted_domains, vec!["skills.sh"]);
    assert!(policy.blocked_domains.is_empty());
}

#[test]
fn ensure_source_domain_trust_prioritizes_blocked_parent_domain() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = SkillDownloadPolicy::default();
    policy.trusted_domains = vec!["downloads.example.com".into()];
    policy.blocked_domains = vec!["example.com".into()];

    let error = ensure_source_domain_trust_with_interactivity(
        "https://downloads.example.com/skill.git",
        &mut policy,
        dir.path(),
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("explicitly blocked"));
    assert!(error.to_string().contains("downloads.example.com"));
}
