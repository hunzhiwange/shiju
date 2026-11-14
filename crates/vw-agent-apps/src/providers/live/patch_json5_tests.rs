use super::patch_json5::{JsonPatch, object_value, replace_object_preserving_trivia};
use serde_json::json;

#[test]
fn json5_edits_preserve_original_tokens_and_trivia() {
    let source = "{\n agents: {defaults: {model: {primary: 'old', fallbacks: ['b', /* order */ 'a',], future: 0xFF}}},\n env: {token: 'keep', n: NaN, inf: +Infinity, decimal: .5},\n}";
    let mut desired = object_value(source.as_bytes()).unwrap();
    desired["agents"]["defaults"]["model"]["primary"] = json!("new");
    let next = replace_object_preserving_trivia(source.as_bytes(), desired).unwrap();
    assert_eq!(next, source.replace("'old'", "\"new\"").as_bytes());
}

#[test]
fn json5_edits_resize_arrays_and_add_remove_keys_without_losing_comments() {
    let source = "{list: ['a', /*keep*/ 'b'], old: 1 /*tail*/}";
    let mut desired = object_value(source.as_bytes()).unwrap();
    desired["list"] = json!(["z", "y", "x"]);
    desired.as_object_mut().unwrap().remove("old");
    desired["new"] = json!({"nested":true});
    let next = replace_object_preserving_trivia(source.as_bytes(), desired.clone()).unwrap();
    assert_eq!(object_value(&next).unwrap(), desired);
    let text = String::from_utf8(next).unwrap();
    assert!(text.contains("/*keep*/"));
    assert!(text.contains("/*tail*/"));
    let patch = JsonPatch {
        set: vec![(vec!["missing".into(), "leaf".into()], json!(1))],
        ..Default::default()
    };
    assert_eq!(
        object_value(&patch.apply(Some(b"{existing: 'ok'}")).unwrap()).unwrap(),
        json!({"existing":"ok", "missing":{"leaf":1}})
    );
}

#[test]
fn json5_edits_reject_escaped_duplicate_keys_and_malformed_documents() {
    for source in [
        "{primary:1, pri\\u006dary:2}",
        "{nested:{'x':1, x:2}}",
        "{x: [1,,2]}",
        "{x:'unterminated}",
        "[]",
    ] {
        assert!(object_value(source.as_bytes()).is_err());
    }
}

#[test]
fn json5_edits_keep_unicode_whitespace_escaped_keys_and_line_continuations() {
    let source = "\u{feff}{\u{a0}a\\u0067ents: {defaults: {model: {primary: 'old'}}}, extra: 'line\\\ncontinued', other: -Infinity,}";
    let mut desired = object_value(source.as_bytes()).unwrap();
    desired["agents"]["defaults"]["model"]["primary"] = json!("new");
    let next = replace_object_preserving_trivia(source.as_bytes(), desired).unwrap();
    assert_eq!(next, source.replace("'old'", "\"new\"").as_bytes());
}
