use super::*;

pub(super) fn indexing_label(i18n: desktop_i18n::I18n, mode: &KnowledgeIndexingMode) -> &'static str {
    match mode {
        KnowledgeIndexingMode::Economy => i18n.__("经济"),
        KnowledgeIndexingMode::HighQuality => i18n.__("高质量"),
    }
}

pub(super) fn chunking_label(mode: &KnowledgeChunkingMode) -> &'static str {
    match mode {
        KnowledgeChunkingMode::General => "General",
        KnowledgeChunkingMode::ParentChild => "Parent-Child",
        KnowledgeChunkingMode::Qa => "Q&A",
    }
}

pub(super) fn retrieval_label(i18n: desktop_i18n::I18n, mode: &KnowledgeRetrievalMode) -> &'static str {
    match mode {
        KnowledgeRetrievalMode::FullText => i18n.__("全文检索"),
        KnowledgeRetrievalMode::Vector => i18n.__("向量检索"),
        KnowledgeRetrievalMode::Hybrid => i18n.__("混合检索"),
    }
}

pub(super) fn document_status_label(
    i18n: desktop_i18n::I18n,
    status: &KnowledgeDocumentStatus,
) -> &'static str {
    match status {
        KnowledgeDocumentStatus::Pending => i18n.__("入队"),
        KnowledgeDocumentStatus::Processing => i18n.__("索引中"),
        KnowledgeDocumentStatus::Completed => i18n.__("成功"),
        KnowledgeDocumentStatus::Failed => i18n.__("失败"),
        KnowledgeDocumentStatus::Disabled => i18n.__("停用"),
        KnowledgeDocumentStatus::Archived => i18n.__("归档"),
    }
}

pub(super) fn effective_document_status(document: &KnowledgeDocumentDto) -> KnowledgeDocumentStatus {
    if !document.enabled && document.status != KnowledgeDocumentStatus::Archived {
        KnowledgeDocumentStatus::Disabled
    } else {
        document.status.clone()
    }
}

pub(super) fn compact_document_count_label(value: u64) -> String {
    compact_count_label(value)
}

pub(super) fn compact_count_label(value: u64) -> String {
    const THOUSAND: u64 = 1_000;
    const MILLION: u64 = 1_000_000;
    if value < THOUSAND {
        return value.to_string();
    }
    if value < MILLION {
        return compact_decimal(value, THOUSAND, "K");
    }
    compact_decimal(value, MILLION, "M")
}

pub(super) fn compact_decimal(value: u64, base: u64, suffix: &str) -> String {
    let whole = value / base;
    let decimal = ((value % base) * 10) / base;
    if decimal == 0 { format!("{whole}{suffix}") } else { format!("{whole}.{decimal}{suffix}") }
}

pub(super) fn import_job_status_label(
    i18n: desktop_i18n::I18n,
    status: &vw_api_knowledge::KnowledgeIndexJobStatus,
) -> &'static str {
    match status {
        vw_api_knowledge::KnowledgeIndexJobStatus::Queued => i18n.__("入队"),
        vw_api_knowledge::KnowledgeIndexJobStatus::Processing => i18n.__("索引中"),
        vw_api_knowledge::KnowledgeIndexJobStatus::Completed => i18n.__("成功"),
        vw_api_knowledge::KnowledgeIndexJobStatus::Failed => i18n.__("失败"),
    }
}

pub(super) fn chunk_type_label(i18n: desktop_i18n::I18n, chunk_type: &KnowledgeChunkType) -> &'static str {
    match chunk_type {
        KnowledgeChunkType::Text => i18n.__("文本"),
        KnowledgeChunkType::ParentText => i18n.__("父块"),
        KnowledgeChunkType::Qa => "Q&A",
    }
}

pub(super) fn match_type_label(i18n: desktop_i18n::I18n, match_type: &KnowledgeMatchType) -> &'static str {
    match match_type {
        KnowledgeMatchType::FullText => i18n.__("全文"),
        KnowledgeMatchType::Vector => i18n.__("向量"),
        KnowledgeMatchType::Hybrid => i18n.__("混合"),
        KnowledgeMatchType::Parent => i18n.__("父块"),
        KnowledgeMatchType::Nearby => i18n.__("邻近"),
    }
}

pub(super) fn support_label(i18n: desktop_i18n::I18n, value: bool) -> &'static str {
    if value { i18n.__("可用") } else { i18n.__("不可用") }
}

pub(super) fn enabled_label(i18n: desktop_i18n::I18n, value: bool) -> &'static str {
    if value { i18n.__("启用") } else { i18n.__("关闭") }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DatasetEmbeddingOption {
    pub(super) selection: KnowledgeDatasetEmbeddingSelection,
    pub(super) label: String,
}

