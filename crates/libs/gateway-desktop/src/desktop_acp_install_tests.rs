use super::*;

#[tokio::test]
async fn installation_rejects_unknown_agents_before_spawning() {
    assert!(install_post(Path("custom; echo unsafe".into())).await.is_err());
    assert!(!INSTALLING.load(Ordering::Acquire));
}
