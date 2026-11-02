use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_8_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "请求失败"),
        (DesktopLocale::TraditionalChinese, "請求失敗"),
        (DesktopLocale::English, "Request failed"),
    ] {
        assert_eq!(I18n::new(locale).__("请求失败"), expected);
    }
}

#[test]
fn desktop_gettext_area_8_display_uses_explicit_locale() {
    let source = include_str!("../components/chat_panel/message_view/assistant_error.rs");
    assert!(
        source.contains(".__(\"请求失败\")"),
        "API error display must translate its own label while preserving service content"
    );
}

#[test]
fn desktop_gettext_area_8_dynamic_api_url_preserves_raw_argument() {
    let raw = "https://example.invalid/中文/%n/%s";
    let message = super::format::Message::new("接口地址: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "接口地址: "),
        (DesktopLocale::TraditionalChinese, "介面位址: "),
        (DesktopLocale::English, "API URL: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_8_header_templates_preserve_typed_values() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "使用率: 12.5%"),
        (DesktopLocale::TraditionalChinese, "使用率: 12.5%"),
        (DesktopLocale::English, "Usage: 12.5%"),
    ] {
        assert_eq!(I18n::new(locale).format("使用率: %.1f%%", &[12.5.into()]).unwrap(), expected);
    }
}

#[test]
fn desktop_gettext_area_8_migrated_display_keys_have_both_translations() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        include_str!("../components/chat_panel/empty.rs"),
        include_str!("../components/chat_panel/global_status.rs"),
        include_str!("../components/chat_panel/header.rs"),
        include_str!("../components/chat_panel/message_view.rs"),
        include_str!("../components/chat_panel/message_view/assistant_error.rs"),
        include_str!("../components/chat_panel/message_view/think_block.rs"),
        include_str!("../components/chat_panel/mod.rs"),
        include_str!("../components/chat_panel/tool_selector.rs"),
        include_str!("../components/chat_panel/tools/advanced_view.rs"),
        include_str!("../components/chat_panel/tools/apply_patch_view.rs"),
        include_str!("../components/chat_panel/tools/bash_view.rs"),
        include_str!("../components/chat_panel/tools/command_summary_view.rs"),
        include_str!("../components/chat_panel/tools/config_view.rs"),
        include_str!("../components/chat_panel/tools/explore_summary_view.rs"),
        include_str!("../components/chat_panel/tools/files_view/list_ui.rs"),
        include_str!("../components/chat_panel/tools/git_diff_view.rs"),
        include_str!("../components/chat_panel/tools/lsp_view.rs"),
        include_str!("../components/chat_panel/tools/mcp_view.rs"),
        include_str!("../components/chat_panel/tools/question_view.rs"),
        include_str!("../components/chat_panel/tools/read_view.rs"),
        include_str!("../components/chat_panel/tools/skill_view.rs"),
        include_str!("../components/chat_panel/tools/todo_views.rs"),
        include_str!("../components/chat_panel/tools/tool_detail_dialog.rs"),
        include_str!("../components/chat_panel/tools/web_view.rs"),
        include_str!("../components/chat_panel/tools/workflow_view.rs"),
        include_str!("../components/chat_panel/trajectory/mod.rs"),
        include_str!("../components/chat_panel/utils/app_state.rs"),
        include_str!("../components/chat_panel/utils/time.rs"),
    ] {
        for tail in source.split(".__(\"").skip(1) {
            let key = tail.split('"').next().unwrap();
            assert!(en.contains_key(key), "missing English: {key}");
            assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
        }
    }
}

#[test]
fn desktop_gettext_area_8_trajectory_fields_follow_locale_and_preserve_unknown_ids() {
    assert_eq!(
        super::chat::trajectory_label(I18n::new(DesktopLocale::SimplifiedChinese), "Source"),
        "来源"
    );
    assert_eq!(
        super::chat::trajectory_label(
            I18n::new(DesktopLocale::TraditionalChinese),
            "No payload captured"
        ),
        "未擷取請求承載資料"
    );
    assert_eq!(
        super::chat::trajectory_label(I18n::new(DesktopLocale::English), "Source"),
        "Source"
    );
    let raw = "工具/%n/%s/custom";
    assert_eq!(super::chat::trajectory_label(I18n::new(DesktopLocale::English), raw), raw);
}

#[test]
fn desktop_gettext_area_8_cached_tool_payload_renders_without_translating_output() {
    let source = crate::app::models::ToolCardTextSource {
        tool_name: "todoread".to_string(),
        value: serde_json::json!({"status": "completed", "output": "[]"}),
    };
    assert_eq!(source.render(I18n::new(DesktopLocale::English)), vec!["No tasks"]);
    assert_eq!(source.render(I18n::new(DesktopLocale::TraditionalChinese)), vec!["暫無任務"]);
    let raw = "中文服务输出/%s/%n";
    let source = crate::app::models::ToolCardTextSource {
        tool_name: "custom_tool".to_string(),
        value: serde_json::json!({"status": "completed", "output": raw}),
    };
    assert_eq!(source.render(I18n::new(DesktopLocale::English)), vec![raw]);
}

#[test]
fn desktop_gettext_area_8_permission_details_keep_raw_reason() {
    let value =
        serde_json::json!({"permission_request": {"reason": "中文/%n/%s"}, "error": "服务错误"});
    let text = crate::app::components::chat_panel::tools::localized_tool_permission_error_text(
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
    let rendered = crate::app::components::chat_panel::tools::localized_tool_inline_summary(
        I18n::new(DesktopLocale::English),
        "question",
        &input,
    )
    .unwrap();
    assert_eq!(rendered, "中文/%n/%s and 2 questions");
}
