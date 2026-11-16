#[tokio::test]
async fn acp_skill_mount_claude_retires_changed_owner_and_preserves_other_sessions() {
    use super::super::{ACP_CLIENT_CACHE, ParsedAcpOptions, session};
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
