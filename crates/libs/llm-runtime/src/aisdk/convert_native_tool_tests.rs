//! 当前原生工具声明、命名空间与协议身份转换。

use serde_json::json;

#[test]
fn native_tool_search_request_request_uses_native_search_type() {
    use tool_types::ToolSpec;
    let tools = std::collections::HashMap::from([(
        "tool_search".into(),
        ToolSpec::new(
            "tool_search",
            "Search tools",
            json!({"type":"object","properties":{"query":{"type":"string"}}}),
        ),
    )]);
    let declarations = super::native_request_tools(&tools, false).unwrap();
    assert!(matches!(
        &declarations[0],
        aisdk::core::NativeToolSpec::ToolSearch {
            execution: aisdk::core::ToolSearchExecution::Client,
            ..
        }
    ));
}

#[test]
fn native_tool_search_request_namespaces_filter_hidden_and_preserve_distinct_names() {
    use tool_types::ToolSpec;
    use vw_api_tools::ToolExposure;
    let mut first = ToolSpec::new("stable_a", "A", json!({"type":"object"}));
    first.identity.name = "read".into();
    first.identity.namespace = Some("alpha".into());
    let mut second = first.clone();
    second.id = "stable_b".into();
    second.identity.namespace = Some("beta".into());
    let mut hidden = ToolSpec::new("secret", "Secret", json!({"type":"object"}));
    hidden.exposure = ToolExposure::Hidden;
    let tools = std::collections::HashMap::from([
        ("a".into(), first),
        ("b".into(), second),
        ("hidden".into(), hidden),
    ]);
    let wire = serde_json::to_value(super::native_request_tools(&tools, false).unwrap()).unwrap();
    assert_eq!(wire.as_array().unwrap().len(), 2);
    assert_eq!(wire[0]["name"], "alpha");
    assert_eq!(wire[1]["name"], "beta");
    assert_eq!(wire[0]["tools"][0]["name"], "read");
    assert_eq!(wire[1]["tools"][0]["name"], "read");
    assert!(!wire.to_string().contains("secret"));
    assert_eq!(super::native_tool_id(&tools, Some("alpha"), "read").unwrap(), "stable_a");
    assert_eq!(super::native_tool_id(&tools, Some("beta"), "read").unwrap(), "stable_b");
    assert!(super::native_tool_id(&tools, None, "read").is_err());
    assert!(super::native_tool_id(&tools, None, "secret").is_err());
}

#[test]
fn native_tool_search_request_schema_and_identity_errors_are_explicit() {
    use tool_types::ToolSpec;
    let mut spec = ToolSpec::new("bad", "Bad", json!(42));
    let mut tools = std::collections::HashMap::from([("a".into(), spec.clone())]);
    assert!(super::native_request_tools(&tools, false).unwrap_err().to_string().contains("schema"));
    spec.input_schema = json!({"type":"object"});
    tools.insert("a".into(), spec.clone());
    tools.insert("b".into(), spec);
    assert!(
        super::native_request_tools(&tools, false).unwrap_err().to_string().contains("duplicate")
    );
}
