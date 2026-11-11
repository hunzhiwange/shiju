//! Full router/authentication tests; the bridge-only harness intentionally excludes these.
use super::*;
use serde_json::json;
use tokio_tungstenite::tungstenite::{Error, client::IntoClientRequest};
const TOKEN: &str = "voice10-synthetic-gateway-key";
struct Server {
    base: String,
    platform: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server() -> Server {
    let platform = format!("voice10-{}", uuid::Uuid::new_v4());
    for company in ["owner", "other"] {
        crate::config::update_global_for_tenant(
            &TenantScope::new(&platform, company).unwrap(),
            json!({
                "gateway":{"auth_enabled":true,"skeys":[{"skey":TOKEN,"name":"fixture"}]},
                "app_ui":{"voice":{"realtime_enabled":true,"provider_id":"openai"}}
            }),
        )
        .await
        .unwrap();
    }
    let info=serde_json::from_value(json!({"id":"chat10","slug":"fixture","projectID":"voice10-project","directory":"/fixture","title":"fixture","version":"1","time":{"created":1,"updated":1}})).unwrap();
    crate::session::ui_store::save_agent_session_scoped_for_tenant(
        &TenantScope::new(&platform, "owner").unwrap(),
        &info,
        Some("voice10-project"),
    )
    .unwrap();
    let app = router::<()>()
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Server { base, platform, task }
}
async fn create(server: &Server, company: &str) -> reqwest::Response {
    reqwest::Client::new().post(format!("{}/voice/sessions",server.base)).bearer_auth(TOKEN)
        .header("x-platform-id",&server.platform).header("x-company-id",company)
        .header("x-shiju-voice-connection-id","desktop-a")
        .json(&json!({"request_id":"req10","chat_session_id":"chat10","generation":1,"provider_id":"openai","turn_mode":"manual"}))
        .send().await.unwrap()
}
async fn rejected_upgrade(
    server: &Server,
    id: &str,
    company: &str,
    connection: &str,
    auth: bool,
) -> u16 {
    let mut request =
        format!("{}/voice/sessions/{id}/stream", server.base.replace("http://", "ws://"))
            .into_client_request()
            .unwrap();
    let h = request.headers_mut();
    h.insert("x-platform-id", server.platform.parse().unwrap());
    h.insert("x-company-id", company.parse().unwrap());
    h.insert("x-shiju-voice-connection-id", connection.parse().unwrap());
    if auth {
        h.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    }
    match tokio_tungstenite::connect_async(request).await {
        Err(Error::Http(response)) => response.status().as_u16(),
        _ => panic!("upgrade unexpectedly accepted"),
    }
}
#[tokio::test]
async fn voice_task_10_upgrade_auth_and_tenant_desktop_isolation() {
    let server = server().await;
    assert_eq!(create(&server, "other").await.status(), StatusCode::FORBIDDEN);
    let response = create(&server, "owner").await;
    assert_eq!(response.status(), StatusCode::OK);
    let session: vw_api_types::voice::VoiceSessionStarted = response.json().await.unwrap();
    let id = session.voice_session_id.as_ref();
    assert_eq!(rejected_upgrade(&server, id, "owner", "desktop-a", false).await, 401);
    assert_eq!(rejected_upgrade(&server, id, "other", "desktop-a", true).await, 403);
    assert_eq!(rejected_upgrade(&server, id, "owner", "desktop-b", true).await, 403);
}
