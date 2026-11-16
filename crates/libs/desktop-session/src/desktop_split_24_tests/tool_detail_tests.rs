fn tool_detail_from_raw(i18n: desktop_i18n::I18n, raw: &str) -> Option<(String, String)> {
    crate::message::chat::input::tool_detail::tool_detail_from_raw(
        i18n,
        raw,
        desktop_chat_tools::tools::mcp_view::is_mcp_tool_raw(raw),
        desktop_chat_tools::tools::workflow_view::workflow_node_detail_body,
    )
}
mod tool_detail_tests {
    #[test]
    fn tool_detail_from_raw_uses_output_text_for_success_payload() {
        let raw = r#"tool shell
{"status":"success","output":"done\n"}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(!title.is_empty());
        assert_eq!(content, "done");
    }

    #[test]
    fn tool_detail_from_raw_marks_error_title_and_uses_error_text() {
        let raw = r#"tool shell
{"status":"error","error":"permission denied"}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(title.ends_with("失败"));
        assert_eq!(content, "permission denied");
    }

    #[test]
    fn tool_detail_from_raw_marks_denied_title_and_uses_output_text() {
        let raw = r#"tool shell
{"status":"denied","output":"blocked by policy"}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(title.ends_with("失败"));
        assert_eq!(content, "blocked by policy");
    }

    #[test]
    fn tool_detail_from_raw_marks_running_title() {
        let raw = r#"tool shell
{"status":"running","output":""}"#;

        let (title, _) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(title.ends_with("运行中"));
    }

    #[test]
    fn tool_detail_from_raw_falls_back_to_trimmed_rest_for_non_json() {
        let raw = "tool unknown\n  plain output  ";

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert_eq!(title, "工具");
        assert_eq!(content, "plain output");
    }

    #[test]
    fn tool_detail_from_raw_formats_json_without_output_or_error() {
        let raw = "  tool shell  \n  {\"status\":\"success\",\"metadata\":{\"attempt\":1}}  ";

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(!title.ends_with("失败"));
        assert!(!title.ends_with("运行中"));
        assert!(content.contains("\n  \"metadata\": {\n    \"attempt\": 1\n  }"));
        assert!(content.contains("\"status\": \"success\""));
    }

    #[test]
    fn tool_detail_from_raw_formats_bash_wait_json_output() {
        let raw = r#"tool bash_wait
{"status":"success","output":"{\"process_id\":\"abc123\",\"output\":\"执行完成\"}"}"#;

        let (_, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid tool detail");

        assert!(content.contains("\n  \"process_id\": \"abc123\""));
        assert!(content.contains("\n  \"output\": \"执行完成\""));
    }

    #[test]
    fn tool_detail_from_raw_formats_mcp_json_and_uses_mcp_title() {
        let raw = r#"tool MCP
{"status":"success","output":"[{\"server\":{\"name\":\"rustcodegraph\"}}]"}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid MCP detail");

        assert_eq!(title, "MCP");
        assert_eq!(
            content,
            "[\n  {\n    \"server\": {\n      \"name\": \"rustcodegraph\"\n    }\n  }\n]"
        );
    }

    #[test]
    fn tool_detail_from_raw_formats_generic_mcp_payload() {
        let raw = r#"tool tools
{"status":"success","output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"resources\":[]}]"}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("valid MCP detail");

        assert_eq!(title, "MCP");
        assert!(content.contains("\"resources\": []"));
    }

    #[test]
    fn tool_detail_from_raw_rejects_missing_tool_header_or_body() {
        assert!(
            super::tool_detail_from_raw(
                desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
                "tool shell"
            )
            .is_none()
        );
        assert!(
            super::tool_detail_from_raw(
                desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
                "shell\n{}"
            )
            .is_none()
        );
    }

    #[test]
    fn tool_detail_from_raw_formats_workflow_node_title_and_pretty_body() {
        let raw = r#"tool workflow_node
{"status":"completed","output":"{\"answer\":\"long node body\",\"usage\":{\"total_tokens\":3}}","metadata":{"title":"实现入口","node_type":"llm"}}"#;

        let (title, content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            raw,
        )
        .expect("workflow detail");

        assert_eq!(title, "实现入口");
        assert!(content.contains("long node body"));
        assert!(content.contains('\n'));
    }

    #[test]
    fn tool_detail_from_raw_marks_workflow_node_running_and_failed_titles() {
        let running = r#"tool workflow_node
{"status":"running","output":"正在执行工作流节点","metadata":{"title":"规划任务"}}"#;
        let failed = r#"tool workflow_node
{"status":"error","error":"boom","metadata":{"title":"规划任务"}}"#;

        let (running_title, _) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            running,
        )
        .expect("running");
        let (failed_title, failed_content) = super::tool_detail_from_raw(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            failed,
        )
        .expect("failed");

        assert_eq!(running_title, "规划任务 · 运行中");
        assert_eq!(failed_title, "规划任务 · 失败");
        assert_eq!(failed_content, "boom");
    }
}
