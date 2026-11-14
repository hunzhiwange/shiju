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
