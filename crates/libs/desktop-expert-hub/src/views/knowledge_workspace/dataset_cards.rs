use super::*;

pub(super) fn dataset_item<'a>(app: ViewInput<'a>, dataset: &'a KnowledgeDatasetDto) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected = app.knowledge.selected_dataset_id.as_deref() == Some(dataset.id.as_str());
    let dataset_id = dataset.id.clone();
    let title = dataset_title(i18n, &dataset.name);
    let description = dataset_description(i18n, &dataset.description);
    let updated_label = desktop_i18n::I18n::new(app.desktop_locale)
        .format("更新于 %s", &[(format_dataset_time(dataset.updated_at_ms)).to_string().into()])
        .expect("valid display template");

    let open_area = button(
        column![
            row![
                icon_badge(Icon::Journals, selected),
                column![
                    text(title).size(16).line_height(1.0).style(dataset_title_text_style),
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
    .style(dataset_card_button_style)
    .padding(0)
    .width(Length::Fill)
    .on_press(Message::Knowledge(KnowledgeToolMessage::SelectDataset(dataset_id)));

    let menu = dataset_actions_menu(app, dataset);

    container(
        column![
            row![open_area, menu].spacing(8).align_y(Alignment::Start),
            dataset_id_row(i18n, dataset),
            row![
                dataset_tag(chunking_label(&dataset.chunking_mode)),
                dataset_tag(indexing_label(i18n, &dataset.indexing_mode)),
                dataset_tag(retrieval_label(i18n, &dataset.retrieval_mode)),
                dataset_tag_static(if selected {
                    desktop_i18n::I18n::new(app.desktop_locale).__("当前选择")
                } else {
                    desktop_i18n::I18n::new(app.desktop_locale).__("知识库")
                }),
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
    .style(dataset_card_container_style)
    .into()
}

/// 卡片中部一行：左侧展示知识库 ID，右侧为紧凑的“复制”按钮。
pub(super) fn dataset_id_row(
    i18n: desktop_i18n::I18n,
    dataset: &KnowledgeDatasetDto,
) -> Element<'_, Message> {
    let copy_content = row![
        themed_inline_icon(Icon::Copy, 13.0),
        text(i18n.__("复制")).size(11).line_height(1.0),
    ]
    .spacing(5)
    .align_y(Alignment::Center);

    row![
        text(format!("ID: {}", dataset.id))
            .size(11)
            .style(settings_muted_text_style)
            .width(Length::Fill),
        settings_value_badge(
            i18n.format("%s 文档", &[(dataset.document_count).to_string().into()])
                .expect("valid display template")
        ),
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
        .height(Length::Fixed(28.0))
        .on_press(Message::Knowledge(KnowledgeToolMessage::CopyDatasetId(dataset.id.clone()))),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// 卡片右上角的“更多操作”下拉菜单（删除）。
pub(super) fn dataset_actions_menu<'a>(
    app: ViewInput<'a>,
    dataset: &'a KnowledgeDatasetDto,
) -> Element<'a, Message> {
    let menu_open = app.knowledge.dataset_menu_open_id.as_deref() == Some(dataset.id.as_str());

    let trigger = button(
        container(themed_inline_icon(Icon::DotsThreeVertical, 16.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .style(round_icon_btn_style)
    .on_press(Message::Knowledge(KnowledgeToolMessage::ToggleDatasetMenu(dataset.id.clone())));

    let delete = button(
        row![
            themed_inline_icon(Icon::Trash, 14.0),
            text(desktop_i18n::I18n::new(app.desktop_locale).__("删除知识库"))
                .size(13)
                .width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::Knowledge(KnowledgeToolMessage::OpenDeleteDatasetConfirmFor(
        dataset.id.clone(),
    )))
    .padding([8, 10])
    .width(Length::Fill)
    .style(knowledge_danger_menu_entry_style);

    PointBelowOverlay::new(
        trigger,
        container(column![delete].spacing(4).width(Length::Fill))
            .width(Length::Fixed(168.0))
            .padding(6)
            .style(knowledge_status_menu_style),
    )
    .show(menu_open)
    .anchor(Point::new(-136.0, 36.0))
    .gap(4.0)
    .on_close(Message::Knowledge(KnowledgeToolMessage::CloseDatasetMenu))
    .capture_outside_click(false)
    .into()
}

pub(super) fn knowledge_danger_menu_entry_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    iced::widget::button::Style {
        background: hovered.then(|| palette.danger.base.color.scale_alpha(0.16).into()),
        text_color: palette.danger.base.color,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub(super) fn dataset_title(i18n: desktop_i18n::I18n, name: &str) -> String {
    if name.trim().is_empty() {
        i18n.__("未命名知识库").to_string()
    } else {
        name.trim().to_string()
    }
}

pub(super) fn dataset_description(i18n: desktop_i18n::I18n, description: &str) -> String {
    if description.trim().is_empty() {
        i18n.__("暂无描述").to_string()
    } else {
        description.trim().to_string()
    }
}

pub(super) fn dataset_tag(label: &'static str) -> Element<'static, Message> {
    dataset_tag_static(label)
}

pub(super) fn dataset_tag_static(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(11))
        .padding([4, 8])
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(
                    palette.background.weak.color.scale_alpha(if is_dark_palette(theme) {
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

pub(super) fn format_dataset_time(timestamp_ms: u64) -> String {
    use chrono::{Local, TimeZone};

    let Ok(timestamp_ms) = i64::try_from(timestamp_ms) else {
        return "--".to_string();
    };
    let Some(dt) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return "--".to_string();
    };
    dt.format("%Y/%m/%d %H:%M").to_string()
}

pub(super) fn dataset_card_container_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_palette(theme) {
            palette.background.weak.color.scale_alpha(0.74)
        } else {
            Color::WHITE.scale_alpha(0.88)
        })),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_palette(theme) {
                0.36
            } else {
                0.14
            }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_palette(theme) { 0.18 } else { 0.08 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

pub(super) fn dataset_card_button_style(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        ..Default::default()
    }
}

pub(super) fn dataset_title_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(if is_dark_palette(theme) { Color::WHITE } else { theme.palette().text }),
    }
}
