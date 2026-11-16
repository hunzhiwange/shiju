#[test]
fn preference_writes_are_serialized_per_endpoint_and_tenant() {
    use crate::config_desktop::DesktopPreferencesPersistence;
    use gateway_endpoint::GatewayEndpoint;
    use serde_json::Map;
    use vw_api_transport::id::TenantScope;

    let mut queue = DesktopPreferencesPersistence::default();
    let endpoint = GatewayEndpoint::new("localhost", 7777);
    let same_endpoint = GatewayEndpoint::new("LOCALHOST", 7777);
    let other_endpoint = GatewayEndpoint::new("localhost", 7778);
    let other_tenant =
        endpoint.clone().with_tenant_scope(TenantScope::new("test", "other").unwrap());
    let fields = || Map::from_iter([("show_settings".into(), true.into())]);
    let has_task = |task| iced_runtime::task::into_stream(task).is_some();

    assert!(has_task(queue.enqueue(endpoint, fields())));
    assert!(!has_task(queue.enqueue(same_endpoint, fields())));
    assert!(has_task(queue.enqueue(other_endpoint, fields())));
    assert!(has_task(queue.enqueue(other_tenant, fields())));
}
