use super::*;
use iced::widget::column;

#[cfg(test)]
pub(super) fn document_row<'a>(app: ViewInput<'a>, document: &'a KnowledgeDocumentDto) -> Element<'a, Message> {
    let chunking_mode = app
        .knowledge
        .selected_dataset()
        .map(|dataset| &dataset.chunking_mode)
        .unwrap_or(&KnowledgeChunkingMode::General);
    document_row_with_index(app, document, 1, chunking_mode)
}

pub(super) fn document_row_with_index<'a>(
    app: ViewInput<'a>,
    document: &'a KnowledgeDocumentDto,
    row_number: usize,
    chunking_mode: &'a KnowledgeChunkingMode,
) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected = app.knowledge.document_preview_open
        && app.knowledge.selected_document_id.as_deref() == Some(document.id.as_str());
    let reindex_enabled = !app.knowledge.is_busy();
    let loading_this = app.knowledge.loading_chunks
        && app.knowledge.selected_document_id.as_deref() == Some(document.id.as_str());
    let reindexing_this =
        app.knowledge.reindexing_document_id.as_deref() == Some(document.id.as_str());
    let updating_this = app.knowledge.updating_document_id.as_deref() == Some(document.id.as_str());
    let status = effective_document_status(document);
    let archived = status == KnowledgeDocumentStatus::Archived;
    let status_label = document_status_label(i18n, &status);
    let open_detail_message = (!loading_this).then_some(Message::Knowledge(
        KnowledgeToolMessage::LoadDocumentChunks(document.id.clone()),
    ));
    let title = document_title_button(&document.name, open_detail_message.clone(), selected);
    let name = if document.error_message.trim().is_empty() {
        column![title].spacing(2).width(Length::FillPortion(4))
    } else {
        column![title, text(&document.error_message).size(10).style(settings_muted_text_style),]
            .spacing(2)
            .width(Length::FillPortion(4))
    };

    let toggle_message = if archived {
        (!updating_this).then_some(Message::Knowledge(KnowledgeToolMessage::SetDocumentArchived {
            document_id: document.id.clone(),
            archived: false,
        }))
    } else {
        (!updating_this).then_some(Message::Knowledge(KnowledgeToolMessage::SetDocumentEnabled {
            document_id: document.id.clone(),
            enabled: !document.enabled,
        }))
    };
    let archive_message = (!archived && !updating_this).then_some(Message::Knowledge(
        KnowledgeToolMessage::SetDocumentArchived {
            document_id: document.id.clone(),
            archived: true,
        },
    ));
    let select_message = Some(Message::Knowledge(KnowledgeToolMessage::ToggleDocumentSelection(
        document.id.clone(),
    )));
    let selection_tooltip = if app.knowledge.selected_document_ids.contains(&document.id) {
        desktop_i18n::I18n::new(app.desktop_locale).__("取消选择")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("选择文档")
    };
    let content = row![
        container(document_selection_button(
            app.knowledge.selected_document_ids.contains(&document.id),
            selection_tooltip,
            select_message,
        ))
        .center_x(Length::Fixed(28.0)),
        text(row_number.to_string()).size(12).width(Length::Fixed(42.0)),
        name,
        text(chunking_label(chunking_mode))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(86.0)),
        text(compact_document_count_label(document.char_count))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(76.0)),
        text(compact_document_count_label(document.recall_count))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(76.0)),
        text(format_time(document.updated_at_ms))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fixed(150.0)),
        container(document_status_badge(document, status_label)).width(Length::Fixed(74.0)),
        row![
            document_icon_action_button(
                centered_knowledge_icon(
                    if archived || !document.enabled { Icon::Check } else { Icon::EyeSlash },
                    10.0,
                ),
                if archived {
                    desktop_i18n::I18n::new(app.desktop_locale).__("恢复文档")
                } else if document.enabled {
                    desktop_i18n::I18n::new(app.desktop_locale).__("停用文档")
                } else {
                    desktop_i18n::I18n::new(app.desktop_locale).__("启用文档")
                },
                toggle_message,
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Eye, 10.0),
                desktop_i18n::I18n::new(app.desktop_locale).__("查看详情"),
                open_detail_message,
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::ArrowRepeat, 10.0),
                desktop_i18n::I18n::new(app.desktop_locale).__("重新入库"),
                (reindex_enabled && !reindexing_this).then_some(Message::Knowledge(
                    KnowledgeToolMessage::ReindexDocument(document.id.clone())
                )),
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Pencil, 10.0),
                desktop_i18n::I18n::new(app.desktop_locale).__("重命名"),
                (!updating_this).then_some(Message::Knowledge(
                    KnowledgeToolMessage::OpenRenameDocument(document.id.clone()),
                )),
                false,
            ),
            document_icon_action_button(
                centered_knowledge_icon(Icon::Box, 10.0),
                desktop_i18n::I18n::new(app.desktop_locale).__("归档文档"),
                archive_message,
                false,
            ),
            document_icon_action_button(
                centered_danger_icon(Icon::Trash, 10.0),
                desktop_i18n::I18n::new(app.desktop_locale).__("删除文档"),
                (!app.knowledge.deleting).then_some(Message::Knowledge(
                    KnowledgeToolMessage::OpenDeleteDocumentConfirm(document.id.clone())
                )),
                true,
            ),
        ]
        .spacing(4)
        .width(Length::Fixed(DOCUMENT_ACTIONS_WIDTH)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(content)
        .padding([6, 8])
        .width(Length::Fill)
        .style(move |theme: &Theme| document_row_style(theme, selected))
        .into()
}

