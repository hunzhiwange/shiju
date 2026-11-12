//! 验证定时任务的查询、运行记录、探测和变更请求。
//!
//! 用例明确区分可触发旧路由回退的兼容错误与必须原样上抛的其他网关错误。

use serde_json::json;

use crate::client::test_support;

use super::{CronAddRequest, CronUpdateRequest};

fn add_request() -> CronAddRequest {
    CronAddRequest {
        name: Some("nightly".to_string()),
        job_type: "command".to_string(),
        schedule_kind: "cron".to_string(),
        schedule: "0 0 * * *".to_string(),
        at: None,
        every_ms: None,
        command: Some("date".to_string()),
        prompt: None,
        session_target: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: Some("/work".to_string()),
        wake: Some(true),
        fallbacks: None,
        full_access: None,
        task_pool: None,
        delivery_mode: None,
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: None,
        delete_after_run: None,
        ..Default::default()
    }
}

#[tokio::test]
async fn cron_api_covers_list_runs_probe_mutations_and_fallbacks() {
    let server = test_support::server(vec![
        (
            200,
            json!({"jobs": [{
                "id": "job_1",
                "name": "nightly",
                "job_type": "command",
                "schedule_kind": "cron",
                "expression": "0 0 * * *",
                "at": null,
                "every_ms": null,
                "command": "date",
                "prompt": null,
                "model": null,
                "agent": null,
                "project_path": "/work",
                "wake": true,
                "fallbacks": [],
                "full_access": false,
                "task_pool": false,
                "delivery_mode": "ui",
                "delivery_channel": null,
                "delivery_to": null,
                "delete_after_run": false,
                "next_run": "2026-06-12T00:00:00Z",
                "last_run": null,
                "last_status": null,
                "last_output": null,
                "enabled": true
            }]}),
        ),
        (404, json!({"error": "missing primary"})),
        (
            200,
            json!({"runs": [{
                "id": 7,
                "job_id": "job_1",
                "started_at": "start",
                "finished_at": "finish",
                "status": "ok",
                "output": "done",
                "duration_ms": 12
            }]}),
        ),
        (404, json!({"error": "not found"})),
        (200, json!({"status": "ok", "job": null})),
        (405, json!({"error": "patch unsupported"})),
        (405, json!({"error": "post unsupported"})),
        (200, json!({"status": "ok", "job": null})),
        (204, json!({})),
    ]);

    let jobs = server.client().cron_jobs_get().await.expect("jobs");
    assert_eq!(jobs[0].id, "job_1");
    assert!(jobs[0].delivery_best_effort);

    let runs = server.client().cron_job_runs_get("job_1").await.expect("runs");
    assert_eq!(runs[0].id, 7);

    assert!(!server.client().cron_extended_api_supported().await.expect("probe"));

    let created = server.client().cron_job_add(&add_request()).await.expect("add");
    assert_eq!(created.status, "ok");

    let updated = server
        .client()
        .cron_job_update("job_1", &CronUpdateRequest { enabled: Some(false), ..Default::default() })
        .await
        .expect("update");
    assert_eq!(updated.status, "ok");

    server.client().cron_job_delete("job_1").await.expect("delete");

    assert_eq!(server.take_request().path, "/v1/cron");
    assert_eq!(server.take_request().path, "/v1/cron/job_1/runs");
    assert_eq!(server.take_request().path, "/v1/cron/runs/job_1");
    assert_eq!(server.take_request().path, "/v1/cron/runs/__probe__");
    let recorded = server.take_request();
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.path, "/v1/cron");
    assert_eq!(recorded.body["name"], "nightly");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "POST");
    let recorded = server.take_request();
    assert_eq!(recorded.method, "PUT");
    assert_eq!(recorded.path, "/v1/cron/job_1");
    assert_eq!(recorded.body["enabled"], false);
    let recorded = server.take_request();
    assert_eq!(recorded.method, "DELETE");
    assert_eq!(recorded.path, "/v1/cron/job_1");
    server.join();
}

