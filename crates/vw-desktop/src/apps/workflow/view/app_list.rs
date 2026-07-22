//! Workflow 应用列表视图模块，负责展示本地数据库中的应用入口。

use super::*;
use chrono::{Local, TimeZone};
use iced::widget::{column, row};

const SAVED_APPS_HEADER_ITEM_HEIGHT: f32 = 38.0;
const SAVED_APPS_SEARCH_INPUT_LINE_HEIGHT: f32 = 20.0;
const SAVED_APP_RUN_PROMPT_WIDTH: f32 = 520.0;
const SAVED_APP_RUN_PROMPT_FORM_MAX_HEIGHT: f32 = 440.0;
const SAVED_APP_RUN_PROMPT_SCROLLBAR_WIDTH: f32 = 4.0;
const SAVED_APP_RUN_PROMPT_SCROLLBAR_GUTTER: f32 = 12.0;

/// 构建 saved apps view 对应的界面元素。
///
/// 参数由当前工作流状态提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_saved_apps_view(state: &WorkflowState) -> Element<'_, Message> {
    let body: Element<'_, Message> = if state.saved_apps_loading && state.saved_apps.is_empty() {
        build_saved_apps_notice("正在读取本地 Workflow 应用...")
    } else if let Some(error) = &state.saved_apps_error {
        build_saved_apps_error(error)
    } else if state.saved_apps.is_empty() {
        build_saved_apps_notice("暂无应用，请创建应用或导入 DSL 后保存到本地数据库。")
    } else {
        let mut cards = row![build_create_app_card()].spacing(18);
        let filtered_apps = state
            .saved_apps
            .iter()
            .filter(|app| saved_app_matches_query(app, &state.saved_app_search_query))
            .collect::<Vec<_>>();

        for app in &filtered_apps {
            cards = cards.push(build_saved_app_card(state, app));
        }

        if filtered_apps.is_empty() {
            cards = cards.push(build_no_result_card());
        }

        scrollable(container(cards.wrap()).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let content = column![build_saved_apps_header(state), body]
        .spacing(26)
        .width(Length::Fill)
        .height(Length::Fill);

    container(content).padding([30, 34]).style(root_style).into()
}

fn build_saved_apps_header(state: &WorkflowState) -> Element<'_, Message> {
    row![
        build_search_input(state),
        saved_apps_header_button("刷新", WorkflowMessage::LoadSavedApps),
        saved_apps_header_button("导入 DSL", WorkflowMessage::OpenFile),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn saved_app_matches_query(app: &state::WorkflowSavedAppSummary, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    app.name.to_lowercase().contains(&query)
        || app.description.to_lowercase().contains(&query)
        || app.uuid.to_lowercase().contains(&query)
}

fn saved_apps_header_button(
    label: &'static str,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    button(
        container(text(label).size(12).line_height(1.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .style(rounded_action_btn_style)
    .padding(0)
    .width(Length::Fixed(82.0))
    .height(Length::Fixed(SAVED_APPS_HEADER_ITEM_HEIGHT))
    .on_press(Message::WorkflowTool(message))
    .into()
}

fn build_search_input(state: &WorkflowState) -> Element<'_, Message> {
    container(
        text_input("搜索", &state.saved_app_search_query)
            .on_input(|query| Message::WorkflowTool(WorkflowMessage::SavedAppSearchChanged(query)))
            .padding([9, 12])
            .size(13)
            .line_height(iced::Pixels(SAVED_APPS_SEARCH_INPUT_LINE_HEIGHT))
            .width(Length::Fill)
            .style(settings_text_input_style),
    )
    .height(Length::Fixed(SAVED_APPS_HEADER_ITEM_HEIGHT))
    .width(Length::Fixed(220.0))
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn build_create_app_card() -> Element<'static, Message> {
    container(
        column![
            text("创建应用").size(14).style(settings_muted_text_style),
            create_action_button(
                Icon::FileEarmarkPlus,
                "创建空白应用",
                WorkflowMessage::OpenCreateAppEditor,
            ),
            create_action_button(Icon::FolderOpen, "导入 DSL 文件", WorkflowMessage::OpenFile),
        ]
        .spacing(12)
        .width(Length::Fill),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .padding([22, 24])
    .style(saved_app_card_container_style)
    .into()
}

fn create_action_button(
    icon: Icon,
    label: &'static str,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    button(create_action_row(icon, label))
        .style(rounded_action_btn_style)
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fixed(42.0))
        .on_press(Message::WorkflowTool(message))
        .into()
}

fn create_action_row(icon: Icon, label: &'static str) -> Element<'static, Message> {
    container(
        row![
            svg(assets::get_icon(icon))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.72))
                }),
            text(label).size(14).line_height(1.0),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([0, 12])
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn build_no_result_card() -> Element<'static, Message> {
    container(
        column![
            text("没有匹配的应用").size(15),
            text("换个搜索词再试。").size(12).style(settings_muted_text_style),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(saved_app_card_container_style)
    .into()
}

fn build_saved_app_card(
    state: &WorkflowState,
    app: &state::WorkflowSavedAppSummary,
) -> Element<'static, Message> {
    let uuid = app.uuid.clone();
    let title = saved_app_title(&app.name);
    let description = saved_app_description(&app.description);
    let updated_label = format!("编辑于 {}", format_saved_app_time(app.updated_at_ms));
    let opening = state.opening_saved_app_uuid.as_deref() == Some(app.uuid.as_str());
    let enabled = state.opening_saved_app_uuid.is_none();
    let copied = state.copied_saved_app_uuid.as_deref() == Some(app.uuid.as_str());
    let deleting = state.deleting_saved_app_uuid.as_deref() == Some(app.uuid.as_str());
    let running = state.running_saved_app_uuid.as_deref() == Some(app.uuid.as_str());
    let run_blocked = state.running_saved_app_uuid.is_some();
    let actions_open = state.saved_app_actions_menu_uuid.as_deref() == Some(app.uuid.as_str());

    let open_area = button(
        column![
            row![
                saved_app_robot_badge(),
                column![
                    text(title).size(16).line_height(1.0).style(saved_app_name_text_style),
                    text(updated_label).size(12).style(settings_muted_text_style),
                ]
                .spacing(6)
                .width(Length::Fill),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            text(description).size(13).style(settings_muted_text_style),
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .style(saved_app_card_button_style)
    .padding(0)
    .width(Length::Fill)
    .on_press_maybe(
        enabled.then_some(Message::WorkflowTool(WorkflowMessage::OpenSavedApp(uuid.clone()))),
    );
    let actions = build_saved_app_actions_menu(uuid, actions_open, deleting, running, run_blocked);

    container(
        column![
            row![open_area, actions].spacing(8).align_y(Alignment::Start),
            saved_app_uuid_row(app.uuid.as_str(), copied),
            row![
                saved_app_tag("内部应用"),
                saved_app_tag("工作流"),
                if opening {
                    saved_app_tag("打开中")
                } else if running {
                    saved_app_tag("运行中")
                } else {
                    saved_app_tag("正式")
                },
            ]
            .spacing(6)
            .wrap(),
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .width(Length::Fixed(380.0))
    .height(Length::Fixed(196.0))
    .padding([16, 20])
    .style(saved_app_card_container_style)
    .into()
}

fn build_saved_app_actions_menu(
    uuid: String,
    actions_open: bool,
    deleting: bool,
    running: bool,
    run_blocked: bool,
) -> Element<'static, Message> {
    let trigger = button(
        container(
            svg(assets::get_icon(Icon::DotsThreeVertical))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.72)),
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .style(round_icon_btn_style)
    .padding(0)
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .on_press(Message::WorkflowTool(WorkflowMessage::ToggleSavedAppActions(uuid.clone())));

    let run_item = build_saved_app_action_item(
        Icon::ArrowRepeat,
        if running { "运行中" } else { "运行" },
        saved_app_run_action_message(uuid.clone(), run_blocked),
        false,
    );

    let delete_message = (!deleting).then_some(WorkflowMessage::RequestDeleteSavedApp(uuid));
    let delete_item = build_saved_app_action_item(
        Icon::Trash,
        if deleting { "删除中" } else { "删除" },
        delete_message,
        true,
    );

    PointBelowOverlay::new(
        trigger,
        container(column![run_item, delete_item].spacing(4).width(Length::Fill))
            .width(Length::Fixed(142.0))
            .padding(6)
            .style(saved_app_actions_menu_style),
    )
    .show(actions_open)
    .anchor(Point::new(-108.0, 30.0))
    .gap(4.0)
    .on_close(Message::WorkflowTool(WorkflowMessage::CloseSavedAppActions))
    .capture_outside_click(false)
    .into()
}

fn saved_app_run_action_message(uuid: String, run_blocked: bool) -> Option<WorkflowMessage> {
    (!run_blocked).then_some(WorkflowMessage::OpenSavedAppRunPrompt(uuid))
}

fn build_saved_app_action_item(
    icon: Icon,
    label: &'static str,
    message: Option<WorkflowMessage>,
    danger: bool,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let icon_el = svg(assets::get_icon(icon))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(move |theme: &Theme, _| {
            let base = if danger {
                theme.extended_palette().danger.base.color
            } else {
                theme.palette().text
            };
            iced::widget::svg::Style {
                color: Some(if enabled { base } else { base.scale_alpha(0.36) }),
            }
        });

    let item = button(
        container(
            row![
                container(icon_el)
                    .width(Length::Fixed(22.0))
                    .align_x(iced::alignment::Horizontal::Center),
                text(label).size(13),
                Space::new().width(Length::Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([7, 10]),
    )
    .style(move |theme: &Theme, status| saved_app_action_item_style(theme, status, danger, enabled))
    .width(Length::Fill);

    if let Some(message) = message {
        item.on_press(Message::WorkflowTool(message)).into()
    } else {
        item.into()
    }
}

fn saved_app_uuid_row(uuid: &str, copied: bool) -> Element<'static, Message> {
    let uuid = uuid.to_string();
    let copy_content: Element<'static, Message> = if copied {
        text("✓").size(12).line_height(1.0).into()
    } else {
        row![
            svg(assets::get_icon(Icon::Copy))
                .width(Length::Fixed(13.0))
                .height(Length::Fixed(13.0))
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.70))
                }),
            text("复制").size(11).line_height(1.0),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into()
    };

    row![
        text(format!("UUID: {uuid}")).size(11).style(settings_muted_text_style).width(Length::Fill),
        button(
            container(copy_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .style(rounded_action_btn_style)
        .padding([5, 8])
        .width(Length::Fixed(58.0))
        .height(Length::Fixed(30.0))
        .on_press(Message::WorkflowTool(WorkflowMessage::CopySavedAppUuid(uuid.clone()))),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn build_saved_app_delete_confirm_dialog(
    state: &WorkflowState,
) -> Option<Element<'_, Message>> {
    let uuid = state.confirm_delete_saved_app_uuid.as_deref()?;
    let app_name = state
        .saved_apps
        .iter()
        .find(|app| app.uuid == uuid)
        .map(|app| saved_app_title(&app.name))
        .unwrap_or_else(|| "该应用".to_string());

    Some(crate::app::components::toast::confirm_dialog(
        "确认删除应用",
        format!("将从本地数据库删除「{app_name}」。此操作不可撤销。\nUUID: {uuid}"),
        "确认删除",
        "取消",
        Message::WorkflowTool(WorkflowMessage::DeleteSavedApp(uuid.to_string())),
        Message::WorkflowTool(WorkflowMessage::CancelDeleteSavedApp),
    ))
}

pub(super) fn build_saved_app_run_prompt_dialog(
    state: &WorkflowState,
) -> Option<Element<'_, Message>> {
    let prompt = state.saved_app_run_prompt.as_ref()?;
    let app_name = state
        .saved_apps
        .iter()
        .find(|app| app.uuid == prompt.uuid)
        .map(|app| saved_app_title(&app.name))
        .unwrap_or_else(|| "该应用".to_string());

    let header = row![
        column![
            text("运行工作流").size(22),
            text(format!("为「{app_name}」填写本次运行参数"))
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(4),
        Space::new().width(Length::Fill),
        settings_close_button(Message::WorkflowTool(WorkflowMessage::CancelSavedAppRunPrompt)),
    ]
    .align_y(Alignment::Center);

    let mut form = column![].spacing(16).width(Length::Fill);

    if !prompt.loading && !saved_app_run_prompt_has_query_input(prompt) {
        form = form.push(build_editor_field(
            "query",
            workflow_text_input("输入本次运行的 query", &prompt.query, |value| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunQueryChanged(value))
            }),
        ));
    }

    form = form.push(build_saved_app_run_inputs(prompt));

    if let Some(error) = &prompt.error {
        form = form.push(
            container(text(error).size(12))
                .width(Length::Fill)
                .padding([9, 12])
                .style(workflow_run_prompt_error_style),
        );
    }

    let form_scroll = container(
        scrollable(container(form).width(Length::Fill).padding(iced::Padding {
            top: 0.0,
            right: SAVED_APP_RUN_PROMPT_SCROLLBAR_WIDTH + SAVED_APP_RUN_PROMPT_SCROLLBAR_GUTTER,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(SAVED_APP_RUN_PROMPT_SCROLLBAR_WIDTH)
                .scroller_width(SAVED_APP_RUN_PROMPT_SCROLLBAR_WIDTH),
        ))
        .height(Length::Shrink),
    )
    .width(Length::Fill)
    .max_height(SAVED_APP_RUN_PROMPT_FORM_MAX_HEIGHT);

    let footer = row![
        Space::new().width(Length::Fill),
        button(text("取消"))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::CancelSavedAppRunPrompt)),
        button(text("运行"))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::SubmitSavedAppRunPrompt)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let card = container(column![header, form_scroll, footer].spacing(18).width(Length::Fill))
        .width(Length::Fixed(SAVED_APP_RUN_PROMPT_WIDTH))
        .padding([22, 24])
        .style(settings_modal_card_style);

    Some(settings_modal_overlay(
        None,
        Message::WorkflowTool(WorkflowMessage::CancelSavedAppRunPrompt),
        card,
    ))
}

fn build_saved_app_run_inputs(prompt: &state::WorkflowRunPromptDraft) -> Element<'_, Message> {
    let mut rows = column![].spacing(8).width(Length::Fill);

    if prompt.loading {
        rows = rows.push(text("正在读取运行参数...").size(12).style(settings_muted_text_style));
    } else if prompt.inputs.iter().all(|input| input.hidden) {
        rows = rows.push(text("没有额外参数").size(12).style(settings_muted_text_style));
    }

    for (index, input) in prompt.inputs.iter().enumerate() {
        if !input.hidden {
            rows = rows.push(build_saved_app_run_input_field(index, input));
        }
    }

    build_editor_field("inputs", rows.into())
}

fn build_saved_app_run_input_field(
    index: usize,
    input: &state::WorkflowRunInputDraft,
) -> Element<'_, Message> {
    let label = if input.required {
        format!("{} *", saved_app_run_input_label(input))
    } else {
        saved_app_run_input_label(input)
    };
    let meta = row![
        text(label).size(12).style(settings_muted_text_style),
        settings_value_badge(start_variable_value_type_label(&input.input_type)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    column![meta, build_saved_app_run_input_control(index, input)]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn build_saved_app_run_input_control(
    index: usize,
    input: &state::WorkflowRunInputDraft,
) -> Element<'_, Message> {
    match input.input_type.as_str() {
        "checkbox" => row![
            toggler(input.value.trim().eq_ignore_ascii_case("true")).on_toggle(move |value| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunInputToggled(index, value))
            }),
            text(saved_app_run_input_placeholder(input)).size(12).style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
        "select" if !input.options.is_empty() => {
            let options = input.options.clone();
            let selected =
                options.iter().find(|option| option.as_str() == input.value.trim()).cloned();
            pick_list(options, selected, move |value| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunInputChanged(index, value))
            })
            .padding([9, 12])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fill)
            .into()
        }
        _ => text_input(saved_app_run_input_placeholder(input), &input.value)
            .on_input(move |value| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunInputChanged(index, value))
            })
            .padding([9, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill)
            .into(),
    }
}

