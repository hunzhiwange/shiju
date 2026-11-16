use super::*;
use iced::widget::column;

pub(super) fn build_chunks_panel(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected_document = app.knowledge.selected_document();
    let title = selected_document
        .map(|document| {
            desktop_i18n::I18n::new(app.desktop_locale)
                .format("分段预览 · %s", &[(document.name).to_string().into()])
                .expect("valid display template")
        })
        .unwrap_or_else(|| {
            desktop_i18n::I18n::new(app.desktop_locale).__("分段预览").to_string()
        });
    let mut content = column![text(title).size(14)].spacing(10).height(Length::Fill);

    if selected_document.is_none() {
        content = content.push(empty_hint(
            desktop_i18n::I18n::new(app.desktop_locale).__("选择文档查看分段"),
        ));
    } else if app.knowledge.loading_chunks {
        content = content
            .push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("正在加载分段")));
    } else if app.knowledge.loaded_chunks.is_empty() {
        content = content
            .push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("暂无分段")));
    } else {
        let mut list = column![].spacing(8);
        for chunk in &app.knowledge.loaded_chunks {
            let selected = app.knowledge.selected_chunk_id.as_deref() == Some(chunk.id.as_str());
            list = list.push(chunk_preview_card(i18n, chunk, selected));
        }
        content = content.push(knowledge_scrollable(list).height(Length::Fill));
        if let Some(chunk) = app.knowledge.selected_chunk() {
            content = content.push(chunk_content_panel(i18n, chunk, app.last_copied_code_hash));
        }
    }

    settings_panel(content).into()
}

