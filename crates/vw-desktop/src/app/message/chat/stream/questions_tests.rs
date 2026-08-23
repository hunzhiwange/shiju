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

fn option(label: &str) -> vw_shared::question::OptionInfo {
    vw_shared::question::OptionInfo {
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
) -> vw_shared::question::Info {
    vw_shared::question::Info {
        question: format!("{header}?"),
        header: header.to_string(),
        options: options.into_iter().map(option).collect(),
        multiple,
        custom,
    }
}

fn request(id: &str, questions: Vec<vw_shared::question::Info>) -> vw_shared::question::Request {
    vw_shared::question::Request {
        id: id.to_string(),
        session_id: "session".to_string(),
        questions,
        tool: None,
    }
}

#[test]
// 首次加载按请求标识排序，并且只为明确的单选题写入默认答案。
fn list_loaded_selects_sorted_first_request_and_defaults_once_for_single_choice() {
    let (mut app, _task) = crate::app::App::new();
    let later = request("b", vec![question("multi", vec!["x"], Some(true), None)]);
    let first = request("a", vec![question("scope", vec!["once", "always"], None, None)]);

    super::handle_question_list_loaded(&mut app, Ok(vec![later, first]));

    assert_eq!(app.question_modal_request_id.as_deref(), Some("a"));
    assert_eq!(app.question_modal_answers, vec![vec!["once".to_string()]]);
    assert_eq!(app.question_modal_custom, vec![String::new()]);
}

#[test]
// 同一请求更新时答案数组随题目数量调整，但仍保留索引有效的已有选择。
fn list_loaded_resizes_existing_answers_for_same_request() {
    let (mut app, _task) = crate::app::App::new();
    let initial = request("a", vec![question("first", vec!["one"], Some(true), None)]);
    super::handle_question_list_loaded(&mut app, Ok(vec![initial]));
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_custom[0] = "custom".to_string();

    let updated = request(
        "a",
        vec![
            question("first", vec!["one"], Some(true), None),
            question("second", vec!["two"], None, Some(true)),
        ],
    );
    super::handle_question_list_loaded(&mut app, Ok(vec![updated]));

    assert_eq!(app.question_modal_answers[0], vec!["one".to_string()]);
    assert_eq!(app.question_modal_answers.len(), 2);
    assert_eq!(app.question_modal_custom[0], "custom");
    assert_eq!(app.question_modal_custom.len(), 2);
}

#[test]
fn list_loaded_truncates_existing_state_when_same_request_has_fewer_questions() {
    let (mut app, _task) = crate::app::App::new();
    let initial = request(
        "a",
        vec![
            question("first", vec!["one"], Some(true), None),
            question("second", vec!["two"], Some(true), Some(true)),
        ],
    );
    super::handle_question_list_loaded(&mut app, Ok(vec![initial]));
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_answers[1].push("two".to_string());
    app.question_modal_custom[0] = "keep".to_string();
    app.question_modal_custom[1] = "drop".to_string();

    let updated = request("a", vec![question("first", vec!["one"], Some(true), None)]);
    super::handle_question_list_loaded(&mut app, Ok(vec![updated]));

    assert_eq!(app.question_modal_answers, vec![vec!["one".to_string()]]);
    assert_eq!(app.question_modal_custom, vec!["keep".to_string()]);
}

#[test]
fn list_loaded_new_request_resets_answers_and_custom_input() {
    let (mut app, _task) = crate::app::App::new();
    let initial = request("a", vec![question("first", vec!["one"], Some(true), Some(true))]);
    super::handle_question_list_loaded(&mut app, Ok(vec![initial]));
    app.question_modal_answers[0].push("one".to_string());
    app.question_modal_custom[0] = "old custom".to_string();

    let replacement = request("b", vec![question("second", vec!["once"], None, Some(true))]);
    super::handle_question_list_loaded(&mut app, Ok(vec![replacement]));

    assert_eq!(app.question_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.question_modal_answers, vec![vec!["once".to_string()]]);
    assert_eq!(app.question_modal_custom, vec![String::new()]);
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
// 单选替换与多选切换共用入口，题型决定是否保留先前选项。
fn option_toggled_replaces_single_choice_and_toggles_multi_choice() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_question_list_loaded(
        &mut app,
        Ok(vec![request(
            "a",
            vec![
                question("single", vec!["once", "always"], None, None),
                question("multi", vec!["x", "y"], Some(true), None),
            ],
        )]),
    );

    super::handle_question_option_toggled(&mut app, 0, "always".to_string());
    super::handle_question_option_toggled(&mut app, 1, "x".to_string());
    super::handle_question_option_toggled(&mut app, 1, "y".to_string());
    super::handle_question_option_toggled(&mut app, 1, "x".to_string());

    assert_eq!(app.question_modal_answers[0], vec!["always".to_string()]);
    assert_eq!(app.question_modal_answers[1], vec!["y".to_string()]);
}

#[test]
fn option_toggled_without_request_or_matching_answer_is_ignored() {
    let (mut app, _task) = crate::app::App::new();
    app.question_modal_answers = vec![vec!["unchanged".to_string()]];

    super::handle_question_option_toggled(&mut app, 0, "new".to_string());

    assert_eq!(app.question_modal_answers, vec![vec!["unchanged".to_string()]]);

    app.question_modal_request =
        Some(request("a", vec![question("first", vec!["new"], None, None)]));
    super::handle_question_option_toggled(&mut app, 2, "new".to_string());

    assert_eq!(app.question_modal_answers, vec![vec!["unchanged".to_string()]]);
}

#[test]
fn custom_changed_ignores_out_of_range_indexes() {
    let (mut app, _task) = crate::app::App::new();
    app.question_modal_custom = vec!["old".to_string()];

    super::handle_question_custom_changed(&mut app, 0, "new".to_string());
    super::handle_question_custom_changed(&mut app, 3, "ignored".to_string());

    assert_eq!(app.question_modal_custom, vec!["new".to_string()]);
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
