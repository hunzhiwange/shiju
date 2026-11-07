//! 系统设置 - 模型配置视图模块
//!
//! 本模块提供模型设置界面的 UI 组件，包括：
//! - 已连接提供商的模型列表展示
//! - 模型搜索与筛选功能
//! - 模型启用/禁用切换
//! - 模型详情弹窗展示
//!
//! ## 主要功能
//!
//! - `main_view`: 渲染模型设置的主视图
//! - `view_overlays`: 渲染模型详情弹窗等覆盖层
//!
//! ## UI 结构
//!
//! 主视图包含：
//! - 顶部标题栏（带刷新按钮）
//! - 搜索输入框
//! - 按提供商分组的模型卡片列表
//!
//! 每个模型项显示：
//! - 模型名称和 ID
//! - 工具调用、附件支持、上下文限制等元信息
//! - 查看详情按钮
//! - 启用/禁用切换开关

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, format_context_limit, icon_btn,
    provider_logo_svg, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_error_banner, settings_group_card, settings_group_heading, settings_modal_card_style,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_page_heading, settings_tab_button, settings_tab_row,
    settings_text_input_style, settings_toggle, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_models::{self, text as model_text};
use crate::app::message::settings::{ModelConfigTabMessage, SettingsMessage};
use crate::app::state::ModelConfigTab;
use crate::app::{App, Message, message};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Background, Element, Length};

