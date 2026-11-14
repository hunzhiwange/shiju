use super::openclaw_catalog::parse_draft;
use serde_json::json;

#[test]
fn openclaw_catalog_editor_preserves_aliases_and_extra_fields() {
    let source = json!({"provider/model": {"alias": "中文名称", "future": {"enabled": true}, "params": [1, 2]}});
    let draft = parse_draft(&source.to_string()).unwrap();
    assert_eq!(serde_json::to_value(draft).unwrap(), source);
}

#[test]
fn openclaw_catalog_editor_preserves_aliases_with_empty_catalog() {
    assert!(parse_draft("{}").unwrap().is_empty());
    assert!(parse_draft("").is_err());
    assert!(parse_draft("null").is_err());
    assert!(parse_draft(r#"{"p/m":{"alias":123}}"#).is_err());
}
