use super::universal_sync::*;
use serde_json::json;
#[test]
fn cc04_universal_sync_dto_rejects_ambient_authority_and_reports_catalog_only() {
    for input in [
        json!({"apps":["claude"],"activate":true}),
        json!({"apps":["claude"],"path":"/private"}),
        json!({"apps":["claude"],"companyId":"other"}),
        json!({"apps":["unknown"]}),
    ] {
        assert!(serde_json::from_value::<UniversalSyncRequest>(input).is_err());
    }
    let row: UniversalSyncResult = serde_json::from_value(json!({"appId":"claude","providerId":"universal-claude-shared","catalog":{"status":"updated"},"live":"not_attempted"})).unwrap();
    assert_eq!(row.catalog, UniversalCatalogResult::Updated);
    assert_eq!(row.live, UniversalLiveResult::NotAttempted);
    assert_eq!(serde_json::to_value(row).unwrap()["live"], "not_attempted");
}
