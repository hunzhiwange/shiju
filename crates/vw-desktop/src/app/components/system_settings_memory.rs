//! 系统设置中 memory 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH,
    settings_checkbox_style, settings_divider, settings_error_banner, settings_muted_text_style,
    settings_page_intro, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_section_content, settings_segment_button_style,
    settings_text_input_style,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::message::settings::{MemoryMessage, SettingsMessage};
use crate::app::state::MemorySettingsTab;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Border, Element, Length};

pub(super) const MEMORY_TABS: &[SettingsTabDescriptor<MemorySettingsTab>] = &[
    SettingsTabDescriptor::new(
        MemorySettingsTab::General,
        "基础行为",
        &["backend", "auto save", "自动保存", "清理"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::RetentionAndCache,
        "保留与缓存",
        &["保留", "缓存", "归档", "快照", "自动恢复", "SQLite 打开超时"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::Embedding,
        "嵌入与检索",
        &["embedding", "嵌入模型", "维度", "混合检索", "权重"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::EmbeddingRoutes,
        "嵌入路由",
        &["embedding routes", "路由规则", "匹配模式", "提供商", "API Key"],
    ),
    SettingsTabDescriptor::new(
        MemorySettingsTab::Storage,
        "记忆存储",
        &["storage", "database", "存储后端", "连接串", "Schema", "表名", "TLS", "持久化"],
    ),
];
pub(super) const QDRANT_TAB: &[SettingsTabDescriptor<MemorySettingsTab>] =
    &[SettingsTabDescriptor::new(
        MemorySettingsTab::Qdrant,
        "Qdrant 连接",
        &["qdrant", "向量数据库", "collection", "API Key"],
    )];

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

fn tab_button<'a>(
    label: &'static str,
    tab: MemorySettingsTab,
    active_tab: MemorySettingsTab,
) -> iced::widget::Button<'a, Message> {
    button(text(label))
        .on_press(Message::Settings(SettingsMessage::Memory(MemoryMessage::TabSelected(tab))))
        .padding([8, 14])
        .style(move |theme: &iced::Theme, status| {
            settings_segment_button_style(theme, status, tab == active_tab)
        })
}

fn parse_bounded_number(value: &str, min: f32, max: f32) -> f32 {
    value.parse::<f32>().unwrap_or(min).clamp(min, max)
}

fn number_row<'a>(
    label: &'static str,
    value: String,
    suffix: &'static str,
    min: f32,
    max: f32,
    step: f32,
    precision: u8,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        "",
        row![
            NumberInput::new(
                parse_bounded_number(&value, min, max),
                min,
                max,
                step,
                precision,
                step.max(0.01),
                on_change,
            )
            .settings_style(),
            text(suffix).size(13).style(settings_muted_text_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.memory_settings;

    let backend_pick = pick_list(
        [
            "sqlite".to_string(),
            "postgres".to_string(),
            "qdrant".to_string(),
            "chroma".to_string(),
            "markdown".to_string(),
            "null".to_string(),
        ],
        Some(s.backend.clone()),
        |value| Message::Settings(SettingsMessage::Memory(MemoryMessage::BackendChanged(value))),
    )
    .padding(SETTINGS_CONTROL_PADDING)
    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let backend_row: Element<'_, Message> =
        field_row("存储后端", "选择记忆系统使用的持久化后端。", backend_pick);

    let general_section = column![
        settings_section_card("基础行为", "控制持久化、清理、快照和自动恢复等核心记忆行为。"),
        settings_section_content(
            column![
                backend_row,
                settings_divider(),
                bool_row(
                    "自动保存",
                    "自动保存用户输入到记忆。",
                    s.auto_save,
                    "自动保存用户输入到记忆",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(MemoryMessage::AutoSaveToggled(
                            value,
                        )))
                    }
                ),
                settings_divider(),
                bool_row(
                    "记忆卫生清理",
                    "定期清理低价值或陈旧记忆。",
                    s.hygiene_enabled,
                    "启用记忆卫生清理",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::HygieneEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    "响应缓存",
                    "缓存部分响应结果以减少重复开销。",
                    s.response_cache_enabled,
                    "启用响应缓存",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ResponseCacheEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    "记忆快照",
                    "启用记忆快照导出。",
                    s.snapshot_enabled,
                    "启用记忆快照导出",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::SnapshotEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    "清理时快照",
                    "执行卫生清理时同步生成快照。",
                    s.snapshot_on_hygiene,
                    "清理时同步生成快照",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::SnapshotOnHygieneToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    "自动恢复",
                    "数据库缺失时自动从快照恢复。",
                    s.auto_hydrate,
                    "数据库缺失时自动从快照恢复",
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::AutoHydrateToggled(value),
                        ))
                    }
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let retention_section = column![
        settings_section_card("保留与缓存", "配置归档、清理、SQLite 打开超时与缓存容量。"),
        settings_section_content(
            column![
                number_row(
                    "归档天数",
                    s.archive_after_days.to_string(),
                    "天后归档",
                    0.0,
                    3650.0,
                    1.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ArchiveAfterDaysChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "清除天数",
                    s.purge_after_days.to_string(),
                    "天后清除",
                    0.0,
                    3650.0,
                    1.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::PurgeAfterDaysChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "对话保留天数",
                    s.conversation_retention_days.to_string(),
                    "天",
                    0.0,
                    3650.0,
                    1.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ConversationRetentionDaysChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "嵌入缓存大小",
                    s.embedding_cache_size.to_string(),
                    "条",
                    0.0,
                    1_000_000.0,
                    100.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::EmbeddingCacheSizeChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "分块最大 Token",
                    s.chunk_max_tokens.to_string(),
                    "token",
                    1.0,
                    32_768.0,
                    16.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ChunkMaxTokensChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "缓存有效期",
                    s.response_cache_ttl_minutes.to_string(),
                    "分钟",
                    0.0,
                    525_600.0,
                    5.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ResponseCacheTtlMinutesChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "缓存最大条目",
                    s.response_cache_max_entries.to_string(),
                    "条",
                    0.0,
                    1_000_000.0,
                    100.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ResponseCacheMaxEntriesChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "SQLite 超时",
                    s.sqlite_open_timeout_secs.to_string(),
                    "秒（0 表示无限等待）",
                    0.0,
                    3600.0,
                    1.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::SqliteOpenTimeoutSecsChanged(value.round() as u32),
                        ))
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let embedding_section = column![
        settings_section_card("嵌入与检索", "配置 embedding provider/model、维度和混合检索权重。"),
        settings_section_content(
            column![
                text_row(
                    "嵌入提供者",
                    "阿里建议在嵌入路由里配置 alibaba-cn、text-embedding-v4 和 DashScope API Key。",
                    "none / openai / alibaba-cn / custom:URL",
                    &s.embedding_provider,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::EmbeddingProviderChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                container(
                    row![
                        column![
                            text("嵌入模型").size(13),
                            row![
                                text("模型名称，可用").size(11).style(settings_muted_text_style),
                                button(text("hint:模式名").size(11))
                                    .on_press(Message::Settings(SettingsMessage::Memory(
                                        MemoryMessage::TabSelected(
                                            MemorySettingsTab::EmbeddingRoutes
                                        ),
                                    )))
                                    .padding([1, 6])
                                    .style(|theme: &iced::Theme, _status| {
                                        let palette = theme.extended_palette();
                                        iced::widget::button::Style {
                                            background: None,
                                            text_color: palette.primary.base.color,
                                            border: Border {
                                                width: 0.0,
                                                color: iced::Color::TRANSPARENT,
                                                radius: 4.0.into(),
                                            },
                                            ..Default::default()
                                        }
                                    }),
                                text("引用路由。").size(11).style(settings_muted_text_style),
                            ]
                            .spacing(0)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(4)
                        .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
                        container(
                            text_input("text-embedding-v4", &s.embedding_model)
                                .on_input(|value| {
                                    Message::Settings(SettingsMessage::Memory(
                                        MemoryMessage::EmbeddingModelChanged(value),
                                    ))
                                })
                                .padding(SETTINGS_CONTROL_PADDING)
                                .size(SETTINGS_CONTROL_TEXT_SIZE)
                                .style(settings_text_input_style)
                                .width(Length::Fill)
                        )
                        .width(Length::Fill),
                    ]
                    .spacing(22)
                    .align_y(Alignment::Center),
                )
                .padding([14, 0])
                .width(Length::Fill),
                settings_divider(),
                number_row(
                    "嵌入维度",
                    s.embedding_dimensions.to_string(),
                    "维",
                    1.0,
                    65_536.0,
                    1.0,
                    0,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::EmbeddingDimensionsChanged(value.round() as u32),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "向量权重",
                    format!("{:.2}", s.vector_weight),
                    "0.0 - 1.0",
                    0.0,
                    1.0,
                    0.01,
                    2,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::VectorWeightChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "关键词权重",
                    format!("{:.2}", s.keyword_weight),
                    "0.0 - 1.0",
                    0.0,
                    1.0,
                    0.01,
                    2,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::KeywordWeightChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                number_row(
                    "最小相关度",
                    format!("{:.2}", s.min_relevance_score),
                    "0.0 - 1.0",
                    0.0,
                    1.0,
                    0.01,
                    2,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::MinRelevanceScoreChanged(value),
                        ))
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let qdrant_section = if s.backend == "qdrant" {
        Some(
            column![
                settings_section_card(
                    "Qdrant 连接",
                    "仅在 backend=qdrant 时显示，用于配置向量数据库地址、集合和 API Key。",
                ),
                settings_section_content(
                    column![
                        text_row(
                            "Qdrant 地址",
                            "Qdrant 服务的 HTTP 地址。",
                            "http://localhost:6333",
                            &s.qdrant_url_input,
                            |value| {
                                Message::Settings(SettingsMessage::Memory(
                                    MemoryMessage::QdrantUrlChanged(value),
                                ))
                            }
                        ),
                        settings_divider(),
                        text_row(
                            "Qdrant 集合",
                            "记忆向量集合名称。",
                            "vibewindow_memories",
                            &s.qdrant_collection,
                            |value| {
                                Message::Settings(SettingsMessage::Memory(
                                    MemoryMessage::QdrantCollectionChanged(value),
                                ))
                            },
                        ),
                        settings_divider(),
                        text_row(
                            "Qdrant API 密钥",
                            "可选的鉴权密钥。",
                            "可选",
                            &s.qdrant_api_key_input,
                            |value| {
                                Message::Settings(SettingsMessage::Memory(
                                    MemoryMessage::QdrantApiKeyChanged(value),
                                ))
                            }
                        ),
                    ]
                    .spacing(0)
                ),
            ]
            .spacing(16),
        )
    } else {
        None
    };

    let mut tabs =
        MEMORY_TABS.iter().fold(row![].spacing(8).align_y(Alignment::Center), |row, descriptor| {
            row.push(tab_button(descriptor.label, descriptor.tab, s.active_tab))
        });

    if s.backend == "qdrant" {
        let descriptor = QDRANT_TAB[0];
        tabs = tabs.push(tab_button(descriptor.label, descriptor.tab, s.active_tab));
    }

    let selected_section: Element<'_, Message> = match s.active_tab {
        MemorySettingsTab::General => general_section.into(),
        MemorySettingsTab::RetentionAndCache => retention_section.into(),
        MemorySettingsTab::Embedding => embedding_section.into(),
        MemorySettingsTab::EmbeddingRoutes => {
            crate::app::components::system_settings_embedding_routes::embedded_view(app)
        }
        MemorySettingsTab::Storage => {
            crate::app::components::system_settings_storage::embedded_view(app)
        }
        MemorySettingsTab::Qdrant => {
            qdrant_section.map(Into::into).unwrap_or_else(|| general_section.into())
        }
    };

    let mut sections = column![
        settings_page_intro("记忆系统配置", "统一配置后端、保留策略、缓存和向量检索参数。"),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(app) {
        sections = sections.push(settings_error_banner(error));
    }

    sections.push(selected_section).into()
}

fn active_save_error(app: &App) -> Option<&str> {
    match app.memory_settings.active_tab {
        MemorySettingsTab::Storage => app.storage_settings.save_error.as_deref(),
        _ => app.memory_settings.save_error.as_deref(),
    }
}
#[cfg(test)]
#[path = "system_settings_memory_tests.rs"]
mod system_settings_memory_tests;
