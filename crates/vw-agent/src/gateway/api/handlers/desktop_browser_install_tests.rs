use super::*;

#[tokio::test]
async fn browser_install_rejects_unsupported_backends_before_spawning() {
    for backend in ["native", "computer_use", "agent_browser; echo unsafe", "../auto", ""] {
        assert!(install_post(Path(backend.into())).await.is_err());
    }
    assert!(!INSTALLING.load(Ordering::Acquire));
}
