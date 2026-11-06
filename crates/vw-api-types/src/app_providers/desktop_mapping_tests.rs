use super::desktop_mapping::*;
use serde_json::json;
#[test]
fn cc04_desktop_mapping_dto_preserves_routes_and_rejects_unknown_fields() {
    let value = json!({"targetHostId":"host","mapping":{"mode":"proxy","routes":{"claude-sonnet-5":{"model":"kimi-k2","supports1m":true,"labelOverride":"Kimi"}}}});
    let dto: DesktopMappingWrite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(dto).unwrap(), value);
    let mut invalid = value;
    invalid["path"] = json!("/tmp/config");
    assert!(serde_json::from_value::<DesktopMappingWrite>(invalid).is_err());
}