pub(super) fn build_document_preview_drawer(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let selected_document = app.knowledge.selected_document();
    let title = selected_document
        .map(|document| document.name.as_str())
        .unwrap_or(desktop_i18n::I18n::new(app.desktop_locale).__("文档预览"));

    let mut body = column![
        row![
            column![
                text(desktop_i18n::I18n::new(app.desktop_locale).__("文档预览")).size(16),
                text(title).size(12).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            settings_close_button(Message::Knowledge(KnowledgeToolMessage::CloseDocumentPreview)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .height(Length::Fill);

    if selected_document.is_none() {
        body = body.push(empty_hint(
            desktop_i18n::I18n::new(app.desktop_locale).__("选择文档查看分段"),
        ));
    } else if app.knowledge.loading_chunks {
        body = body
            .push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("正在加载分段")));
    } else if app.knowledge.loaded_chunks.is_empty() {
        body =
            body.push(empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("暂无分段")));
    } else {
        let mut list = column![].spacing(8);
        for chunk in &app.knowledge.loaded_chunks {
            let selected = app.knowledge.selected_chunk_id.as_deref() == Some(chunk.id.as_str());
            list = list.push(chunk_preview_card(i18n, chunk, selected));
        }
        body = body.push(knowledge_scrollable(list).height(Length::FillPortion(3)));
        if let Some(chunk) = app.knowledge.selected_chunk() {
            body = body.push(
                container(chunk_content_panel(i18n, chunk, app.last_copied_code_hash))
                    .height(Length::FillPortion(2)),
            );
        }
    }

    container(
        container(body)
            .width(Length::Fixed(430.0))
            .height(Length::Fill)
            .padding([18, 20])
            .style(document_preview_drawer_style),
    )
    .padding(iced::Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 0.0 })
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

pub(super) fn document_preview_drawer_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_palette(theme) {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::WHITE.scale_alpha(0.98)
        })),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_palette(theme) {
                0.56
            } else {
                0.18
            }),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_palette(theme) { 0.28 } else { 0.14 }),
            offset: Vector::new(-8.0, 0.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub(super) fn build_retrieval_tab(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let results = if app.knowledge.retrieve_results.is_empty() {
        empty_hint(desktop_i18n::I18n::new(app.desktop_locale).__("召回结果会显示在这里"))
    } else {
        let mut list = column![].spacing(10);
        for chunk in &app.knowledge.retrieve_results {
            list = list.push(chunk_card(i18n, chunk));
        }
        knowledge_scrollable(list).height(Length::Fill).into()
    };

    settings_panel(
        column![
            row![
                text_input(
                    desktop_i18n::I18n::new(app.desktop_locale).__("输入查询文本"),
                    &app.knowledge.retrieve_query_input
                )
                .on_input(|value| {
                    Message::Knowledge(KnowledgeToolMessage::RetrieveQueryChanged(value))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
                text_input("Top K", &app.knowledge.retrieve_top_k_input)
                    .on_input(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveTopKChanged(value))
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fixed(90.0)),
                button(
                    row![
                        primary_action_icon(Icon::Search, 14.0),
                        text(desktop_i18n::I18n::new(app.desktop_locale).__("测试")).size(13)
                    ]
                    .spacing(8)
                )
                .on_press_maybe(
                    (!app.knowledge.retrieving)
                        .then_some(Message::Knowledge(KnowledgeToolMessage::Retrieve))
                )
                .padding([10, 14])
                .style(primary_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                checkbox(app.knowledge.retrieve_score_threshold_enabled)
                    .label(desktop_i18n::I18n::new(app.desktop_locale).__("Score 阈值"))
                    .on_toggle(|value| {
                        Message::Knowledge(
                            KnowledgeToolMessage::RetrieveScoreThresholdEnabledChanged(value),
                        )
                    })
                    .style(settings_checkbox_style),
                text_input("0.15", &app.knowledge.retrieve_score_threshold_input)
                    .on_input(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveScoreThresholdChanged(
                            value,
                        ))
                    })
                    .padding([9, 10])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fixed(110.0)),
                checkbox(app.knowledge.retrieve_include_context)
                    .label(desktop_i18n::I18n::new(app.desktop_locale).__("包含上下文"))
                    .on_toggle(|value| {
                        Message::Knowledge(KnowledgeToolMessage::RetrieveIncludeContextChanged(
                            value,
                        ))
                    })
                    .style(settings_checkbox_style),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(results).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(12)
        .height(Length::Fill),
    )
    .into()
}

pub(super) fn build_settings_tab<'a>(app: ViewInput<'a>, dataset: &'a KnowledgeDatasetDto) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let mut status_items = column![].spacing(10);
    if let Some(status) = &app.knowledge.runtime_status {
        status_items = status_items
            .push(kv_row(
                desktop_i18n::I18n::new(app.desktop_locale).__("全文检索"),
                support_label(i18n, status.full_text),
            ))
            .push(kv_row(
                desktop_i18n::I18n::new(app.desktop_locale).__("向量检索"),
                support_label(i18n, status.vector),
            ))
            .push(kv_row(
                desktop_i18n::I18n::new(app.desktop_locale).__("混合检索"),
                support_label(i18n, status.hybrid),
            ))
            .push(kv_row("Rerank", support_label(i18n, status.rerank)));
        for (key, value) in &status.notes {
            status_items = status_items.push(kv_row(key, value));
        }
    } else {
        status_items = status_items.push(kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("运行状态"),
            desktop_i18n::I18n::new(app.desktop_locale).__("检测中"),
        ));
    }

    let content = column![
        build_embedding_model_section(app),
        kv_row(desktop_i18n::I18n::new(app.desktop_locale).__("知识库 ID"), &dataset.id),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("分段模式"),
            chunking_label(&dataset.chunking_mode)
        ),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("索引模式"),
            indexing_label(i18n, &dataset.indexing_mode)
        ),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("检索模式"),
            retrieval_label(i18n, &dataset.retrieval_mode)
        ),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("关键词数量"),
            dataset.keyword_count.to_string()
        ),
        kv_row("Top K", dataset.top_k.to_string()),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("Score 阈值"),
            score_threshold_label(i18n, dataset)
        ),
        kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("Rerank 开关"),
            enabled_label(i18n, dataset.rerank_enabled)
        ),
        kv_row("Embedding", dataset_embedding_model_label(app, dataset)),
        kv_row(
            "Rerank",
            dataset
                .rerank_model
                .as_deref()
                .unwrap_or(desktop_i18n::I18n::new(app.desktop_locale).__("未启用"))
        ),
        container(status_items).padding([8, 0]),
    ]
    .spacing(12);
    settings_panel(knowledge_scrollable(content).height(Length::Fill)).into()
}

