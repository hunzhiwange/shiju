use super::presentation::AppProviderPresentation;
#[test]
fn cc04_presentation_dto_rejects_runtime_and_ownership_fields() {
    let value = serde_json::json!({"sortIndex":2,"category":"分组","notes":"备注\n第二行"});
    let dto: AppProviderPresentation = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(dto).unwrap(), value);
    for invalid in [
        serde_json::json!({"sortIndex":-1}),
        serde_json::json!({"sortIndex":1.5}),
        serde_json::json!({"sortIndex":4294967296u64}),
    ] {
        assert!(serde_json::from_value::<AppProviderPresentation>(invalid).is_err());
    }
    for key in
        ["tenant", "appId", "id", "apiKey", "meta", "path", "current", "addToLive", "authBinding"]
    {
        assert!(
            serde_json::from_value::<AppProviderPresentation>(serde_json::json!({key:true}))
                .is_err()
        );
    }
    assert_eq!(
        serde_json::from_str::<AppProviderPresentation>("{}").unwrap(),
        AppProviderPresentation::default()
    );
}
