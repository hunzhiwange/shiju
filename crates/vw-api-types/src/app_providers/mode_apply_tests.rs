use super::mode_apply::*;
use serde_json::json;
#[test]
fn cc04_mode_apply_request_rejects_injected_configuration_and_runtime() {
    let request = json!({"targetHostId":"host","action":{"kind":"proxy","providerId":"route"}});
    let parsed: ModeApplyRequest = serde_json::from_value(request.clone()).unwrap();
    assert!(matches!(parsed.action,ModeApplyAction::Proxy{provider_id} if provider_id=="route"));
    for key in ["settings", "contract", "path", "runtime", "tenant", "stack"] {
        let mut bad = request.clone();
        bad[key] = json!({});
        assert!(serde_json::from_value::<ModeApplyRequest>(bad).is_err());
        let mut bad = request.clone();
        bad["action"][key] = json!({});
        assert!(serde_json::from_value::<ModeApplyRequest>(bad).is_err());
    }
    assert!(
        serde_json::from_value::<ModeApplyRequest>(
            json!({"targetHostId":"host","action":{"kind":"stack"}})
        )
        .is_err()
    );
}
