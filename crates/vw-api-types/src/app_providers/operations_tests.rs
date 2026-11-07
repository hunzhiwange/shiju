use super::operations::*;
#[test]
fn cc04_operations_dto_rejects_arbitrary_execution_fields() {
    let request = serde_json::json!({"targetHostId":"host","requestId":"request","action":{"kind":"import","app":"claude","trigger":"manual"}});
    let _: ProviderOperationRequest = serde_json::from_value(request.clone()).unwrap();
    let mut bad = request;
    bad["command"] = serde_json::json!("whoami");
    assert!(serde_json::from_value::<ProviderOperationRequest>(bad).is_err());
}
