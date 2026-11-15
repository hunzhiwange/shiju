use super::mcp_management::{
    McpAcpMount, McpAppCapability, McpBatchOutcome, McpCatalogRecord, McpCliBinding,
    McpConnectionCheckResult, McpConnectionDefinition, McpConnectionOutcome,
    McpDistributionOutcome, McpDistributionResult, McpFailureCode, McpNativeStatus,
    McpRuntimeSelection, McpSupport,
};
use serde_json::json;

#[test]
fn cc_mcp_t03_unverified_app_capability_has_its_own_wire_status() {
    let status = serde_json::from_value::<McpSupport>(json!("not_verified"));
    assert!(status.is_ok(), "an unverified capability needs a valid wire status: {status:?}");
    assert_eq!(status.unwrap(), McpSupport::NotVerified);

    let input = json!({
        "appId": "agy",
        "stdio": "supported",
        "streamableHttp": "not_verified",
        "sse": "unsupported"
    });
    let capability: McpAppCapability = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(capability.stdio, McpSupport::Supported);
    assert_eq!(capability.streamable_http, McpSupport::NotVerified);
    assert_eq!(capability.sse, McpSupport::Unsupported);
    assert_eq!(serde_json::to_value(capability).unwrap(), input);
}

#[test]
fn cc_mcp_t03_connection_transports_round_trip_their_own_fields() {
    let credential = json!({
        "kind": "credential",
        "reference": {"service": "codex_oauth", "accountId": "account-1"}
    });
    let definitions = [
        json!({
            "transport": "stdio",
            "command": "test-mcp",
            "args": ["--name", " test "],
            "env": {"APP_MODE": {"kind": "literal", "value": "local"}, "TOKEN": credential},
            "cwd": "/tmp/fixture"
        }),
        json!({
            "transport": "streamable_http",
            "url": "http://127.0.0.1:1234/mcp",
            "headers": {"Authorization": credential}
        }),
        json!({
            "transport": "sse",
            "url": "http://127.0.0.1:1234/events"
        }),
    ];
    let [stdio, http, sse] = definitions.each_ref().map(|input| {
        let connection: McpConnectionDefinition = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&connection).unwrap(), *input);
        connection
    });
    assert!(matches!(stdio, McpConnectionDefinition::Stdio { .. }));
    assert!(matches!(http, McpConnectionDefinition::StreamableHttp { .. }));
    assert!(matches!(sse, McpConnectionDefinition::Sse { .. }));

    let wrong_fields =
        json!({"transport": "stdio", "command": "test-mcp", "url": "https://example.test"});
    assert!(serde_json::from_value::<McpConnectionDefinition>(wrong_fields).is_err());
    let unknown_transport = json!({"transport": "websocket", "url": "https://example.test"});
    assert!(serde_json::from_value::<McpConnectionDefinition>(unknown_transport).is_err());
}

