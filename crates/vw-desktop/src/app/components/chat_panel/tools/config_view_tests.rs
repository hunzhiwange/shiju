//! 验证配置工具成功结果在空数据和超长分区下的摘要呈现。
//! 超长配置仅影响预览截断，不应丢失分区存在性或破坏视图构造。

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
