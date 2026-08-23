//! 验证工作流模板的选择器替换、Jinja 值渲染与缺失值回退。

use super::{render_jinja_value_template, render_template, value_to_text};
use crate::workflow::variables::VariablePool;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn render_template_replaces_known_selectors_and_blanks_missing_values() {
    let mut pool = VariablePool::default();
    pool.insert_selector(&["start".to_string(), "name".to_string()], Value::String("Alice".into()));
    pool.insert_selector(&["start".to_string(), "count".to_string()], Value::Number(3.into()));

    assert_eq!(
        render_template("{{# start.name #}}: {{#start.count#}} {{#missing.value#}}", &pool),
        "Alice: 3 "
    );
}

#[test]
fn render_template_ignores_empty_selector_parts() {
    let mut pool = VariablePool::default();
    pool.insert_selector(&["sys".to_string(), "query".to_string()], Value::String("hello".into()));

    assert_eq!(render_template("{{# sys..query #}}", &pool), "hello");
}

#[test]
fn render_template_blanks_whitespace_selector_and_preserves_malformed_markers() {
    let pool = VariablePool::default();

    assert_eq!(render_template("before {{#   #}} after", &pool), "before  after");
    assert_eq!(render_template("before {{#missing after", &pool), "before {{#missing after");
}

#[test]
fn render_jinja_value_template_supports_values_and_reports_errors() {
    let values = BTreeMap::from([
        ("name".to_string(), Value::String("Alice".into())),
        ("items".to_string(), json!(["a", "b"])),
    ]);

    assert_eq!(
        render_jinja_value_template(
            "{% for item in items %}{{ name }}:{{ item }};{% endfor %}",
            &values
        )
        .expect("render"),
        "Alice:a;Alice:b;"
    );
    assert!(
        render_jinja_value_template("{% if", &values)
            .expect_err("template error")
            .contains("template 节点渲染失败")
    );
}

#[test]
fn value_to_text_covers_json_value_kinds() {
    assert_eq!(value_to_text(&Value::Null), "");
    assert_eq!(value_to_text(&Value::Bool(true)), "true");
    assert_eq!(value_to_text(&Value::Number(42.into())), "42");
    assert_eq!(value_to_text(&Value::String("text".into())), "text");
    assert_eq!(value_to_text(&json!(["x", 1])), r#"["x",1]"#);
    assert_eq!(value_to_text(&json!({"a": true})), r#"{"a":true}"#);
}
