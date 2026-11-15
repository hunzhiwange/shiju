use super::*;
use desktop_workflow_state::state::*;
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn saved_app_title_uses_fallback_for_blank_name() {
    assert_eq!(
        saved_app_title(desktop_i18n::I18n::new(Default::default()), "  "),
        "未命名应用"
    );
}

#[test]
fn saved_app_title_trims_non_blank_name() {
    assert_eq!(
        saved_app_title(desktop_i18n::I18n::new(Default::default()), "  客服助手  "),
        "客服助手"
    );
}

#[test]
fn saved_app_description_uses_fallback_for_blank_description() {
    assert_eq!(
        saved_app_description(desktop_i18n::I18n::new(Default::default()), "  "),
        "暂无描述"
    );
}

#[test]
fn saved_app_description_trims_non_blank_description() {
    assert_eq!(
        saved_app_description(
            desktop_i18n::I18n::new(Default::default()),
            "  自动回复客户问题  "
        ),
        "自动回复客户问题"
    );
}

#[test]
fn format_saved_app_time_rejects_invalid_timestamp() {
    assert_eq!(format_saved_app_time(u64::MAX), "--");
}

#[test]
fn format_saved_app_time_rejects_out_of_range_i64_timestamp() {
    assert_eq!(format_saved_app_time(i64::MAX as u64), "--");
}

#[test]
fn format_saved_app_time_formats_valid_timestamp() {
    let formatted = format_saved_app_time(0);

    assert_ne!(formatted, "--");
    assert_eq!(formatted.len(), "1970/01/01 08:00".len());
}

#[test]
fn saved_app_matches_search_query() {
    let app = WorkflowSavedAppSummary {
        name: "DHB 数据洞察".to_string(),
        description: "数据智能分析".to_string(),
        ..saved_app_summary()
    };

    assert!(saved_app_matches_query(&app, "dhb"));
    assert!(saved_app_matches_query(&app, "智能"));
    assert!(saved_app_matches_query(&app, " UUID "));
    assert!(saved_app_matches_query(&app, "  "));
    assert!(!saved_app_matches_query(&app, "客服"));
}

#[test]
fn saved_apps_view_builds_loading_error_empty_and_filtered_states() {
    let mut state = WorkflowState { saved_apps_loading: true, ..WorkflowState::default() };
    let _ = build_saved_apps_view(desktop_i18n::I18n::new(Default::default()), &state);

    state.saved_apps_loading = false;
    state.saved_apps_error = Some("数据库不可用".to_string());
    let _ = build_saved_apps_view(desktop_i18n::I18n::new(Default::default()), &state);

    state.saved_apps_error = None;
    let _ = build_saved_apps_view(desktop_i18n::I18n::new(Default::default()), &state);

    state.saved_apps = vec![saved_app_summary()];
    let _ = build_saved_apps_view(desktop_i18n::I18n::new(Default::default()), &state);

    state.saved_app_search_query = "无匹配".to_string();
    let _ = build_saved_apps_view(desktop_i18n::I18n::new(Default::default()), &state);
}

#[test]
fn saved_apps_header_and_empty_state_parts_build() {
    let state = WorkflowState::default();

    let _header = build_saved_apps_header(desktop_i18n::I18n::new(Default::default()), &state);
    let _search = build_search_input(desktop_i18n::I18n::new(Default::default()), &state);
    let _refresh = saved_apps_header_button("刷新", WorkflowMessage::LoadSavedApps);
    let _create_card = build_create_app_card(desktop_i18n::I18n::new(Default::default()));
    let _create_button =
        create_action_button(Icon::FileEarmarkPlus, "创建空白应用", WorkflowMessage::OpenFile);
    let _create_row = create_action_row(Icon::FolderOpen, "导入 DSL 文件");
    let _no_result = build_no_result_card(desktop_i18n::I18n::new(Default::default()));
    let _notice =
        build_saved_apps_notice(desktop_i18n::I18n::new(Default::default()), "暂无应用");
    let _error =
        build_saved_apps_error(desktop_i18n::I18n::new(Default::default()), "数据库不可用");
}

