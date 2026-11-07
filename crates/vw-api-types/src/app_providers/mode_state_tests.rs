use super::mode_state::*;
use serde_json::json;
#[test]
fn cc04_mode_state_query_rejects_client_contract_stack_and_third_mode() {
    let query = json!({"targetHostId":"host","providerId":"saved","mode":"proxy"});
    assert_eq!(
        serde_json::from_value::<ModeStateQuery>(query.clone()).unwrap().mode,
        ProviderMode::Proxy
    );
    for (key, value) in [
        ("contract", json!({"key":"forged"})),
        ("settings", json!({})),
        ("stack", json!({})),
        ("mode", json!("stack")),
        ("tenant", json!("foreign")),
    ] {
        let mut q = query.clone();
        q[key] = value;
        assert!(serde_json::from_value::<ModeStateQuery>(q).is_err());
    }
    let query = serde_json::from_value::<ModeStateQuery>(json!({"targetHostId":"host"})).unwrap();
    assert_eq!(query.mode, ProviderMode::Direct);
    assert!(query.provider_id.is_none());
}
