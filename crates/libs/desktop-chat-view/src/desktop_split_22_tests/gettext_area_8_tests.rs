use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_8_cached_tool_payload_renders_without_translating_output() {
    let source = crate::render_cache::ToolCardTextSource {
        tool_name: "todoread".to_string(),
        value: serde_json::json!({"status": "completed", "output": "[]"}),
    };
    assert_eq!(source.render(I18n::new(DesktopLocale::English)), vec!["No tasks"]);
    assert_eq!(source.render(I18n::new(DesktopLocale::TraditionalChinese)), vec!["暫無任務"]);
    let raw = "中文服务输出/%s/%n";
    let source = crate::render_cache::ToolCardTextSource {
        tool_name: "custom_tool".to_string(),
        value: serde_json::json!({"status": "completed", "output": raw}),
    };
    assert_eq!(source.render(I18n::new(DesktopLocale::English)), vec![raw]);
}
