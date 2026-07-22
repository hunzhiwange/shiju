//! Question 工具测试模块
//!
//! 本模块包含针对 `QuestionTool` 及其相关辅助函数的单元测试。
//! 主要测试以下内容：
//! - 工具 schema 的结构完整性（必需字段验证）
//! - 问题头部生成函数的长度限制行为
//!
//! # 测试覆盖范围
//!
//! - `question_schema_has_required_fields`: 验证 JSON schema 符合规范
//! - `header_from_question_limits_length`: 验证字符串截断逻辑

use super::super::*;
use crate::app::agent::question as runtime_question;
use crate::app::agent::tools::question::header_from_question;
use crate::app::agent::tools::traits::Tool;
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};

fn validation_error(input: Value) -> String {
    QuestionTool::new("session-123".to_string())
        .validate_input(input)
        .expect_err("input should be rejected")
        .to_string()
}

fn normalized_question(
    question: &str,
    multi_select: bool,
    options: Vec<(&str, Option<&str>)>,
) -> super::NormalizedQuestion {
    super::NormalizedQuestion {
        question: question.to_string(),
        header: super::header_from_question(question),
        options: options
            .into_iter()
            .map(|(label, preview)| super::NormalizedOption {
                label: label.to_string(),
                description: format!("{label} description"),
                preview: preview.map(str::to_string),
            })
            .collect(),
        multi_select,
        custom: true,
    }
}

async fn pending_question_for_session(session_id: &str) -> runtime_question::Request {
    loop {
        if let Some(request) =
            runtime_question::list().into_iter().find(|request| request.session_id == session_id)
        {
            return request;
        }
        sleep(Duration::from_millis(10)).await;
    }
}

/// 测试 QuestionTool 的 schema 是否包含所有必需字段
///
/// # 测试目标
///
/// 验证 `QuestionTool::schema()` 返回的 JSON schema 满足以下条件：
/// 1. 类型为 "object"（表示这是一个对象结构）
/// 2. 包含 "questions" 属性定义
/// 3. "questions" 被标记为必需字段
///
/// # 重要性
///
/// Schema 的正确性对于工具的自动验证和文档生成至关重要。
/// 如果缺少必需字段标记，可能导致客户端提交无效的问题数据。
#[test]
fn question_schema_has_required_fields() {
    // 获取 QuestionTool 的 JSON schema
    let schema = QuestionTool::schema();

    // 验证 schema 类型为对象
    assert_eq!(schema["type"], "object");

    // 验证 questions 属性存在且为对象类型
    assert!(schema["properties"]["questions"].is_object());
    assert_eq!(schema["properties"]["questions"]["minItems"], 1);
    assert_eq!(schema["properties"]["questions"]["maxItems"], 4);
    assert!(schema["properties"]["answers"].is_object());
    assert!(schema["properties"]["annotations"].is_object());
    assert!(schema["properties"]["metadata"].is_object());

    // 验证 questions 字段被标记为必需
    assert_eq!(schema["required"], json!(["questions"]));
}

/// 测试 header_from_question 函数的长度限制功能
///
/// # 测试目标
///
/// 验证 `header_from_question` 函数能够正确截断过长的字符串，
/// 确保生成的头部字符串不超过预期的最大长度。
///
/// # 测试数据
///
/// 输入：36 个字符的字母数字字符串 "abcdefghijklmnopqrstuvwxyz1234567890"
/// 预期输出：前 30 个字符 "abcdefghijklmnopqrstuvwxyz1234"
///
/// # 重要性
///
/// 长度限制可以防止：
/// - 日志显示时的格式破坏
/// - UI 布局溢出
/// - 存储时的字段超限
#[test]
fn header_from_question_limits_length() {
    // 测试用例：36 字符的输入字符串
    let header = header_from_question("abcdefghijklmnopqrstuvwxyz1234567890");

    // 验证输出被截断为 12 个字符
    assert_eq!(header, "abcdefghijkl");
}

#[test]
fn question_tool_spec_uses_claude_surface() {
    let tool = QuestionTool::new("session-123".to_string());
    let spec = tool.spec();

    assert_eq!(tool.name(), "question");
    assert!(!tool.description().is_empty());
    assert_eq!(spec.id, ASK_USER_QUESTION_TOOL_ID);
    assert!(spec.aliases.iter().any(|alias| alias == QUESTION_TOOL_ALIAS));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.requires_user_interaction);
    assert!(spec.concurrency_safe);
    assert!(spec.strict);
}

