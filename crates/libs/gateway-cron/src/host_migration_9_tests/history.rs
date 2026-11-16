use super::*;

#[tokio::test]
async fn cron_history_endpoint_merges_active_and_finished() {
    use agent_cron as cron;
    let tmp = tempfile::tempdir().unwrap();
    let config = temp_config(tmp.path());
    let cfg = config.lock().clone();
    let state = state_with_config(config.clone(), false);
    let (status, empty) = history_request(state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["runs"], serde_json::json!([]));
    let job = cron::add_shell_job(
        &cfg,
        Some("任务名".into()),
        Schedule::Every { every_ms: 60_000 },
        "echo ok",
    )
    .unwrap();
    let now = Utc::now();
    let guard = cron::active_runs::register(&cfg, &job, now);
    let (_, active) = history_request(state.clone()).await;
    assert_eq!(active["runs"].as_array().unwrap().len(), 1);
    assert_eq!(active["runs"][0]["status"], "running");
    assert_eq!(active["runs"][0]["name"], "任务名");
    assert!(active["runs"][0]["execution_id"].is_string());
    for field in ["id", "finished_at", "duration_ms", "output"] {
        assert!(active["runs"][0][field].is_null());
    }
    cron::record_run(&cfg, &job.id, now, now, "error", Some(&"文".repeat(20000)), 4).unwrap();
    let (_, finished) = history_request(state.clone()).await;
    assert_eq!(finished["runs"].as_array().unwrap().len(), 1);
    assert_eq!(finished["runs"][0]["status"], "error");
    assert!(finished["runs"][0]["execution_id"].is_null());
    assert!(finished["runs"][0]["output"].as_str().unwrap().len() <= 16 * 1024);
    assert_eq!(finished["finished_limit"], 200);
    assert_eq!(finished["truncated"], false);
    drop(guard);
    let deleted = cron::active_runs::register(&cfg, &job, now + chrono::Duration::seconds(1));
    cron::remove_job(&cfg, &job.id).unwrap();
    assert_eq!(history_request(state).await.1["runs"], serde_json::json!([]));
    drop(deleted);
}

#[tokio::test]
async fn cron_history_endpoint_reports_finished_window_and_workspace_isolation() {
    use agent_cron as cron;
    let tmp = tempfile::tempdir().unwrap();
    let config = temp_config(tmp.path());
    config.lock().cron.max_run_history = 300;
    let cfg = config.lock().clone();
    let state = state_with_config(config, false);
    let job = cron::add_job(&cfg, "0 0 * * * *", "echo ok").unwrap();
    let now = Utc::now();
    for i in 0..200 {
        let at = now + chrono::Duration::seconds(i);
        cron::record_run(&cfg, &job.id, at, at, "ok", None, 0).unwrap();
    }
    let (_, window) = history_request(state.clone()).await;
    assert_eq!(window["runs"].as_array().unwrap().len(), 200);
    assert_eq!(window["truncated"], false);
    cron::record_run(&cfg, &job.id, now - chrono::Duration::seconds(1), now, "unknown", None, 0)
        .unwrap();
    let guard = cron::active_runs::register(&cfg, &job, now + chrono::Duration::seconds(201));
    let (_, window) = history_request(state).await;
    assert_eq!(window["runs"].as_array().unwrap().len(), 201);
    assert_eq!(window["truncated"], true);
    assert_eq!(window["runs"][0]["status"], "running");
    let other = tempfile::tempdir().unwrap();
    let (_, isolated) = history_request(state_with_config(temp_config(other.path()), false)).await;
    assert_eq!(isolated["runs"], serde_json::json!([]));
    drop(guard);
}

#[tokio::test]
async fn cron_history_endpoint_respects_retention_and_cascade_deletion() {
    use agent_cron as cron;
    let tmp = tempfile::tempdir().unwrap();
    let config = temp_config(tmp.path());
    config.lock().cron.max_run_history = 2;
    let cfg = config.lock().clone();
    let state = state_with_config(config, false);
    let job = cron::add_shell_job(
        &cfg,
        Some("中文保留上限验收".into()),
        Schedule::Every { every_ms: 60_000 },
        "echo local-only",
    )
    .unwrap();
    let now = Utc::now();
    for i in 0..4 {
        let at = now + chrono::Duration::seconds(i);
        cron::record_run(&cfg, &job.id, at, at, "ok", Some(&format!("输出{i}")), 0).unwrap();
    }
    let (status, response) = history_request(state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let rows = response["runs"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["output"], "输出3");
    assert_eq!(rows[1]["output"], "输出2");
    assert_eq!(response["truncated"], false);
    cron::remove_job(&cfg, &job.id).unwrap();
    assert!(cron::list_runs(&cfg, &job.id, 10).unwrap().is_empty());
    assert_eq!(history_request(state).await.1["runs"], serde_json::json!([]));
}
