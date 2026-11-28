use super::{gateway, request, respond};
use crate::apps_usage_cards::Failure;
use crate::refresh::refresh_one;
use vw_api_apps_usage::apps_usage::UsageSnapshotQuery;

const QUEUED: &str = r#"{"operationId":"op-1","status":"queued"}"#;
const ITEM: &str = r#"[{"appId":"codex","providerId":"provider-a","accountId":"account-a","kind":"official_subscription","status":"completed","snapshotVersion":"2"}]"#;

fn selection() -> UsageSnapshotQuery {
    serde_json::from_value(serde_json::json!({
        "projectId": "project-a",
        "targetHostId": "local",
        "appId": "codex",
        "providerId": "provider-a",
        "accountId": "account-a",
        "kind": "official_subscription",
    }))
    .unwrap()
}

fn operation(project: &str, status: &str, phase: &str, items: &str) -> String {
    format!(
        r#"{{"operationId":"op-1","status":"{status}","phase":"{phase}","projectId":"{project}","targetHostId":"local","correlationId":"op-1","cancelRequested":false,"items":{items}}}"#
    )
}

#[tokio::test]
async fn usage_usage_events_returns_expected_result() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, _, body) = request(&listener, "POST /v1/apps/usage/refresh ");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            serde_json::to_value(selection()).unwrap()
        );
        respond(&mut stream, "202 Accepted", QUEUED);

        let (mut stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1?targetHostId=local");
        respond(&mut stream, "200 OK", &operation("project-a", "running", "executing", "[]"));
        let (stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1/events?targetHostId=local");
        drop(stream);
        let (stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1?targetHostId=local");
        drop(stream);

        let (mut stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1?targetHostId=local");
        respond(&mut stream, "200 OK", &operation("project-a", "completed", "terminal", ITEM));
    });
    refresh_one(&client, &selection(), "fixture-token").await.unwrap();
    server.join().unwrap();
}

#[tokio::test]
async fn usage_usage_events_rejects_invalid_input() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, _, _) = request(&listener, "POST /v1/apps/usage/refresh ");
        respond(&mut stream, "202 Accepted", QUEUED);
        let (mut stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1?targetHostId=local");
        respond(&mut stream, "200 OK", &operation("other-project", "completed", "terminal", ITEM));
    });
    let result = refresh_one(&client, &selection(), "fixture-token").await;
    assert!(matches!(result, Err(Failure::InvalidResponse)));
    server.join().unwrap();
}