#[test]
fn question_validate_input_normalizes_claude_fields() {
    let tool = QuestionTool::new("session-123".to_string());

    let normalized = tool
        .validate_input(json!({
            "questions": [
                {
                    "question": "Pick a library?",
                    "options": [
                        {
                            "label": "Zod",
                            "description": "Stay close to Claude Code",
                            "preview": "<pre>import { z } from 'zod'</pre>"
                        },
                        {
                            "label": "Valibot",
                            "description": "Smaller runtime"
                        }
                    ],
                    "multiSelect": false
                }
            ]
        }))
        .expect("input should normalize");

    assert_eq!(normalized["questions"][0]["header"], "Pick a libra");
    assert_eq!(normalized["questions"][0]["multiSelect"], false);
    assert_eq!(normalized["questions"][0]["custom"], true);
    assert_eq!(
        normalized["questions"][0]["options"][0]["preview"],
        "<pre>import { z } from 'zod'</pre>"
    );
}

#[test]
fn question_validate_input_rejects_duplicate_question_text() {
    let tool = QuestionTool::new("session-123".to_string());

    let err = tool
        .validate_input(json!({
            "questions": [
                {
                    "question": "Same question?",
                    "options": [
                        { "label": "A", "description": "first" },
                        { "label": "B", "description": "second" }
                    ]
                },
                {
                    "question": "Same question?",
                    "options": [
                        { "label": "C", "description": "third" },
                        { "label": "D", "description": "fourth" }
                    ]
                }
            ]
        }))
        .expect_err("duplicate questions should fail");

    assert!(err.to_string().contains("Question texts must be unique"));
}

#[tokio::test]
async fn question_call_with_prefilled_answers_returns_claude_shape() {
    let tool = QuestionTool::new("session-123".to_string());
    let input = tool
        .validate_input(json!({
            "questions": [
                {
                    "question": "Which library should we use?",
                    "header": "Library",
                    "options": [
                        {
                            "label": "Zod",
                            "description": "Stay close to Claude Code",
                            "preview": "<pre>import { z } from 'zod'</pre>"
                        },
                        {
                            "label": "Valibot",
                            "description": "Smaller runtime"
                        }
                    ]
                }
            ],
            "answers": {
                "Which library should we use?": "Zod"
            },
            "annotations": {
                "Which library should we use?": {
                    "preview": "<pre>import { z } from 'zod'</pre>",
                    "notes": "Keep parity with Claude"
                }
            }
        }))
        .expect("input should normalize");

    let result = tool.call(input).await.expect("call should succeed");

    assert_eq!(result.data["questions"][0]["header"], "Library");
    assert_eq!(result.data["questions"][0]["multiSelect"], false);
    assert_eq!(result.data["answers"]["Which library should we use?"], "Zod");
    assert_eq!(
        result.data["annotations"]["Which library should we use?"]["notes"],
        "Keep parity with Claude"
    );
    assert!(
        result
            .model_result
            .as_str()
            .is_some_and(|text| text.contains("User has answered your questions"))
    );
}

#[test]
fn question_validate_input_rejects_question_count_outside_bounds() {
    let empty_error = validation_error(json!({ "questions": [] }));
    assert!(empty_error.contains("questions must contain 1-4 items"));

    let questions =
        (0..5).map(|index| json!({ "question": format!("Question {index}?") })).collect::<Vec<_>>();
    let excessive_error = validation_error(json!({ "questions": questions }));
    assert!(excessive_error.contains("questions must contain 1-4 items"));
}

#[test]
fn question_validate_input_rejects_blank_question_and_orphan_annotations() {
    let blank_error = validation_error(json!({
        "questions": [{ "question": "   " }]
    }));
    assert!(blank_error.contains("question text must not be empty"));

    let annotation_error = validation_error(json!({
        "questions": [{ "question": "Choose?" }],
        "annotations": { "Choose?": { "notes": "context" } }
    }));
    assert!(annotation_error.contains("annotations require answers"));
}

#[test]
fn question_validate_input_normalizes_open_question_defaults_and_header() {
    let tool = QuestionTool::new("session-123".to_string());
    let normalized = tool
        .validate_input(json!({
            "questions": [{
                "question": "  Which implementation should we choose?  ",
                "header": "   "
            }]
        }))
        .expect("open question should normalize");

    assert_eq!(normalized["questions"][0]["question"], "Which implementation should we choose?");
    assert_eq!(normalized["questions"][0]["header"], "Which implem");
    assert_eq!(normalized["questions"][0]["options"], json!([]));
    assert_eq!(normalized["questions"][0]["multiSelect"], false);
    assert_eq!(normalized["questions"][0]["custom"], true);
}

