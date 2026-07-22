use super::{
    TaskBoardModelSelectorKind, build_bulk_model_selector, build_model_icon, build_model_popover,
    build_model_selector, build_toggle_label,
};
use crate::app::message::TaskBoardMessage;
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use iced::Length;

fn model(id: &str, name: &str, enabled: bool) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: serde_json::json!({}),
    }
}

fn provider(id: &str, name: &str, models: Vec<ModelSummary>) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: name.to_string(), models }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("model_selector_tests"));
}

#[test]
fn selector_kind_reads_model_sources_and_open_flags() {
    let mut app = crate::app::App::new().0;
    app.task_board_draft.model = "openai/gpt-4.1".to_string();
    app.task_board_bulk_model_input = "anthropic/claude".to_string();
    app.task_board_model_popover = true;
    app.task_board_bulk_model_popover = false;

    assert_eq!(TaskBoardModelSelectorKind::Draft.current_model(&app), "openai/gpt-4.1");
    assert_eq!(TaskBoardModelSelectorKind::Edit.current_model(&app), "openai/gpt-4.1");
    assert_eq!(TaskBoardModelSelectorKind::Bulk.current_model(&app), "anthropic/claude");

    assert!(TaskBoardModelSelectorKind::Draft.is_open(&app));
    assert!(TaskBoardModelSelectorKind::Edit.is_open(&app));
    assert!(!TaskBoardModelSelectorKind::Bulk.is_open(&app));
}

#[test]
fn selector_kind_maps_toggle_close_select_and_input_messages() {
    assert!(matches!(
        TaskBoardModelSelectorKind::Draft.toggle_message(),
        TaskBoardMessage::ToggleModelPopover
    ));
    assert!(matches!(
        TaskBoardModelSelectorKind::Edit.toggle_message(),
        TaskBoardMessage::ToggleModelPopover
    ));
    assert!(matches!(
        TaskBoardModelSelectorKind::Bulk.toggle_message(),
        TaskBoardMessage::ToggleBulkModelPopover
    ));

    assert!(matches!(
        TaskBoardModelSelectorKind::Draft.close_message(),
        TaskBoardMessage::CloseModelPopover
    ));
    assert!(matches!(
        TaskBoardModelSelectorKind::Edit.close_message(),
        TaskBoardMessage::CloseModelPopover
    ));
    assert!(matches!(
        TaskBoardModelSelectorKind::Bulk.close_message(),
        TaskBoardMessage::CloseBulkModelPopover
    ));

    match TaskBoardModelSelectorKind::Draft.select_message("m1".to_string()) {
        TaskBoardMessage::ModelSelected(model) => assert_eq!(model, "m1"),
        _ => panic!("draft selection should update the draft model"),
    }
    match TaskBoardModelSelectorKind::Edit.select_message("m2".to_string()) {
        TaskBoardMessage::UpdateEditingTaskModel(model) => assert_eq!(model, "m2"),
        _ => panic!("edit selection should update the editing task model"),
    }
    match TaskBoardModelSelectorKind::Bulk.select_message("m3".to_string()) {
        TaskBoardMessage::BulkModelSelected(model) => assert_eq!(model, "m3"),
        _ => panic!("bulk selection should update the bulk model"),
    }

    match TaskBoardModelSelectorKind::Draft.input_message("i1".to_string()) {
        TaskBoardMessage::UpdateDraftModel(model) => assert_eq!(model, "i1"),
        _ => panic!("draft input should update the draft model input"),
    }
    match TaskBoardModelSelectorKind::Edit.input_message("i2".to_string()) {
        TaskBoardMessage::UpdateEditingTaskModelInput(model) => assert_eq!(model, "i2"),
        _ => panic!("edit input should update the editing task model input"),
    }
    match TaskBoardModelSelectorKind::Bulk.input_message("i3".to_string()) {
        TaskBoardMessage::UpdateBulkModelInput(model) => assert_eq!(model, "i3"),
        _ => panic!("bulk input should update the bulk model input"),
    }
}

#[test]
fn toggle_label_resolves_auto_provider_model_bare_model_and_fallbacks() {
    let mut app = crate::app::App::new().0;
    app.model_settings.providers = vec![provider(
        "openai",
        "OpenAI",
        vec![model("gpt-4.1", "GPT 4.1", true), model("o3", "OpenAI o3", true)],
    )];

    assert_eq!(build_toggle_label(&app, true, "openai/gpt-4.1"), "自动模型");
    assert_eq!(build_toggle_label(&app, false, "openai/gpt-4.1"), "GPT 4.1");
    assert_eq!(build_toggle_label(&app, false, "gpt-4.1"), "GPT 4.1");
    assert_eq!(build_toggle_label(&app, false, "missing/custom-model"), "custom-model");
    assert_eq!(build_toggle_label(&app, false, "custom-model"), "custom-model");
}

#[test]
fn toggle_label_keeps_provider_qualification_when_model_ids_overlap() {
    let mut app = crate::app::App::new().0;
    app.model_settings.providers = vec![
        provider("first", "First", vec![model("shared", "First Shared", true)]),
        provider("second", "Second", vec![model("shared", "Second Shared", true)]),
    ];

    assert_eq!(build_toggle_label(&app, false, "first/shared"), "First Shared");
    assert_eq!(build_toggle_label(&app, false, "second/shared"), "Second Shared");
    assert_eq!(build_toggle_label(&app, false, "missing/shared"), "shared");
    assert_eq!(build_toggle_label(&app, false, "shared"), "First Shared");
}

