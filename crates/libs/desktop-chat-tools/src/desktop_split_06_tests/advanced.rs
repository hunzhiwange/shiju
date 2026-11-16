use crate::tools::view_fixture::TestApp as App;
fn tool_advanced_view<'a>(
    app: &'a App,
    m: usize,
    t: usize,
    raw: &str,
) -> Option<iced::Element<'a, crate::advanced_input::Message>> {
    let card = super::card(app);
    crate::tools::advanced_view::tool_advanced_view(
        crate::advanced_input::AdvancedInput {
            i18n: card.i18n,
            hovered: card.hovered,
            text: card.text,
        },
        m,
        t,
        raw,
    )
}
#[test]
fn advanced_view_rejects_unknown_tools_and_invalid_json() {
    let app = App::new().0;

    assert!(tool_advanced_view(&app, 0, 0, "tool browser").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "browser\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool browser\nnot-json").is_none());
    assert!(tool_advanced_view(&app, 0, 0, "tool AgentTool\n{}").is_none());
}

#[test]
fn advanced_view_renders_agent_browser_and_errors() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((4_u64 << 32) | 2);

    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool AgentTool
{"input":"{\"agent\":\"reviewer\",\"prompt\":\"inspect the patch\"}","result":{"data":{"message":"done","agent":"reviewer","session_id":"s1"}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"renderHint":{"metadata":{"action":"open","backend":"playwright"}},"result":{"data":{"result":{"title":"Example","url":"https://example.com"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"result":{"data":{"result":{"text":"Captured page snapshot\n\nsnapshot · browser\nGet text"}}}}"#
        )
        .is_some()
    );
    assert!(
        tool_advanced_view(
            &app,
            4,
            2,
            r#"tool browser
{"status":"error","error":"navigation failed"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_running_and_planned_explicit_surfaces() {
    let app = App::new().0;

    assert!(
        tool_advanced_view(
            &app,
            0,
            0,
            r#"tool mcp_linear
{"input":"{}"}"#
        )
        .is_some()
    );
}

#[test]
fn advanced_view_renders_agent_tool_compact_cards_for_all_states() {
    let mut app = App::new().0;
    app.project_path = Some("/work/project".to_string());

    for (index, raw) in [
        (
            0,
            r#"tool AgentTool
{"status":"running","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-running"}}"#,
        ),
        (
            1,
            r#"tool AgentTool
{"status":"completed","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","output":"完整返回结果","result":{"data":{"session_id":"child-done"}}}"#,
        ),
        (
            2,
            r#"tool AgentTool
{"status":"error","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","error":"执行失败","metadata":{"child_session_id":"child-failed"}}"#,
        ),
        (
            3,
            r#"tool AgentTool
{"status":"waiting","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-waiting"}}"#,
        ),
        (
            4,
            r#"tool AgentTool
{"status":"cancelled","input":"{\"agent\":\"coder\",\"task\":\"分析文件\"}","metadata":{"child_session_id":"child-cancelled"}}"#,
        ),
    ] {
        assert!(
            tool_advanced_view(&app, 8, index, raw).is_some(),
            "expected compact AgentTool card for state {index}"
        );
    }
}
