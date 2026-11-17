
#[test]
fn desktop_gettext_area_8_display_uses_explicit_locale() {
    let source = include_str!("../../../desktop-chat-view/src/message_view/assistant_error.rs");
    assert!(
        source.contains(".__(\"请求失败\")"),
        "API error display must translate its own label while preserving service content"
    );
}

#[test]
fn desktop_gettext_area_8_migrated_display_keys_have_both_translations() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    for source in [
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/empty.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/global_status.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/header.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/message_view.rs"),
        include_str!("../../../desktop-chat-view/src/message_view/assistant_error.rs"),
        include_str!("../../../desktop-chat-view/src/message_view/think_block.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tool_selector.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/advanced_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/apply_patch_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/bash_view.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/command_summary_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/config_view.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/explore_summary_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/files_view/list_ui.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/git_diff_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/lsp_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/mcp_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/question_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/read_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/skill_view.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/todo_views.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/tool_detail_dialog.rs"),
        include_str!("../../../desktop-chat-tools/src/tools/web_view.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/tools/workflow_view.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/trajectory/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/utils/app_state.rs"),
        include_str!("../../../../vw-desktop/src/app/components/chat_panel/utils/time.rs"),
    ] {
        for tail in source.split(".__(\"").skip(1) {
            let key = tail.split('"').next().unwrap();
            assert!(en.contains_key(key), "missing English: {key}");
            assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
        }
    }
}
