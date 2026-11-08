//! 系统设置中 memory 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style, settings_checkbox_style,
    settings_divider, settings_error_banner, settings_field_label, settings_group_card,
    settings_group_heading, settings_muted_text_style, settings_page_heading, settings_select,
    settings_tab_button, settings_tab_row, settings_text_input,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_runtime::text as runtime_text;
use crate::app::message::settings::{MemoryMessage, SettingsMessage};
use crate::app::state::MemorySettingsTab;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Alignment, Element, Length};

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

pub(super) fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut labels = column![settings_field_label(label)].spacing(6);
    if !description.is_empty() {
        labels = labels.push(text(description).size(13).style(settings_muted_text_style));
    }
    aligned_field_row(labels, control)
}

fn aligned_field_row<'a>(
    labels: iced::widget::Column<'a, Message>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            labels.width(Length::FillPortion(2)),
            container(control.into()).align_right(Length::FillPortion(3)),
        ]
        .spacing(24)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
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
        checkbox(checked)
            .label(checkbox_label)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .on_toggle(on_toggle)
            .style(settings_checkbox_style),
    )
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
            container(
                NumberInput::new(
                    parse_bounded_number(&value, min, max),
                    min,
                    max,
                    step,
                    precision,
                    step.max(0.01),
                    on_change,
                )
                .settings_style()
            )
            .width(110),
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
    let locale = app.desktop_locale;
    let s = &app.memory_settings;

    let backend_pick = settings_select(
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
    .width(Length::Fill);

    let backend_row: Element<'_, Message> = field_row(
        runtime_text(locale, "存储后端"),
        runtime_text(locale, "选择记忆系统使用的持久化后端。"),
        backend_pick,
    );

    let general_section = column![
        settings_group_heading(
            runtime_text(locale, "基础行为"),
            runtime_text(locale, "控制持久化、清理、快照和自动恢复等核心记忆行为。")
        ),
        settings_group_card(
            column![
                backend_row,
                settings_divider(),
                bool_row(
                    runtime_text(locale, "自动保存"),
                    runtime_text(locale, "自动保存用户输入到记忆。"),
                    s.auto_save,
                    runtime_text(locale, "自动保存用户输入到记忆"),
                    |value| {
                        Message::Settings(SettingsMessage::Memory(MemoryMessage::AutoSaveToggled(
                            value,
                        )))
                    }
                ),
                settings_divider(),
                bool_row(
                    runtime_text(locale, "记忆卫生清理"),
                    runtime_text(locale, "定期清理低价值或陈旧记忆。"),
                    s.hygiene_enabled,
                    runtime_text(locale, "启用记忆卫生清理"),
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::HygieneEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    runtime_text(locale, "响应缓存"),
                    runtime_text(locale, "缓存部分响应结果以减少重复开销。"),
                    s.response_cache_enabled,
                    runtime_text(locale, "启用响应缓存"),
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::ResponseCacheEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    runtime_text(locale, "记忆快照"),
                    runtime_text(locale, "启用记忆快照导出。"),
                    s.snapshot_enabled,
                    runtime_text(locale, "启用记忆快照导出"),
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::SnapshotEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    runtime_text(locale, "清理时快照"),
                    runtime_text(locale, "执行卫生清理时同步生成快照。"),
                    s.snapshot_on_hygiene,
                    runtime_text(locale, "清理时同步生成快照"),
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::SnapshotOnHygieneToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    runtime_text(locale, "自动恢复"),
                    runtime_text(locale, "数据库缺失时自动从快照恢复。"),
                    s.auto_hydrate,
                    runtime_text(locale, "数据库缺失时自动从快照恢复"),
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
        settings_group_heading(
            runtime_text(locale, "保留与缓存"),
            runtime_text(locale, "配置归档、清理、SQLite 打开超时与缓存容量。")
        ),
        settings_group_card(
            column![
                number_row(
                    runtime_text(locale, "归档天数"),
                    s.archive_after_days.to_string(),
                    runtime_text(locale, "天后归档"),
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
                    runtime_text(locale, "清除天数"),
                    s.purge_after_days.to_string(),
                    runtime_text(locale, "天后清除"),
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
                    runtime_text(locale, "对话保留天数"),
                    s.conversation_retention_days.to_string(),
                    runtime_text(locale, "天"),
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
                    runtime_text(locale, "嵌入缓存大小"),
                    s.embedding_cache_size.to_string(),
                    runtime_text(locale, "条"),
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
                    runtime_text(locale, "分块最大 Token"),
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
                    runtime_text(locale, "缓存有效期"),
                    s.response_cache_ttl_minutes.to_string(),
                    runtime_text(locale, "分钟"),
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
                    runtime_text(locale, "缓存最大条目"),
                    s.response_cache_max_entries.to_string(),
                    runtime_text(locale, "条"),
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
                    runtime_text(locale, "SQLite 超时"),
                    s.sqlite_open_timeout_secs.to_string(),
                    runtime_text(locale, "秒（0 表示无限等待）"),
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
        settings_group_heading(
            runtime_text(locale, "嵌入与检索"),
            runtime_text(locale, "配置 embedding provider/model、维度和混合检索权重。")
        ),
        settings_group_card(
            column![
                text_row(
                    runtime_text(locale, "嵌入提供者"),
                    runtime_text(locale, "阿里建议在嵌入路由里配置 alibaba-cn、text-embedding-v4 和 DashScope API Key。"),
                    "none / openai / alibaba-cn / custom:URL",
                    &s.embedding_provider,
                    |value| {
                        Message::Settings(SettingsMessage::Memory(
                            MemoryMessage::EmbeddingProviderChanged(value),
                        ))
                    },
                ),
                settings_divider(),
                aligned_field_row(
                    column![
                        settings_field_label(runtime_text(locale, "嵌入模型")),
                        row![
                            text(runtime_text(locale, "模型名称，可用"))
                                .size(13)
                                .style(settings_muted_text_style),
                            button(text(runtime_text(locale, "hint:模式名")).size(13))
                                .on_press(Message::Settings(SettingsMessage::Memory(
                                    MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes),
                                )))
                                .padding([3, 6])
                                .style(rounded_action_btn_style),
                            text(runtime_text(locale, "引用路由。"))
                                .size(13)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .wrap(),
                    ]
                    .spacing(6),
                    settings_text_input("text-embedding-v4", &s.embedding_model)
                        .on_input(|value| {
                            Message::Settings(SettingsMessage::Memory(
                                MemoryMessage::EmbeddingModelChanged(value),
                            ))
                        })
                        .width(Length::Fill),
                ),
                settings_divider(),
                number_row(
                    runtime_text(locale, "嵌入维度"),
                    s.embedding_dimensions.to_string(),
                    runtime_text(locale, "维"),
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
                    runtime_text(locale, "向量权重"),
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
                    runtime_text(locale, "关键词权重"),
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
                    runtime_text(locale, "最小相关度"),
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
                settings_group_heading(
                    runtime_text(locale, "Qdrant 连接"),
                    runtime_text(
                        locale,
                        "仅在 backend=qdrant 时显示，用于配置向量数据库地址、集合和 API Key。"
                    ),
                ),
                settings_group_card(
                    column![
                        text_row(
                            runtime_text(locale, "Qdrant 地址"),
                            runtime_text(locale, "Qdrant 服务的 HTTP 地址。"),
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
                            runtime_text(locale, "Qdrant 集合"),
                            runtime_text(locale, "记忆向量集合名称。"),
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
                            runtime_text(locale, "Qdrant API 密钥"),
                            runtime_text(locale, "可选的鉴权密钥。"),
                            runtime_text(locale, "可选"),
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

    let qdrant_tabs = if s.backend == "qdrant" { QDRANT_TAB } else { &[] };
    let tabs = settings_tab_row(MEMORY_TABS.iter().chain(qdrant_tabs).map(|descriptor| {
        settings_tab_button(
            memory_tab_label(locale, descriptor.tab),
            descriptor.tab == s.active_tab,
            Some(Message::Settings(SettingsMessage::Memory(MemoryMessage::TabSelected(
                descriptor.tab,
            )))),
        )
        .into()
    }))
    .wrap();

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
        settings_page_heading(
            runtime_text(locale, "记忆系统配置"),
            runtime_text(locale, "统一配置后端、保留策略、缓存和向量检索参数。")
        ),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(app) {
        sections = sections.push(settings_error_banner(
            crate::app::i18n::settings_runtime::validation_error(locale, error),
        ));
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

pub(super) fn memory_tab_label(
    locale: vw_config_types::ui::DesktopLocale,
    tab: MemorySettingsTab,
) -> &'static str {
    runtime_text(
        locale,
        match tab {
            MemorySettingsTab::General => "基础行为",
            MemorySettingsTab::RetentionAndCache => "保留与缓存",
            MemorySettingsTab::Embedding => "嵌入与检索",
            MemorySettingsTab::EmbeddingRoutes => "嵌入路由",
            MemorySettingsTab::Storage => "记忆存储",
            MemorySettingsTab::Qdrant => "Qdrant 连接",
        },
    )
}

#[cfg(test)]
#[path = "system_settings_memory_ui_refresh_20_tests.rs"]
mod ui_refresh_20_tests;