#[test]
fn cc_mcp_t03_catalog_cli_runtime_and_acp_use_distinct_wire_records() {
    let reference = json!({"id": "server-a", "revision": 7});
    let project =
        json!({"platform_id": "platform-a", "company_id": "tenant-a", "project_id": "p1"});
    let target = json!({"appId": "codex", "installationId": "install-1", "targetHostId": "host-a"});
    let catalog = json!({
        "record": reference,
        "scope": {"kind": "project", "scope": project},
        "name": "Test MCP",
        "transport": "sse",
        "connectionRevision": 4
    });
    let cli = json!({
        "record": reference,
        "target": target,
        "enabled": true,
        "nativeStatus": "missing"
    });
    let runtime = json!({"record": reference, "project": project});
    let acp = json!({"record": reference, "sessionId": "session-a"});

    let catalog_record: McpCatalogRecord = serde_json::from_value(catalog.clone()).unwrap();
    let binding: McpCliBinding = serde_json::from_value(cli.clone()).unwrap();
    let selection: McpRuntimeSelection = serde_json::from_value(runtime.clone()).unwrap();
    let mount: McpAcpMount = serde_json::from_value(acp.clone()).unwrap();
    assert_eq!(binding.native_status, McpNativeStatus::Missing);
    assert_eq!(catalog_record.record, binding.record);
    assert_eq!(binding.record, selection.record);
    assert_eq!(selection.record, mount.record);
    assert_eq!(serde_json::to_value(catalog_record).unwrap(), catalog);
    assert_eq!(serde_json::to_value(binding).unwrap(), cli);
    assert_eq!(serde_json::to_value(selection).unwrap(), runtime);
    assert_eq!(serde_json::to_value(mount).unwrap(), acp);
    assert!(serde_json::from_value::<McpCatalogRecord>(cli.clone()).is_err());
    assert!(serde_json::from_value::<McpRuntimeSelection>(acp).is_err());
    assert!(serde_json::from_value::<McpAcpMount>(runtime).is_err());

    let mut path_target = cli;
    path_target["target"]["path"] = json!("/tmp/config.toml");
    assert!(serde_json::from_value::<McpCliBinding>(path_target).is_err());
    let mut raw_catalog = catalog;
    raw_catalog["connection"] = json!({"url": "https://example.test/?token=fixture"});
    assert!(serde_json::from_value::<McpCatalogRecord>(raw_catalog).is_err());
}

#[test]
fn cc_mcp_t03_distribution_preserves_partial_success_and_desired_state() {
    let target = json!({"appId": "codex", "installationId": "install-1", "targetHostId": "host-a"});
    let wire = json!({
        "outcome": "partially_succeeded",
        "items": [
            {
                "record": {"id": "server-a", "revision": 3},
                "target": target, "enabled": true, "desiredCommitted": true,
                "nativeOutcome": {"status": "applied"}
            },
            {
                "record": {"id": "server-b", "revision": 5},
                "target": target, "enabled": true, "desiredCommitted": true,
                "nativeOutcome": {"status": "failed", "code": "native_write_failed"}
            },
            {
                "record": {"id": "server-c", "revision": 2},
                "target": target, "enabled": true, "desiredCommitted": false,
                "nativeOutcome": {"status": "unsupported"}
            }
        ]
    });
    let result: McpDistributionResult = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(result.outcome, McpBatchOutcome::PartiallySucceeded);
    assert_eq!(result.items[0].native_outcome, McpDistributionOutcome::Applied);
    assert!(result.items[1].desired_committed);
    assert_eq!(
        result.items[1].native_outcome,
        McpDistributionOutcome::Failed { code: McpFailureCode::NativeWriteFailed }
    );
    assert!(!result.items[2].desired_committed);
    assert_eq!(result.items[2].native_outcome, McpDistributionOutcome::Unsupported);
    assert_eq!(serde_json::to_value(result).unwrap(), wire);
}

#[test]
fn cc_mcp_t03_connection_results_keep_generation_and_reject_raw_diagnostics() {
    let wire = json!({
        "operationId": "job-1", "generation": 9, "outcome": "partially_succeeded",
        "items": [
            {"id": "server-a", "connectionRevision": 2, "outcome": {"status": "connected", "toolCount": 0}},
            {"id": "server-b", "connectionRevision": 3, "outcome": {"status": "not_verified"}},
            {"id": "server-c", "connectionRevision": 4, "outcome": {"status": "failed", "code": "timed_out"}}
        ]
    });
    let result: McpConnectionCheckResult = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(result.generation, 9);
    assert_eq!(result.items[0].outcome, McpConnectionOutcome::Connected { tool_count: Some(0) });
    assert_eq!(result.items[1].outcome, McpConnectionOutcome::NotVerified);
    assert_eq!(
        result.items[2].outcome,
        McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut }
    );
    assert_eq!(serde_json::to_value(result).unwrap(), wire);

    let mut unsafe_wire = wire;
    unsafe_wire["items"][2]["outcome"]["message"] = json!("Authorization: Bearer fixture");
    assert!(serde_json::from_value::<McpConnectionCheckResult>(unsafe_wire).is_err());
}
