use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_8_permission_details_keep_raw_reason() {
    let value =
        serde_json::json!({"permission_request": {"reason": "中文/%n/%s"}, "error": "服务错误"});
    let text = crate::tools::localized_tool_permission_error_text(
        I18n::new(DesktopLocale::English),
        "bash",
        &value,
    )
    .unwrap();
    assert!(text.contains("Reason: 中文/%n/%s"));
    assert!(text.contains("服务错误"));
}

#[test]
fn desktop_gettext_area_8_inline_question_summary_keeps_question_text() {
    let input = serde_json::json!({"questions": [{"header": "中文/%n/%s"}, {"header": "other"}]})
        .to_string();
    let rendered = crate::tools::localized_tool_inline_summary(
        I18n::new(DesktopLocale::English),
        "question",
        &input,
    )
    .unwrap();
    assert_eq!(rendered, "中文/%n/%s and 2 questions");
}
