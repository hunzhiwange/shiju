#[tokio::test]
async fn channel_first_response_hides_ordinary_and_explicit_drafts() {
    let (_temp, ctx, mut msg) = runtime_command::session_commands_tests::lifecycle_fixture().await;
    channel_runtime::first_response_contracts::channel_first_response_hides_ordinary_and_explicit_drafts(manager::contract_host(), &ctx, &mut msg).await;
}

#[tokio::test]
async fn channel_first_response_reuses_failed_ingress_then_publishes_success() {
    let (_temp, ctx, mut msg) = runtime_command::session_commands_tests::lifecycle_fixture().await;
    let id = channel_runtime::first_response_contracts::channel_first_response_reuses_failed_ingress_then_publishes_success(manager::contract_host(), &ctx, &mut msg).await;
    let history = crate::session::message::messages(&id, None).await.unwrap();
    assert_eq!(history.len(), 2, "only the successful turn was committed");
}

#[tokio::test]
async fn channel_first_response_requests_title_once_after_commit() {
    use axum::{Json, Router, routing::{get, post}};
    use runtime_command::session_commands_tests::lifecycle_fixture;
    use serde_json::{Value, json};

    let (_temp, ctx, mut msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let posted = requests.clone();
    let service = Router::new()
        .route("/v1/models", get(|| async { Json(json!({"data": [{"id": "default-model"}]})) }))
        .route("/v1/chat/completions", post(move |Json(body): Json<Value>| {
            let posted = posted.clone();
            async move {
                posted.lock().unwrap().push(body);
                ([("content-type", "text/event-stream")], concat!(
                    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Fixture title\"}}]}\n\n",
                    "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                ))
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    const KEY: &str = "VW_CHANNEL_FIRST_RESPONSE_FIXTURE_KEY";
    struct RemoveKey;
    impl Drop for RemoveKey {
        fn drop(&mut self) { agent_env::remove(KEY); }
    }
    agent_env::set(KEY, "local-fixture");
    let _key = RemoveKey;
    crate::config::update_project_for_tenant(
        &snapshot.scope.chat.tenant,
        &snapshot.directory,
        json!({"providers": {"default-provider": {
            "api": format!("http://{address}/v1"),
            "env": [KEY],
            "models": {"default-model": {"status": "active"}}
        }}}),
    )
    .await
    .unwrap();
    channel_runtime::first_response_contracts::channel_first_response_requests_title_once_after_commit(manager::contract_host(), &ctx, &mut msg, &requests).await;
    server.abort();
}
