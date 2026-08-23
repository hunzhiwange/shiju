//! 验证提问工具的问题与答案解析、兼容载荷及交互状态视图。
//! 结构化答案优先于旧输出，自定义和空答案组需归一化后再参与展示。

use super::question_view::{
    derived_summary, parse_answers, parse_questions, question_request_targets_message,
    tool_question_view,
};
use crate::app::{App, Message};
use serde_json::json;
use std::collections::BTreeMap;
use vw_shared::question::{Info, Request, ToolMeta};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn question(header: &str, question: &str) -> Info {
    Info {
        header: header.to_string(),
        question: question.to_string(),
        options: Vec::new(),
        multiple: None,
        custom: None,
    }
}

// 始终通过真实 JSON 输入构造问题列表，避免测试绕过生产解析入口。
fn question_input(questions: &[Info]) -> String {
    serde_json::to_string(&json!({ "questions": questions })).expect("question input json")
}

fn request_with_tool(message_id: &str) -> Request {
    Request {
        id: "req".to_string(),
        session_id: "s".to_string(),
        questions: Vec::new(),
        tool: Some(ToolMeta { message_id: message_id.to_string(), call_id: "c1".to_string() }),
    }
}

#[test]
fn parse_questions_accepts_json_array() {
    let questions =
        parse_questions(r#"[{"header":"Confirm","question":"Continue?","options":[]}]"#);

    assert_eq!(questions.len(), 1);
}

#[test]
fn parse_questions_accepts_wrapped_questions_and_rejects_plain_text() {
    let questions =
        parse_questions(r#"{"questions":[{"header":"Confirm","question":"Ship?","options":[]}]}"#);

    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].header, "Confirm");
    assert!(parse_questions("Ship?").is_empty());
    assert!(parse_questions("{bad json").is_empty());
}