#[tokio::test]
async fn cron_api_covers_primary_successes_and_legacy_mutation_errors() {
    let server = test_support::server(vec![
        (200, json!({"runs": []})),
        (404, json!({"error": "missing primary"})),
        (404, json!({"error": "missing fallback"})),
        (200, json!({"runs": []})),
        (500, json!({"error": "boom"})),
        (200, json!(null)),
        (200, json!({"status": "ok", "job": null})),
        (405, json!({"error": "patch unsupported"})),
        (200, json!({"status": "ok", "job": null})),
        (405, json!({"error": "patch unsupported"})),
        (200, json!(null)),
    ]);

    assert!(server.client().cron_job_runs_get("ok").await.expect("primary").is_empty());
    assert!(
        server
            .client()
            .cron_job_runs_get("missing")
            .await
            .expect_err("unsupported")
            .contains("定时任务历史接口")
    );
    assert!(server.client().cron_extended_api_supported().await.expect("probe ok"));
    assert!(
        server.client().cron_extended_api_supported().await.expect_err("probe err").contains("500")
    );
    assert_eq!(
        server.client().cron_job_add(&add_request()).await.expect("legacy add").status,
        "ok"
    );
    assert_eq!(
        server
            .client()
            .cron_job_update(
                "patch_ok",
                &CronUpdateRequest { enabled: Some(true), ..Default::default() }
            )
            .await
            .expect("patch ok")
            .status,
        "ok"
    );
    assert_eq!(
        server
            .client()
            .cron_job_update(
                "post_ok",
                &CronUpdateRequest { enabled: Some(true), ..Default::default() }
            )
            .await
            .expect("post ok")
            .status,
        "ok"
    );
    assert_eq!(
        server
            .client()
            .cron_job_update(
                "legacy_post",
                &CronUpdateRequest { enabled: Some(true), ..Default::default() }
            )
            .await
            .expect("legacy post")
            .status,
        "ok"
    );

    assert_eq!(server.take_request().path, "/v1/cron/ok/runs");
    assert_eq!(server.take_request().path, "/v1/cron/missing/runs");
    assert_eq!(server.take_request().path, "/v1/cron/runs/missing");
    assert_eq!(server.take_request().path, "/v1/cron/runs/__probe__");
    assert_eq!(server.take_request().path, "/v1/cron/runs/__probe__");
    assert_eq!(server.take_request().path, "/v1/cron");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "POST");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "POST");
    server.join();
}

#[tokio::test]
async fn cron_api_preserves_non_legacy_errors() {
    let server = test_support::server(vec![
        (500, json!({"error": "primary failed"})),
        (404, json!({"error": "missing primary"})),
        (500, json!({"error": "fallback failed"})),
        (500, json!({"error": "add failed"})),
        (500, json!({"error": "patch failed"})),
        (405, json!({"error": "patch unsupported"})),
        (405, json!({"error": "post unsupported"})),
        (500, json!({"error": "put failed"})),
    ]);

    assert!(
        server
            .client()
            .cron_job_runs_get("primary")
            .await
            .expect_err("primary error")
            .contains("500")
    );
    assert!(
        server
            .client()
            .cron_job_runs_get("fallback")
            .await
            .expect_err("fallback error")
            .contains("500")
    );
    assert!(
        server.client().cron_job_add(&add_request()).await.expect_err("add error").contains("500")
    );
    assert!(
        server
            .client()
            .cron_job_update(
                "patch",
                &CronUpdateRequest { enabled: Some(true), ..Default::default() }
            )
            .await
            .expect_err("patch error")
            .contains("500")
    );
    assert!(
        server
            .client()
            .cron_job_update(
                "put",
                &CronUpdateRequest { enabled: Some(true), ..Default::default() }
            )
            .await
            .expect_err("put error")
            .contains("500")
    );

    assert_eq!(server.take_request().path, "/v1/cron/primary/runs");
    assert_eq!(server.take_request().path, "/v1/cron/fallback/runs");
    assert_eq!(server.take_request().path, "/v1/cron/runs/fallback");
    assert_eq!(server.take_request().path, "/v1/cron");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "PATCH");
    assert_eq!(server.take_request().method, "POST");
    assert_eq!(server.take_request().method, "PUT");
    server.join();
}

#[tokio::test]
async fn cron_api_encodes_job_ids_and_starts_updates_with_canonical_patch() {
    let server = test_support::server(vec![
        (200, json!({"runs": []})),
        (200, json!({"status": "ok", "job": null})),
        (204, json!(null)),
    ]);

    server.client().cron_job_runs_get("job /一").await.expect("runs");
    server
        .client()
        .cron_job_update(
            "job /一",
            &CronUpdateRequest { enabled: Some(false), ..Default::default() },
        )
        .await
        .expect("update");
    server.client().cron_job_delete("job /一").await.expect("delete");

    let runs = server.take_request();
    assert_eq!(runs.method, "GET");
    assert_eq!(runs.path, "/v1/cron/job%20%2F%E4%B8%80/runs");
    let update = server.take_request();
    assert_eq!(update.method, "PATCH");
    assert_eq!(update.path, "/v1/cron/job%20%2F%E4%B8%80");
    let delete = server.take_request();
    assert_eq!(delete.method, "DELETE");
    assert_eq!(delete.path, "/v1/cron/job%20%2F%E4%B8%80");
    server.join();
}