impl fmt::Display for DatasetEmbeddingOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub(super) fn build_embedding_model_picker(app: ViewInput<'_>) -> Element<'_, Message> {
    let options = dataset_embedding_options(app);
    let selected = options
        .iter()
        .find(|option| option.selection == app.knowledge.dataset_embedding_selection)
        .cloned()
        .or_else(|| options.first().cloned());

    settings_select(options, selected, |option| {
        Message::Knowledge(KnowledgeToolMessage::DatasetEmbeddingSelectionChanged(option.selection))
    })
    .width(Length::Fill)
    .into()
}

pub(super) fn dataset_embedding_options(app: ViewInput<'_>) -> Vec<DatasetEmbeddingOption> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let mut options = vec![DatasetEmbeddingOption {
        selection: KnowledgeDatasetEmbeddingSelection::FollowGlobal,
        label: desktop_i18n::I18n::new(app.desktop_locale)
            .__("跟随全局配置（推荐）")
            .to_string(),
    }];
    let Some(status) = app.knowledge.embedding_status.as_ref() else {
        return options;
    };
    let model = status.model.trim();
    if model.is_empty() || !status.vector_available {
        return options;
    }
    let current_label = app
        .knowledge
        .embedding_presets
        .iter()
        .find(|preset| preset.provider_id == status.provider_id && preset.model == status.model)
        .map(|preset| {
            embedding_preset_label(desktop_i18n::I18n::new(app.desktop_locale), preset)
        })
        .unwrap_or_else(|| {
            desktop_i18n::I18n::new(app.desktop_locale)
                .format(
                    "%s · %s / %s维",
                    &[
                        (embedding_status_provider_label(i18n, Some(status))).to_string().into(),
                        (model).to_string().into(),
                        (status.dimensions).to_string().into(),
                    ],
                )
                .expect("valid display template")
        });
    options.push(DatasetEmbeddingOption {
        selection: KnowledgeDatasetEmbeddingSelection::LockCurrent,
        label: desktop_i18n::I18n::new(app.desktop_locale)
            .format("锁定当前 · %s", &[(current_label).to_string().into()])
            .expect("valid display template"),
    });
    options
}

pub(super) fn embedding_vector_available(app: ViewInput<'_>) -> bool {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| status.vector_available)
        .or_else(|| app.knowledge.runtime_status.as_ref().map(|status| status.vector))
        .unwrap_or(false)
}

pub(super) fn embedding_hybrid_available(app: ViewInput<'_>) -> bool {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| status.hybrid_available)
        .or_else(|| app.knowledge.runtime_status.as_ref().map(|status| status.hybrid))
        .unwrap_or(false)
}

pub(super) fn embedding_status_provider_label(
    i18n: desktop_i18n::I18n,
    status: Option<&KnowledgeEmbeddingStatusDto>,
) -> String {
    status
        .map(|status| status.provider_id.trim())
        .filter(|provider| !provider.is_empty())
        .unwrap_or(i18n.__("检测中"))
        .to_string()
}

pub(super) fn embedding_status_model_label(
    i18n: desktop_i18n::I18n,
    status: Option<&KnowledgeEmbeddingStatusDto>,
) -> String {
    status
        .map(|status| status.model.trim())
        .filter(|model| !model.is_empty())
        .unwrap_or(i18n.__("检测中"))
        .to_string()
}

pub(super) fn embedding_status_dimensions_label(
    i18n: desktop_i18n::I18n,
    status: Option<&KnowledgeEmbeddingStatusDto>,
) -> String {
    status
        .map(|status| {
            i18n.format("%s 维", &[(status.dimensions).to_string().into()])
                .expect("valid display template")
        })
        .unwrap_or_else(|| i18n.__("检测中").to_string())
}

pub(super) fn embedding_status_route_label(
    i18n: desktop_i18n::I18n,
    status: Option<&KnowledgeEmbeddingStatusDto>,
) -> String {
    status
        .and_then(|status| status.route_hint.as_deref())
        .map(str::trim)
        .filter(|hint| !hint.is_empty())
        .unwrap_or(i18n.__("默认"))
        .to_string()
}

pub(super) fn embedding_preset_label(
    i18n: desktop_i18n::I18n,
    preset: &KnowledgeEmbeddingPresetDto,
) -> String {
    let provider = preset.provider_id.to_ascii_lowercase();
    let label = preset.label.to_ascii_lowercase();
    let family = if preset.id.starts_with("alibaba")
        || provider.contains("alibaba")
        || label.contains("dashscope")
    {
        i18n.__("阿里百炼/DashScope")
    } else if preset.id.contains("doubao")
        || preset.id.contains("volcengine")
        || provider.contains("volces")
        || label.contains("doubao")
        || label.contains("volcengine")
    {
        i18n.__("豆包/火山方舟")
    } else {
        preset.label.trim()
    };
    i18n.format(
        "%s · %s / %s维",
        &[
            (family).to_string().into(),
            (preset.model.trim()).to_string().into(),
            (preset.dimensions).to_string().into(),
        ],
    )
    .expect("valid display template")
}

