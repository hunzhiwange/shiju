//! 原费用回执断言：局部会话状态调用真实费用合并。
struct Fixture {
    active_session_id: Option<String>,
    active_session_view_state: session_ui_types::ChatSession,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            active_session_id: None,
            active_session_view_state: session_ui_types::ChatSession {
                id: "session-1".into(),
                title: "Session".into(),
                messages: Vec::new(),
                message_ids: Vec::new(),
                calls: Vec::new(),
                steps: Vec::new(),
                created_ms: 1,
                updated_ms: 1,
            },
        }
    }
}
fn handle_agent_step_cost_loaded(
    app: &mut Fixture,
    _session: String,
    index: u32,
    model: Option<String>,
    cost: Option<f64>,
) {
    super::apply_step_cost(&mut app.active_session_view_state, index, model, cost, 100);
}
#[test]
fn handle_agent_step_cost_loaded_returns_none_for_empty_session_id() {
    let mut app = Fixture::default();
    let _task = handle_agent_step_cost_loaded(&mut app, "".to_string(), 1, None, None);
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_cost_loaded_noop_when_step_missing_and_both_fields_none() {
    let mut app = Fixture::default();
    let session_id = "session-1".to_string();
    let _task = handle_agent_step_cost_loaded(&mut app, session_id, 1, None, None);
    // 没有目标步骤且没有新字段时，不应凭空创建记录。
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_cost_loaded_creates_step_when_model_provided() {
    let mut app = Fixture::default();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    let _task = handle_agent_step_cost_loaded(
        &mut app,
        session_id,
        1,
        Some("openai/gpt-4".to_string()),
        None,
    );

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, None);
}

#[test]
fn handle_agent_step_cost_loaded_creates_step_when_cost_provided() {
    let mut app = Fixture::default();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    let _task = handle_agent_step_cost_loaded(&mut app, session_id, 1, None, Some(0.05));

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.05));
    assert_eq!(app.active_session_view_state.steps[0].model, None);
}

#[test]
fn handle_agent_step_cost_loaded_updates_existing_step_model_and_cost() {
    let mut app = Fixture::default();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    // 首次回填在步骤缺失时创建最小记录。
    let _task1 = handle_agent_step_cost_loaded(
        &mut app,
        session_id.clone(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.03),
    );

    // 后续回填只更新模型和成本，不复制步骤。
    let _task2 = handle_agent_step_cost_loaded(
        &mut app,
        session_id,
        1,
        Some("openai/gpt-4-turbo".to_string()),
        Some(0.05),
    );

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4-turbo"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.05));
}

#[test]
fn handle_agent_step_cost_loaded_does_not_overwrite_with_none_on_existing_step() {
    let mut app = Fixture::default();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    // 先建立同时含模型与成本的步骤，作为保留字段的基线。
    let _task1 = handle_agent_step_cost_loaded(
        &mut app,
        session_id.clone(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.03),
    );

    // 空回填不得覆盖已经记录的模型与成本。
    let _task2 = handle_agent_step_cost_loaded(&mut app, session_id, 1, None, None);

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.03));
}
