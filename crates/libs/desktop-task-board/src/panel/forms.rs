//! 任务字段、导入内容与提交选项。
use super::*;
use desktop_assets::assets::Icon;
use desktop_chat_input::input_panel::{
    input_editor::composer_editor,
    layout::{COMPOSER_TOOLBAR_PADDING, composer_card},
};
use iced::widget::column;

pub(super) fn build_priority_field<'a>(app: PanelView<'a>, is_edit_mode: bool) -> iced::Element<'a, Message> {
    let priority_msg = if is_edit_mode {
        TaskBoardMessage::UpdateEditingTaskPriority
    } else {
        TaskBoardMessage::UpdateDraftPriority
    };
    text_input(
        desktop_i18n::I18n::new(app.desktop_locale).__("1-99999，默认999"),
        &app.task_board_draft.priority,
    )
    .on_input(move |v| Message::TaskBoard(priority_msg(v)))
    .padding(SETTINGS_CONTROL_PADDING)
    .size(SETTINGS_CONTROL_TEXT_SIZE)
    .width(Length::Fill)
    .style(input_style)
    .into()
}



pub(super) fn build_agent_field(app: PanelView<'_>, is_edit_mode: bool) -> iced::Element<'_, Message> {
    let selected_key =
        app.task_board_draft.agent.clone().unwrap_or_else(|| MAIN_AGENT_KEY.to_string());
    let options = build_agent_options(app, &selected_key);

    let selected = options
        .iter()
        .find(|option| option.key == selected_key)
        .cloned()
        .or_else(|| options.iter().find(|option| option.key == MAIN_AGENT_KEY).cloned());

    let select = settings_select(options, selected, move |option| {
        Message::TaskBoard(if is_edit_mode {
            TaskBoardMessage::UpdateEditingTaskAgent(Some(option.key))
        } else {
            TaskBoardMessage::UpdateDraftAgent(Some(option.key))
        })
    })
    .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("选择代理"));
    if is_edit_mode {
        select.width(Length::Fill).into()
    } else {
        select.toolbar_icon(Icon::Hexagon).width(132).into()
    }
}

pub fn build_bulk_agent_selector(app: PanelView<'_>) -> iced::Element<'_, Message> {
    let selected_key = if app.task_board_bulk_agent.trim().is_empty() {
        MAIN_AGENT_KEY.to_string()
    } else {
        app.task_board_bulk_agent.trim().to_string()
    };
    let options = build_agent_options(app, &selected_key);
    let selected = options
        .iter()
        .find(|option| option.key == selected_key)
        .cloned()
        .or_else(|| options.iter().find(|option| option.key == MAIN_AGENT_KEY).cloned());

    settings_select(options, selected, |option| {
        Message::TaskBoard(TaskBoardMessage::BulkAgentSelected(option.key))
    })
    .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("选择代理"))
    .width(Length::Fill)
    .into()
}

pub(super) fn build_agent_options(app: PanelView<'_>, selected_key: &str) -> Vec<TaskBoardAgentOption> {
    let mut options = Vec::new();

    for entry in &app.agents_settings.entries {
        if is_retired_builtin_agent_key(&entry.key) {
            continue;
        }
        if !crate::agent_activity::agent_available(
            &entry.key,
            &app.agents_settings.entries,
        ) && selected_key != entry.key
        {
            continue;
        }

        options.push(TaskBoardAgentOption {
            key: entry.key.clone(),
            label: if entry.key == MAIN_AGENT_KEY {
                desktop_i18n::I18n::new(app.desktop_locale).__("主智能体").to_owned()
            } else {
                entry.label.clone()
            },
        });
    }

    if !options.iter().any(|option| option.key == MAIN_AGENT_KEY) {
        options.insert(
            0,
            TaskBoardAgentOption { key: MAIN_AGENT_KEY.to_string(), label: desktop_i18n::I18n::new(app.desktop_locale).__("主智能体").to_owned() },
        );
    }

    options
}

pub(super) fn build_prompt_field<'a>(app: PanelView<'a>) -> iced::Element<'a, Message> {
    let prompt_editor = text_editor(app.task_board_prompt_editor)
        .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("输入大模型提示词..."))
        .on_action(|a| Message::TaskBoard(TaskBoardMessage::PromptEditorAction(a)))
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .padding(SETTINGS_CONTROL_PADDING)
        .height(Length::Fixed(150.0))
        .style(editor_style);

    column![
        input_label(desktop_i18n::I18n::new(app.desktop_locale).__("大模型提示词")),
        prompt_editor
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub(super) fn build_create_form<'a>(app: PanelView<'a>, slots: PanelSlots<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let (editor, _) = composer_editor(app.task_board_prompt_editor);
    let editor = editor
        .placeholder(i18n.__("输入大模型提示词..."))
        .on_action(|action| Message::TaskBoard(TaskBoardMessage::PromptEditorAction(action)));
    let toolbar = row![
        build_agent_field(app, false),
        slots.executor.expect("visible executor selector"),
    ]
    .extend(slots.reasoning)
    .push(slots.model.expect("visible model selector"))
    .spacing(4)
    .align_y(Alignment::Center)
    .wrap();
    let composer = composer_card(
        column![editor, container(toolbar).padding(COMPOSER_TOOLBAR_PADDING)],
        false,
        None,
    )
    .padding(iced::Padding { top: 8.0, ..iced::Padding::ZERO });

    let priority = column![
        settings_field_label(i18n.__("优先级")),
        build_priority_field(app, false),
    ]
    .width(144)
    .spacing(6);

    column![composer, priority, build_create_options_row(app)].spacing(14).into()
}

