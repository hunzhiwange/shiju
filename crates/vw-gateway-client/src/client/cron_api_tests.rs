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
