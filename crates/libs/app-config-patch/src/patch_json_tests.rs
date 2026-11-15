use super::patch_json::{JsonPatch, PatchError};
use serde_json::json;

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).into()).collect()
}

#[test]
fn cc04_patch_json_keeps_unrelated_bytes_and_comments() {
    let pre = "\u{feff}{\r\n\t// 中文😀\r\n\t\"env\": {\"token\" : \"old\" /*尾*/},\r\n\t\"other\": [1e2, \"\\u00e9\"],\r\n}\r\n// 结束😀";
    let patch =
        JsonPatch { set: vec![(path(&["env", "token"]), json!("new"))], ..Default::default() };
    let out = patch.apply(Some(pre.as_bytes())).unwrap();
    assert_eq!(out, pre.replace("\"old\"", "\"new\"").as_bytes());
    assert_eq!(patch.apply(Some(&out)).unwrap(), out);
}

#[test]
fn cc04_patch_json_rejects_invalid_shape() {
    let patch =
        JsonPatch { set: vec![(path(&["env", "token"]), json!("new"))], ..Default::default() };
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("config.jsonc");
    for pre in [
        "[]",
        "null",
        "{\"env\":null}",
        "{\"env\":[]}",
        "{broken",
        "",
        "/* unterminated",
        "{\"x\":1,\"x\":2}",
    ] {
        std::fs::write(&file, pre).unwrap();
        let before = std::fs::read(&file).unwrap();
        assert!(patch.apply(Some(&before)).is_err(), "{pre}");
        assert_eq!(std::fs::read(&file).unwrap(), before);
    }
    assert_eq!(
        JsonPatch { remove: vec![vec![]], ..Default::default() }.apply(None),
        Err(PatchError::InvalidPath)
    );
}

#[test]
fn cc04_patch_json_add_remove_seed_and_escaped_keys() {
    let pre = "{\"first\":1, /*a*/ \"\\u0074oken\":\"old\", /*b*/ \"last\":3 /*c*/}";
    let patch = JsonPatch {
        remove: vec![path(&["first"]), path(&["last"]), path(&["token"])],
        set: vec![(path(&["token"]), json!("new")), (path(&["new", "中文\""]), json!([1, 2]))],
        seed: vec![(path(&["token"]), json!("ignored")), (path(&["seed"]), json!(true))],
    };
    let out = patch.apply(Some(pre.as_bytes())).unwrap();
    let text = String::from_utf8(out.clone()).unwrap();
    for comment in ["/*a*/", "/*b*/", "/*c*/"] {
        assert!(text.contains(comment));
    }
    assert!(text.contains("\"\\u0074oken\":\"new\""));
    assert!(text.contains("\"seed\":true"));
    assert!(text.contains("\"new\":{\"中文\\\"\":[1,2]}"));
    assert_eq!(patch.apply(Some(&out)).unwrap(), out);
    let empty = JsonPatch { set: vec![(path(&["a", "b"]), json!(null))], ..Default::default() }
        .apply(None)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty).unwrap(),
        json!({"a":{"b":null}})
    );
}

#[test]
fn cc04_patch_json_removal_positions_and_trailing_commas() {
    for pre in [r#"{"a":1,"b":2,"c":3}"#, r#"{"a":1,"b":2,"c":3,}"#] {
        for name in ["a", "b", "c"] {
            let patch = JsonPatch { remove: vec![path(&[name])], ..Default::default() };
            let out = patch.apply(Some(pre.as_bytes())).unwrap();
            assert_eq!(patch.apply(Some(&out)).unwrap(), out);
            assert!(!String::from_utf8(out).unwrap().contains(&format!("\"{name}\":")));
        }
    }
    let patch = JsonPatch { remove: vec![path(&["a"])], ..Default::default() };
    assert_eq!(
        patch.apply(Some(b"{ /*keep*/ \"a\":1, /*tail*/}")).unwrap(),
        b"{ /*keep*/  /*tail*/}"
    );
    for pre in [
        b"{,}".as_slice(),
        b"{\"a\":,}",
        b"{\"a\":[,]}",
        b"{\"a\":1,,}",
        b"{\"a\":{\"x\":1,\"x\":2}}",
        b"\xff",
    ] {
        assert!(patch.apply(Some(pre)).is_err(), "{pre:?}");
    }
}

#[test]
fn cc04_patch_json_noop_and_failed_batch_preserve_input() {
    let pre = b"{\"v\": {/*keep*/\"a\":[1,],},\"env\":false}";
    let patch = JsonPatch { set: vec![(path(&["v"]), json!({"a":[1]}))], ..Default::default() };
    assert_eq!(patch.apply(Some(pre)).unwrap(), pre);
    let patch = JsonPatch {
        set: vec![(path(&["new"]), json!(2)), (path(&["env", "token"]), json!("fake"))],
        ..Default::default()
    };
    assert_eq!(patch.apply(Some(pre)), Err(PatchError::Shape));
    for suffix in ["// 中文", "// 😀", "/* 中文 */ // 尾", "// 中文\u{2028}"] {
        let pre = format!("{{}} {suffix}");
        let patch = JsonPatch {
            set: vec![(path(&["url"]), json!("https://example.test/a/*x*/"))],
            ..Default::default()
        };
        let out = patch.apply(Some(pre.as_bytes())).unwrap();
        assert!(String::from_utf8(out.clone()).unwrap().ends_with(suffix));
        assert_eq!(patch.apply(Some(&out)).unwrap(), out);
    }
}
