use super::*;

pub fn configure_wecom(config: &mut Config) {
    config.channels_config.wecom = Some(serde_json::from_value(serde_json::json!({"bot_id":"bot","secret":"fixture-secret"})).unwrap());
    config.channels_config.wecom_webhook = Some(serde_json::from_value(serde_json::json!({"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture-secret"})).unwrap());
}

pub async fn cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target(state: CronState) {
    let config = state.config.clone();
    for channel in ["wecom", "wecom_webhook"] {
        let mut body = add_body();
        body.delivery_mode = Some("announce".into());
        body.delivery_channel = Some(channel.into());
        let target = if channel == "wecom" { "user-id" } else { "default" };
        body.delivery_to = Some(target.into());
        body.delivery_best_effort = Some(false);
        let (status, added) = response_json(handle_api_cron_add(State(state.clone()), HeaderMap::new(), JsonResponse(body)).await).await;
        assert_eq!(status, StatusCode::OK, "{added}");
        assert!(!added.to_string().contains("fixture-secret"));
        let id = added["job"]["id"].as_str().unwrap().to_string();
        let patch = CronUpdateBody { delivery_channel: Some(format!(" {channel} ")), delivery_best_effort:Some(true), ..Default::default() };
        let (status, updated) = response_json(handle_api_cron_update(State(state.clone()),HeaderMap::new(),Path(id.clone()),JsonResponse(patch)).await).await;
        assert_eq!(status, StatusCode::OK, "{updated}");
        assert_eq!(updated["job"]["delivery_mode"], "announce");
        assert_eq!(updated["job"]["delivery_channel"], channel);
        assert_eq!(updated["job"]["delivery_to"], target);
        assert_eq!(updated["job"]["delivery_best_effort"], true);
        let patch = CronUpdateBody { delivery_to:Some(" ".into()), ..Default::default() };
        let (status, _) = response_json(handle_api_cron_update(State(state.clone()),HeaderMap::new(),Path(id.clone()),JsonResponse(patch)).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(agent_cron::get_job(&config.lock(), &id).unwrap().delivery.to.as_deref(), Some(target));
    }
    let (_, listed) = response_json(handle_api_cron_list(State(state), HeaderMap::new()).await).await;
    assert_eq!(listed["jobs"].as_array().unwrap().len(), 2);
    assert!(!listed.to_string().contains("fixture-secret"));
}
