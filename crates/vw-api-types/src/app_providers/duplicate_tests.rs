use super::duplicate::AppProviderDuplicate;
#[test]
fn cc04_duplicate_dto_rejects_ownership_secrets_and_activation() {
    let value = serde_json::json!({"newId":"copy","name":"fixture copy"});
    let input: AppProviderDuplicate = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(input).unwrap(), value);
    for field in ["tenant", "appId", "apiKey", "authBinding", "addToLive", "force", "meta", "path"]
    {
        let mut invalid = value.clone();
        invalid[field] = true.into();
        assert!(serde_json::from_value::<AppProviderDuplicate>(invalid).is_err());
    }
    assert!(serde_json::from_str::<AppProviderDuplicate>(r#"{"newId":"copy"}"#).is_err());
}
