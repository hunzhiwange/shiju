use super::*;

#[test]
fn extracts_hosts_from_supported_git_urls() {
    assert_eq!(
        extract_link_host("zip:ssh://git@github.com:22/org/repo.git").as_deref(),
        Some("github.com")
    );
    assert_eq!(
        extract_link_host("https://Example.COM/org/repo?x=1").as_deref(),
        Some("example.com")
    );
    assert_eq!(extract_link_host("not-a-url"), None);
}

#[test]
fn skills_sh_source_rejects_traversal() {
    let parsed = parse_skills_sh_source("https://skills.sh/openai/skills/rust");
    assert_eq!(parsed.unwrap().github_repo_url(), "https://github.com/openai/skills.git");
    assert!(parse_skills_sh_source("https://skills.sh/openai/../secret/rust").is_none());
    assert!(parse_skills_sh_source("https://example.com/openai/skills/rust").is_none());
}

#[test]
fn trust_check_urls_include_direct_and_skills_sh_github_repo_once() {
    assert_eq!(
        source_urls_for_trust_check("https://github.com/acme/skill.git"),
        vec!["https://github.com/acme/skill.git".to_string()]
    );
    assert_eq!(
        source_urls_for_trust_check("https://skills.sh/acme/repo/rust"),
        vec![
            "https://skills.sh/acme/repo/rust".to_string(),
            "https://github.com/acme/repo.git".to_string(),
        ]
    );
    assert!(source_urls_for_trust_check("local-alias").is_empty());
}

#[test]
fn skills_sh_parsing_ignores_query_fragment_and_normalizes_dir_names() {
    let parsed = parse_skills_sh_source("https://skills.sh/Owner/Repo/My_Skill?ref=main#readme")
        .expect("skills.sh source");

    assert_eq!(parsed.owner, "Owner");
    assert_eq!(parsed.repo, "Repo");
    assert_eq!(parsed.skill, "My_Skill");
    assert!(is_skills_sh_source("https://skills.sh/owner/repo/skill"));
    assert_eq!(normalize_skills_sh_dir_name("My Skill/@2026!"), "myskill2026");
}

#[test]
fn extract_host_rejects_empty_hosts_and_strips_ports() {
    assert_eq!(extract_link_host("https:///missing-host"), None);
    assert_eq!(extract_link_host("git://example.com:9418/repo").as_deref(), Some("example.com"));
}

#[test]
fn extract_host_handles_http_userinfo_port_and_query() {
    assert_eq!(
        extract_link_host("http://build@Example.COM:8080?ref=main").as_deref(),
        Some("example.com")
    );
}

#[test]
fn extract_host_rejects_supported_schemes_without_hosts() {
    assert_eq!(extract_link_host("ssh://#fragment"), None);
    assert_eq!(extract_link_host("git://?ref=main"), None);
    assert_eq!(extract_link_host("zip:file:///tmp/skill.zip"), None);
}

#[test]
fn normalize_domain_strips_protocol_wildcard_path_and_port() {
    assert_eq!(
        normalize_domain_entry("  HTTPS://*.Example.COM:443/path?ref=main  "),
        "example.com"
    );
    assert_eq!(normalize_domain_entry("http://.Sub.Example.COM#readme"), "sub.example.com");
    assert_eq!(normalize_domain_entry("   "), "");
}

#[test]
fn trust_check_urls_preserve_each_supported_direct_protocol() {
    for source in [
        "http://example.com/acme/skill.git",
        "ssh://git@example.com/acme/skill.git",
        "git://example.com/acme/skill.git",
    ] {
        assert_eq!(source_urls_for_trust_check(source), vec![source.to_string()]);
    }
}

#[test]
fn skills_sh_parsing_rejects_incomplete_and_unsafe_segments() {
    for source in [
        "https://skills.sh/",
        "https://skills.sh/owner",
        "https://skills.sh/owner/repo",
        "https://skills.sh/ow..ner/repo/skill",
        "https://skills.sh/owner/re..po/skill",
        "https://skills.sh/owner/repo/sk..ill",
        r"https://skills.sh/ow\ner/repo/skill",
        r"https://skills.sh/owner/re\po/skill",
        r"https://skills.sh/owner/repo/sk\ill",
    ] {
        assert!(parse_skills_sh_source(source).is_none(), "accepted unsafe source: {source}");
    }
}

#[test]
fn skills_sh_parsing_filters_empty_path_segments() {
    let parsed = parse_skills_sh_source("https://skills.sh//owner///repo//skill/")
        .expect("source with empty path segments");

    assert_eq!(parsed.owner, "owner");
    assert_eq!(parsed.repo, "repo");
    assert_eq!(parsed.skill, "skill");
}

#[test]
fn skills_sh_detection_requires_https_and_exact_host_casing() {
    assert!(!is_skills_sh_source("http://skills.sh/owner/repo/skill"));
    assert!(!is_skills_sh_source("https://Skills.sh/owner/repo/skill"));
    assert!(!is_skills_sh_source("skills.sh/owner/repo/skill"));
}

#[test]
fn normalize_dir_name_keeps_only_ascii_safe_characters() {
    assert_eq!(normalize_skills_sh_dir_name("A_Z-09/skill.toml"), "a_z-09skilltoml");
    assert_eq!(normalize_skills_sh_dir_name("技能/🙂"), "");
}