#[test]
fn saved_app_card_builds_action_and_progress_states() {
    let state = WorkflowState {
        saved_apps: vec![saved_app_summary()],
        opening_saved_app_uuid: Some("uuid".to_string()),
        deleting_saved_app_uuid: Some("uuid".to_string()),
        saved_app_actions_menu_uuid: Some("uuid".to_string()),
        copied_saved_app_uuid: Some("uuid".to_string()),
        ..WorkflowState::default()
    };

    let _ = build_saved_app_card(
        desktop_i18n::I18n::new(Default::default()),
        &state,
        &state.saved_apps[0],
    );
    let _ = build_saved_app_actions_menu(
        desktop_i18n::I18n::new(Default::default()),
        "uuid".to_string(),
        true,
        true,
        true,
        true,
    );
    assert!(matches!(
        saved_app_run_action_message("uuid".to_string(), false),
        Some(WorkflowMessage::OpenSavedAppRunPrompt(uuid)) if uuid == "uuid"
    ));
    assert!(saved_app_run_action_message("uuid".to_string(), true).is_none());
    let _ = build_saved_app_action_item(Icon::ArrowRepeat, "运行中", None, false);
    let _ = build_saved_app_action_item(Icon::Trash, "删除中", None, true);
    let _ = saved_app_uuid_row(desktop_i18n::I18n::new(Default::default()), "uuid", true);
    let _ = saved_app_uuid_row(desktop_i18n::I18n::new(Default::default()), "uuid", false);
}

#[test]
fn delete_confirm_dialog_uses_matching_app_or_generic_name() {
    let state = WorkflowState {
        saved_apps: vec![saved_app_summary()],
        confirm_delete_saved_app_uuid: Some("uuid".to_string()),
        ..WorkflowState::default()
    };

    assert!(
        build_saved_app_delete_confirm_dialog(
            desktop_i18n::I18n::new(Default::default()),
            &state
        )
        .is_some()
    );

    let missing_state = WorkflowState {
        confirm_delete_saved_app_uuid: Some("missing".to_string()),
        ..WorkflowState::default()
    };

    assert!(
        build_saved_app_delete_confirm_dialog(
            desktop_i18n::I18n::new(Default::default()),
            &missing_state
        )
        .is_some()
    );
    assert!(
        build_saved_app_delete_confirm_dialog(
            desktop_i18n::I18n::new(Default::default()),
            &WorkflowState::default()
        )
        .is_none()
    );
}

#[test]
fn saved_app_run_prompt_dialog_builds_inputs_and_error_states() {
    let mut state =
        WorkflowState { saved_apps: vec![saved_app_summary()], ..WorkflowState::default() };
    assert!(
        build_saved_app_run_prompt_dialog(desktop_i18n::I18n::new(Default::default()), &state)
            .is_none()
    );

    state.open_saved_app_run_prompt("uuid".to_string());
    let _ =
        build_saved_app_run_prompt_dialog(desktop_i18n::I18n::new(Default::default()), &state)
            .expect("prompt should build");

    state.finish_saved_app_run_prompt_load(
        "uuid",
        Ok(vec![
            state::WorkflowRunInputDraft {
                name: "query".to_string(),
                label: "Query".to_string(),
                input_type: "paragraph".to_string(),
                required: true,
                value: "hello".to_string(),
                ..Default::default()
            },
            state::WorkflowRunInputDraft {
                name: "enabled".to_string(),
                label: "Enabled".to_string(),
                input_type: "checkbox".to_string(),
                value: "true".to_string(),
                ..Default::default()
            },
        ]),
    );
    let _ =
        build_saved_app_run_prompt_dialog(desktop_i18n::I18n::new(Default::default()), &state)
            .expect("prompt with inputs should build");

    state.set_saved_app_run_prompt_error("请输入 query 后再运行".to_string());
    let _ =
        build_saved_app_run_prompt_dialog(desktop_i18n::I18n::new(Default::default()), &state)
            .expect("prompt with error should build");
}

#[test]
fn saved_app_run_input_metadata_uses_type_specific_fallbacks() {
    let mut input = state::WorkflowRunInputDraft {
        name: "amount".to_string(),
        input_type: "number".to_string(),
        ..Default::default()
    };

    assert_eq!(saved_app_run_input_label(&input), "amount");
    assert_eq!(
        saved_app_run_input_placeholder(desktop_i18n::I18n::new(Default::default()), &input),
        "请输入数字"
    );

    input.label = "金额".to_string();
    input.input_type = "file-list".to_string();
    assert_eq!(saved_app_run_input_label(&input), "金额");
    assert_eq!(
        saved_app_run_input_placeholder(desktop_i18n::I18n::new(Default::default()), &input),
        "输入文件 URL 或路径，逗号分隔"
    );

    input.input_type = "paragraph".to_string();
    input.hint = "补充说明".to_string();
    assert_eq!(
        saved_app_run_input_placeholder(desktop_i18n::I18n::new(Default::default()), &input),
        "补充说明"
    );

    input.placeholder = "自定义提示".to_string();
    assert_eq!(
        saved_app_run_input_placeholder(desktop_i18n::I18n::new(Default::default()), &input),
        "自定义提示"
    );
}

