//! 验证待办面板的数据选择、排序、完成计数与折叠展示边界。
//! 会话匹配、数字和文本编号混排以及活动请求回退共同约束数据来源优先级。

use super::todo_panel::{
    compute_todo_data, read_todos_for_panel,
};
use crate::app::state::AcpHistoryReplayMode;
use crate::app::{AgentRequest, App, TodoPanelPlacement};
use todo_types::Todo;

fn test_app() -> App {
    App::new().0
}

fn todo(id: &str, status: &str, content: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: status.to_string(),
        content: content.to_string(),
        priority: "medium".to_string(),
    }
}

fn agent_request(query: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id: 1,
        session: "session".to_string(),
        query: query.to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: Some(false),
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
}

// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn todo_panel_placement_labels_match_ui_options() {
    assert_eq!(TodoPanelPlacement::ChatTopRight.label(), "右上角");
    assert_eq!(TodoPanelPlacement::InputBottom.label(), "输入底部");
}













// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn compute_todo_data_ignores_runtime_when_there_is_no_active_session() {
    let mut app = test_app();
    app.get_session_runtime_mut("inactive").request_lifecycle.active_agent_request = Some(agent_request("hidden"));

    let data = compute_todo_data(&app, &[]);

    assert_eq!(data.running_task, "无执行中任务");
}

// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn read_todos_for_panel_returns_empty_when_session_does_not_match() {
    let mut app = test_app();
    app.session_state.active_session_id = Some("active".to_string());
    app.todo_state.chat_todo_session_id = Some("other".to_string());
    app.todo_state.chat_todo_items = vec![todo("1", "pending", "hidden")];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert!(items.is_empty());
    assert_eq!(error, None);
}

// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn read_todos_for_panel_sorts_numeric_ids_before_text_ids() {
    let mut app = test_app();
    app.session_state.active_session_id = Some("session".to_string());
    app.todo_state.chat_todo_session_id = Some("session".to_string());
    app.todo_state.chat_todo_items = vec![
        todo("b", "pending", "b"),
        todo("10", "pending", "ten"),
        todo("2", "pending", "two"),
        todo("a", "pending", "a"),
    ];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["2", "10", "a", "b"]
    );
}

// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn read_todos_for_panel_reads_global_todos_when_both_sessions_are_absent() {
    let mut app = test_app();
    app.todo_state.chat_todo_items = vec![todo("task_2", "pending", "second"), todo("1", "pending", "first")];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>(),
        ["first", "second"]
    );
}

// 宿主保留：宿主会话标识、活动请求查找与待办位置枚举契约。
#[test]
fn read_todos_for_panel_preserves_input_order_for_equal_numeric_ids() {
    let mut app = test_app();
    app.session_state.active_session_id = Some("session".to_string());
    app.todo_state.chat_todo_session_id = Some("session".to_string());
    app.todo_state.chat_todo_items = vec![
        todo("02", "pending", "zero padded"),
        todo("2", "pending", "plain"),
        todo("1", "pending", "first"),
    ];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>(),
        ["first", "zero padded", "plain"]
    );
}
