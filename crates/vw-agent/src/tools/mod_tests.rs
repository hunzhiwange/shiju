use super::*;

#[test]
fn tool_id_helpers_accept_supported_ids_and_aliases() {
    assert!([ASK_USER_QUESTION_TOOL_ID, QUESTION_TOOL_ALIAS].into_iter().all(is_question_tool_id));
    assert!([TODO_READ_TOOL_ID, TODO_READ_TOOL_ALIAS].into_iter().all(is_todo_read_tool_id));
    assert!([TODO_WRITE_TOOL_ID, TODO_WRITE_TOOL_ALIAS].into_iter().all(is_todo_write_tool_id));
    assert!(
        [WEB_FETCH_TOOL_ID, WEB_FETCH_TOOL_ALIAS, "webfetch"].into_iter().all(is_web_fetch_tool_id)
    );
    assert!(
        [WEB_SEARCH_TOOL_ID, WEB_SEARCH_TOOL_ALIAS, "websearch", "web_search",]
            .into_iter()
            .all(is_web_search_tool_id)
    );
    assert!([BROWSER_TOOL_ID, BROWSER_TOOL_ALIAS].into_iter().all(is_browser_tool_id));
    assert!(
        [BROWSER_OPEN_TOOL_ID, BROWSER_OPEN_TOOL_ALIAS].into_iter().all(is_browser_open_tool_id)
    );
    assert!(
        [ENTER_PLAN_MODE_TOOL_ID, "EnterPlanMode", "enter_plan_mode",]
            .into_iter()
            .all(is_enter_plan_mode_tool_id)
    );
    assert!(
        [EXIT_PLAN_MODE_TOOL_ID, "ExitPlanMode", "exit_plan_mode",]
            .into_iter()
            .all(is_exit_plan_mode_tool_id)
    );
}

#[test]
fn tool_id_helpers_reject_unknown_and_case_mismatched_ids() {
    for id in ["", "unknown", "WEB_SEARCH"] {
        assert!(!is_question_tool_id(id));
        assert!(!is_todo_read_tool_id(id));
        assert!(!is_todo_write_tool_id(id));
        assert!(!is_web_fetch_tool_id(id));
        assert!(!is_web_search_tool_id(id));
        assert!(!is_browser_tool_id(id));
        assert!(!is_browser_open_tool_id(id));
        assert!(!is_enter_plan_mode_tool_id(id));
        assert!(!is_exit_plan_mode_tool_id(id));
    }
}
