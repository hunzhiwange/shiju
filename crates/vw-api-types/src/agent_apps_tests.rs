use crate::agent_apps::{
    AddCustomIconRequest, AddCustomIconResult, AgentAppsErrorCode, AgentAppsErrorResponse,
    CustomIconEntry, RenameCustomIconRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

fn round_trip<T: Serialize + DeserializeOwned>(value: Value) {
    let parsed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

fn icon() -> Value {
    json!({
        "id": "99daa440-2df2-4427-a282-b0e008bb8a85",
        "name": "工作图标",
        "data": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=",
        "createdAt": 1793894400123_i64
    })
}

#[test]
fn cc_base_icon_contract_round_trip_entry_and_list() {
    round_trip::<CustomIconEntry>(icon());
    round_trip::<Vec<CustomIconEntry>>(json!([icon()]));
    round_trip::<Vec<CustomIconEntry>>(json!([]));
}

#[test]
fn cc_base_icon_contract_round_trip_requests_and_results() {
    round_trip::<AddCustomIconRequest>(json!({"name": "工作图标", "data": icon()["data"]}));
    round_trip::<RenameCustomIconRequest>(json!({"name": "新名称"}));
    for existed in [false, true] {
        let result = json!({"icon": icon(), "existed": existed});
        round_trip::<AddCustomIconResult>(result.clone());
        assert!(!result.to_string().contains("error"));
        assert!(!result.to_string().contains("source"));
    }
}

#[test]
fn cc_base_icon_contract_round_trip_errors() {
    for (code, wire) in [
        (AgentAppsErrorCode::InvalidInput, "invalid_input"),
        (AgentAppsErrorCode::NotFound, "not_found"),
        (AgentAppsErrorCode::Conflict, "conflict"),
        (AgentAppsErrorCode::StorageUnavailable, "storage_unavailable"),
    ] {
        assert_eq!(serde_json::to_value(code).unwrap(), json!(wire));
        let response = json!({"code": wire, "message": "图标操作失败"});
        round_trip::<AgentAppsErrorResponse>(response.clone());
        assert_eq!(response.as_object().unwrap().len(), 2);
        let mut internal = response.clone();
        internal["source"] = json!("SQL SELECT secret FROM /private/icons Authorization: sentinel");
        let public: AgentAppsErrorResponse = serde_json::from_value(internal).unwrap();
        assert_eq!(serde_json::to_value(public).unwrap(), response);
    }
    assert!(serde_json::from_value::<AgentAppsErrorCode>(json!("database_error")).is_err());
}

#[test]
fn cc_base_icon_contract_round_trip_rejects_malformed_fields() {
    let mut missing = icon();
    missing.as_object_mut().unwrap().remove("createdAt");
    assert!(serde_json::from_value::<CustomIconEntry>(missing).is_err());
    let mut invalid = icon();
    invalid["createdAt"] = json!("2026-11-05");
    assert!(serde_json::from_value::<CustomIconEntry>(invalid).is_err());
    assert!(serde_json::from_value::<AddCustomIconRequest>(json!({"name": "图标"})).is_err());
    assert!(serde_json::from_value::<RenameCustomIconRequest>(json!({"name": 42})).is_err());
}
