use super::{parse_config_result_from_output, success_body};

#[test]
fn success_body_renders_empty_config_overview() {
    let result =
        parse_config_result_from_output(r#"{"success":true,"operation":"get","value":{}}"#)
            .expect("empty config overview should parse");

    assert!(success_body(&result).is_some());
}

#[test]
fn success_body_renders_truncated_config_section_summary() {
    let result = parse_config_result_from_output(
        r#"{"success":true,"operation":"get","value":{"a":{},"b":{},"c":{},"d":{},"e":{},"f":{}}}"#,
    )
    .expect("multi-section config overview should parse");

    assert!(success_body(&result).is_some());
}
