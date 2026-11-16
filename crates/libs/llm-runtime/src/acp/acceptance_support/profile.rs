//! Real HTTP credential reissue contract; the caller supplies the native relay server.
use crate::acp::model_source::profile::AcpLaunchProfile;
pub async fn http_reissue<F, Fut>(id: model_transport::RelayIdentity, start: F)
where
    F: Fn(std::sync::Arc<model_transport::RelayBinding>) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<model_relay_http::RelayServer>>,
{
    let mut profile = AcpLaunchProfile::new(&Default::default(), id.clone()).unwrap();
    let token = profile.binding.bearer_token().to_owned();
    let server = start(profile.binding.clone()).await.unwrap();
    let url = format!("http://{}/v1/chat/completions", server.address());
    profile.attach_server(server);
    let client = model_transport::upstream_http_client().unwrap();
    let body =
        serde_json::json!({"model": "model", "messages": [{"role": "user", "content": "hello"}]});
    assert_eq!(
        client.post(&url).bearer_auth(&token).json(&body).send().await.unwrap().status(),
        200
    );
    let reference = profile.config().env[crate::acp::model_source::profile::REFERENCE_ENV].clone();
    drop(profile);
    let mut restored = AcpLaunchProfile::new(&Default::default(), id).unwrap();
    assert_eq!(reference, restored.config().env[crate::acp::model_source::profile::REFERENCE_ENV]);
    let server = start(restored.binding.clone()).await.unwrap();
    let url = format!("http://{}/v1/chat/completions", server.address());
    restored.attach_server(server);
    assert_eq!(
        client.post(&url).bearer_auth(token).json(&body).send().await.unwrap().status(),
        401
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(restored.binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
}