fn saved_app_run_prompt_has_query_input(prompt: &state::WorkflowRunPromptDraft) -> bool {
    prompt.inputs.iter().any(|input| input.name == "query")
}

fn saved_app_run_input_label(input: &state::WorkflowRunInputDraft) -> String {
    if input.label.trim().is_empty() { input.name.clone() } else { input.label.clone() }
}

fn saved_app_run_input_placeholder(input: &state::WorkflowRunInputDraft) -> &str {
    if !input.placeholder.trim().is_empty() {
        return input.placeholder.as_str();
    }
    match input.input_type.as_str() {
        "number" => "请输入数字",
        "file" => "输入文件 URL 或路径",
        "file-list" => "输入文件 URL 或路径，逗号分隔",
        _ if !input.hint.trim().is_empty() => input.hint.as_str(),
        _ => "请输入",
    }
}

fn workflow_run_prompt_error_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.extended_palette().danger.weak.color)),
        text_color: Some(theme.extended_palette().danger.strong.color),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.extended_palette().danger.base.color.scale_alpha(0.5),
        },
        ..Default::default()
    }
}

fn build_saved_apps_notice(message: &'static str) -> Element<'static, Message> {
    container(
        column![
            text(message).size(15).style(settings_muted_text_style),
            button(text("创建应用").size(13))
                .style(primary_action_btn_style)
                .padding([10, 16])
                .on_press(Message::WorkflowTool(WorkflowMessage::OpenCreateAppEditor)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn build_saved_apps_error(error: &str) -> Element<'_, Message> {
    container(
        column![
            text(format!("读取应用列表失败: {error}")).size(14),
            button(text("重试").size(13))
                .style(primary_action_btn_style)
                .padding([10, 16])
                .on_press(Message::WorkflowTool(WorkflowMessage::LoadSavedApps)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn saved_app_robot_badge() -> Element<'static, Message> {
    container(
        svg(assets::get_icon(Icon::Robot))
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .style(|theme: &Theme, _| iced::widget::svg::Style {
                color: Some(theme.palette().primary),
            }),
    )
    .width(Length::Fixed(48.0))
    .height(Length::Fixed(48.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(|theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(theme.palette().primary.scale_alpha(0.12))),
        border: Border {
            width: 1.0,
            color: theme.palette().primary.scale_alpha(0.16),
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn saved_app_tag(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(
                    palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                        0.70
                    } else {
                        0.52
                    }),
                )),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.18),
                    radius: 4.0.into(),
                },
                text_color: Some(theme.palette().text.scale_alpha(0.72)),
                ..Default::default()
            }
        })
        .into()
}

fn saved_app_actions_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::WHITE
        })),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.56 } else { 0.18 }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.28 } else { 0.12 }),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn saved_app_action_item_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    danger: bool,
    enabled: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let text_color = if danger { palette.danger.base.color } else { theme.palette().text };
    let background = if !enabled {
        None
    } else {
        match status {
            iced::widget::button::Status::Hovered => {
                let color =
                    if danger { palette.danger.weak.color } else { palette.background.weak.color };
                Some(Background::Color(color.scale_alpha(0.50)))
            }
            iced::widget::button::Status::Pressed => {
                let color = if danger {
                    palette.danger.strong.color
                } else {
                    palette.background.strong.color
                };
                Some(Background::Color(color.scale_alpha(0.52)))
            }
            _ => None,
        }
    };

    iced::widget::button::Style {
        background,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
        text_color: if enabled { text_color } else { text_color.scale_alpha(0.36) },
        ..Default::default()
    }
}

fn saved_app_card_container_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            palette.background.weak.color.scale_alpha(0.74)
        } else {
            Color::WHITE.scale_alpha(0.88)
        })),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.36
            } else {
                0.14
            }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.18 } else { 0.08 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

fn saved_app_card_button_style(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        ..Default::default()
    }
}

fn saved_app_name_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(if is_dark_theme(theme) { Color::WHITE } else { theme.palette().text }),
    }
}

fn saved_app_title(name: &str) -> String {
    if name.trim().is_empty() { "未命名应用".to_string() } else { name.trim().to_string() }
}

fn saved_app_description(description: &str) -> String {
    if description.trim().is_empty() {
        "暂无描述".to_string()
    } else {
        description.trim().to_string()
    }
}

fn format_saved_app_time(timestamp_ms: u64) -> String {
    let Ok(timestamp_ms) = i64::try_from(timestamp_ms) else {
        return "--".to_string();
    };
    let Some(dt) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return "--".to_string();
    };
    dt.format("%Y/%m/%d %H:%M").to_string()
}

#[cfg(test)]
#[path = "app_list_tests.rs"]
mod app_list_tests;
