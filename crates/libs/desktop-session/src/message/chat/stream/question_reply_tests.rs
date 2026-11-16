//! 原宿主问题回复断言：局部弹窗夹具直接调用生产转换。
#[derive(Default)]
struct Fixture {
    question_modal_request_id: Option<String>,
    question_modal_request: Option<question_types::Request>,
    question_modal_answers: Vec<Vec<String>>,
    question_modal_custom: Vec<String>,
}
fn state(app: &mut Fixture) -> super::QuestionState<'_> {
    super::QuestionState {
        question_modal_request_id: &mut app.question_modal_request_id,
        question_modal_request: &mut app.question_modal_request,
        question_modal_answers: &mut app.question_modal_answers,
        question_modal_custom: &mut app.question_modal_custom,
    }
}
fn handle_question_list_loaded(
    app: &mut Fixture,
    result: Result<Vec<question_types::Request>, String>,
) {
    super::apply_list_result(&mut state(app), result);
}
fn handle_question_option_toggled(app: &mut Fixture, index: usize, label: String) {
    super::handle_question_option_toggled(&mut state(app), index, label);
}
fn handle_question_custom_changed(app: &mut Fixture, index: usize, value: String) {
    super::handle_question_custom_changed(&mut state(app), index, value);
}
fn handle_question_submit(app: &mut Fixture) {
    let _ = super::prepare_reply(&mut state(app), |_, _| true);
}
fn handle_question_reject(app: &mut Fixture) {
    let _ = super::reject_request(&mut state(app));
}
fn option(label: &str) -> question_types::OptionInfo {
    question_types::OptionInfo {
        label: label.to_string(),
        description: format!("{label} option"),
        preview: None,
    }
}

fn question(
    header: &str,
    options: Vec<&str>,
    multiple: Option<bool>,
    custom: Option<bool>,
) -> question_types::Info {
    question_types::Info {
        question: format!("{header}?"),
        header: header.to_string(),
        options: options.into_iter().map(option).collect(),
        multiple,
        custom,
    }
}

fn request(id: &str, questions: Vec<question_types::Info>) -> question_types::Request {
    question_types::Request {
        mcp_form: None,
        id: id.to_string(),
        session_id: "session".to_string(),
        questions,
        tool: None,
    }
}

#[test]
fn list_loaded_empty_or_error_clears_modal_state() {
    let mut app = Fixture::default();
    handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, None)])]),
    );

    handle_question_list_loaded(&mut app, Ok(Vec::new()));

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());

    handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, None)])]),
    );
    handle_question_list_loaded(&mut app, Err("offline".to_string()));

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
}

#[test]
fn submit_and_reject_without_request_are_noops() {
    let mut app = Fixture::default();

    handle_question_submit(&mut app);
    handle_question_reject(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
}

#[test]
fn submit_with_request_id_but_missing_request_preserves_pending_id() {
    let mut app = Fixture::default();
    app.question_modal_request_id = Some("pending".to_string());

    handle_question_submit(&mut app);

    assert_eq!(app.question_modal_request_id.as_deref(), Some("pending"));
    assert!(app.question_modal_request.is_none());
}

#[test]
// 提交时仅合并允许自定义答案的题目，随后一次性清理弹窗状态。
fn submit_clears_modal_and_combines_allowed_custom_answers() {
    let mut app = Fixture::default();
    handle_question_list_loaded(
        &mut app,
        Ok(vec![request(
            "a",
            vec![
                question("single", vec!["once"], None, Some(true)),
                question("multi", vec!["x"], Some(true), Some(true)),
            ],
        )]),
    );
    handle_question_option_toggled(&mut app, 0, "once".to_string());
    handle_question_option_toggled(&mut app, 1, "x".to_string());
    handle_question_custom_changed(&mut app, 0, "custom-one".to_string());
    handle_question_custom_changed(&mut app, 1, "x".to_string());

    handle_question_submit(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}

#[test]
fn submit_with_distinct_custom_multi_answer_clears_modal_after_building_reply() {
    let mut app = Fixture::default();
    handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("multi", vec!["x"], Some(true), Some(true))])]),
    );
    handle_question_option_toggled(&mut app, 0, "x".to_string());
    handle_question_custom_changed(&mut app, 0, "custom-y".to_string());

    handle_question_submit(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}

#[test]
fn reject_with_pending_request_clears_modal_state() {
    let mut app = Fixture::default();
    handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, Some(true))])]),
    );
    app.question_modal_custom[0] = "custom".to_string();

    handle_question_reject(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}