pub(super) fn chunk_card(i18n: desktop_i18n::I18n, chunk: &KnowledgeChunkDto) -> Element<'_, Message> {
    let score = chunk.score.map(|value| format!("{value:.3}")).unwrap_or_else(|| "-".to_string());
    let mut details = column![text(&chunk.content).size(13)].spacing(8);
    if let Some(matched) = chunk.matched_content.as_deref().filter(|value| !value.trim().is_empty())
    {
        details = details.push(
            column![
                text(i18n.__("匹配内容")).size(11).style(settings_muted_text_style),
                text(matched).size(12),
            ]
            .spacing(3),
        );
    }
    if let Some(context) = chunk.context.as_deref().filter(|value| !value.trim().is_empty()) {
        details = details.push(
            column![
                text(i18n.__("上下文")).size(11).style(settings_muted_text_style),
                text(context).size(12),
            ]
            .spacing(3),
        );
    }

    container(
        column![
            row![
                text(&chunk.title).size(14).width(Length::Fill),
                settings_value_badge(match_type_label(i18n, &chunk.match_type)),
                settings_value_badge(format!("score {score}")),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            details,
        ]
        .spacing(8),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(chrome_style)
    .into()
}

pub(super) fn chunk_preview_card(
    i18n: desktop_i18n::I18n,
    chunk: &KnowledgeChunkDto,
    selected: bool,
) -> Element<'_, Message> {
    let mut badges = row![
        settings_value_badge(format!("#{}", chunk.ordinal)),
        settings_value_badge(chunk_type_label(i18n, &chunk.chunk_type)),
        settings_value_badge(format!("{}..{}", chunk.start_at, chunk.end_at)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if chunk.parent_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("parent"));
    }
    if chunk.prev_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("prev"));
    }
    if chunk.next_chunk_id.is_some() {
        badges = badges.push(settings_value_badge("next"));
    }

    let title = chunk_preview_title(i18n, chunk);
    let preview = chunk.content.lines().next().unwrap_or("").trim();
    let body = column![
        row![
            text(title).size(13).width(Length::Fill),
            settings_value_badge(match_type_label(i18n, &chunk.match_type)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(preview).size(12).style(settings_muted_text_style),
        badges,
    ]
    .spacing(7);

    button(
        container(body)
            .padding([10, 12])
            .width(Length::Fill)
            .style(move |theme: &Theme| item_style(theme, selected)),
    )
    .padding(0)
    .width(Length::Fill)
    .style(button::text)
    .on_press(Message::Knowledge(KnowledgeToolMessage::SelectChunk(chunk.id.clone())))
    .into()
}

pub(super) fn chunk_preview_title(i18n: desktop_i18n::I18n, chunk: &KnowledgeChunkDto) -> String {
    let title = chunk.title.trim();
    if !title.is_empty() {
        return title.to_string();
    }
    let first_line = chunk.content.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        i18n.format("分段 %s", &[(chunk.ordinal).to_string().into()])
            .expect("valid display template")
    } else {
        first_line.to_string()
    }
}