#[test]
fn wb_schedule_api_patch_window_distinguishes_missing_null_and_value() {
    let missing: CronUpdateRequest = serde_json::from_value(json!({})).unwrap();
    let clear: CronUpdateRequest = serde_json::from_value(json!({"starts_at":null})).unwrap();
    let set: CronUpdateRequest =
        serde_json::from_value(json!({"ends_at":"2099-01-01T00:00:00Z"})).unwrap();
    assert!(serde_json::to_value(missing).unwrap().get("starts_at").is_none());
    assert_eq!(serde_json::to_value(clear).unwrap().get("starts_at"), Some(&json!(null)));
    assert_eq!(serde_json::to_value(set).unwrap()["ends_at"], "2099-01-01T00:00:00Z");
}

#[tokio::test]
async fn wb_schedule_api_preview_transports_normalized_fields() {
    use super::{CronPreviewRequest, CronPreviewResponse};
    let normalized = json!({"schedule_kind":"biweekly", "anchor_date":"2099-01-01", "weekdays":[1,3], "hour":9, "minute":30, "timezone":"Asia/Shanghai", "starts_at":"2098-12-31T16:00:00+00:00", "ends_at":"2099-03-01T16:00:00+00:00"});
    let server = test_support::server(vec![(
        200,
        json!({"normalized":normalized, "timezone":"Asia/Shanghai", "candidates":["2099-01-05T01:30:00+00:00"]}),
    )]);
    let request: CronPreviewRequest = serde_json::from_value(json!({"schedule_kind":"biweekly", "anchor_date":"2099-01-01", "weekdays":[1,3], "hour":9, "minute":30, "timezone":"Asia/Shanghai", "start_date":"2099-01-01", "end_date":"2099-03-01"})).unwrap();
    let result: CronPreviewResponse =
        server.client().cron_schedule_preview(&request).await.unwrap();
    assert_eq!(result.timezone, "Asia/Shanghai");
    assert_eq!(result.normalized.weekdays, Some(vec![1, 3]));
    assert_eq!(result.normalized.starts_at, Some(Some("2098-12-31T16:00:00+00:00".into())));
    assert_eq!(result.candidates, vec!["2099-01-05T01:30:00+00:00"]);
    let sent = server.take_request();
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.path, "/v1/cron/preview");
    assert_eq!(sent.body["timezone"], "Asia/Shanghai");
    assert_eq!(sent.body["start_date"], "2099-01-01");
    assert_eq!(sent.body["weekdays"], json!([1, 3]));
    server.join();
}

#[tokio::test]
async fn wb_schedule_api_new_fields_reject_unsupported_gateway_before_mutating() {
    for (status, response) in [
        (404, json!({"error":"unsupported"})),
        (405, json!({"error":"unsupported"})),
        (500, json!({"error":"Failed to update cron job: Cron job 'preview' not found"})),
        (200, json!({"status":"ok","job":null})),
    ] {
        let server = test_support::server(vec![(status, response)]);
        let request =
            super::CronAddRequest { timezone: Some("Asia/Shanghai".into()), ..add_request() };
        let error = server.client().cron_job_add(&request).await.unwrap_err();
        assert!(error.contains("不支持"), "{error}");
        assert_eq!(server.take_request().path, "/v1/cron/preview");
        server.join();
    }
}

#[tokio::test]
async fn wb_schedule_api_new_update_never_retries_legacy_methods() {
    let server = test_support::server(vec![
        (
            200,
            json!({"normalized":{"schedule_kind":"every","every_ms":1000,"timezone":"UTC"},"timezone":"UTC","candidates":["2099-01-01T00:00:00+00:00"]}),
        ),
        (405, json!({"error":"unsupported patch"})),
    ]);
    let request = CronUpdateRequest { ends_at: Some(None), ..Default::default() };
    assert!(server.client().cron_job_update("job-1", &request).await.unwrap_err().contains("405"));
    assert_eq!(server.take_request().path, "/v1/cron/preview");
    let sent = server.take_request();
    assert_eq!(sent.method, "PATCH");
    assert_eq!(sent.body.get("ends_at"), Some(&json!(null)));
    server.join();
}

