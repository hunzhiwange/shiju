use super::*;
#[tokio::test]
async fn local_import_runtime_supplies_real_peer_address() {
    let opts = ServeOptions { hostname: "127.0.0.1".into(), port: 0, cors: vec![] };
    let (addr, handle) = start(opts).await.expect("start gateway");
    let response = reqwest::Client::new().post(format!("http://{addr}/v1/files/import-local"))
        .header(vw_api_transport::PLATFORM_ID_HEADER, "platform-test")
        .header(vw_api_transport::COMPANY_ID_HEADER, "company-test")
        .json(&serde_json::json!({"project_id":"missing-local-import-project","target_directory":"","source_paths":["/missing"]}))
        .send().await.expect("request");
    // A real loopback peer reaches project validation rather than a missing-peer 403.
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    handle.abort();
}
