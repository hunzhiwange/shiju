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
    let modal_request = None;
    let answers: Vec<Vec<String>> = vec![];
    let custom: Vec<String> = vec![];

    let root = text("root").into();

    let element = crate::modals::question_modal::with_question_modal(
        modal_request.as_ref(),
        &answers,
        &custom,
        Default::default(),
        None,
        |event| event,
        root,
    );

    std::hint::black_box(element);
}

#[test]
fn request_with_options_preview_and_custom_input_builds_modal() {
    let modal_request = Some(request(vec![
        question(
            "single",
            vec![option("once", Some("preview text")), option("always", None)],
            None,
            Some(true),
        ),
        question("multi", vec![option("a", None), option("b", None)], Some(true), None),
    ]));
    let answers = vec![vec!["once".to_string()], vec!["b".to_string()]];
    let custom = vec!["custom answer".to_string(), String::new()];

    let element = crate::modals::question_modal::with_question_modal(
        modal_request.as_ref(),
        &answers,
        &custom,
        Default::default(),
        None,
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn empty_question_list_uses_default_header() {
    let answers: Vec<Vec<String>> = vec![];
    let custom: Vec<String> = vec![];

    let modal_request = Some(request(Vec::new()));

    let element = crate::modals::question_modal::with_question_modal(
        modal_request.as_ref(),
        &answers,
        &custom,
        Default::default(),
        None,
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}

#[test]
fn sparse_question_state_builds_modal_and_suppresses_inapplicable_previews() {
    let custom: Vec<String> = vec![];

    let modal_request = Some(request(vec![
        question("no answer", vec![option("one", Some("preview"))], None, None),
        question("unknown answer", vec![option("known", Some("preview"))], None, None),
        question("blank preview", vec![option("blank", Some("   "))], None, None),
        question("multiple", vec![option("many", Some("preview"))], Some(true), None),
        question("no options", Vec::new(), None, Some(true)),
    ]));
    let answers = vec![
        Vec::new(),
        vec!["missing".to_string()],
        vec!["blank".to_string()],
        vec!["many".to_string()],
    ];

    let element = crate::modals::question_modal::with_question_modal(
        modal_request.as_ref(),
        &answers,
        &custom,
        Default::default(),
        None,
        |event| event,
        text("root").into(),
    );

    std::hint::black_box(element);
}
#[test]
fn with_chat_reset_dialog_returns_root_when_no_index_or_missing_message() {
    let mut index = None;
    let mut chat_len = 0;
    let _element: iced::Element<'_, crate::modals::chat_reset::Event> =
        crate::modals::chat_reset::with_chat_reset_dialog(
            index,
            chat_len,
            Default::default(),
            |event| event,
            text("root").into(),
        );

    index = Some(1);
    chat_len += 1;
    let _element: iced::Element<'_, crate::modals::chat_reset::Event> =
        crate::modals::chat_reset::with_chat_reset_dialog(
            index,
            chat_len,
            Default::default(),
            |event| event,
            text("root").into(),
        );
}

#[test]
fn with_chat_reset_dialog_builds_modal_for_existing_message() {
    let index = Some(0);
    let mut chat_len = 0;
    chat_len += 1;

    let _element: iced::Element<'_, crate::modals::chat_reset::Event> =
        crate::modals::chat_reset::with_chat_reset_dialog(
            index,
            chat_len,
            Default::default(),
            |event| event,
            text("root").into(),
        );
}
#[test]
fn with_chat_fork_dialog_builds_without_an_anchor() {
    let _element: iced::Element<'_, crate::modals::chat_fork::Event> =
        crate::modals::chat_fork::with_chat_fork_dialog(
            None,
            false,
            Default::default(),
            |event| event,
            text("root").into(),
        );
}
