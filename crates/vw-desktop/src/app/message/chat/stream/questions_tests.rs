//! 验证流式问题请求的排序、默认答案、状态重置和提交流程。
//!
//! 同一请求更新与新请求替换分别测试，答案数组必须随题目数量安全扩缩。

#![allow(unused_must_use)]
#[test]
fn questions_tests_module_is_wired() {
    assert!(module_path!().ends_with("questions_tests"));
}

#[test]
fn question_poll_tick_schedules_gateway_refresh() {
    let task = super::handle_question_poll_tick();

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
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
    let (mut app, _task) = crate::app::App::new();
    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, None)])]),
    );

    super::handle_question_list_loaded(&mut app, Ok(Vec::new()));

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());

    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, None)])]),
    );
    super::handle_question_list_loaded(&mut app, Err("offline".to_string()));

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
}

#[test]
fn submit_and_reject_without_request_are_noops() {
    let (mut app, _task) = crate::app::App::new();

    super::handle_question_submit(&mut app);
    super::handle_question_reject(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
}

#[test]
fn submit_with_request_id_but_missing_request_preserves_pending_id() {
    let (mut app, _task) = crate::app::App::new();
    app.question_modal_request_id = Some("pending".to_string());

    super::handle_question_submit(&mut app);

    assert_eq!(app.question_modal_request_id.as_deref(), Some("pending"));
    assert!(app.question_modal_request.is_none());
}

#[test]
// 提交时仅合并允许自定义答案的题目，随后一次性清理弹窗状态。
fn submit_clears_modal_and_combines_allowed_custom_answers() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request(
            "a",
            vec![
                question("single", vec!["once"], None, Some(true)),
                question("multi", vec!["x"], Some(true), Some(true)),
            ],
        )]),
    );
    super::handle_question_option_toggled(&mut app, 0, "once".to_string());
    super::handle_question_option_toggled(&mut app, 1, "x".to_string());
    super::handle_question_custom_changed(&mut app, 0, "custom-one".to_string());
    super::handle_question_custom_changed(&mut app, 1, "x".to_string());

    super::handle_question_submit(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}

#[test]
fn submit_with_distinct_custom_multi_answer_clears_modal_after_building_reply() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("multi", vec!["x"], Some(true), Some(true))])]),
    );
    super::handle_question_option_toggled(&mut app, 0, "x".to_string());
    super::handle_question_custom_changed(&mut app, 0, "custom-y".to_string());

    super::handle_question_submit(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}

#[test]
fn reject_with_pending_request_clears_modal_state() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request("a", vec![question("first", vec!["one"], None, Some(true))])]),
    );
    app.question_modal_custom[0] = "custom".to_string();

    super::handle_question_reject(&mut app);

    assert!(app.question_modal_request_id.is_none());
    assert!(app.question_modal_request.is_none());
    assert!(app.question_modal_answers.is_empty());
    assert!(app.question_modal_custom.is_empty());
}

#[test]
fn reply_and_reject_results_accept_success_and_error_outcomes() {
    let _success_reply = super::handle_question_reply_submitted(Ok(()));
    let _failed_reply = super::handle_question_reply_submitted(Err("reply failed".to_string()));
    let _success_reject = super::handle_question_rejected(Ok(()));
    let _failed_reject = super::handle_question_rejected(Err("reject failed".to_string()));
}
