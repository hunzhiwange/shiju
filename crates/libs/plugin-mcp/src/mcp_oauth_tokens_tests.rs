use super::{Grant, access_with, read, remove, write};
use serde_json::json;
fn grant(expires: u64) -> Grant {
    Grant {
        issuer: "https://issuer.example".into(),
        client: "client".into(),
        resource: "https://server.example".into(),
        endpoint: "https://issuer.example/token".into(),
        access: "fixture-access".into(),
        refresh: Some("fixture-refresh".into()),
        expires,
    }
}
#[tokio::test]
async fn codex_plugin_compat_t28_invalidates_tokens_after_logout() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner", &grant(u64::MAX)).unwrap();
    assert!(read(dir.path(), "owner").unwrap().is_some());
    remove(dir.path(), "owner").unwrap();
    let result =
        access_with(dir.path(), "owner", |_| async { panic!("logged-out grant cannot refresh") })
            .await
            .unwrap();
    assert!(result.is_none());
}
#[tokio::test]
async fn codex_plugin_compat_t28_rejects_invalid_input() {
    assert!(
        Grant::response(
            &json!({"token_type":"bearer","access_token":"unsafe\nvalue","expires_in":60}),
            "issuer".into(),
            "client".into(),
            "resource".into(),
            "endpoint".into(),
            None
        )
        .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner", &grant(0)).unwrap();
    assert!(
        access_with(dir.path(), "owner", |_| async { anyhow::bail!("provider rejected refresh") })
            .await
            .is_err()
    );
    assert!(
        read(dir.path(), "owner").unwrap().is_none(),
        "failed refresh must invalidate old credentials"
    );
}
#[tokio::test]
async fn codex_plugin_compat_t28_preserves_isolation() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner-a", &grant(0)).unwrap();
    write(dir.path(), "owner-b", &grant(u64::MAX)).unwrap();
    let access = access_with(dir.path(), "owner-a", |old| async move {
        assert_eq!(old.refresh.as_deref(), Some("fixture-refresh"));
        let mut new = grant(u64::MAX);
        new.access = "rotated-access".into();
        Ok(new)
    })
    .await
    .unwrap();
    assert_eq!(access.as_deref(), Some("rotated-access"));
    assert_eq!(read(dir.path(), "owner-b").unwrap().unwrap().access, "fixture-access");
    let bytes = std::fs::read_to_string(dir.path().join("owner-a.grant")).unwrap();
    assert!(!bytes.contains("rotated-access"));
    remove(dir.path(), "owner-a").unwrap();
    assert!(read(dir.path(), "owner-b").unwrap().is_some());
}

#[test]
fn codex_plugin_compat_t28_rejects_transplanted_ciphertext() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner-a", &grant(u64::MAX)).unwrap();
    std::fs::copy(dir.path().join("owner-a.grant"), dir.path().join("owner-b.grant")).unwrap();
    assert!(read(dir.path(), "owner-b").is_err(), "ciphertext must authenticate its owner binding");
}

