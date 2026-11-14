use super::*;

#[test]
fn openclaw_api_roundtrips_defaults_preserves_sections() {
    let pre = br#"{ // keep
        "env":{"TOKEN":"private"}, "agents":{"defaults":{"timeout":30,"extension":{"x":1}}}
    }"#;
    let defaults = OpenClawAgentsDefaults(serde_json::from_value(serde_json::json!({
        "timeout":45,"extension":{"x":1},"workspace":"中文"
    })).unwrap());
    let domain = convert::<openclaw_defaults::OpenClawAgentsDefaults>(defaults).unwrap();
    let next = openclaw_defaults::prepare_agents_defaults(Some(pre), &domain).unwrap();
    let result = project(Section::Defaults, Some(&next)).unwrap();
    assert_eq!(result["timeoutSeconds"], 45);
    assert_eq!(result["extension"]["x"], 1);
    assert_eq!(result["workspace"], "中文");
    assert!(result.get("timeout").is_none());
    assert!(String::from_utf8(next.clone()).unwrap().contains("// keep"));
    let model = convert::<openclaw_model::OpenClawDefaultModel>(OpenClawDefaultModel {
        primary:"provider/model".into(), fallbacks:vec!["other/model".into()], extra: Default::default()
    }).unwrap();
    let next = openclaw_model::prepare_default_model(Some(&next), &model).unwrap();
    assert_eq!(project(Section::Model, Some(&next)).unwrap()["primary"], "provider/model");
    assert_eq!(project(Section::Defaults, Some(&next)).unwrap()["extension"]["x"], 1);
    assert!(project(Section::Catalog, None).unwrap().is_null());
    assert!(project(Section::Defaults, Some(b"{agents: []}")).is_err());
}

#[test]
fn openclaw_api_roundtrips_defaults_denies_before_io() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = Scope { project_id:"project".into(), target_host_id:"remote".into() };
    for write in [false, true] {
        assert!(matches!(authorize(&Policy(None), &tenant, &HeaderMap::new(), &scope, write), Err(StatusCode::UNAUTHORIZED)));
    }
    assert_eq!(error(WriteError::Conflict), StatusCode::CONFLICT);
    assert_eq!(error(WriteError::Recovered), StatusCode::CONFLICT);
}

#[tokio::test]
async fn openclaw_api_protects_sensitive_env() {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let path = root.path().join("policy.json");
    let policy = |sensitive: bool| serde_json::json!({
        "principals":[{"id":"member", "token_sha256":hex::encode(Sha256::digest("fixture-token")),
            "grants":[{"platform_id":"platform","company_id":"company","host_id":"host",
                "openclaw_settings_read":["project"],
                "openclaw_env_sensitive_read":if sensitive { vec!["project"] } else { vec![] }}]}],
        "hosts":[{"id":"host","binding":"local","online":true,"home":home,
            "applications":home,"search_paths":[]}]
    });
    std::fs::write(&path, policy(false).to_string()).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", "fixture-token".parse().unwrap());
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = || Scope { project_id: "project".into(), target_host_id: "host".into() };
    let result = read(Policy(Some(path.clone())), tenant.clone(), headers.clone(), scope(), Section::Env).await;
    assert_eq!(result.err(), Some(StatusCode::FORBIDDEN));
    assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    std::fs::create_dir(home.join(".openclaw")).unwrap();
    let config = home.join(".openclaw/openclaw.json");
    let original = br#"{env:{vars:{TOKEN:'secret'}},tools:{profile:'private-profile'}}"#;
    std::fs::write(&config, original).unwrap();
    let health = read(Policy(Some(path.clone())), tenant.clone(), headers.clone(), scope(), Section::Health).await.unwrap().0;
    assert!(!health.to_string().contains("secret"));
    assert!(!health.to_string().contains("private-profile"));
    assert_eq!(std::fs::read(&config).unwrap(), original);
    std::fs::write(&path, policy(true).to_string()).unwrap();
    let env = read(Policy(Some(path)), tenant, headers, scope(), Section::Env).await.unwrap().0;
    assert_eq!(env["vars"]["TOKEN"], "secret");
}
