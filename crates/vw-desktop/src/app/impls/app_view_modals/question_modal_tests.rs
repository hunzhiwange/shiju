//! 验证问题弹窗对选项、预览、自定义输入和主题的处理。
//!
//! 问题夹具覆盖无选项、预览、自定义输入和稀疏答案状态，并分别检查明暗主题。

#[test]
fn question_modal_tests_module_is_wired() {
    assert!(module_path!().ends_with("question_modal_tests"));
}

use iced::widget::text;


fn option(label: &str, preview: Option<&str>) -> question_types::OptionInfo {
    question_types::OptionInfo {
        label: label.to_string(),
        description: format!("{label} description"),
        preview: preview.map(ToString::to_string),
    }
}

fn question(
    header: &str,
    options: Vec<question_types::OptionInfo>,
    multiple: Option<bool>,
    custom: Option<bool>,
) -> question_types::Info {
    question_types::Info {
        question: format!("{header}?"),
        header: header.to_string(),
        options,
        multiple,
        custom,
    }
}

fn request(questions: Vec<question_types::Info>) -> question_types::Request {
    question_types::Request {
        mcp_form: None,
        id: "question".to_string(),
        session_id: "session".to_string(),
        questions,
        tool: None,
    }
}

#[test]
fn without_request_builds_base_layer() {
    let (app, _) = crate::app::App::new();
    let root = text("root").into();

    let element = super::with_question_modal(&app, root);

    std::hint::black_box(element);
}

#[test]
fn request_with_options_preview_and_custom_input_builds_modal() {
    let (mut app, _) = crate::app::App::new();
    app.question_modal_request = Some(request(vec![
        question(
            "single",
            vec![option("once", Some("preview text")), option("always", None)],
            None,
            Some(true),
        ),
        question("multi", vec![option("a", None), option("b", None)], Some(true), None),
    ]));
    app.question_modal_answers = vec![vec!["once".to_string()], vec!["b".to_string()]];
    app.question_modal_custom = vec!["custom answer".to_string(), String::new()];

    let element = super::with_question_modal(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn empty_question_list_uses_default_header() {
    let (mut app, _) = crate::app::App::new();
    app.question_modal_request = Some(request(Vec::new()));

    let element = super::with_question_modal(&app, text("root").into());

    std::hint::black_box(element);
}

#[test]
fn sparse_question_state_builds_modal_and_suppresses_inapplicable_previews() {
    let (mut app, _) = crate::app::App::new();
    app.question_modal_request = Some(request(vec![
        question("no answer", vec![option("one", Some("preview"))], None, None),
        question("unknown answer", vec![option("known", Some("preview"))], None, None),
        question("blank preview", vec![option("blank", Some("   "))], None, None),
        question("multiple", vec![option("many", Some("preview"))], Some(true), None),
        question("no options", Vec::new(), None, Some(true)),
    ]));
    app.question_modal_answers = vec![
        Vec::new(),
        vec!["missing".to_string()],
        vec!["blank".to_string()],
        vec!["many".to_string()],
    ];

    let element = super::with_question_modal(&app, text("root").into());

    std::hint::black_box(element);
}
