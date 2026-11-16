use super::*;

#[tokio::test]
async fn cron_handlers_manage_shell_job_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();
    let mut body = add_body();
    body.schedule_kind = Some("every".to_string());
    body.schedule = None;
    body.every_ms = Some(60_000);
    body.command = Some("echo ok".to_string());
    body.project_path = Some("/tmp/project".to_string());

    let (status, add_json) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(body),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(add_json["status"], "ok");
    assert_eq!(add_json["job"]["schedule_kind"], "every");
    let job_id = add_json["job"]["id"].as_str().unwrap().to_string();

    let (status, list_json) = response_json(
        handle_api_cron_list(State(state_with_config(config.clone(), false)), headers.clone())
            .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list_json["jobs"].as_array().unwrap().len(), 1);

    let mut update = update_body();
    update.name = Some("Updated".to_string());
    update.schedule_kind = Some("cron".to_string());
    update.schedule = Some("0 */5 * * * *".to_string());
    update.command = Some("echo updated".to_string());
    update.enabled = Some(false);
    let (status, update_json) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id.clone()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update_json["job"]["name"], "Updated");
    assert_eq!(update_json["job"]["enabled"], false);
    assert_eq!(update_json["job"]["project_path"], "/tmp/project");

    let (status, clear_json) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id.clone()),
            JsonResponse(CronUpdateBody { project_path: Some(String::new()), ..update_body() }),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(clear_json["job"]["project_path"], serde_json::Value::Null);
    assert_eq!(agent_cron::list_jobs(&config.lock()).unwrap()[0].project_path, None);

    let (status, runs_json) = response_json(
        handle_api_cron_runs(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(runs_json["runs"].as_array().unwrap().is_empty());

    let (status, delete_json) = response_json(
        handle_api_cron_delete(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(delete_json["status"], "ok");

    let (status, list_json) =
        response_json(handle_api_cron_list(State(state_with_config(config, false)), headers).await)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert!(list_json["jobs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn cron_add_handler_covers_agent_and_bad_request_branches() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let mut missing_command = add_body();
    missing_command.command = Some(" ".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(missing_command),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Shell command"));

    let mut bad_schedule = add_body();
    bad_schedule.schedule_kind = Some("every".to_string());
    bad_schedule.schedule = None;
    bad_schedule.every_ms = Some(0);
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(bad_schedule),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("every_ms"));

    let mut invalid_cron = add_body();
    invalid_cron.schedule = Some("not a cron".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(invalid_cron),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Failed to add cron job"));

    let mut missing_prompt = add_body();
    missing_prompt.job_type = Some("agent".to_string());
    missing_prompt.command = None;
    missing_prompt.prompt = Some(" ".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(missing_prompt),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Agent prompt"));

    let mut agent = add_body();
    agent.job_type = Some("agent".to_string());
    agent.command = None;
    agent.prompt = Some("summarize daily state".to_string());
    agent.session_target = Some("main".to_string());
    agent.model = Some("model-a".to_string());
    agent.delivery_mode = Some("announce".to_string());
    agent.delivery_channel = Some("telegram".to_string());
    agent.delivery_to = Some("me".to_string());
    agent.delete_after_run = Some(true);
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(agent),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["job"]["job_type"], "agent");
    assert_eq!(body["job"]["delivery_mode"], "announce");
    assert_eq!(body["job"]["delete_after_run"], true);

    let mut unsupported = add_body();
    unsupported.job_type = Some("python".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config, false)),
            headers,
            JsonResponse(unsupported),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Unsupported job_type"));
}

#[tokio::test]
async fn cron_update_and_delete_handlers_report_errors() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let mut update = update_body();
    update.schedule_kind = Some("at".to_string());
    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("RFC3339"));

    let mut update = update_body();
    update.job_type = Some("invalid".to_string());
    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Invalid job type"));

    let (status, body) = response_json(
        handle_api_cron_delete(
            State(state_with_config(config, false)),
            headers,
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"].as_str().unwrap().contains("Failed to remove"));
}

#[tokio::test]
async fn cron_update_handler_reports_missing_job_after_valid_patch() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let mut update = update_body();
    update.name = Some("Updated".to_string());
    update.delivery_channel = Some(" telegram ".to_string());

    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config, false)),
            HeaderMap::new(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"].as_str().unwrap().contains("Failed to update cron job"));
}