pub(super) fn build_mode_toggle(app: PanelView<'_>) -> iced::Element<'_, Message> {
    row![
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("单个任务")).size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(false)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, !app.task_board_is_import_mode)
            }),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("批量导入")).size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ToggleImportMode(true)))
            .padding([6, 12])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, app.task_board_is_import_mode)
            }),
    ]
    .spacing(8)
    .wrap()
    .into()
}

pub(super) fn build_import_field<'a>(app: PanelView<'a>) -> iced::Element<'a, Message> {
    let import_tools = row![
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("上传文件")).size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ImportFilePick))
            .padding([6, 12])
            .style(rounded_action_btn_style),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("清空")).size(13))
            .on_press(Message::TaskBoard(TaskBoardMessage::ClearImportEditor))
            .padding([6, 12])
            .style(rounded_action_btn_style)
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let examples = settings_select(
        ["JSON 示例", "CSV 示例", "TSV 示例"]
            .map(|key| desktop_i18n::I18n::new(app.desktop_locale).__(key))
            .to_vec(),
        None::<&str>,
        move |selected| {
            let selected = ["JSON 示例", "CSV 示例", "TSV 示例"]
                .into_iter()
                .find(|key| desktop_i18n::I18n::new(app.desktop_locale).__(key) == selected)
                .expect("known example option");
            Message::TaskBoard(TaskBoardMessage::InsertDemoData(selected.to_string()))
        },
    )
    .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("插入示例到表单"))
    .width(Length::Fill);

    let (import_editor, _) = composer_editor(app.task_board_import_editor);
    let import_editor = import_editor
        .placeholder(
            desktop_i18n::I18n::new(app.desktop_locale)
                .__("粘贴 JSON 数组或 CSV 内容...")
                .to_owned(),
        )
        .on_action(|a| Message::TaskBoard(TaskBoardMessage::ImportEditorAction(a)))
        .height(200);
    let import_composer = composer_card(
        column![import_editor, container(import_tools).padding(COMPOSER_TOOLBAR_PADDING)],
        false,
        None,
    )
    .padding(iced::Padding { top: 8.0, ..iced::Padding::ZERO });

    let help_text = text(desktop_i18n::I18n::new(app.desktop_locale).__("支持 JSON 数组（priority, prompt, model, acp_agent, reasoning_effort）\n或 CSV/TSV（首行需包含 priority, prompt，建议同时包含 model, acp_agent, reasoning_effort）"))
    .size(12)
    .style(|theme: &Theme| iced::widget::text::Style {
        color: Some(theme.extended_palette().background.base.text.scale_alpha(0.8)),
    });

    column![
        input_label(desktop_i18n::I18n::new(app.desktop_locale).__("批量导入内容")),
        examples,
        import_composer,
        help_text,
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub(super) fn build_clear_prompt_checkbox(app: PanelView<'_>) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_clear_prompt_after_create)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleClearPromptAfterCreate(enabled))
            })
            .style(settings_checkbox_style),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("创建后自动清空提示词")).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_close_after_create_checkbox(app: PanelView<'_>) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_close_after_create)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleCloseAfterCreate(enabled))
            })
            .style(settings_checkbox_style),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("创建后关闭")).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_close_after_edit_checkbox(app: PanelView<'_>) -> iced::Element<'_, Message> {
    row![
        checkbox(app.task_board_close_after_edit)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleCloseAfterEdit(enabled))
            })
            .style(settings_checkbox_style),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("编辑后关闭")).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_create_options_row(app: PanelView<'_>) -> iced::Element<'_, Message> {
    row![build_close_after_create_checkbox(app), build_clear_prompt_checkbox(app)]
        .spacing(16)
        .align_y(Alignment::Center)
        .wrap()
        .into()
}

pub(super) fn build_edit_options_row(app: PanelView<'_>) -> iced::Element<'_, Message> {
    section_block(
        app,
        "编辑行为",
        "保存成功后的面板处理策略。",
        row![build_close_after_edit_checkbox(app)].spacing(16).align_y(Alignment::Center),
    )
}

pub(super) fn build_action_buttons(app: PanelView<'_>, is_edit_mode: bool) -> iced::Element<'static, Message> {
    let (submit_text, submit_msg) = if is_edit_mode {
        if app.task_board_edit_submit_success {
            ("保存成功", TaskBoardMessage::SaveEditingTask)
        } else {
            ("保存修改", TaskBoardMessage::SaveEditingTask)
        }
    } else if app.task_board_is_import_mode {
        ("导入任务", TaskBoardMessage::ImportTasksSubmitted)
    } else if app.task_board_create_submit_success {
        ("保存成功", TaskBoardMessage::CreateTaskSubmitted)
    } else {
        ("创建任务", TaskBoardMessage::CreateTaskSubmitted)
    };
    let succeeded = if is_edit_mode {
        app.task_board_edit_submit_success
    } else {
        app.task_board_create_submit_success
    };
    let submit_style = if succeeded { button_style_success } else { primary_action_btn_style };

    row![
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(14))
            .on_press(Message::TaskBoard(if is_edit_mode {
                TaskBoardMessage::CloseTaskLogs
            } else {
                TaskBoardMessage::CreateTaskCancelled
            }))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        Space::new().width(Length::Fill),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__(submit_text)).size(14))
            .on_press(Message::TaskBoard(submit_msg))
            .padding([8, 14])
            .style(submit_style),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}
