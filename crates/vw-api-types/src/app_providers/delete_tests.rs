use super::delete::AppProviderDelete;
#[test]
fn cc04_delete_dto_rejects_reference_and_ownership_claims() {
    assert!(serde_json::from_str::<AppProviderDelete>(r#"{"targetHostId":"host"}"#).is_ok());
    for field in ["unreferenced", "companyId", "home", "direct", "route"] {
        let mut value = serde_json::json!({"targetHostId":"host"});
        value.as_object_mut().unwrap().insert(field.into(), false.into());
        assert!(serde_json::from_value::<AppProviderDelete>(value).is_err());
    }
}