pub(super) fn hash_copy_text(content: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn chunk_content_panel<'a>(
    i18n: desktop_i18n::I18n,
    chunk: &'a KnowledgeChunkDto,
    last_copied_code_hash: Option<u64>,
) -> Element<'a, Message> {
    let copied = last_copied_code_hash == Some(hash_copy_text(&chunk.content));
    let copy_icon = if copied { Icon::Check } else { Icon::Copy };
    let copy_label = if copied { i18n.__("已复制") } else { i18n.__("复制") };
    container(
        column![
            row![
                text(i18n.__("完整内容"))
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
                button(
                    row![
                        icon_svg(copy_icon, 12.0).style(|theme: &Theme, _status| {
                            iced::widget::svg::Style { color: Some(theme.palette().text) }
                        }),
                        text(copy_label).size(12)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .on_press(Message::CopyCode(chunk.content.clone()))
                .padding([6, 10])
                .style(knowledge_toolbar_button_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            knowledge_scrollable(
                container(text(&chunk.content).size(12).style(settings_muted_text_style))
                    .width(Length::Fill)
            )
            .height(Length::Fill),
        ]
        .spacing(10)
        .height(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(chrome_style)
    .into()
}

pub(super) fn kv_row<'a>(label: &'a str, value: impl ToString + 'a) -> Element<'a, Message> {
    row![
        text(label).size(13).style(settings_muted_text_style).width(Length::Fixed(120.0)),
        text(value.to_string()).size(13).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn embedding_kv_row<'a>(label: &'a str, value: impl ToString + 'a) -> Element<'a, Message> {
    row![
        text(label).size(12).style(settings_muted_text_style).width(Length::Fixed(76.0)),
        text(value.to_string()).size(12).width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn knowledge_scrollable<'a>(
    content: impl Into<Element<'a, Message>> + 'a,
) -> scrollable::Scrollable<'a, Message> {
    scrollable(content).direction(Direction::Vertical(
        Scrollbar::new()
            .width(KNOWLEDGE_SCROLLBAR_WIDTH)
            .scroller_width(KNOWLEDGE_SCROLLBAR_WIDTH)
            .spacing(KNOWLEDGE_SCROLLBAR_GUTTER - KNOWLEDGE_SCROLLBAR_WIDTH),
    ))
}

pub(super) fn segment_button<'a>(label: &'a str, active: bool, message: Message) -> Element<'a, Message> {
    button(text(label).size(12))
        .on_press(message)
        .padding([7, 11])
        .style(move |theme: &Theme, status| segment_style(theme, status, active))
        .into()
}

pub(super) fn icon_badge(icon: Icon, active: bool) -> Element<'static, Message> {
    container(icon_svg(icon, 18.0).style(move |theme: &Theme, _status| iced::widget::svg::Style {
        color: Some(if active { theme.palette().primary } else { theme.palette().text }),
    }))
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(if active {
                theme.palette().primary.scale_alpha(0.12)
            } else {
                palette.background.weak.color.scale_alpha(0.42)
            })),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: if active {
                    theme.palette().primary.scale_alpha(0.72)
                } else {
                    palette.background.strong.color.scale_alpha(0.88)
                },
            },
            ..Default::default()
        }
    })
    .into()
}

pub(super) fn empty_hint<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(13).style(settings_muted_text_style))
        .padding([18, 16])
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(chrome_style)
        .into()
}

pub(super) fn item_style(theme: &Theme, selected: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let base = if selected {
        theme.palette().primary.scale_alpha(0.10)
    } else {
        palette.background.base.color
    };
    iced::widget::container::Style {
        background: Some(base.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if selected { theme.palette().primary } else { palette.background.strong.color },
        },
        ..Default::default()
    }
}

pub(super) fn option_card_style(theme: &Theme, active: bool, enabled: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let background = if active {
        theme.palette().primary.scale_alpha(0.10)
    } else {
        palette.background.weak.color.scale_alpha(if enabled { 0.42 } else { 0.22 })
    };
    let border_color = if active {
        theme.palette().primary
    } else {
        palette.background.strong.color.scale_alpha(if enabled { 0.80 } else { 0.42 })
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border {
            radius: 8.0.into(),
            width: if active { 2.0 } else { 1.0 },
            color: border_color,
        },
        text_color: (!enabled).then_some(theme.palette().text.scale_alpha(0.52)),
        ..Default::default()
    }
}
