use super::AppRoutingError;
use crate::client::test_support;
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticJobIdentity, DiagnosticJobState, DiagnosticOutcome, DiagnosticProbeKind,
        DiagnosticResult,
    },
    id::{ProjectId, TaskId},
};

fn identity() -> Value {
    json!({
        "taskId": "fixture/job",
        "correlationId": "fixture-correlation",
        "scope": {
            "project": {"platform_id": "p", "company_id": "c", "project_id": "fixture"},
            "targetHostId": "mac /",
            "appId": "claude-desktop"
        },
        "kind": "outbound_scan"
    })
}

#[test]
fn cc_route_030_sdk_decodes_outbound_scan_identity() {
    let wire = identity();
    let job: DiagnosticJobIdentity =
        serde_json::from_value(wire.clone()).expect("SDK must decode the proxy job kind");
    assert_eq!(serde_json::to_value(job).unwrap(), wire);
}

#[tokio::test]
async fn cc_route_030_sdk_starts_reads_and_cancels_with_typed_evidence() {
    let snapshot = json!({
        "job": identity(),
        "state": {"status":"succeeded"},
        "progress": {"completed":1,"total":8},
        "results": [{
            "kind":"outbound_scan",
            "report":{
                "source":"real_service",
                "timings":[{"phase":"connect","elapsedMs":17}],
                "outcome":{"status":"succeeded","evidence":{"port":7890,"tcpOpen":true,"protocolCandidates":["http","socks5"]}}
            }
        }]
    });
    let cancelling = json!({
        "job": identity(),
        "state": {"status":"cancelling"},
        "progress": {"completed":0,"total":8}
    });
    let missing = json!({"error":{"code":"not_found","message":"Diagnostic task not found"}});
    let server = test_support::server(vec![
        (202, identity()),
        (200, snapshot),
        (200, cancelling),
        (404, missing),
    ]);
    let client = server.client();
    let host = "mac /";
    let project = ProjectId::from("fixture ?");
    let app = AppId::ClaudeDesktop;
    let token = "fixture-proxy-token";
    let job = client.app_routing_start_outbound_scan(host, &project, app, token).await.unwrap();
    assert_eq!(job.kind, DiagnosticProbeKind::OutboundScan);
    assert_eq!(job.task_id, TaskId::from("fixture/job"));
    let snapshot = client
        .app_routing_outbound_scan_status(host, &project, app, &job.task_id, token)
        .await
        .unwrap();
    let [DiagnosticResult::OutboundScan { report }] = snapshot.results.as_deref().unwrap() else {
        panic!("expected one typed proxy report");
    };
    assert_eq!(report.timings[0].elapsed_ms, 17);
    assert!(
        matches!(&report.outcome, DiagnosticOutcome::Succeeded { evidence } if evidence.port == 7890 && evidence.tcp_open && evidence.protocol_candidates == ["http", "socks5"])
    );
    let snapshot = client
        .app_routing_cancel_outbound_scan(host, &project, app, &job.task_id, token)
        .await
        .unwrap();
    assert_eq!(snapshot.state, DiagnosticJobState::Cancelling {});
    assert!(matches!(
        client.app_routing_outbound_scan_status(host, &project, app, &job.task_id, token).await,
        Err(AppRoutingError::Http { status: 404, body: Some(body) })
            if body.error.code == "not_found"
    ));
    let base =
        "/v1/app-routing/hosts/mac%20%2F/projects/fixture%20%3F/apps/claude-desktop/egress/scans";
    for (method, suffix, body) in [
        ("POST", "", json!({})),
        ("GET", "/fixture%2Fjob", Value::Null),
        ("DELETE", "/fixture%2Fjob", Value::Null),
        ("GET", "/fixture%2Fjob", Value::Null),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("{base}{suffix}"));
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.body, body);
    }
    server.join();
}
