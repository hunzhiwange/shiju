use super::*;
use serde_json::json;

#[tokio::test]
async fn agent_config_cleanup_full_access_acp_preserves_false_and_deny() {
    isolated(async {
    use config_agent_profile::AgentDefault;
    let workspace = tempfile::tempdir().unwrap();
    let mut cfg = config::schema::Config::default();
    let mut agent =
        agent_config::catalog::merged_agent_catalog(&cfg.agents).remove("acp-codex").unwrap();
    agent.defaults.full_access = true;
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    agent.defaults.mcp = AgentDefault::Fixed(vec![]);
    cfg.agents.insert("acp-codex".into(), agent);
    for (options, expected) in [
        (json!({"agent":"acp-codex"}), true),
        (json!({"agent":"acp-codex", "full_access":false}), false),
        (json!({"agent":"acp-codex", "full_access":true}), true),
    ] {
        let mut parsed = ParsedAcpOptions::default();
        apply_agent_defaults(&cfg, &options, "codex", workspace.path(), &mut parsed, &json!([]))
            .await
            .unwrap();
        assert_eq!(parsed.full_access, expected);
    }
    cfg.agents.get_mut("acp-codex").unwrap().defaults.permission =
        AgentDefault::Fixed(json!({"*":"deny"}));
    let mut parsed = ParsedAcpOptions::default();
    apply_agent_defaults(
        &cfg,
        &json!({"agent":"acp-codex", "full_access":true}),
        "codex",
        workspace.path(),
        &mut parsed,
        &json!([]),
    )
    .await
    .unwrap();
    assert!(!parsed.full_access);
    assert_eq!(parsed.permission_mode, Some(PermissionMode::DenyAll));
    cfg.agents.get_mut("acp-codex").unwrap().defaults.permission = AgentDefault::Inherit;
    cfg.agents.get_mut("acp-codex").unwrap().permission = json!({"*":"deny"});
    let mut parsed = ParsedAcpOptions::default();
    apply_agent_defaults(
        &cfg,
        &json!({"agent":"acp-codex", "full_access":true}),
        "codex",
        workspace.path(),
        &mut parsed,
        &json!([]),
    )
    .await
    .unwrap();
    assert!(!parsed.full_access);
    assert_eq!(parsed.permission_mode, Some(PermissionMode::DenyAll));
    cfg.agents.get_mut("acp-codex").unwrap().permission = json!({"*":"allow", "file_read":"deny"});
    assert!(
        apply_agent_defaults(
            &cfg,
            &json!({"agent":"acp-codex", "full_access":true}),
            "codex",
            workspace.path(),
            &mut parsed,
            &json!([])
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("selective host permission denials")
    );
    }).await;
}