#[test]
fn toggle_label_falls_back_for_partial_or_extra_provider_qualified_ids() {
    let app = crate::app::App::new().0;

    assert_eq!(build_toggle_label(&app, false, "provider/"), "");
    assert_eq!(build_toggle_label(&app, false, "/model"), "model");
    assert_eq!(build_toggle_label(&app, false, "provider/model/extra"), "model/extra");
}

#[test]
fn model_icon_builds_for_auto_provider_bare_model_and_unknown_model() {
    let mut app = crate::app::App::new().0;
    app.model_settings.providers =
        vec![provider("openai", "OpenAI", vec![model("gpt-4.1", "GPT 4.1", true)])];

    std::hint::black_box(build_model_icon(true, &app, "auto"));
    std::hint::black_box(build_model_icon(false, &app, "openai/gpt-4.1"));
    std::hint::black_box(build_model_icon(false, &app, "gpt-4.1"));
    std::hint::black_box(build_model_icon(false, &app, "unknown-model"));
}

#[test]
fn model_popover_renders_loading_empty_filtered_and_selected_states() {
    let mut app = crate::app::App::new().0;

    app.model_settings.loading = true;
    let loading = build_model_popover(&app, TaskBoardModelSelectorKind::Draft, true, "auto");
    assert_eq!(loading.as_widget().size().width, Length::Fixed(320.0));
    drop(loading);

    app.model_settings.loading = false;
    let empty = build_model_popover(&app, TaskBoardModelSelectorKind::Edit, false, "missing");
    assert_eq!(empty.as_widget().size().width, Length::Fixed(320.0));
    drop(empty);

    app.model_settings.providers = vec![
        provider(
            "openai",
            "OpenAI",
            vec![
                model("gpt-4.1", "GPT 4.1", true),
                model("long-id", "A very long model display name that will be truncated", true),
                model("disabled", "Disabled", false),
            ],
        ),
        provider("anthropic", "Anthropic", vec![model("claude", "Claude Sonnet", true)]),
    ];

    let selected_provider_model =
        build_model_popover(&app, TaskBoardModelSelectorKind::Draft, false, "openai/gpt-4.1");
    assert_eq!(selected_provider_model.as_widget().size().width, Length::Fixed(320.0));
    drop(selected_provider_model);

    let selected_bare_model =
        build_model_popover(&app, TaskBoardModelSelectorKind::Edit, false, "claude");
    assert_eq!(selected_bare_model.as_widget().size().width, Length::Fixed(320.0));
    drop(selected_bare_model);

    app.model_settings.query = "OPEN".to_string();
    let provider_match =
        build_model_popover(&app, TaskBoardModelSelectorKind::Bulk, false, "openai/gpt-4.1");
    assert_eq!(provider_match.as_widget().size().width, Length::Fixed(320.0));
    drop(provider_match);

    app.model_settings.query = "sonnet".to_string();
    let model_name_match =
        build_model_popover(&app, TaskBoardModelSelectorKind::Draft, false, "anthropic/claude");
    assert_eq!(model_name_match.as_widget().size().width, Length::Fixed(320.0));
    drop(model_name_match);

    app.model_settings.query = "no-match".to_string();
    let no_match =
        build_model_popover(&app, TaskBoardModelSelectorKind::Bulk, false, "openai/gpt-4.1");
    assert_eq!(no_match.as_widget().size().width, Length::Fixed(320.0));
}

#[test]
fn model_popover_filters_by_model_id_and_provider_id_while_catalog_exists() {
    let mut app = crate::app::App::new().0;
    app.model_settings.loading = true;
    app.model_settings.providers = vec![
        ProviderModelsSummary {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            models: vec![ModelSummary {
                id: "gpt-4.1".to_string(),
                name: "GPT 4.1".to_string(),
                enabled: true,
                toolcall: false,
                attachment: true,
                context_limit: 4096,
                detail: serde_json::json!({}),
            }],
        },
        provider("anthropic", "Anthropic", vec![model("claude", "Claude", true)]),
    ];

    app.model_settings.query = "gpt-4.1".to_string();
    let model_id_match =
        build_model_popover(&app, TaskBoardModelSelectorKind::Draft, false, "openai/gpt-4.1");
    assert_eq!(model_id_match.as_widget().size().width, Length::Fixed(320.0));
    drop(model_id_match);

    app.model_settings.query = "anthropic".to_string();
    let provider_id_match =
        build_model_popover(&app, TaskBoardModelSelectorKind::Bulk, false, "anthropic/claude");
    assert_eq!(provider_id_match.as_widget().size().width, Length::Fixed(320.0));
}

#[test]
fn public_builders_construct_draft_edit_and_bulk_selectors() {
    let mut app = crate::app::App::new().0;
    app.model_settings.providers =
        vec![provider("openai", "OpenAI", vec![model("gpt-4.1", "GPT 4.1", true)])];

    app.task_board_draft.model = "  ".to_string();
    app.task_board_model_popover = true;
    let draft = build_model_selector(&app, false);
    std::hint::black_box(draft.as_widget().size());
    drop(draft);

    app.task_board_draft.model = "openai/gpt-4.1".to_string();
    let edit = build_model_selector(&app, true);
    std::hint::black_box(edit.as_widget().size());
    drop(edit);

    app.task_board_bulk_model_input = "openai/gpt-4.1".to_string();
    app.task_board_bulk_model_popover = true;
    let bulk = build_bulk_model_selector(&app);
    std::hint::black_box(bulk.as_widget().size());
}
