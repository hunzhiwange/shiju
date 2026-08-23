// 验证身份模型的默认字段、序列化及合并语义。
use super::*;

#[test]
fn default_identity_sections_are_absent() {
    let identity = AieosIdentity::default();
    assert!(identity.identity.is_none());
    assert!(identity.psychology.is_none());
    assert!(identity.interests.is_none());
}

#[test]
fn serde_preserves_nested_identity_fields() {
    let json = r#"{"identity":{"names":{"first":"Ada","last":"Lovelace","nickname":"Enchantress"},"bio":"writes careful programs"},"capabilities":{"skills":["analysis"],"tools":["shell"]}}"#;
    let parsed: AieosIdentity = serde_json::from_str(json).unwrap();
    let identity = parsed.identity.unwrap();
    let names = identity.names.unwrap();
    assert_eq!(names.first.as_deref(), Some("Ada"));
    assert_eq!(names.last.as_deref(), Some("Lovelace"));
    assert_eq!(names.nickname.as_deref(), Some("Enchantress"));
    assert_eq!(identity.bio.as_deref(), Some("writes careful programs"));
    assert_eq!(parsed.capabilities.unwrap().tools.unwrap(), vec!["shell"]);
}
