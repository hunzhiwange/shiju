use super::openclaw_catalog::*;
use crate::providers::live::patch_json::object_value;
use serde_json::json;

#[test]
fn openclaw_catalog_preserves_defaults_and_unknown_fields() {
    let source = br#"{
      // root comment
      "env": {"KEEP": "value"},
      "agents": {"defaults": {
        "model": {"primary": "provider/old"}, // model comment
        "workspace": "/workspace",
        "models": {"provider/new": {"alias": "Old", "future": {"x": true}}}
      }}
    }"#;
    let mut catalog = get_model_catalog(Some(source)).unwrap().unwrap();
    catalog.get_mut("provider/new").unwrap().alias = Some("中文名称".into());
    let next = set_model_catalog(Some(source), &catalog).unwrap();
    let text = std::str::from_utf8(&next).unwrap();
    assert!(text.contains("// root comment"));
    assert!(text.contains("// model comment"));
    let before = object_value(source).unwrap();
    let after = object_value(&next).unwrap();
    assert_eq!(after["env"], before["env"]);
    assert_eq!(after["agents"]["defaults"]["model"], before["agents"]["defaults"]["model"]);
    assert_eq!(after["agents"]["defaults"]["workspace"], json!("/workspace"));
    assert!(get_model_catalog(Some(&next)).unwrap() == Some(catalog));
}

#[test]
fn openclaw_catalog_preserves_defaults_missing_and_empty() {
    assert!(get_model_catalog(None).unwrap().is_none());
    assert!(get_model_catalog(Some(b"{}")).unwrap().is_none());
    let empty = OpenClawModelCatalog::new();
    let next = set_model_catalog(None, &empty).unwrap();
    assert!(get_model_catalog(Some(&next)).unwrap().unwrap().is_empty());
    let source = br#"{"agents":{"defaults":{"model":{"primary":"p/m"},"models":{"p/m":{}}}}}"#;
    let next = set_model_catalog(Some(source), &empty).unwrap();
    let value = object_value(&next).unwrap();
    assert_eq!(value.pointer("/agents/defaults/models"), Some(&json!({})));
    assert_eq!(value.pointer("/agents/defaults/model/primary"), Some(&json!("p/m")));
}

#[test]
fn openclaw_catalog_preserves_defaults_rejects_invalid_shapes() {
    for source in [
        r#"{"agents":null}"#, r#"{"agents":{"defaults":[]}}"#,
        r#"{"agents":{"defaults":{"models":null}}}"#,
        r#"{"agents":{"defaults":{"models":{"p/m":{"alias":12}}}}}"#,
        "broken",
    ] {
        assert!(get_model_catalog(Some(source.as_bytes())).is_err());
    }
    for source in [r#"{"agents":null}"#, r#"{"agents":{"defaults":[]}}"#] {
        assert!(set_model_catalog(Some(source.as_bytes()), &OpenClawModelCatalog::new()).is_err());
    }
}