#[tokio::test]
async fn wb_schedule_api_local_at_and_date_fields_survive_transport() {
    let server = test_support::server(vec![
        (
            200,
            json!({"normalized":{"schedule_kind":"at","at":"2099-01-01T01:00:00+00:00","timezone":"UTC"},"timezone":"UTC","candidates":["2099-01-01T01:00:00+00:00"]}),
        ),
        (200, json!({"status":"ok","job":null})),
    ]);
    let request = super::CronAddRequest {
        schedule_kind: "at".into(),
        schedule: String::new(),
        local_at: Some("2099-01-01T09:00".into()),
        timezone: Some("Asia/Shanghai".into()),
        ..add_request()
    };
    server.client().cron_job_add(&request).await.unwrap();
    server.take_request();
    let sent = server.take_request();
    assert_eq!(sent.body["local_at"], "2099-01-01T09:00");
    assert_eq!(sent.body["timezone"], "Asia/Shanghai");
    server.join();
}

#[tokio::test]
async fn wb_schedule_api_capability_probe_rejects_legacy_before_updates() {
    let server = test_support::server(vec![(
        400,
        json!({"error":"RFC3339 time is required for at schedule"}),
    )]);
    let request = super::CronAddRequest { timezone: Some("UTC".into()), ..add_request() };
    let error = server.client().cron_job_add(&request).await.unwrap_err();
    let sent = server.take_request();
    // The old update parser requires absolute at, so this probe fails before touching a job.
    assert_eq!(sent.body["schedule_kind"], "at");
    assert!(sent.body.get("at").is_none());
    assert_eq!(sent.body["local_at"], "9999-01-01T00:00");
    assert_eq!(sent.body["timezone"], "UTC");
    assert!(error.contains("不支持"), "{error}");
    server.join();
}

#[tokio::test]
async fn cron_history_client_decodes_running_and_finished() {
    let server = test_support::server(vec![(
        200,
        json!({"runs":[
        {"id":null,"execution_id":"active-1","job_id":"a","name":null,"started_at":"start","finished_at":null,"duration_ms":null,"status":"running","output":null},
        {"id":1,"execution_id":null,"job_id":"b","name":"任务","started_at":"start","finished_at":"end","duration_ms":12,"status":"ok","output":"done"},
        {"id":2,"execution_id":null,"job_id":"b","name":"任务","started_at":"start","finished_at":"end","duration_ms":12,"status":"error","output":null},
        {"id":3,"execution_id":null,"job_id":"b","name":"任务","started_at":"start","finished_at":"end","duration_ms":12,"status":"unknown","output":null}
    ],"finished_limit":200,"truncated":true}),
    )]);
    let history = server.client().cron_history_get().await.unwrap();
    assert_eq!(history.runs.len(), 4);
    assert_eq!(history.runs[0].execution_id.as_deref(), Some("active-1"));
    assert!(history.runs[0].finished_at.is_none());
    assert_eq!(history.runs[1].id, Some(1));
    assert_eq!(history.runs[2].status, "error");
    assert_eq!(history.runs[3].status, "unknown");
    assert!(history.truncated);
    assert_eq!(history.finished_limit, 200);
    assert_eq!(server.take_request().path, "/v1/cron/history");
    server.join();
}

#[tokio::test]
async fn cron_history_client_propagates_errors() {
    let server = test_support::server(vec![
        (401, json!({"error":"unauthorized"})),
        (404, json!({"error":"unsupported"})),
        (500, json!({"error":"failed"})),
    ]);
    for code in ["401", "404", "500"] {
        assert!(server.client().cron_history_get().await.unwrap_err().contains(code));
        assert_eq!(server.take_request().path, "/v1/cron/history");
    }
    server.join();
}

#[tokio::test]
async fn cron_history_client_reports_network_and_decode_errors() {
    let server = test_support::server(vec![]);
    let client = server.client().clone();
    server.join();
    assert!(client.cron_history_get().await.is_err());
    let server = test_support::server(vec![(200, json!({"runs":[]}))]);
    assert!(server.client().cron_history_get().await.is_err());
    assert_eq!(server.take_request().path, "/v1/cron/history");
    server.join();
}
