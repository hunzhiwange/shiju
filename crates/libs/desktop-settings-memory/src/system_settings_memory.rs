use desktop_ui_controls::system_settings_common::help::settings_selectable_help_label as settings_hover_help_label;
use crate::{fields::*, state::*, messages::{MemoryMessage, PageMessage}};
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_runtime::text as runtime_text};
use iced::{Element, Length};
use iced::widget::{button, column, text};
pub use crate::memory_tabs::{SettingsTabDescriptor, MEMORY_TABS, QDRANT_TAB, memory_tab_label};
pub fn view<'a, E: desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage + 'static>(s: &'a MemorySettingsState, storage: &'a StorageSettingsState, locale: config_ui::DesktopLocale, theme: &iced::Theme, embedding_routes: Element<'a, PageMessage<E>>) -> Element<'a, PageMessage<E>> {
    let i18n = I18n::new(locale);

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
        |value| PageMessage::Memory(MemoryMessage::BackendChanged(value)),
    )
    .width(Length::Fill);

    let backend_row = field_row_with_help(
        locale, theme,
        runtime_text(locale, "存储后端"),
        runtime_text(locale, "选择记忆系统使用的持久化后端。"),
        backend_pick,
    );

    let general_section = column![
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "基础行为"),
            runtime_text(locale, "控制持久化、清理、快照和自动恢复等核心记忆行为。")
        ),
        settings_group_card(
            column![
                backend_row,
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "自动保存"),
                    "",
                    s.auto_save,
                    runtime_text(locale, "自动保存用户输入到记忆"),
                    |value| {
                        PageMessage::Memory(MemoryMessage::AutoSaveToggled(
                            value,
                        ))
                    }
                ),
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "记忆卫生清理"),
                    runtime_text(locale, "定期清理低价值或陈旧记忆。"),
                    s.hygiene_enabled,
                    runtime_text(locale, "启用记忆卫生清理"),
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::HygieneEnabledToggled(value),
                        )
                    },
                ),
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "响应缓存"),
                    runtime_text(locale, "缓存部分响应结果以减少重复开销。"),
                    s.response_cache_enabled,
                    runtime_text(locale, "启用响应缓存"),
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ResponseCacheEnabledToggled(value),
                        )
                    },
                ),
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "记忆快照"),
                    "",
                    s.snapshot_enabled,
                    runtime_text(locale, "启用记忆快照导出"),
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::SnapshotEnabledToggled(value),
                        )
                    },
                ),
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "清理时快照"),
                    runtime_text(locale, "执行卫生清理时同步生成快照。"),
                    s.snapshot_on_hygiene,
                    runtime_text(locale, "清理时同步生成快照"),
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::SnapshotOnHygieneToggled(value),
                        )
                    },
                ),
                settings_divider(),
                bool_row(
                    locale, theme,
                    runtime_text(locale, "自动恢复"),
                    "",
                    s.auto_hydrate,
                    runtime_text(locale, "数据库缺失时自动从快照恢复"),
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::AutoHydrateToggled(value),
                        )
                    }
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let retention_section = column![
        settings_group_heading_with_help(
            i18n,
            theme,
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ArchiveAfterDaysChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::PurgeAfterDaysChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ConversationRetentionDaysChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::EmbeddingCacheSizeChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ChunkMaxTokensChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ResponseCacheTtlMinutesChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::ResponseCacheMaxEntriesChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::SqliteOpenTimeoutSecsChanged(value.round() as u32),
                        )
                    },
                ),
            ]
            .spacing(0)
        ),
    ]
    .spacing(16);

    let embedding_section = column![
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "嵌入与检索"),
            runtime_text(locale, "配置 embedding provider/model、维度和混合检索权重。")
        ),
        settings_group_card(
            column![
                text_row_with_help(
                    locale, theme,
                    runtime_text(locale, "嵌入提供者"),
                    runtime_text(locale, "阿里建议在嵌入路由里配置 alibaba-cn、text-embedding-v4 和 DashScope API Key。"),
                    "none / openai / alibaba-cn / custom:URL",
                    &s.embedding_provider,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::EmbeddingProviderChanged(value),
                        )
                    },
                ),
                settings_divider(),
                aligned_field_row(
                    column![
                        settings_hover_help_label(
                            i18n,
                            theme,
                            runtime_text(locale, "嵌入模型"),
                            settings_field_label(runtime_text(locale, "嵌入模型")),
                            runtime_text(locale, "填写模型名称，或用 hint:模式名 引用嵌入路由。"),
                        )
                        .wrap(),
                        button(text(runtime_text(locale, "hint:模式名")).size(13))
                            .on_press(PageMessage::Memory(
                                MemoryMessage::TabSelected(MemorySettingsTab::EmbeddingRoutes),
                            ))
                            .padding([3, 6])
                            .style(rounded_action_btn_style),
                    ]
                    .spacing(6),
                    settings_text_input("text-embedding-v4", &s.embedding_model)
                        .on_input(|value| {
                            PageMessage::Memory(
                                MemoryMessage::EmbeddingModelChanged(value),
                            )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::EmbeddingDimensionsChanged(value.round() as u32),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::VectorWeightChanged(value),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::KeywordWeightChanged(value),
                        )
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
                    PageMessage::None,
                    |value| {
                        PageMessage::Memory(
                            MemoryMessage::MinRelevanceScoreChanged(value),
                        )
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
                settings_group_heading_with_help(
                    i18n,
                    theme,
                    runtime_text(locale, "Qdrant 连接"),
                    runtime_text(
                        locale,
                        "仅在 backend=qdrant 时显示，用于配置向量数据库地址、集合和 API Key。"
                    ),
                ),
                settings_group_card(
                    column![
                        text_row_with_help(
                            locale, theme,
                            runtime_text(locale, "Qdrant 地址"),
                            runtime_text(locale, "Qdrant 服务的 HTTP 地址。"),
                            "http://localhost:6333",
                            &s.qdrant_url_input,
                            |value| {
                                PageMessage::Memory(
                                    MemoryMessage::QdrantUrlChanged(value),
                                )
                            }
                        ),
                        settings_divider(),
                        text_row_with_help(
                            locale, theme,
                            runtime_text(locale, "Qdrant 集合"),
                            runtime_text(locale, "记忆向量集合名称。"),
                            "vibewindow_memories",
                            &s.qdrant_collection,
                            |value| {
                                PageMessage::Memory(
                                    MemoryMessage::QdrantCollectionChanged(value),
                                )
                            },
                        ),
                        settings_divider(),
                        text_row_with_help(
                            locale, theme,
                            runtime_text(locale, "Qdrant API 密钥"),
                            runtime_text(locale, "可选的鉴权密钥。"),
                            runtime_text(locale, "可选"),
                            &s.qdrant_api_key_input,
                            |value| {
                                PageMessage::Memory(
                                    MemoryMessage::QdrantApiKeyChanged(value),
                                )
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
            Some(PageMessage::Memory(MemoryMessage::TabSelected(
                descriptor.tab,
            ))),
        )
        .into()
    }))
    .wrap();

    let selected_section: Element<'_, PageMessage<E>> = match s.active_tab {
        MemorySettingsTab::General => general_section.into(),
        MemorySettingsTab::RetentionAndCache => retention_section.into(),
        MemorySettingsTab::Embedding => embedding_section.into(),
        MemorySettingsTab::EmbeddingRoutes => {
            embedding_routes
        }
        MemorySettingsTab::Storage => {
            crate::system_settings_storage::embedded_view(storage, locale, theme).map(PageMessage::Storage)
        }
        MemorySettingsTab::Qdrant => {
            qdrant_section.map(Into::into).unwrap_or_else(|| general_section.into())
        }
    };

    let mut sections = column![
        settings_page_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "记忆系统配置"),
            runtime_text(locale, "统一配置后端、保留策略、缓存和向量检索参数。")
        ),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(s, storage) {
        sections = sections.push(settings_error_banner(
            desktop_i18n::settings_runtime::validation_error(locale, error),
        ));
    }

    sections.push(selected_section).into()
}

pub fn active_save_error<'a>(s: &'a MemorySettingsState, storage: &'a StorageSettingsState) -> Option<&'a str> {
    match s.active_tab { MemorySettingsTab::Storage => storage.save_error.as_deref(), _ => s.save_error.as_deref() }
}
