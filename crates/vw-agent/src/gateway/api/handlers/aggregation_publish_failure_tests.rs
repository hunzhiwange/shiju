use super::*;
#[tokio::test]
async fn aggregation_t41_unavailable_controller_does_not_claim_publication() {
    let f = Fixture::new();
    f.policy(true, true, true);
    let request = f.publish_request().await;
    let before = f.tx.state(AppId::Claude).unwrap();
    let (status, body) = f.publish(request).await;
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["error"]["code"], "target_unavailable");
    assert!(f.tx.state(AppId::Claude).unwrap() == before);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    assert!(!f.native().exists());
}
#[tokio::test]
async fn aggregation_t41_native_io_failure_reports_target_failed_without_a_false_commit() {
    let mut f = Fixture::new();
    f.prepare_native();
    let listener = f.listener();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        let before = store.claude_record(&tenant(), id).unwrap();
        store
            .claude_save(
                &tenant(),
                id,
                &vw_api_types::app_providers::claude::ClaudeProfile {
                    name: id.into(),
                    config: vw_api_types::app_providers::claude::ClaudeConfig::ThirdParty {
                        base_url: format!("http://{address}"),
                        api_key: "synthetic-private-key".into(),
                        api_key_header: true,
                    },
                    model: Some(model.into()),
                },
                before.as_ref(),
                false,
            )
            .unwrap();
    }
    let request = f.publish_request().await;
    let before = f.tx.state(AppId::Claude).unwrap();
    let bytes = std::fs::read(f.native()).unwrap();
    let resource = f.root.path().join("private/provider-live/aG9zdA");
    struct RestorePermissions(std::path::PathBuf);
    impl Drop for RestorePermissions {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
        }
    }
    let _restore = RestorePermissions(resource.clone());
    drop(f.tx.lock_claude_directory().unwrap());
    std::fs::set_permissions(&resource, std::fs::Permissions::from_mode(0o500)).unwrap();
    let (status, progress) = f.publish(request).await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "failed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["code"], "target_failed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["status"], 502);
    assert_eq!(terminal["aggregationOutcome"]["recovery"], "none");
    assert!(f.tx.state(AppId::Claude).unwrap() == before);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    listener.stop().await.unwrap();
}
