use super::openclaw_model::*;
use serde_json::json;

#[test]
fn openclaw_model_preserves_document_comments_extensions_and_fallback_order() {
    let pre = br#"{
 // root
 "agents": {"defaults": {"model": {
   "primary" : "old", // model
   "fallbacks": ["b", /* order */ "a"],
   "extension": {"future":true}
 }, "workspace":"keep"}},
 "env": {"secret":"untouched"}
}"#;
    let mut model = get_default_model(Some(pre)).unwrap().unwrap();
    model.primary = "new".into();
    model.fallbacks = vec!["z".into(), "y".into()];
    let next = prepare_default_model(Some(pre), &model).unwrap();
    let expected = String::from_utf8(pre.to_vec())
        .unwrap()
        .replace("\"old\"", "\"new\"")
        .replace("\"b\"", "\"z\"")
        .replace("\"a\"", "\"y\"");
    assert_eq!(next, expected.as_bytes());
    assert_eq!(get_default_model(Some(&next)).unwrap(), Some(model.clone()));
    assert_eq!(prepare_default_model(Some(&next), &model).unwrap(), next);
    model.extra.clear();
    let next = prepare_default_model(Some(&next), &model).unwrap();
    assert_eq!(
        get_default_model(Some(&next)).unwrap().unwrap().extra["extension"],
        json!({"future":true})
    );
}

#[test]
fn openclaw_model_preserves_document_missing_empty_and_invalid_sections() {
    assert_eq!(get_default_model(None).unwrap(), None);
    let model = OpenClawDefaultModel {
        primary: "provider/model".into(),
        fallbacks: vec![],
        extra: Default::default(),
    };
    let next = prepare_default_model(None, &model).unwrap();
    assert_eq!(get_default_model(Some(&next)).unwrap(), Some(model.clone()));
    for source in [
        "",
        "[]",
        "{\"agents\":null}",
        "{\"agents\":{\"defaults\":[]}}",
        "{\"agents\":{\"defaults\":{\"model\":null}}}",
    ] {
        assert!(prepare_default_model(Some(source.as_bytes()), &model).is_err());
    }
    let mut invalid = model;
    invalid.extra.insert("primary".into(), json!("shadow"));
    assert!(prepare_default_model(None, &invalid).is_err());
}

#[test]
fn openclaw_model_preserves_document_json5_quotes_keys_and_numeric_extensions() {
    let source = "\u{feff}{\r\n // 中文\r\n agents: {defaults: {model: {pri\\u006dary: 'old', fallbacks: ['b', /*顺序*/ 'a',], future: 0xFF,},},},\r\n env: {text: 'keep\\\ncontinuation', number: +.5, infinite: -Infinity, nan: NaN,},\r\n}\r\n";
    let mut model = get_default_model(Some(source.as_bytes())).unwrap().unwrap();
    model.primary = "new".into();
    model.fallbacks = vec!["z".into(), "y".into()];
    let next = prepare_default_model(Some(source.as_bytes()), &model).unwrap();
    assert_eq!(
        next,
        source
            .replace("'old'", "\"new\"")
            .replace("'b'", "\"z\"")
            .replace("'a'", "\"y\"")
            .as_bytes()
    );
    assert_eq!(get_default_model(Some(&next)).unwrap(), Some(model.clone()));
    assert_eq!(prepare_default_model(Some(&next), &model).unwrap(), next);
}