pub(super) fn embedding_test_result_label(
    i18n: desktop_i18n::I18n,
    result: Option<&KnowledgeEmbeddingTestResponse>,
) -> String {
    let Some(result) = result else {
        return i18n.__("尚未测试").to_string();
    };
    let status = if result.ok { i18n.__("通过") } else { i18n.__("失败") };
    let dimensions = result
        .observed_dimensions
        .map(|value| {
            i18n.format("%s 维", &[(value).to_string().into()]).expect("valid display template")
        })
        .unwrap_or_else(|| {
            i18n.format("配置 %s 维", &[(result.configured_dimensions).to_string().into()])
                .expect("valid display template")
        });
    let latency = result
        .latency_ms
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| i18n.__("耗时未知").to_string());
    if result.ok {
        format!("{status} · {dimensions} · {latency}")
    } else {
        format!("{status} · {} · {latency}", result.message)
    }
}

pub(super) fn dataset_embedding_model_label(app: ViewInput<'_>, _dataset: &KnowledgeDatasetDto) -> String {
    app.knowledge
        .embedding_status
        .as_ref()
        .map(|status| {
            desktop_i18n::I18n::new(app.desktop_locale)
                .format(
                    "%s / %s维",
                    &[(status.model).to_string().into(), (status.dimensions).to_string().into()],
                )
                .expect("valid display template")
        })
        .unwrap_or_else(|| {
            desktop_i18n::I18n::new(app.desktop_locale).__("跟随记忆配置/嵌入路由").to_string()
        })
}

pub(super) fn score_threshold_label(i18n: desktop_i18n::I18n, dataset: &KnowledgeDatasetDto) -> String {
    if dataset.score_threshold_enabled {
        format!("{:.2}", dataset.score_threshold)
    } else {
        i18n.__("关闭").to_string()
    }
}

pub(super) fn format_time(ms: u64) -> String {
    use chrono::{Local, TimeZone};

    if ms == 0 {
        return "-".to_string();
    }
    let Ok(timestamp_ms) = i64::try_from(ms) else {
        return "-".to_string();
    };
    let Some(dt) = Local.timestamp_millis_opt(timestamp_ms).single() else {
        return "-".to_string();
    };
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(super) fn document_row_style(theme: &Theme, selected: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(
            if selected {
                theme.palette().primary.scale_alpha(0.10)
            } else if is_dark_palette(theme) {
                palette.background.base.color.scale_alpha(0.26)
            } else {
                Color::WHITE.scale_alpha(0.68)
            }
            .into(),
        ),
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

pub(super) fn document_title_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: if hovered || selected {
            theme.palette().primary
        } else {
            theme.palette().text
        },
        border: Border { radius: 6.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

pub(super) fn document_status_badge_style(
    theme: &Theme,
    document: &KnowledgeDocumentDto,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let (background, border_color, text_color) = if !document.enabled {
        (
            palette.background.strong.color.scale_alpha(0.18),
            palette.background.strong.color.scale_alpha(0.34),
            theme.palette().text.scale_alpha(0.72),
        )
    } else {
        match document.status {
            KnowledgeDocumentStatus::Completed => (
                Color::from_rgb8(34, 197, 94).scale_alpha(0.16),
                Color::from_rgb8(34, 197, 94).scale_alpha(0.38),
                Color::from_rgb8(22, 163, 74),
            ),
            KnowledgeDocumentStatus::Processing => (
                Color::from_rgb8(59, 130, 246).scale_alpha(0.16),
                Color::from_rgb8(59, 130, 246).scale_alpha(0.34),
                Color::from_rgb8(37, 99, 235),
            ),
            KnowledgeDocumentStatus::Pending => (
                Color::from_rgb8(245, 158, 11).scale_alpha(0.18),
                Color::from_rgb8(245, 158, 11).scale_alpha(0.36),
                Color::from_rgb8(217, 119, 6),
            ),
            KnowledgeDocumentStatus::Failed => (
                Color::from_rgb8(239, 68, 68).scale_alpha(0.16),
                Color::from_rgb8(239, 68, 68).scale_alpha(0.34),
                Color::from_rgb8(220, 38, 38),
            ),
            KnowledgeDocumentStatus::Disabled => (
                palette.background.strong.color.scale_alpha(0.18),
                palette.background.strong.color.scale_alpha(0.34),
                theme.palette().text.scale_alpha(0.72),
            ),
            KnowledgeDocumentStatus::Archived => (
                palette.background.strong.color.scale_alpha(0.18),
                palette.background.strong.color.scale_alpha(0.34),
                theme.palette().text.scale_alpha(0.72),
            ),
        }
    };
    iced::widget::container::Style {
        background: Some(background.into()),
        border: Border { radius: 999.0.into(), width: 1.0, color: border_color },
        text_color: Some(text_color),
        ..Default::default()
    }
}

pub(super) fn knowledge_tooltip_content<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        text(label)
            .size(12)
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
    )
    .padding([6, 8])
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Color::from_rgba8(15, 23, 42, 0.94).into()),
        text_color: Some(Color::WHITE),
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    })
    .into()
}
