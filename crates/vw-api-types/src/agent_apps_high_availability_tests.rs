use super::agent_apps_high_availability::{
    FailoverCandidatesResponse, FailoverQueueEnabledRequest, FailoverQueueEntry,
    FailoverQueueMemberRequest, FailoverQueueQuery, FailoverQueueReorderRequest,
    FailoverQueueResponse, FailoverQueueScope, FailoverQueueVersion,
};
use crate::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};
use serde_json::json;

#[test]
fn ha_queue_dto_roundtrip_keeps_scope_and_order() {
    let query = FailoverQueueQuery {
        project_id: "project-1".into(),
        target_host_id: "host-1".into(),
        app_id: AppId::Claude,
    };
    let query_wire = json!({
        "projectId": "project-1",
        "targetHostId": "host-1",
        "appId": "claude"
    });
    assert_eq!(serde_json::to_value(&query).unwrap(), query_wire);
    assert_eq!(serde_json::from_value::<FailoverQueueQuery>(query_wire.clone()).unwrap(), query);
    let mut unsafe_query = query_wire;
    unsafe_query["companyId"] = json!("tenant-override");
    assert!(serde_json::from_value::<FailoverQueueQuery>(unsafe_query).is_err());

    let scope = FailoverQueueScope {
        project: ProjectScope::new(
            TenantScope::new("platform-1", "company-1").unwrap(),
            query.project_id,
        )
        .unwrap(),
        target_host_id: query.target_host_id,
        app_id: query.app_id,
    };
    let queue = FailoverQueueResponse {
        scope: scope.clone(),
        version: FailoverQueueVersion(17),
        enabled: true,
        entries: vec![
            FailoverQueueEntry { provider_id: "provider-z".into(), name: "Primary".into() },
            FailoverQueueEntry { provider_id: "provider-a".into(), name: "Backup".into() },
        ],
    };
    let scope_wire = json!({
        "project": {
            "platform_id": "platform-1",
            "company_id": "company-1",
            "project_id": "project-1"
        },
        "targetHostId": "host-1",
        "appId": "claude"
    });
    let queue_wire = json!({
        "scope": scope_wire,
        "version": 17,
        "enabled": true,
        "entries": [
            {"providerId": "provider-z", "name": "Primary"},
            {"providerId": "provider-a", "name": "Backup"}
        ]
    });
    assert_eq!(serde_json::to_value(&queue).unwrap(), queue_wire);
    assert_eq!(serde_json::from_value::<FailoverQueueResponse>(queue_wire.clone()).unwrap(), queue);

    let candidates = FailoverCandidatesResponse {
        scope,
        candidates: vec![FailoverQueueEntry {
            provider_id: "provider-b".into(),
            name: "Candidate".into(),
        }],
    };
    let candidates_wire = json!({
        "scope": scope_wire,
        "candidates": [{"providerId": "provider-b", "name": "Candidate"}]
    });
    assert_eq!(serde_json::to_value(&candidates).unwrap(), candidates_wire);
    assert_eq!(
        serde_json::from_value::<FailoverCandidatesResponse>(candidates_wire.clone()).unwrap(),
        candidates
    );

    let mut unsafe_candidates = candidates_wire;
    unsafe_candidates["candidates"][0]["settingsConfig"] = json!({"apiKey": "fixture-secret"});
    assert!(
        serde_json::from_value::<FailoverCandidatesResponse>(unsafe_candidates).is_err(),
        "candidate entries must reject provider configuration"
    );
    let mut unsafe_queue = queue_wire;
    unsafe_queue["entries"][0]["apiKey"] = json!("fixture-secret");
    assert!(serde_json::from_value::<FailoverQueueResponse>(unsafe_queue).is_err());

    let member_wire = json!({"expectedVersion": 17, "providerId": "provider-b"});
    let member = serde_json::from_value::<FailoverQueueMemberRequest>(member_wire.clone()).unwrap();
    assert_eq!(member.expected_version, queue.version);
    assert_eq!(serde_json::to_value(member).unwrap(), member_wire);
    assert!(
        serde_json::from_value::<FailoverQueueMemberRequest>(json!({
            "expectedVersion": 17, "providerId": "provider-b", "accountId": "override"
        }))
        .is_err()
    );

    let reorder_wire = json!({"expectedVersion": 17, "providerIds": ["provider-a", "provider-z"]});
    let reorder =
        serde_json::from_value::<FailoverQueueReorderRequest>(reorder_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(reorder).unwrap(), reorder_wire);

    let enabled_wire = json!({"expectedVersion": 17, "enabled": false});
    let enabled =
        serde_json::from_value::<FailoverQueueEnabledRequest>(enabled_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(enabled).unwrap(), enabled_wire);
    assert!(
        serde_json::from_value::<FailoverQueueEnabledRequest>(json!({
            "expectedVersion": 17, "enabled": false, "scope": scope_wire
        }))
        .is_err()
    );
}
