use super::*;
use crate::apps::workflow::message::WorkflowMessage;
use crate::apps::workflow::state::{WorkflowRunInputDraft, WorkflowRunPromptDraft};

fn input(name: &str, input_type: &str, value: &str) -> WorkflowRunInputDraft {
    WorkflowRunInputDraft {
        name: name.to_string(),
        label: name.to_string(),
        input_type: input_type.to_string(),
        value: value.to_string(),
        ..WorkflowRunInputDraft::default()
    }
}

#[test]
fn request_inputs_trim_query_skip_empty_names_and_parse_values() {
    let mut count = input("count", "number", "42");
    count.required = true;
    let prompt = WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        query: "  summarize this  ".to_string(),
        inputs: vec![
            input("", "text-input", "ignored"),
            input("optional", "text-input", "  "),
            count,
            input("enabled", "checkbox", "TRUE"),
        ],
        ..WorkflowRunPromptDraft::default()
    };

    let (query, values) = build_run_request_inputs(&prompt).expect("inputs should be valid");

    assert_eq!(query, "summarize this");
    assert_eq!(values.len(), 2);
    assert_eq!(values["count"], serde_json::json!(42));
    assert_eq!(values["enabled"], serde_json::json!(true));
}

#[test]
fn request_inputs_report_required_fields_and_missing_query() {
    let mut required = input("topic", "text-input", " ");
    required.label = "主题".to_string();
    required.required = true;
    let prompt =
        WorkflowRunPromptDraft { inputs: vec![required], ..WorkflowRunPromptDraft::default() };
    assert_eq!(build_run_request_inputs(&prompt), Err("主题 必填".to_string()));

    let prompt = WorkflowRunPromptDraft::default();
    assert_eq!(build_run_request_inputs(&prompt), Err("请输入 query 后再运行".to_string()));
}

#[test]
fn query_input_supplies_query_when_prompt_query_is_blank() {
    let prompt = WorkflowRunPromptDraft {
        query: "  ".to_string(),
        inputs: vec![input("query", "text-input", "from input")],
        ..WorkflowRunPromptDraft::default()
    };

    let (query, values) = build_run_request_inputs(&prompt).expect("query input should be used");
    assert_eq!(query, "from input");
    assert_eq!(values["query"], serde_json::json!("from input"));
}

#[test]
fn number_boolean_select_and_text_validation_reject_invalid_values() {
    let number = input("amount", "number", "not-a-number");
    assert_eq!(parse_run_input_value(&number), Err("amount 必须是数字".to_string()));

    let non_finite = input("amount", "number", "NaN");
    assert_eq!(parse_run_input_value(&non_finite), Err("amount 必须是有效数字".to_string()));

    let boolean = input("enabled", "checkbox", "yes");
    assert_eq!(parse_run_input_value(&boolean), Err("enabled 必须是 boolean".to_string()));

    let mut select = input("mode", "select", "unsafe");
    select.options = vec!["safe".to_string(), "strict".to_string()];
    assert_eq!(parse_run_input_value(&select), Err("mode 必须从选项中选择".to_string()));

    let mut text = input("title", "text-input", "四个字符");
    text.max_length_input = "3".to_string();
    assert_eq!(parse_run_input_value(&text), Err("title 超过最大长度 3".to_string()));
}

#[test]
fn file_input_distinguishes_single_paths_urls_and_multiple_values() {
    let path = input("attachment", "file", " /tmp/report.txt ");
    assert_eq!(
        parse_run_input_value(&path).expect("path should parse"),
        serde_json::json!({"path": "/tmp/report.txt", "name": "report.txt"})
    );

    let url = input("attachment", "file", "https://example.test/files/report.pdf");
    assert_eq!(
        parse_run_input_value(&url).expect("URL should parse"),
        serde_json::json!({
            "url": "https://example.test/files/report.pdf",
            "name": "report.pdf"
        })
    );

    let multiple = input("attachment", "file", "one.txt,two.txt");
    assert_eq!(parse_run_input_value(&multiple), Err("attachment 只能填写一个文件".to_string()));
}

#[test]
fn file_list_accepts_json_objects_arrays_and_delimited_paths() {
    let object = input("files", "file-list", r#"{"path":"one.txt"}"#);
    assert_eq!(
        parse_run_input_value(&object).expect("object should become a list"),
        serde_json::json!([{"path": "one.txt"}])
    );

    let array = input("files", "file-list", r#"[{"url":"https://example.test/a"}]"#);
    assert_eq!(
        parse_run_input_value(&array).expect("array should be preserved"),
        serde_json::json!([{"url": "https://example.test/a"}])
    );

    let paths = input("files", "file-list", "one.txt\nfolder/two.txt");
    assert_eq!(
        parse_run_input_value(&paths).expect("paths should become file objects"),
        serde_json::json!([
            {"path": "one.txt", "name": "one.txt"},
            {"path": "folder/two.txt", "name": "two.txt"}
        ])
    );
}

#[test]
fn empty_checks_and_labels_follow_input_type_semantics() {
    let empty_list = input("files", "file-list", " ,\n ");
    assert!(run_input_value_is_empty(&empty_list));

    let checkbox = input("enabled", "checkbox", "");
    assert_eq!(parse_run_input_value(&checkbox), Ok(serde_json::json!(false)));

    let mut unlabeled = input("query", "text-input", "hello");
    unlabeled.label = "  ".to_string();
    assert_eq!(run_input_label(&unlabeled), "query");
}

#[test]
fn handle_submit_saved_app_run_prompt_noops_without_prompt_and_reports_loading_or_invalid_inputs() {
    let (mut app, _) = crate::app::App::new();

    assert!(handle(&mut app, WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert!(app.workflow_state.saved_app_run_prompt.is_none());

    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        loading: true,
        ..WorkflowRunPromptDraft::default()
    });
    assert!(handle(&mut app, WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().unwrap().error.as_deref(),
        Some("运行参数仍在加载")
    );

    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        inputs: vec![WorkflowRunInputDraft {
            name: "topic".to_string(),
            label: "主题".to_string(),
            required: true,
            ..WorkflowRunInputDraft::default()
        }],
        ..WorkflowRunPromptDraft::default()
    });
    assert!(handle(&mut app, WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().unwrap().error.as_deref(),
        Some("主题 必填")
    );
}

#[test]
fn handle_cancel_prompt_and_ignores_finished_event_for_other_uuid() {
    let (mut app, _) = crate::app::App::new();
    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        query: "hello".to_string(),
        ..WorkflowRunPromptDraft::default()
    });

    assert!(handle(&mut app, WorkflowMessage::CancelSavedAppRunPrompt).is_some());
    assert!(app.workflow_state.saved_app_run_prompt.is_none());

    app.workflow_state.running_saved_app_uuid = Some("active".to_string());
    assert!(
        handle(
            &mut app,
            WorkflowMessage::SavedAppRunFinished {
                uuid: "other".to_string(),
                result: Err("network down".to_string()),
            },
        )
        .is_some()
    );
    assert_eq!(app.workflow_state.running_saved_app_uuid.as_deref(), Some("active"));
}