pub(super) const MODEL_TABS: &[SettingsTabDescriptor<ModelConfigTab>] = &[
    SettingsTabDescriptor::new(
        ModelConfigTab::Models,
        "模型",
        &[
            "models",
            "搜尋模型",
            "搜索模型",
            "模型详情",
            "提供商",
            "模型 id",
            "上下文窗口",
            "最大输出",
            "温度",
        ],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::ModelRoutes,
        "模型路由",
        &["model routes", "模型路由", "routing", "匹配模式", "pattern", "priority", "优先级"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::QueryClassification,
        "模型分类",
        &["model classification", "模型分類", "classification", "rules", "查询分类", "规则"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::Multimodal,
        "多模态",
        &[
            "multimodal",
            "image",
            "vision",
            "图片模型",
            "OCR",
            "文字提取",
            "远程抓取",
            "remote fetch",
        ],
    ),
];

pub(super) fn active_tab_save_error(app: &App) -> Option<&str> {
    match app.model_settings.active_tab {
        ModelConfigTab::Models => app.model_settings.save_error.as_deref(),
        ModelConfigTab::ModelRoutes => app.model_routes_settings.save_error.as_deref(),
        ModelConfigTab::QueryClassification => {
            app.query_classification_settings.save_error.as_deref()
        }
        ModelConfigTab::Multimodal => app.multimodal_settings.save_error.as_deref(),
    }
}

/// 渲染模型配置页及其内部标签。
pub fn view(app: &App) -> Element<'_, Message> {
    let active_tab = app.model_settings.active_tab;
    let tabs = settings_tab_row(MODEL_TABS.iter().map(|descriptor| {
        settings_tab_button(
            model_text(app.desktop_locale, descriptor.label),
            descriptor.tab == active_tab,
            Some(Message::Settings(SettingsMessage::ModelConfigTab(
                ModelConfigTabMessage::Selected(descriptor.tab),
            ))),
        )
        .into()
    }))
    .wrap();
    let content = match active_tab {
        ModelConfigTab::Models => main_view(app),
        ModelConfigTab::ModelRoutes => {
            crate::app::components::system_settings_model_routes::view(app)
        }
        ModelConfigTab::QueryClassification => {
            crate::app::components::system_settings_query_classification::view(app)
        }
        ModelConfigTab::Multimodal => crate::app::components::system_settings_multimodal::view(app),
    };

    let mut sections = column![tabs].spacing(16).width(Length::Fill);
    if let Some(error) = active_tab_save_error(app) {
        let message = match app.model_settings.active_tab {
            ModelConfigTab::Models => {
                app.model_settings.save_error.as_ref().map(|e| e.display(app.desktop_locale))
            }
            ModelConfigTab::ModelRoutes => {
                app.model_routes_settings.save_error.as_ref().map(|e| e.display(app.desktop_locale))
            }
            ModelConfigTab::QueryClassification => Some(
                app.query_classification_settings
                    .save_error_translation
                    .as_ref()
                    .map(|message| {
                        message
                            .render(crate::app::i18n::I18n::new(app.desktop_locale))
                            .expect("valid classification error template")
                    })
                    .unwrap_or_else(|| error.to_string()),
            ),
            _ => Some(error.to_string()),
        };
        if let Some(message) = message {
            sections = sections.push(settings_error_banner(message));
        }
    }

    sections.push(content).into()
}

/// 渲染模型设置的主视图
///
/// 该函数构建模型设置页面的主要 UI 布局，包括：
/// - 带刷新按钮的标题栏
/// - 模型搜索输入框
/// - 按提供商分组的模型列表
///
/// # 参数
///
/// * `app` - 应用状态引用，从中读取 `model_settings` 数据
///
/// # 返回值
///
/// 返回一个 `Element`，包含完整的模型设置视图 UI
///
/// # UI 交互
///
/// - 刷新按钮：触发 `SettingsMessage::ModelsRefresh` 消息
/// - 搜索框：输入时触发 `SettingsMessage::ModelQueryChanged` 消息
/// - 模型切换：触发 `SettingsMessage::ModelToggle` 消息
/// - 查看详情：触发 `SettingsMessage::ModelDetailOpen` 消息
///
/// # 搜索逻辑
///
/// 搜索支持模糊匹配：
/// - 空查询：显示所有模型
/// - 非空查询：匹配提供商 ID/名称或模型 ID/名称
/// - 提供商匹配时：显示该提供商下所有模型
pub fn main_view(app: &App) -> Element<'_, Message> {
    let s = &app.model_settings;
    let q = s.query.trim().to_ascii_lowercase();
    let busy = s.loading || s.saving;
    let enabled_model_count = s
        .providers
        .iter()
        .flat_map(|provider| &provider.models)
        .filter(|model| model.enabled)
        .count();
    let total_model_count = s.providers.iter().map(|provider| provider.models.len()).sum::<usize>();
    let mut list = column![].spacing(14).width(Length::Fill);
    let mut any = false;
    let mut matched_provider_count = 0usize;
    let mut matched_model_count = 0usize;

    for p in &s.providers {
        let pid_lower = p.id.to_ascii_lowercase();
        let pname_lower = p.name.to_ascii_lowercase();
        let provider_match = !q.is_empty() && (pid_lower.contains(&q) || pname_lower.contains(&q));

        let models = if q.is_empty() {
            p.models.iter().collect::<Vec<_>>()
        } else if provider_match {
            p.models.iter().collect::<Vec<_>>()
        } else {
            p.models
                .iter()
                .filter(|m| {
                    m.id.to_ascii_lowercase().contains(&q)
                        || m.name.to_ascii_lowercase().contains(&q)
                })
                .collect::<Vec<_>>()
        };

        if models.is_empty() {
            continue;
        }

        any = true;
        matched_provider_count += 1;
        matched_model_count += models.len();

        let expanded = s.expanded_provider_ids.contains(&p.id);
        let provider_enabled_count = p.models.iter().filter(|model| model.enabled).count();
        let provider_model_count = p.models.len();
        let mut enable_provider_btn =
            button(text(model_text(app.desktop_locale, "全部启用")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_enabled_count < provider_model_count {
            enable_provider_btn = enable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), true),
            ));
        }
        let mut disable_provider_btn =
            button(text(model_text(app.desktop_locale, "全部禁用")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_enabled_count > 0 {
            disable_provider_btn = disable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), false),
            ));
        }
        let mut reset_provider_btn =
            button(text(model_text(app.desktop_locale, "恢复默认")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style);
        if !busy && provider_model_count > 0 {
            reset_provider_btn = reset_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsReset(p.id.clone()),
            ));
        }
        let expand_provider_btn = button(
            text(if expanded {
                model_text(app.desktop_locale, "收起")
            } else {
                model_text(app.desktop_locale, "展开")
            })
            .size(12),
        )
        .on_press(Message::Settings(message::SettingsMessage::ProviderModelsExpansionToggled(
            p.id.clone(),
        )))
        .padding([6, 10])
        .style(rounded_action_btn_style);

        let provider_header = container(
            column![
                row![
                    container(provider_logo_svg(&p.id, 18.0)).center_x(24).center_y(24),
                    column![
                        text(&p.name).size(15).wrapping(Wrapping::WordOrGlyph),
                        text(&p.id)
                            .size(13)
                            .wrapping(Wrapping::WordOrGlyph)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    settings_value_badge(settings_models::enabled_count(
                        app.desktop_locale,
                        provider_enabled_count,
                        provider_model_count,
                    )),
                    expand_provider_btn,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![enable_provider_btn, disable_provider_btn, reset_provider_btn]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .wrap(),
            ]
            .spacing(12),
        )
        .padding([14, 0])
        .width(Length::Fill);

        let mut card_rows = column![provider_header];
        if expanded {
            for m in models {
                let mut model_toggle = settings_toggle(m.enabled);
                if !busy {
                    let provider_id = p.id.clone();
                    let model_id = m.id.clone();
                    model_toggle = model_toggle.on_toggle(move |enabled| {
                        Message::Settings(SettingsMessage::ModelToggle(
                            provider_id.clone(),
                            model_id.clone(),
                            enabled,
                        ))
                    });
                }
                let metadata = settings_models::model_metadata(
                    app.desktop_locale,
                    m.toolcall,
                    m.attachment,
                    &format_context_limit(m.context_limit),
                );
                let item = container(
                    row![
                        column![
                            text(&m.name).size(15).wrapping(Wrapping::WordOrGlyph),
                            text(&m.id)
                                .size(13)
                                .wrapping(Wrapping::WordOrGlyph)
                                .style(settings_muted_text_style),
                            text(metadata).size(12).style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        icon_btn(
                            Icon::QuestionCircle,
                            model_text(app.desktop_locale, "更多"),
                            Some(Message::Settings(SettingsMessage::ModelDetailOpen(
                                p.id.clone(),
                                m.id.clone(),
                            ))),
                        ),
                        model_toggle,
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([12, 0])
                .width(Length::Fill);
                card_rows = card_rows.push(settings_divider()).push(item);
            }
        }
        list = list.push(settings_group_card(card_rows));
    }

    if !any {
        let empty_title = if s.loading {
            model_text(app.desktop_locale, "正在刷新模型目录…")
        } else {
            model_text(app.desktop_locale, "暂无已连接的提供商或模型")
        };
        let empty_description = if s.loading {
            model_text(app.desktop_locale, "稍后会展示当前已发现的 provider 与模型。")
        } else {
            model_text(app.desktop_locale, "请先完成 provider 连接，或调整搜索关键字后重试。")
        };

        list = list.push(settings_group_card(
            container(settings_group_heading(empty_title, empty_description)).padding([16, 0]),
        ));
    }

    let query_input = text_input(model_text(app.desktop_locale, "搜索模型"), &s.query)
        .on_input(|v| Message::Settings(message::SettingsMessage::ModelQueryChanged(v)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let refresh_btn = button(text(model_text(
        app.desktop_locale,
        if s.loading {
            "刷新中…"
        } else if s.saving {
            "保存中…"
        } else {
            "刷新"
        },
    )))
    .on_press_maybe((!busy).then_some(Message::Settings(SettingsMessage::ModelsRefresh)))
    .padding([8, 14])
    .style(rounded_action_btn_style);

    let filter_content = settings_group_card(
        container(
            row![
                query_input,
                row![
                    settings_value_badge(settings_models::provider_count(
                        app.desktop_locale,
                        matched_provider_count,
                    )),
                    settings_value_badge(settings_models::model_count(
                        app.desktop_locale,
                        matched_model_count,
                    )),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([14, 0]),
    );

    let mut content = column![
        row![
            container(settings_page_heading(
                model_text(app.desktop_locale, "模型配置"),
                model_text(
                    app.desktop_locale,
                    "浏览已连接 provider 的模型目录，搜索、筛选并切换模型可用状态。",
                ),
            ))
            .width(Length::Fill),
            refresh_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        settings_group_heading(
            model_text(app.desktop_locale, "搜索与过滤"),
            model_text(
                app.desktop_locale,
                "按 provider 名称、provider ID、模型名称或模型 ID 过滤结果。",
            ),
        ),
        filter_content,
    ]
    .spacing(16)
    .width(Length::Fill);

    let mut enable_all_btn = button(text(model_text(app.desktop_locale, "全部启用")).size(12))
        .padding([7, 12])
        .style(rounded_action_btn_style);
    if !busy && enabled_model_count < total_model_count {
        enable_all_btn = enable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(true)));
    }

    let mut disable_all_btn = button(text(model_text(app.desktop_locale, "全部禁用")).size(12))
        .padding([7, 12])
        .style(rounded_action_btn_style);
    if !busy && enabled_model_count > 0 {
        disable_all_btn = disable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(false)));
    }

    content = content.push(
        row![
            container(settings_group_heading(
                model_text(app.desktop_locale, "已连接模型"),
                if s.saving {
                    model_text(app.desktop_locale, "正在保存模型状态，完成后即可继续操作。")
                } else {
                    model_text(
                        app.desktop_locale,
                        "按 provider 分组展示可用模型，可单独或一次性切换启用状态。",
                    )
                },
            ))
            .width(Length::Fill),
            row![enable_all_btn, disable_all_btn].spacing(8).align_y(Alignment::Center),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    content = content.push(list);

    content.into()
}

/// 渲染模型设置视图的覆盖层
///
/// 该函数用于渲染模型设置页面上方的弹窗层，目前支持：
/// - 模型详情弹窗
///
/// # 参数
///
/// * `app` - 应用状态引用，从中读取 `model_settings.detail_modal` 数据
/// * `dialog` - 基础对话框元素，覆盖层将在此之上渲染
///
/// # 返回值
///
/// 返回一个 `Element`，包含基础对话框和可能的覆盖层
///
/// # 模型详情弹窗结构
///
/// 弹窗包含：
/// - 固定标题栏：弹窗标题、切换按钮（字段列表/原始 JSON）、关闭按钮
/// - 滚动内容区：模型和提供商标识，以及字段列表或原始 JSON
///
/// # UI 交互
///
/// - 关闭按钮：触发 `SettingsMessage::ModelDetailClose` 消息
/// - 切换按钮：触发 `SettingsMessage::ModelDetailToggleRaw` 消息
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some(mm) = &app.model_settings.detail_modal else {
        return dialog;
    };
    let locale = app.desktop_locale;
    let close_message = Message::Settings(SettingsMessage::ModelDetailClose);
    let toggle_raw =
        button(text(model_text(locale, if mm.show_raw { "字段列表" } else { "原始 JSON" })))
            .on_press(Message::Settings(SettingsMessage::ModelDetailToggleRaw))
            .padding([8, 14])
            .style(rounded_action_btn_style);

    let header = settings_modal_header(
        text(model_text(locale, "模型详情")).size(20),
        row![toggle_raw, settings_close_button(close_message.clone())]
            .spacing(8)
            .align_y(Alignment::Center),
    );
    let identity = column![
        text(&mm.model_name).size(17).wrapping(Wrapping::WordOrGlyph),
        text(&mm.provider_name)
            .size(13)
            .wrapping(Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        detail_row(model_text(locale, "提供商").into(), mm.provider_id.clone()),
        settings_divider(),
        detail_row(
            settings_models::connection_text::text(locale, "模型").into(),
            mm.model_id.clone(),
        ),
    ]
    .spacing(6);

    let details = if mm.show_raw {
        settings_group_card(
            container(text(&mm.raw_json).size(13).wrapping(Wrapping::WordOrGlyph)).padding([14, 0]),
        )
    } else {
        let mut rows = column![];
        for (index, field) in mm.rows.iter().enumerate() {
            if index != 0 {
                rows = rows.push(settings_divider());
            }
            rows = rows.push(detail_row(field.label.display(locale), field.value.display(locale)));
        }
        settings_group_card(rows)
    };

    let body = column![identity, details].spacing(14);
    let card = settings_modal_layout(header, body, None)
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            // Keep catalog text from showing through the detail card.
            if let Some(Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        })
        .width(780)
        .height(520);
    settings_modal_overlay(Some(dialog), close_message, container(card).padding(12))
}

fn detail_row(label: String, value: String) -> Element<'static, Message> {
    container(
        row![
            text(label)
                .size(13)
                .width(Length::FillPortion(2))
                .wrapping(Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
            text(value).size(13).width(Length::FillPortion(3)).wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_models_tests.rs"]
mod system_settings_models_tests;

#[cfg(test)]
#[path = "system_settings_models_ui_refresh_10_tests.rs"]
mod ui_refresh_10_tests;
