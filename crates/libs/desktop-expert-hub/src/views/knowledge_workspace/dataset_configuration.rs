use super::*;
use iced::widget::column;

pub(super) fn build_chunking_mode_section(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    column![
        section_help(app, "分段模式", "选择文档入库结构。"),
        option_card(
            i18n,
            "General",
            desktop_i18n::I18n::new(app.desktop_locale).__("通用文本分块。"),
            Icon::ListUl,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::General,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::General,
            )),
        ),
        option_card(
            i18n,
            "Parent-Child",
            desktop_i18n::I18n::new(app.desktop_locale).__("子块检索，父块返回。"),
            Icon::Columns,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::ParentChild,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::ParentChild,
            )),
        ),
        option_card(
            i18n,
            "Q&A",
            desktop_i18n::I18n::new(app.desktop_locale).__("问题检索，答案返回。"),
            Icon::ChatTextFill,
            app.knowledge.dataset_chunking_mode == KnowledgeChunkingMode::Qa,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetChunkingModeChanged(
                KnowledgeChunkingMode::Qa,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

pub(super) fn build_indexing_mode_section(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let vector_available = embedding_vector_available(app);
    column![
        section_header(
            desktop_i18n::I18n::new(app.desktop_locale).__("索引模式"),
            desktop_i18n::I18n::new(app.desktop_locale).__("高质量需要嵌入能力。")
        ),
        option_card(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("高质量"),
            desktop_i18n::I18n::new(app.desktop_locale).__("生成向量索引。"),
            Icon::Speedometer2,
            app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::HighQuality,
            vector_available,
            true,
            Message::Knowledge(KnowledgeToolMessage::DatasetIndexingModeChanged(
                KnowledgeIndexingMode::HighQuality,
            )),
        ),
        option_card(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("经济"),
            desktop_i18n::I18n::new(app.desktop_locale).__("关键词检索。"),
            Icon::HddNetwork,
            app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::Economy,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetIndexingModeChanged(
                KnowledgeIndexingMode::Economy,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

pub(super) fn build_embedding_model_section(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let status = app.knowledge.embedding_status.as_ref();
    let test_label = if app.knowledge.testing_embedding {
        desktop_i18n::I18n::new(app.desktop_locale).__("测试中")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("测试连接")
    };
    let test_enabled = !app.knowledge.testing_embedding && !app.knowledge.loading_embedding_status;

    let summary = row![
        embedding_summary_item("Provider", embedding_status_provider_label(i18n, status)),
        embedding_summary_item(
            desktop_i18n::I18n::new(app.desktop_locale).__("模型"),
            embedding_status_model_label(i18n, status)
        ),
        embedding_summary_item(
            desktop_i18n::I18n::new(app.desktop_locale).__("维度"),
            embedding_status_dimensions_label(i18n, status)
        ),
        embedding_summary_item(
            desktop_i18n::I18n::new(app.desktop_locale).__("路由"),
            embedding_status_route_label(i18n, status)
        ),
    ]
    .spacing(8)
    .width(Length::Fill);

    let mut content = column![
        section_help(app, "嵌入模型", "跟随全局配置，或锁定当前已生效模型。"),
        build_embedding_model_picker(app),
        summary,
        row![
            settings_value_badge(
                desktop_i18n::I18n::new(app.desktop_locale)
                    .format(
                        "向量 %s",
                        &[(support_label(i18n, embedding_vector_available(app)))
                            .to_string()
                            .into()]
                    )
                    .expect("valid display template")
            ),
            settings_value_badge(
                desktop_i18n::I18n::new(app.desktop_locale)
                    .format(
                        "混合 %s",
                        &[(support_label(i18n, embedding_hybrid_available(app)))
                            .to_string()
                            .into()]
                    )
                    .expect("valid display template")
            ),
        ]
        .spacing(6),
        row![
            button(
                row![primary_action_icon(Icon::Search, 13.0), text(test_label).size(12)]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .on_press_maybe(
                test_enabled.then_some(Message::Knowledge(KnowledgeToolMessage::RunEmbeddingTest))
            )
            .padding([8, 10])
            .style(primary_action_btn_style),
            button(
                row![
                    primary_action_icon(Icon::Gear, 13.0),
                    text(desktop_i18n::I18n::new(app.desktop_locale).__("记忆配置")).size(12)
                ]
                .spacing(6)
                .align_y(Alignment::Center)
            )
            .on_press(Message::OpenMemorySettings)
            .padding([8, 10])
            .style(primary_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(9);

    if app.knowledge.embedding_test_result.is_some() {
        content = content.push(embedding_kv_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("测试结果"),
            embedding_test_result_label(i18n, app.knowledge.embedding_test_result.as_ref()),
        ));
    }

    container(content).padding([16, 16]).width(Length::Fill).style(chrome_style).into()
}

pub(super) fn embedding_summary_item<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    container(
        column![text(label).size(10).style(settings_muted_text_style), text(value).size(12),]
            .spacing(3),
    )
    .padding([8, 9])
    .width(Length::FillPortion(1))
    .style(|theme: &Theme| item_style(theme, false))
    .into()
}

pub(super) fn build_retrieval_mode_section(app: ViewInput<'_>) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let vector_available = embedding_vector_available(app);
    let hybrid_available = embedding_hybrid_available(app);
    column![
        section_help(app, "检索设置", "选择召回策略。"),
        option_card(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("向量检索"),
            desktop_i18n::I18n::new(app.desktop_locale).__("语义相似召回。"),
            Icon::Grid1x2,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::Vector,
            vector_available,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::Vector,
            )),
        ),
        option_card(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("全文检索"),
            desktop_i18n::I18n::new(app.desktop_locale).__("按词汇召回。"),
            Icon::FileText,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::FullText,
            true,
            false,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::FullText,
            )),
        ),
        option_card(
            i18n,
            desktop_i18n::I18n::new(app.desktop_locale).__("混合检索"),
            desktop_i18n::I18n::new(app.desktop_locale).__("全文和向量合并。"),
            Icon::Grid1x2,
            app.knowledge.dataset_retrieval_mode == KnowledgeRetrievalMode::Hybrid,
            hybrid_available,
            true,
            Message::Knowledge(KnowledgeToolMessage::DatasetRetrievalModeChanged(
                KnowledgeRetrievalMode::Hybrid,
            )),
        ),
    ]
    .spacing(8)
    .into()
}

pub(super) fn build_dataset_param_section(app: ViewInput<'_>) -> Element<'_, Message> {
    let mut content = column![section_help(app, "参数", "默认召回参数。")].spacing(8);
    if app.knowledge.dataset_indexing_mode == KnowledgeIndexingMode::Economy {
        content = content.push(compact_input_row(
            desktop_i18n::I18n::new(app.desktop_locale).__("关键词数量"),
            &app.knowledge.dataset_keyword_count_input,
            |value| Message::Knowledge(KnowledgeToolMessage::DatasetKeywordCountChanged(value)),
        ));
    }
    content =
        content.push(compact_input_row("Top K", &app.knowledge.dataset_top_k_input, |value| {
            Message::Knowledge(KnowledgeToolMessage::DatasetTopKChanged(value))
        }));
    content = content.push(
        checkbox(app.knowledge.dataset_score_threshold_enabled)
            .label(desktop_i18n::I18n::new(app.desktop_locale).__("Score 阈值"))
            .on_toggle(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetScoreThresholdEnabledChanged(value))
            })
            .style(settings_checkbox_style),
    );
    content = content.push(compact_input_row(
        desktop_i18n::I18n::new(app.desktop_locale).__("阈值"),
        &app.knowledge.dataset_score_threshold_input,
        |value| Message::Knowledge(KnowledgeToolMessage::DatasetScoreThresholdChanged(value)),
    ));
    content = content.push(build_rerank_toggle(app));
    if app.knowledge.dataset_rerank_enabled {
        content = content.push(
            settings_text_input(
                desktop_i18n::I18n::new(app.desktop_locale).__("rerank 模型"),
                &app.knowledge.dataset_rerank_model_input,
            )
            .on_input(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetRerankModelChanged(value))
            }),
        );
    }
    content.into()
}

pub(super) fn build_rerank_toggle(app: ViewInput<'_>) -> Element<'_, Message> {
    let rerank_available =
        app.knowledge.runtime_status.as_ref().is_some_and(|status| status.rerank);
    let label = if rerank_available {
        desktop_i18n::I18n::new(app.desktop_locale).__("Rerank 模型")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("Rerank 模型（未配置）")
    };
    let toggle = checkbox(app.knowledge.dataset_rerank_enabled && rerank_available)
        .label(desktop_i18n::I18n::new(app.desktop_locale).__(label))
        .style(settings_checkbox_style);
    if rerank_available {
        toggle
            .on_toggle(|value| {
                Message::Knowledge(KnowledgeToolMessage::DatasetRerankEnabledChanged(value))
            })
            .into()
    } else {
        toggle.into()
    }
}

pub(super) fn section_header<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(12), text(description).size(11).style(settings_muted_text_style),]
        .spacing(2)
        .into()
}