#[test]
fn question_validate_input_rejects_option_count_outside_bounds() {
    for options in [
        json!([{ "label": "A", "description": "first" }]),
        json!([
            { "label": "A", "description": "first" },
            { "label": "B", "description": "second" },
            { "label": "C", "description": "third" },
            { "label": "D", "description": "fourth" },
            { "label": "E", "description": "fifth" }
        ]),
    ] {
        let error = validation_error(json!({
            "questions": [{ "question": "Choose?", "options": options }]
        }));
        assert!(error.contains("must have 2-4 options"));
    }
}

#[test]
fn question_validate_input_rejects_empty_and_duplicate_option_labels() {
    let empty_error = validation_error(json!({
        "questions": [{
            "question": "Choose?",
            "options": [
                { "label": "   ", "description": "first" },
                { "label": "B", "description": "second" }
            ]
        }]
    }));
    assert!(empty_error.contains("contains an empty option label"));

    let duplicate_error = validation_error(json!({
        "questions": [{
            "question": "Choose?",
            "options": [
                { "label": " Same ", "description": "first" },
                { "label": "Same", "description": "second" }
            ]
        }]
    }));
    assert!(duplicate_error.contains("Option labels must be unique"));
}

#[test]
fn question_validate_input_rejects_invalid_optionless_modes() {
    let no_custom_error = validation_error(json!({
        "questions": [{ "question": "Choose?", "custom": false }]
    }));
    assert!(no_custom_error.contains("must allow custom input"));

    let multi_select_error = validation_error(json!({
        "questions": [{ "question": "Choose?", "multiSelect": true }]
    }));
    assert!(multi_select_error.contains("cannot enable multiSelect without options"));
}

#[test]
fn question_validate_input_rejects_preview_for_multi_select() {
    let error = validation_error(json!({
        "questions": [{
            "question": "Choose several?",
            "multiple": true,
            "options": [
                { "label": "A", "description": "first", "preview": "preview" },
                { "label": "B", "description": "second" }
            ]
        }]
    }));

    assert!(error.contains("cannot use previews when multiSelect is enabled"));
}

#[test]
fn question_validate_input_trims_nested_text_and_multiple_alias() {
    let tool = QuestionTool::new("session-123".to_string());
    let normalized = tool
        .validate_input(json!({
            "questions": [{
                "question": "Choose?",
                "header": " Choice ",
                "multiple": false,
                "options": [
                    { "label": " A ", "description": " first ", "preview": " preview " },
                    { "label": " B ", "description": " second ", "preview": "   " }
                ]
            }],
            "answers": { "Choose?": " A " },
            "annotations": { "Choose?": { "preview": " detail ", "notes": " note " } },
            "metadata": { "source": " agent " }
        }))
        .expect("nested text should normalize");

    assert_eq!(normalized["questions"][0]["header"], "Choice");
    assert_eq!(normalized["questions"][0]["options"][0]["label"], "A");
    assert_eq!(normalized["questions"][0]["options"][0]["description"], "first");
    assert_eq!(normalized["questions"][0]["options"][0]["preview"], "preview");
    assert!(normalized["questions"][0]["options"][1].get("preview").is_none());
    assert_eq!(normalized["answers"]["Choose?"], "A");
    assert_eq!(normalized["annotations"]["Choose?"]["notes"], "note");
    assert_eq!(normalized["metadata"]["source"], "agent");
}

#[test]
fn question_validate_input_rejects_incomplete_or_invalid_answers() {
    let missing_error = validation_error(json!({
        "questions": [{ "question": "Choose?" }],
        "answers": {}
    }));
    assert!(missing_error.contains("one entry for each question"));

    let wrong_key_error = validation_error(json!({
        "questions": [{ "question": "Choose?" }],
        "answers": { "Different?": "A" }
    }));
    assert!(wrong_key_error.contains("exact question text"));

    let empty_value_error = validation_error(json!({
        "questions": [{ "question": "Choose?" }],
        "answers": { "Choose?": "   " }
    }));
    assert!(empty_value_error.contains("must not contain empty values"));
}

#[test]
fn question_validate_input_rejects_malformed_shape() {
    let error = validation_error(json!({ "questions": "invalid" }));

    assert!(error.contains("Missing or invalid parameters"));
}

