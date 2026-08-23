//! 验证技能工具名称从输入、输出和错误载荷中提取并正确构造视图。
//! 缺失、空白和未闭合字段必须拒绝，避免展示看似有效但不可执行的技能名。

use super::skill_view::{
    quoted_attr_value, skill_display_name, skill_name_from_input, skill_name_from_json_value,
    skill_name_from_output, tool_skill_view, yaml_name_value,
};
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn skill_names_are_read_from_supported_input_shapes() {
    assert_eq!(skill_name_from_input(r#"{"name":"review"}"#).as_deref(), Some("review"));
    assert_eq!(skill_name_from_input(r#"{"skill_name":"docs"}"#).as_deref(), Some("docs"));
    assert_eq!(skill_name_from_input(r#""plain-skill""#).as_deref(), Some("plain-skill"));
    assert_eq!(
        skill_name_from_json_value(&serde_json::json!({"id":"fallback"})).as_deref(),
        Some("fallback")
    );
    assert_eq!(skill_name_from_input("   "), None);
}

#[test]
fn skill_names_fall_back_to_output_and_errors() {
    assert_eq!(quoted_attr_value(r#"<skill name="builder">"#, "name").as_deref(), Some("builder"));
    assert_eq!(quoted_attr_value("name='quoted'", "name").as_deref(), Some("quoted"));
    assert_eq!(quoted_attr_value("name=bare", "name"), None);
    assert_eq!(yaml_name_value("title: x\nname: 'yaml-skill'").as_deref(), Some("yaml-skill"));
    assert_eq!(skill_name_from_output("name: yaml-skill").as_deref(), Some("yaml-skill"));
    assert_eq!(skill_display_name("", "", "name: error-skill"), "error-skill");
    assert_eq!(skill_display_name("", "", ""), "未知技能");
}

#[test]
fn skill_view_builds_for_skill_tool_only() {
    let app = app();
    let visible = r#"tool skill
{"status":"completed","input":"{\"name\":\"imagegen\"}","output":"ok"}"#;
    let error_visible = r#"tool skill
{"status":"error","input":"","error":"name: broken"}"#;

    assert!(tool_skill_view(&app, 0, 0, visible).is_some());
    assert!(tool_skill_view(&app, 0, 1, error_visible).is_some());
    assert!(tool_skill_view(&app, 0, 2, "tool bash\n{}").is_none());
    assert!(tool_skill_view(&app, 0, 3, "tool skill\nnot-json").is_none());
}

#[test]
fn skill_name_extractors_reject_missing_empty_and_unclosed_values() {
    assert_eq!(skill_name_from_json_value(&serde_json::json!([])), None);
    assert_eq!(skill_name_from_json_value(&serde_json::json!({"name":"   "})), None);
    assert_eq!(quoted_attr_value("title=builder", "name"), None);
    assert_eq!(quoted_attr_value("name=", "name"), None);
    assert_eq!(quoted_attr_value("name=\"unterminated", "name"), None);
    assert_eq!(quoted_attr_value("name='   '", "name"), None);
    assert_eq!(yaml_name_value("name: '   '\ntitle: builder"), None);
}