pub(super) fn section_help<'a>(app: ViewInput<'a>, title: &'a str, description: &'a str) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let title = i18n.__(title);
    settings_hover_help_label(
        i18n,
        app.app_theme,
        title,
        text(title).size(12),
        i18n.__(description),
    )
    .wrap()
    .into()
}

pub(super) fn compact_input_row<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(12).style(settings_muted_text_style).width(Length::Fixed(82.0)),
        settings_text_input("", value).on_input(on_input).width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn option_card<'a>(
    i18n: desktop_i18n::I18n,
    title: &'a str,
    description: &'a str,
    icon: Icon,
    active: bool,
    enabled: bool,
    recommended: bool,
    message: Message,
) -> Element<'a, Message> {
    let mut title_row = row![text(title).size(13)].spacing(6).align_y(Alignment::Center);
    if recommended {
        title_row = title_row.push(settings_value_badge(i18n.__("推荐")));
    }
    let content = row![
        icon_badge(icon, active),
        column![title_row, text(description).size(11).style(settings_muted_text_style),]
            .spacing(3)
            .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    button(
        container(content)
            .padding([10, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme| option_card_style(theme, active, enabled)),
    )
    .padding(0)
    .width(Length::Fill)
    .style(button::text)
    .on_press_maybe(enabled.then_some(message))
    .into()
}