pub(super) fn document_selection_button<'a>(
    selected: bool,
    tooltip_label: &'a str,
    message: Option<Message>,
) -> Element<'a, Message> {
    let button = button(centered_knowledge_icon(
        if selected { Icon::CheckSquare } else { Icon::Square },
        14.0,
    ))
    .on_press_maybe(message)
    .padding(0)
    .width(Length::Fixed(22.0))
    .height(Length::Fixed(22.0))
    .style(document_selection_button_style);
    Tooltip::new(button, knowledge_tooltip_content(tooltip_label), TooltipPosition::Top)
        .gap(6.0)
        .into()
}

pub(super) fn document_selection_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub(super) fn build_document_bulk_toolbar<'a>(
    app: ViewInput<'a>,
    _visible_documents: &[&'a KnowledgeDocumentDto],
) -> Element<'a, Message> {
    let selected_count = app.knowledge.selected_document_ids.len();
    let pending_action = app.knowledge.pending_batch_document_action.as_ref();
    let busy = app.knowledge.batch_updating_documents;
    let enable_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Enable) {
        desktop_i18n::I18n::new(app.desktop_locale).__("启用中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("批量启用")
    };
    let disable_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Disable) {
        desktop_i18n::I18n::new(app.desktop_locale).__("停用中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("批量停用")
    };
    let archive_label = if busy && pending_action == Some(&KnowledgeDocumentsBatchAction::Archive) {
        desktop_i18n::I18n::new(app.desktop_locale).__("归档中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("批量归档")
    };

    container(
        row![
            text(
                desktop_i18n::I18n::new(app.desktop_locale)
                    .format("已选择 %s 篇文档", &[(selected_count).to_string().into()])
                    .expect("valid display template")
            )
            .size(12)
            .width(Length::Fill)
            .style(settings_muted_text_style),
            button(text(enable_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Enable,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text(disable_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Disable,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text(archive_label).size(12))
                .on_press_maybe((!busy).then_some(Message::Knowledge(
                    KnowledgeToolMessage::SubmitBatchDocumentAction {
                        action: KnowledgeDocumentsBatchAction::Archive,
                    },
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
            button(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(12))
                .on_press(Message::Knowledge(KnowledgeToolMessage::ClearDocumentSelection))
                .padding([8, 12])
                .style(knowledge_toolbar_button_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

pub(super) fn build_document_rename_modal<'a>(
    app: ViewInput<'a>,
    base: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let Some(document_id) = app.knowledge.rename_document_id.as_deref() else {
        return base.unwrap_or_else(|| Space::new().into());
    };
    let current_name = app
        .knowledge
        .documents
        .iter()
        .find(|document| document.id == document_id)
        .map(|document| document.name.as_str())
        .unwrap_or(desktop_i18n::I18n::new(app.desktop_locale).__("当前文档"));
    let submitting = app.knowledge.updating_document_id.as_deref() == Some(document_id);
    let can_submit = !submitting && !app.knowledge.rename_document_name_input.trim().is_empty();
    let submit_label = if submitting {
        desktop_i18n::I18n::new(app.desktop_locale).__("保存中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("保存")
    };
    let title_row = row![
        column![
            text(desktop_i18n::I18n::new(app.desktop_locale).__("重命名文档")).size(22),
            text(
                desktop_i18n::I18n::new(app.desktop_locale)
                    .format("当前名称：%s", &[(current_name).to_string().into()])
                    .expect("valid display template")
            )
            .size(12)
            .style(settings_muted_text_style),
        ]
        .spacing(6)
        .width(Length::Fill),
        settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("取消")).size(13))
            .on_press(Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument))
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(submit_label).size(13))
            .on_press_maybe(can_submit.then_some(Message::Knowledge(
                KnowledgeToolMessage::SubmitDocumentRename {
                    document_id: document_id.to_string(),
                    name: app.knowledge.rename_document_name_input.clone(),
                },
            )))
            .padding([9, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_card(
        column![
            title_row,
            text_input(
                desktop_i18n::I18n::new(app.desktop_locale).__("输入新的文档名称"),
                &app.knowledge.rename_document_name_input
            )
            .on_input(|value| {
                Message::Knowledge(KnowledgeToolMessage::RenameDocumentDraftChanged(value))
            })
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style),
            actions,
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(520.0))
    .max_width(560.0);

    settings_modal_overlay(
        base,
        Message::Knowledge(KnowledgeToolMessage::CloseRenameDocument),
        card,
    )
}

pub(super) fn document_title_button<'a>(
    label: &'a str,
    message: Option<Message>,
    selected: bool,
) -> Element<'a, Message> {
    button(
        container(text(label).size(12).line_height(1.0))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left),
    )
    .on_press_maybe(message)
    .padding([2, 4])
    .width(Length::Fill)
    .style(move |theme: &Theme, status| document_title_button_style(theme, status, selected))
    .into()
}

pub(super) fn document_icon_action_button<'a>(
    content: Element<'a, Message>,
    tooltip_label: &'a str,
    message: Option<Message>,
    danger: bool,
) -> Element<'a, Message> {
    let button = button(content)
        .on_press_maybe(message)
        .padding(0)
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0))
        .style(if danger { round_danger_icon_btn_style } else { round_icon_btn_style });
    Tooltip::new(button, knowledge_tooltip_content(tooltip_label), TooltipPosition::Top)
        .gap(6.0)
        .into()
}

pub(super) fn document_status_badge<'a>(
    document: &'a KnowledgeDocumentDto,
    status_label: &'a str,
) -> Element<'a, Message> {
    container(text(status_label).size(10).line_height(1.0))
        .padding([3, 7])
        .style(move |theme: &Theme| document_status_badge_style(theme, document))
        .into()
}
