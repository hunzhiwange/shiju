#[test]
fn acp_skill_mount_claude_accepts_native_plugin_contract() {
    assert!(super::super::validate("claude", true).is_ok());
    assert!(super::super::validate("claude", false).is_ok());
}

#[test]
fn acp_skill_mount_claude_creates_manifest_over_selected_view_only() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: example\ndescription: fixture\n---\nPRIVATE_BODY",
    )
    .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[vw_acp::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "session",
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    super::prepare(&view.0).unwrap();
    let bytes =
        std::fs::read(view.0.root().parent().unwrap().join(".claude-plugin/plugin.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["skills"], "./skills/");
    assert!(!String::from_utf8(bytes).unwrap().contains("PRIVATE_BODY"));
    view.0.validate().unwrap();
}

#[tokio::test]
async fn acp_skill_mount_claude_retires_changed_owner_and_preserves_other_sessions() {
    use super::super::super::{ACP_CLIENT_CACHE, ParsedAcpOptions, session};
    let directory = tempfile::tempdir().unwrap();
    let config = crate::config::schema::AcpAgentConfig {
        command: "claude-acp".into(),
        args: vec![],
        env: Default::default(),
    };
    let owner = directory.path().to_string_lossy().into_owned();
    let old = ParsedAcpOptions { runtime_identity: Some(owner.clone()), ..Default::default() };
    let current = ParsedAcpOptions { full_access: true, ..old.clone() };
    let foreign =
        ParsedAcpOptions { runtime_identity: Some(format!("{owner}-other")), ..Default::default() };
    let old_entry = session::get_cached_acp_client("claude", &config, directory.path(), &old);
    let current_entry =
        session::get_cached_acp_client("claude", &config, directory.path(), &current);
    let foreign_entry =
        session::get_cached_acp_client("claude", &config, directory.path(), &foreign);
    session::stop_incompatible_cached_clients("claude", &config, directory.path(), &current)
        .await
        .unwrap();
    let mut cache = ACP_CLIENT_CACHE.lock();
    assert!(!cache.contains_key(&old_entry.key));
    assert!(cache.contains_key(&current_entry.key));
    assert!(cache.contains_key(&foreign_entry.key));
    cache.remove(&current_entry.key);
    cache.remove(&foreign_entry.key);
}