#[test]
fn parse_questions_returns_empty_for_missing_or_invalid_question_lists() {
    assert!(parse_questions(r#"{"other":[]}"#).is_empty());
    assert!(parse_questions(r#"{"questions":"not a list"}"#).is_empty());
    assert!(parse_questions(r#"[{"header":12,"question":"Ship?","options":[]}]"#).is_empty());
}

#[test]
fn parse_answers_reads_structured_and_output_answers() {
    let structured = parse_answers(&json!({"data":{"answers":{"q1":"yes"}}}), "");
    let fallback = parse_answers(&json!({}), r#"[["yes","__custom__:later"]]"#);

    assert_eq!(structured.get("q1").map(String::as_str), Some("yes"));
    assert_eq!(fallback.get("0").map(String::as_str), Some("yes / later"));
}

#[test]
fn parse_answers_reads_nested_data_and_falls_back_after_invalid_structured_answers() {
    let nested = parse_answers(&json!({"result":{"data":{"answers":{"Ship?":"yes"}}}}), "");
    let fallback = parse_answers(&json!({"data":{"answers":["bad"]}}), r#"[["fallback"]]"#);

    assert_eq!(nested.get("Ship?").map(String::as_str), Some("yes"));
    assert_eq!(fallback.get("0").map(String::as_str), Some("fallback"));
    assert!(parse_answers(&json!({}), "not json").is_empty());
}

#[test]
fn parse_answers_prefers_valid_structured_answers_over_legacy_output() {
    let answers =
        parse_answers(&json!({"data":{"answers":{"Ship?":"structured"}}}), r#"[["legacy"]]"#);

    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get("Ship?").map(String::as_str), Some("structured"));
    assert!(!answers.contains_key("0"));
}

#[test]
fn parse_answers_normalizes_custom_and_empty_legacy_answer_groups() {
    let answers = parse_answers(&json!({}), r#"[[],["__custom__:later","plain","__custom__:"]]"#);

    assert_eq!(answers.get("0").map(String::as_str), Some(""));
    assert_eq!(answers.get("1").map(String::as_str), Some("later / plain / "));
}

#[test]
fn question_request_targets_message_requires_matching_message_id() {
    assert!(!question_request_targets_message(None, Some("m1")));

    let request_without_tool = Request {
        id: "req".to_string(),
        session_id: "s".to_string(),
        questions: Vec::new(),
        tool: None,
    };
    assert!(question_request_targets_message(Some(&request_without_tool), None));

    let request_with_tool = request_with_tool("m1");
    assert!(question_request_targets_message(Some(&request_with_tool), Some("m1")));
    assert!(!question_request_targets_message(Some(&request_with_tool), Some("m2")));
    assert!(!question_request_targets_message(Some(&request_with_tool), None));
}

#[test]
fn derived_summary_prefers_explicit_summary_and_counts_states() {
    let app = test_app();
    let questions = vec![question("A", "One?"), question("B", "Two?")];
    let mut answers = BTreeMap::new();

    assert_eq!(derived_summary(&app, &questions, &answers, false, " explicit "), "explicit");
    assert_eq!(derived_summary(&app, &questions, &answers, true, ""), "等待 2 个问题的回答");
    assert_eq!(derived_summary(&app, &questions[..1], &answers, false, ""), "1 个问题");

    answers.insert("One?".to_string(), "yes".to_string());
    assert_eq!(derived_summary(&app, &questions, &answers, false, ""), "已回答 1 个问题");
    answers.insert("Two?".to_string(), "no".to_string());
    assert_eq!(derived_summary(&app, &questions, &answers, false, ""), "已回答 2 个问题");
}

#[test]
fn derived_summary_covers_empty_single_and_modal_running_states() {
    let mut app = test_app();
    let answers = BTreeMap::new();
    let questions = vec![question("A", "One?")];

    assert_eq!(derived_summary(&app, &[], &answers, false, ""), "");
    assert_eq!(derived_summary(&app, &[], &answers, true, ""), "等待回答");
    assert_eq!(derived_summary(&app, &questions, &answers, true, ""), "等待 1 个问题的回答");

    app.question_modal_request = Some(request_with_tool("m1"));
    assert_eq!(derived_summary(&app, &questions, &answers, true, ""), "等待你的回答");
}

#[test]
fn tool_question_view_rejects_invalid_or_empty_inputs() {
    let app = test_app();

    assert!(tool_question_view(&app, 0, 0, "tool question").is_none());
    assert!(tool_question_view(&app, 0, 0, "question\n{}").is_none());
    assert!(tool_question_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(tool_question_view(&app, 0, 0, "tool question\nnot json").is_none());
    assert!(
        tool_question_view(
            &app,
            0,
            0,
            r#"tool question
{"status":"completed","input":"{}"}"#
        )
        .is_none()
    );
}

#[test]
fn tool_question_view_builds_completed_running_and_error_cards() {
    let mut app = test_app();
    app.chat_message_ids = vec![Some("m1".to_string())];
    app.question_modal_request = Some(request_with_tool("m1"));

    let completed = tool_question_view(&app, 0, 1, r#"tool question
{"status":"completed","input":"{\"questions\":[{\"header\":\"Confirm\",\"question\":\"Ship?\",\"options\":[]}]}","output":"[[\"yes\"]]"}"#)
    .expect("completed question view");
    keep_element(completed);

    let running = tool_question_view(&app, 0, 2, r#"tool question
{"status":"running","input":"{\"questions\":[{\"header\":\"\",\"question\":\"Continue?\",\"options\":[]}]}"}"#)
    .expect("running question view");
    keep_element(running);

    let error = tool_question_view(&app, 0, 3, r#"tool question
{"status":"error","summary":"failed","input":"{\"questions\":[{\"header\":\"Confirm\",\"question\":\"Ship?\",\"options\":[]}]}"}"#)
    .expect("error question view");
    keep_element(error);
}

#[test]
fn tool_question_view_renders_summary_without_questions() {
    let app = test_app();
    let visible = r#"tool question
{"status":"completed","summary":"finished"}"#;

    let element = tool_question_view(&app, 0, 0, visible).expect("summary-only question view");
    keep_element(element);
}

#[test]
fn tool_question_view_renders_hovered_structured_answers_and_extra_count() {
    let mut app = test_app();
    app.chat_tool_hovered_idx = Some((2_u64 << 32) | 3);
    let questions = vec![
        question("First", "Ship?"),
        question("Second", "Rollback?"),
        question("Third", "Notify?"),
        question("Fourth", "Archive?"),
    ];
    let input = question_input(&questions);
    let visible = format!(
        "tool question\n{}",
        json!({
            "status": "completed",
            "input": input,
            "data": {
                "answers": {
                    "Ship?": "yes",
                    "Rollback?": "  "
                }
            }
        })
    );

    let element =
        tool_question_view(&app, 2, 3, &visible).expect("hovered structured question view");
    keep_element(element);
}

#[test]
fn tool_question_view_handles_denied_and_inactive_running_states() {
    let mut app = test_app();
    app.chat_message_ids = vec![Some("other".to_string())];
    app.question_modal_request = Some(request_with_tool("m1"));
    let input = question_input(&[question("", "Continue?")]);
    let running = format!("tool question\n{}", json!({"status": "running", "input": input}));
    let denied = r#"tool question
{"status":"denied","input":"{\"questions\":[{\"header\":\"Confirm\",\"question\":\"Ship?\",\"options\":[]}]}","output":"[[\"ignored\"]]"}"#;

    let running = tool_question_view(&app, 0, 0, &running).expect("inactive running question view");
    keep_element(running);
    let denied = tool_question_view(&app, 0, 1, denied).expect("denied question view");
    keep_element(denied);
}

#[test]
fn tool_question_view_accepts_alias_for_active_request_without_tool_metadata() {
    let mut app = test_app();
    app.question_modal_request = Some(Request {
        id: "req".to_string(),
        session_id: "s".to_string(),
        questions: Vec::new(),
        tool: None,
    });
    let visible = r#"tool AskUserQuestion
{"status":"running","input":"{\"questions\":[{\"header\":\"Confirm\",\"question\":\"Continue?\",\"options\":[]}]}"}"#;

    let element = tool_question_view(&app, 9, 4, visible)
        .expect("active aliased question view without message metadata");
    keep_element(element);
}
