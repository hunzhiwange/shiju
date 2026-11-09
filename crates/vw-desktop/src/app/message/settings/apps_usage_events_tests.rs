use super::apps_usage_events::refresh_batch;
use crate::app::components::apps_usage_cards::{
    Failure,
    tests::{gateway, request, respond},
};
use vw_gateway_client::vw_api_types::id::ProjectId;

const QUEUED: &str = r#"{"operationId":"op-1","status":"queued"}"#;
const ITEMS: &str = r#"[
    {"appId":"codex","providerId":"provider-a","accountId":"account-a","kind":"official_subscription","status":"completed","snapshotVersion":"2"},
    {"appId":"codex","providerId":"provider-b","accountId":"account-b","kind":"official_subscription","status":"failed","errorCode":"reauth_required"}
]"#;
const PRO: &str = r#"{
    "capabilities":["official_subscription"],
    "snapshot":{"result":{"source":{"kind":"official","id":"codex"},"credentialState":"valid",
    "outcome":{"status":"success","data":{"kind":"official_subscription","value":{"subscriptionType":"Pro","windows":[]}}}},"stale":false}
}"#;
const EMPTY: &str = r#"{"capabilities":["official_subscription"],"snapshot":null}"#;

fn operation(project: &str, status: &str, phase: &str, items: &str) -> String {
    format!(
        r#"{{"operationId":"op-1","status":"{status}","phase":"{phase}","projectId":"{project}","targetHostId":"local","correlationId":"op-1","cancelRequested":false,"items":{items}}}"#
    )
}

#[tokio::test]
async fn usage_usage_events_returns_expected_result() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, _, body) = request(&listener, "POST /v1/apps/usage/refresh-all ");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"projectId":"project-a","targetHostId":"local"})
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
        respond(&mut stream, "200 OK", &operation("project-a", "failed", "terminal", ITEMS));
        for (suffix, body) in [("a", PRO), ("b", EMPTY)] {
            let (mut stream, header, _) = request(&listener, "GET /v1/apps/usage/snapshot?");
            for field in [
                "projectId=project-a",
                "targetHostId=local",
                "appId=codex",
                &format!("providerId=provider-{suffix}"),
                &format!("accountId=account-{suffix}"),
                "kind=official_subscription",
            ] {
                assert!(header.contains(field), "missing {field}");
            }
            respond(&mut stream, "200 OK", body);
        }
    });
    let result = refresh_batch(&client, &ProjectId::from("project-a"), "local", "fixture-token")
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].snapshot.as_ref().unwrap().snapshot.is_some());
    assert_eq!(result[0].refresh_failure, None);
    assert!(result[1].snapshot.as_ref().unwrap().snapshot.is_none());
    assert_eq!(result[1].refresh_failure, Some(Failure::Reauth));
    server.join().unwrap();
}

#[tokio::test]
async fn usage_usage_events_rejects_invalid_input() {
    let (listener, client) = gateway();
    let server = std::thread::spawn(move || {
        let (mut stream, _, _) = request(&listener, "POST /v1/apps/usage/refresh-all ");
        respond(&mut stream, "202 Accepted", QUEUED);
        let (mut stream, _, _) =
            request(&listener, "GET /v1/agent-apps/operations/op-1?targetHostId=local");
        respond(&mut stream, "200 OK", &operation("other-project", "failed", "terminal", ITEMS));
    });
    let result =
        refresh_batch(&client, &ProjectId::from("project-a"), "local", "fixture-token").await;
    assert!(matches!(result, Err(Failure::InvalidResponse)));
    server.join().unwrap();
}
