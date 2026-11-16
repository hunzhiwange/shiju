use super::*;

#[tokio::test]
async fn cron_wecom_delivery_add_rejects_invalid_target_without_creating_job() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = temp_config(tmp.path());
    let state = state_with_config(config.clone(), false);
    let mut body = add_body();
    body.delivery_mode = Some("announce".into());
    body.delivery_channel = Some("wecom_webhook".into());
    body.delivery_to =
        Some("https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=must-not-persist".into());
    let (status, value) = response_json(
        handle_api_cron_add(State(state), HeaderMap::new(), JsonResponse(body)).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{value}");
    assert!(!value.to_string().contains("must-not-persist"));
    assert!(agent_cron::list_jobs(&config.lock()).unwrap().is_empty());
}

#[tokio::test]
async fn cron_wecom_delivery_add_requires_configuration_but_none_does_not() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = temp_config(tmp.path());
    let state = state_with_config(config.clone(), false);
    for channel in ["wecom", "wecom_webhook"] {
        let mut body = add_body();
        body.delivery_mode = Some("announce".into());
        body.delivery_channel = Some(channel.into());
        body.delivery_to = Some(if channel == "wecom" { "userid" } else { "default" }.into());
        let (status, _) = response_json(
            handle_api_cron_add(State(state.clone()), HeaderMap::new(), JsonResponse(body)).await,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let mut body = add_body();
    body.delivery_mode = Some("none".into());
    body.delivery_channel = Some("wecom".into());
    let (status, _) = response_json(
        handle_api_cron_add(State(state), HeaderMap::new(), JsonResponse(body)).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(agent_cron::list_jobs(&config.lock()).unwrap().len(), 1);
}
