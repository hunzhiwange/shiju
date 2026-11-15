use super::icons::{AppProviderIcon, AppProviderIconChange};
#[test]
fn cc04_icons_dto_rejects_paths_and_unowned_fields() {
    let input = serde_json::json!({"action":"select","iconId":"fixture"});
    let typed: AppProviderIconChange = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), input);
    for key in ["path", "data", "tenant", "current"] {
        let mut v = input.clone();
        v[key] = serde_json::json!("fixture");
        assert!(serde_json::from_value::<AppProviderIconChange>(v).is_err());
    }
    assert!(serde_json::from_value::<AppProviderIconChange>(serde_json::json!({})).is_err());
    let response: AppProviderIcon =
        serde_json::from_value(serde_json::json!({"data":null})).unwrap();
    assert_eq!(response.data, None);
}