#[test]
fn question_validate_input_rejects_unknown_annotations_and_drops_empty_ones() {
    let unknown_error = validation_error(json!({
        "questions": [{ "question": "Choose?" }],
        "answers": { "Choose?": "A" },
        "annotations": { "Different?": { "notes": "context" } }
    }));
    assert!(unknown_error.contains("annotations keys must match"));

    let tool = QuestionTool::new("session-123".to_string());
    let normalized = tool
        .validate_input(json!({
            "questions": [{ "question": "Choose?" }],
            "answers": { "Choose?": "A" },
            "annotations": { "Choose?": { "preview": " ", "notes": " " } },
            "metadata": { "source": " " }
        }))
        .expect("empty optional text should be omitted");
    assert!(normalized.get("annotations").is_none());
    assert!(normalized.get("metadata").is_none());
}

#[test]
fn normalize_selected_answers_strips_custom_prefix_blanks_and_duplicates() {
    let answers = vec![
        " A ".to_string(),
        "__custom__: custom answer ".to_string(),
        "A".to_string(),
        "__custom__:   ".to_string(),
    ];

    assert_eq!(super::normalize_selected_answers(&answers), vec!["A", "custom answer"]);
}

#[test]
fn build_answer_map_aligns_questions_and_fills_missing_answers() {
    let questions = vec![
        normalized_question("First?", false, Vec::new()),
        normalized_question("Second?", false, Vec::new()),
    ];
    let answers = vec![vec![" A ".to_string(), "A".to_string(), "B".to_string()]];

    let mapped = super::build_answer_map(&questions, &answers);

    assert_eq!(mapped.get("First?").map(String::as_str), Some("A, B"));
    assert_eq!(mapped.get("Second?").map(String::as_str), Some(""));
}

#[test]
fn inferred_annotations_only_include_single_preview_selection() {
    let questions = vec![
        normalized_question("Single?", false, vec![("A", Some("preview A"))]),
        normalized_question("Multiple?", true, vec![("B", Some("preview B"))]),
        normalized_question("Custom?", false, vec![("C", Some("preview C"))]),
        normalized_question("Many answers?", false, vec![("D", Some("preview D"))]),
        normalized_question("No preview?", false, vec![("E", None)]),
    ];
    let answers = vec![
        vec!["A".to_string()],
        vec!["B".to_string()],
        vec!["custom value".to_string()],
        vec!["D".to_string(), "other".to_string()],
        vec!["E".to_string()],
    ];

    let annotations = super::inferred_annotations(&questions, &answers)
        .expect("the matching single selection should infer an annotation");

    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations["Single?"].preview.as_deref(), Some("preview A"));
    assert!(annotations["Single?"].notes.is_none());

    let no_annotations = super::inferred_annotations(&questions, &[]);
    assert!(no_annotations.is_none());
}

#[test]
fn build_questions_preserves_normalized_interaction_fields() {
    let args = super::NormalizedArgs {
        questions: vec![normalized_question("Choose?", true, vec![("A", None), ("B", None)])],
        answers: None,
        annotations: None,
        metadata: None,
    };

    let questions = QuestionTool::build_questions(&args);

    assert_eq!(questions[0].question, "Choose?");
    assert_eq!(questions[0].header, "Choose?");
    assert_eq!(questions[0].options[0].label, "A");
    assert_eq!(questions[0].multiple, Some(true));
    assert_eq!(questions[0].custom, Some(true));
}

#[test]
fn model_result_text_handles_missing_empty_and_annotated_answers() {
    let fallback =
        "User has answered your questions. You can now continue with the user's answers in mind.";
    assert_eq!(super::model_result_text(None, None), fallback);

    let empty = json!({});
    assert_eq!(super::model_result_text(empty.as_object(), None), fallback);

    let answers = json!({ "Choose?": "A", "Count?": 3 });
    let annotations = json!({
        "Choose?": { "preview": "preview A", "notes": "user note" }
    });
    let text = super::model_result_text(answers.as_object(), annotations.as_object());
    assert!(text.contains("\"Choose?\"=\"A\""));
    assert!(text.contains("selected preview:\npreview A"));
    assert!(text.contains("user notes: user note"));
    assert!(text.contains("\"Count?\"=\"\""));
}

