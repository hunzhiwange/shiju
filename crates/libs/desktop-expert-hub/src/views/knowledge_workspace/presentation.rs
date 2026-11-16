use super::*;
use iced::widget::column;

pub(super) fn chrome_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    iced::widget::container::Style {
        background: Some(
            if is_dark {
                palette.background.base.color.scale_alpha(0.72)
            } else {
                palette.background.weak.color.scale_alpha(0.42)
            }
            .into(),
        ),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 0.80 }),
        },
        ..Default::default()
    }
}

pub(super) fn centered_knowledge_icon(icon: Icon, size: f32) -> Element<'static, Message> {
    container(icon_svg(icon, size).style(|theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(theme.palette().text),
    }))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

pub(super) fn centered_danger_icon(icon: Icon, size: f32) -> Element<'static, Message> {
    container(
        icon_svg(icon, size).style(|_theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(Color::WHITE),
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

pub(super) fn themed_inline_icon(icon: Icon, size: f32) -> iced::widget::svg::Svg<'static> {
    icon_svg(icon, size).style(|theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(theme.palette().text),
    })
}

pub(super) fn primary_action_icon(icon: Icon, size: f32) -> iced::widget::svg::Svg<'static> {
    icon_svg(icon, size)
        .style(|_theme: &Theme, _status| iced::widget::svg::Style { color: Some(Color::WHITE) })
}

pub(super) fn centered_header_action_content(icon: Icon, label: &'static str) -> Element<'static, Message> {
    container(
        row![themed_inline_icon(icon, 14.0), text(label).size(13)]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .width(Length::Shrink)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .padding([0, 14])
    .into()
}

pub(super) fn round_danger_icon_btn_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = danger_action_btn_style(theme, status);
    style.border.radius = 999.0.into();
    style
}

pub(super) fn build_delete_confirm_modal<'a>(
    app: ViewInput<'a>,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let (title, description, confirm_message) = if let Some(document_id) =
        app.knowledge.pending_delete_document_id.as_deref()
    {
        let document_name = app
            .knowledge
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .map(|document| document.name.as_str())
            .unwrap_or(desktop_i18n::I18n::new(app.desktop_locale).__("当前文档"));
        (
            desktop_i18n::I18n::new(app.desktop_locale).__("确认删除文档"),
            desktop_i18n::I18n::new(app.desktop_locale)
                .format("将删除文档「%s」。此操作不可撤销。", &[(document_name).to_string().into()])
                .expect("valid display template"),
            Message::Knowledge(KnowledgeToolMessage::DeleteDocument(document_id.to_string())),
        )
    } else {
        let dataset_name = app
            .knowledge
            .pending_delete_dataset_id
            .as_deref()
            .and_then(|dataset_id| app.knowledge.dataset_by_id(dataset_id))
            .or_else(|| app.knowledge.selected_dataset())
            .map(|dataset| dataset.name.as_str())
            .unwrap_or(desktop_i18n::I18n::new(app.desktop_locale).__("当前知识库"));
        (
            desktop_i18n::I18n::new(app.desktop_locale).__("确认删除知识库"),
            desktop_i18n::I18n::new(app.desktop_locale)
                .format(
                    "将删除知识库「%s」及其内容。此操作不可撤销。",
                    &[(dataset_name).to_string().into()],
                )
                .expect("valid display template"),
            Message::Knowledge(KnowledgeToolMessage::DeleteSelectedDataset),
        )
    };

    let title_row = row![
        column![text(title).size(22), text(description).size(12).style(settings_muted_text_style),]
            .spacing(6)
            .width(Length::Fill),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let actions = row![
        Space::new().width(Length::Fill),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(13))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm))
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("确认删除")).size(13))
            .on_press_maybe((!app.knowledge.deleting).then_some(confirm_message))
            .padding([9, 14])
            .style(danger_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let card = settings_modal_card(
        column![
            title_row,
            container(
                text(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .__("删除后无法恢复，请确认当前选择无误。")
                )
                .size(13)
                .style(settings_muted_text_style),
            )
            .padding([14, 16])
            .width(Length::Fill),
            actions,
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(520.0))
    .max_width(560.0);

    settings_modal_overlay(base, Message::Knowledge(KnowledgeToolMessage::CloseDeleteConfirm), card)
}

pub(super) fn file_drop_zone_style(
    theme: &Theme,
    highlighted: bool,
    has_selection: bool,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let border_color = if highlighted {
        theme.palette().primary
    } else if has_selection {
        palette.primary.base.color.scale_alpha(0.70)
    } else {
        palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 0.78 })
    };
    let background = if highlighted {
        theme.palette().primary.scale_alpha(if is_dark { 0.18 } else { 0.12 })
    } else if has_selection {
        palette.background.weak.color.scale_alpha(if is_dark { 0.72 } else { 0.48 })
    } else if is_dark {
        palette.background.base.color.scale_alpha(0.84)
    } else {
        palette.background.weak.color.scale_alpha(0.34)
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border {
            radius: 18.0.into(),
            width: if highlighted { 2.0 } else { 1.0 },
            color: border_color,
        },
        ..Default::default()
    }
}

pub(super) fn segment_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let background = if active {
        theme.palette().primary.scale_alpha(0.88)
    } else if hovered {
        if is_dark {
            palette.background.strong.color.scale_alpha(0.88)
        } else {
            palette.background.strong.color.scale_alpha(0.62)
        }
    } else {
        if is_dark {
            palette.background.base.color.scale_alpha(0.72)
        } else {
            palette.background.weak.color.scale_alpha(0.42)
        }
    };
    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: if active { Color::WHITE } else { theme.palette().text },
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if active {
                theme.palette().primary
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.92 } else { 1.0 })
            },
        },
        ..Default::default()
    }
}

pub(super) fn knowledge_toolbar_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_palette(theme);
    let disabled = matches!(status, iced::widget::button::Status::Disabled);
    let background = match status {
        iced::widget::button::Status::Hovered => {
            if is_dark {
                palette.background.weak.color
            } else {
                Color::WHITE.scale_alpha(0.94)
            }
        }
        iced::widget::button::Status::Pressed => {
            if is_dark {
                palette.background.strong.color.scale_alpha(0.94)
            } else {
                palette.background.weak.color.scale_alpha(0.90)
            }
        }
        iced::widget::button::Status::Disabled => {
            if is_dark {
                palette.background.base.color.scale_alpha(0.32)
            } else {
                palette.background.weak.color.scale_alpha(0.52)
            }
        }
        _ => {
            if is_dark {
                palette.background.base.color.scale_alpha(0.68)
            } else {
                Color::WHITE.scale_alpha(0.72)
            }
        }
    };
    let border_color = if is_dark {
        palette.background.strong.color.scale_alpha(if disabled { 0.44 } else { 0.92 })
    } else {
        Color::from_rgba8(15, 23, 42, if disabled { 0.08 } else { 0.14 })
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text.scale_alpha(if disabled { 0.42 } else { 0.92 }),
        border: Border { radius: 12.0.into(), width: 1.0, color: border_color },
        ..Default::default()
    }
}

pub(super) fn is_dark_palette(theme: &Theme) -> bool {
    let bg = theme.palette().background;
    bg.r + bg.g + bg.b < 1.5
}
