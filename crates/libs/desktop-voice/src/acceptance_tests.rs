//! Domain HTTP/WS lifecycle acceptance.
#[tokio::test]
async fn voice_task_19_gateway_http_ws_roundtrip_releases_connection() {
    let captions = super::acceptance_fixture::roundtrip().await.unwrap();
    assert_eq!(captions[0].1, "中文动态字幕 🌱");
    assert_eq!(captions[1].1, "English caption");
}