#[tokio::test]
async fn question_execute_with_prefilled_answers_returns_legacy_shape() {
    let tool = QuestionTool::new("session-123".to_string());
    let input = tool
        .validate_input(json!({
            "questions": [{ "question": "Choose?" }],
            "answers": { "Choose?": "custom answer" }
        }))
        .expect("input should normalize");

    let result = tool.execute(input).await.expect("prefilled execution should succeed");
    let output: Value = serde_json::from_str(&result.output).expect("output should be JSON");

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(output["questions"][0]["question"], "Choose?");
    assert_eq!(output["answers"]["Choose?"], "custom answer");
    assert!(output["annotations"].is_null());
}

#[tokio::test]
async fn question_call_with_prefilled_answers_includes_trimmed_metadata() {
    let tool = QuestionTool::new("session-123".to_string());
    let input = tool
        .validate_input(json!({
            "questions": [{ "question": "Choose?" }],
            "answers": { "Choose?": "A" },
            "metadata": { "source": " agent " }
        }))
        .expect("input should normalize");

    let result = tool.call(input).await.expect("prefilled call should succeed");
    let hint = result.render_hint.expect("success should include a render hint");

    assert_eq!(hint.summary.as_deref(), Some("Collected 1 answer(s)"));
    assert_eq!(hint.metadata["question_count"], 1);
    assert_eq!(hint.metadata["answer_count"], 1);
    assert_eq!(hint.metadata["source"], "agent");
    assert_eq!(result.telemetry.map(|telemetry| telemetry.success), Some(true));
}

#[tokio::test]
async fn question_call_and_execute_reject_non_normalized_input() {
    let tool = QuestionTool::new("session-123".to_string());

    let call_error = tool
        .call(json!({ "questions": "invalid" }))
        .await
        .expect_err("call should reject malformed normalized arguments");
    assert!(call_error.to_string().contains("Missing or invalid parameters"));

    let execute_error = tool
        .execute(json!({ "questions": "invalid" }))
        .await
        .expect_err("execute should reject malformed normalized arguments");
    assert!(execute_error.to_string().contains("Missing or invalid parameters"));
}

#[tokio::test]
async fn question_call_collects_interactive_reply_and_infers_preview() {
    let session_id = "question-tool-interactive-reply".to_string();
    let tool = QuestionTool::new(session_id.clone());
    let input = tool
        .validate_input(json!({
            "questions": [{
                "question": "Choose interactively?",
                "options": [
                    { "label": "A", "description": "first", "preview": "preview A" },
                    { "label": "B", "description": "second" }
                ]
            }]
        }))
        .expect("interactive input should normalize");

    let handle = tokio::spawn(async move { tool.call(input).await });
    let request = pending_question_for_session(&session_id).await;
    runtime_question::reply(runtime_question::ReplyInput {
        request_id: request.id,
        answers: vec![vec!["A".to_string()]],
    });

    let result =
        handle.await.expect("question task should join").expect("interactive reply should succeed");
    assert_eq!(result.data["answers"]["Choose interactively?"], "A");
    assert_eq!(result.data["annotations"]["Choose interactively?"]["preview"], "preview A");
}

#[tokio::test]
async fn question_call_maps_interactive_rejection_to_denied_result() {
    let session_id = "question-tool-interactive-rejection".to_string();
    let tool = QuestionTool::new(session_id.clone());
    let input = tool
        .validate_input(json!({
            "questions": [{ "question": "Decline interactively?" }]
        }))
        .expect("interactive input should normalize");

    let handle = tokio::spawn(async move { tool.call(input).await });
    let request = pending_question_for_session(&session_id).await;
    runtime_question::reject(request.id);

    let result = handle
        .await
        .expect("question task should join")
        .expect("rejection should be represented as a tool result");
    assert_eq!(
        result.render_hint.as_ref().and_then(|hint| hint.summary.as_deref()),
        Some("User declined to answer")
    );
    assert!(result.data["error"].as_str().is_some_and(|error| error.contains("Denied")));
}

#[tokio::test]
async fn question_execute_maps_interactive_rejection_to_denied_legacy_result() {
    let session_id = "question-tool-execute-rejection".to_string();
    let tool = QuestionTool::new(session_id.clone());
    let input = tool
        .validate_input(json!({
            "questions": [{ "question": "Decline legacy execution?" }]
        }))
        .expect("interactive input should normalize");

    let handle = tokio::spawn(async move { tool.execute(input).await });
    let request = pending_question_for_session(&session_id).await;
    runtime_question::reject(request.id);

    let result = handle
        .await
        .expect("question task should join")
        .expect("rejection should be represented as a legacy tool result");
    assert!(!result.success);
    assert!(result.error.as_deref().is_some_and(|error| error.contains("Denied")));
}