#[test]
fn saved_app_run_inputs_build_loading_hidden_select_and_text_variants() {
    let mut prompt = state::WorkflowRunPromptDraft {
        uuid: "uuid".to_string(),
        loading: true,
        ..Default::default()
    };
    let _ = build_saved_app_run_inputs(desktop_i18n::I18n::new(Default::default()), &prompt);

    prompt.loading = false;
    prompt.inputs = vec![state::WorkflowRunInputDraft {
        name: "internal".to_string(),
        hidden: true,
        ..Default::default()
    }];
    let _ = build_saved_app_run_inputs(desktop_i18n::I18n::new(Default::default()), &prompt);
    assert!(!saved_app_run_prompt_has_query_input(&prompt));

    prompt.inputs = vec![
        state::WorkflowRunInputDraft {
            name: "query".to_string(),
            label: "Query".to_string(),
            input_type: "select".to_string(),
            options: vec!["a".to_string(), "b".to_string()],
            value: "b".to_string(),
            required: true,
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "enabled".to_string(),
            input_type: "checkbox".to_string(),
            value: "false".to_string(),
            hint: "是否启用".to_string(),
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "attachment".to_string(),
            input_type: "file".to_string(),
            ..Default::default()
        },
        state::WorkflowRunInputDraft {
            name: "fallback".to_string(),
            input_type: "select".to_string(),
            value: "free text".to_string(),
            ..Default::default()
        },
    ];

    assert!(saved_app_run_prompt_has_query_input(&prompt));
    let _ = build_saved_app_run_inputs(desktop_i18n::I18n::new(Default::default()), &prompt);
    for (index, input) in prompt.inputs.iter().enumerate() {
        let _ = build_saved_app_run_input_field(
            desktop_i18n::I18n::new(Default::default()),
            index,
            input,
        );
        let _ = build_saved_app_run_input_control(
            desktop_i18n::I18n::new(Default::default()),
            index,
            input,
        );
    }
}

#[test]
fn saved_app_styles_cover_light_dark_and_button_statuses() {
    let light = Theme::Light;
    let dark = Theme::Dark;

    assert!(saved_app_actions_menu_style(&light).background.is_some());
    assert!(saved_app_actions_menu_style(&dark).background.is_some());
    assert!(saved_app_card_container_style(&light).background.is_some());
    assert!(saved_app_card_container_style(&dark).background.is_some());
    assert!(saved_app_name_text_style(&dark).color.is_some());

    let enabled_hovered = saved_app_action_item_style(&light, button::Status::Hovered, true, true);
    let enabled_pressed = saved_app_action_item_style(&light, button::Status::Pressed, false, true);
    let enabled_active = saved_app_action_item_style(&light, button::Status::Active, false, true);
    let disabled_active = saved_app_action_item_style(&light, button::Status::Active, false, false);
    let disabled_hovered =
        saved_app_action_item_style(&light, button::Status::Hovered, true, false);

    assert!(enabled_hovered.background.is_some());
    assert!(enabled_pressed.background.is_some());
    assert!(enabled_active.background.is_none());
    assert!(disabled_active.background.is_none());
    assert!(disabled_hovered.background.is_none());
    assert!(saved_app_card_button_style(&light, button::Status::Active).background.is_none());
}

#[test]
fn saved_app_tag_and_badge_styles_build_for_dark_theme() {
    let _ = saved_app_robot_badge();
    let _ = saved_app_tag("工作流");

    let card_style = saved_app_card_container_style(&Theme::Dark);
    let Background::Color(background) = card_style.background.expect("card background") else {
        panic!("card background should be a color");
    };

    assert!(background.a > 0.0);
}

fn saved_app_summary() -> WorkflowSavedAppSummary {
    WorkflowSavedAppSummary {
        uuid: "uuid".to_string(),
        name: "应用".to_string(),
        description: String::new(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}
