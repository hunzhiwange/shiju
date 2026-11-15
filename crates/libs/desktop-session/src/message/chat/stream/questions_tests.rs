#[derive(Default)]
struct Fixture {
    question_modal_request_id: Option<String>,
    question_modal_request: Option<question_types::Request>,
    question_modal_answers: Vec<Vec<String>>,
    question_modal_custom: Vec<String>,
}
impl Fixture {
    fn state(&mut self) -> super::QuestionState<'_> {
    super::QuestionState {
    question_modal_request_id: &mut self.question_modal_request_id,
    question_modal_request: &mut self.question_modal_request,
    question_modal_answers: &mut self.question_modal_answers,
    question_modal_custom: &mut self.question_modal_custom,
    }
    }
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
fn list_loaded_selects_sorted_first_request_and_defaults_once_for_single_choice() {
    let mut app = Fixture::default();
    let later = request("b", vec![question("multi", vec!["x"], Some(true), None)]);
    let first = request("a", vec![question("scope", vec!["once", "always"], None, None)]);

    super::apply_question_request(&mut app.state(), vec![later, first]);

    assert_eq!(app.question_modal_request_id.as_deref(), Some("a"));
    assert_eq!(app.question_modal_answers, vec![vec!["once".to_string()]]);
    assert_eq!(app.question_modal_custom, vec![String::new()]);
}
#[test]
fn list_loaded_resizes_existing_answers_for_same_request() {
    let mut app = Fixture::default();
    let initial = request("a", vec![question("first", vec!["one"], Some(true), None)]);
    super::apply_question_request(&mut app.state(), vec![initial]);
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_custom[0] = "custom".to_string();

    let updated = request(
        "a",
        vec![
            question("first", vec!["one"], Some(true), None),
            question("second", vec!["two"], None, Some(true)),
        ],
    );
    super::apply_question_request(&mut app.state(), vec![updated]);

    assert_eq!(app.question_modal_answers[0], vec!["one".to_string()]);
    assert_eq!(app.question_modal_answers.len(), 2);
    assert_eq!(app.question_modal_custom[0], "custom");
    assert_eq!(app.question_modal_custom.len(), 2);
}
#[test]
fn list_loaded_truncates_existing_state_when_same_request_has_fewer_questions() {
    let mut app = Fixture::default();
    let initial = request(
        "a",
        vec![
            question("first", vec!["one"], Some(true), None),
            question("second", vec!["two"], Some(true), Some(true)),
        ],
    );
    super::apply_question_request(&mut app.state(), vec![initial]);
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_answers[1].push("two".to_string());
    app.question_modal_custom[0] = "keep".to_string();
    app.question_modal_custom[1] = "drop".to_string();

    let updated = request("a", vec![question("first", vec!["one"], Some(true), None)]);
    super::apply_question_request(&mut app.state(), vec![updated]);

    assert_eq!(app.question_modal_answers, vec![vec!["one".to_string()]]);
    assert_eq!(app.question_modal_custom, vec!["keep".to_string()]);
}
#[test]
fn list_loaded_new_request_resets_answers_and_custom_input() {
    let mut app = Fixture::default();
    let initial = request("a", vec![question("first", vec!["one"], Some(true), Some(true))]);
    super::apply_question_request(&mut app.state(), vec![initial]);
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_custom[0] = "old custom".to_string();

    let replacement = request("b", vec![question("second", vec!["once"], None, Some(true))]);
    super::apply_question_request(&mut app.state(), vec![replacement]);

    assert_eq!(app.question_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.question_modal_answers, vec![vec!["once".to_string()]]);
    assert_eq!(app.question_modal_custom, vec![String::new()]);
}
#[test]
fn option_toggled_replaces_single_choice_and_toggles_multi_choice() {
    let mut app = Fixture::default();
    super::apply_question_request(&mut app.state(), vec![request(
            "a",
            vec![
                question("single", vec!["once", "always"], None, None),
                question("multi", vec!["x", "y"], Some(true), None),
            ],
        )]);

    super::handle_question_option_toggled(&mut app.state(), 0, "always".to_string());
    super::handle_question_option_toggled(&mut app.state(), 1, "x".to_string());
    super::handle_question_option_toggled(&mut app.state(), 1, "y".to_string());
    super::handle_question_option_toggled(&mut app.state(), 1, "x".to_string());

    assert_eq!(app.question_modal_answers[0], vec!["always".to_string()]);
    assert_eq!(app.question_modal_answers[1], vec!["y".to_string()]);
}
#[test]
fn option_toggled_without_request_or_matching_answer_is_ignored() {
    let mut app = Fixture::default();
    app.question_modal_answers = vec![vec!["unchanged".to_string()]];

    super::handle_question_option_toggled(&mut app.state(), 0, "new".to_string());

    assert_eq!(app.question_modal_answers, vec![vec!["unchanged".to_string()]]);

    app.question_modal_request =
        Some(request("a", vec![question("first", vec!["new"], None, None)]));
    super::handle_question_option_toggled(&mut app.state(), 2, "new".to_string());

    assert_eq!(app.question_modal_answers, vec![vec!["unchanged".to_string()]]);
}
#[test]
fn custom_changed_ignores_out_of_range_indexes() {
    let mut app = Fixture::default();
    app.question_modal_custom = vec!["old".to_string()];

    super::handle_question_custom_changed(&mut app.state(), 0, "new".to_string());
    super::handle_question_custom_changed(&mut app.state(), 3, "ignored".to_string());

    assert_eq!(app.question_modal_custom, vec!["new".to_string()]);
}
