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
    bool_support_label, format_context_limit, icon_btn, provider_logo_svg,
    rounded_action_btn_style, settings_close_button, settings_divider, settings_error_banner,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_panel_style, settings_section_card, settings_section_content,
    settings_segment_button_style, settings_text_input_style, settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::message::settings::{ModelConfigTabMessage, SettingsMessage};
use crate::app::state::ModelConfigTab;
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler};
use iced::{Alignment, Element, Length, Theme};

pub(super) const MODEL_TABS: &[SettingsTabDescriptor<ModelConfigTab>] = &[
    SettingsTabDescriptor::new(
        ModelConfigTab::Models,
        "模型",
        &["搜索模型", "模型详情", "提供商", "模型 id", "上下文窗口", "最大输出", "温度"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::ModelRoutes,
        "模型路由",
        &["routing", "匹配模式", "pattern", "priority", "优先级"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::QueryClassification,
        "模型分类",
        &["classification", "rules", "查询分类", "规则"],
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
    let tab_button = |tab, label| {
        let is_active = tab == active_tab;
        button(text(label).size(13))
            .on_press(Message::Settings(SettingsMessage::ModelConfigTab(
                ModelConfigTabMessage::Selected(tab),
            )))
            .padding([8, 14])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };

    let tabs =
        MODEL_TABS.iter().fold(row![].spacing(8).align_y(Alignment::Center), |row, descriptor| {
            row.push(tab_button(descriptor.tab, descriptor.label))
        });
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
        sections = sections.push(settings_error_banner(error));
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
        let busy = s.loading || s.saving;
        let mut enable_provider_btn =
            button(text("全部启用").size(12)).padding([6, 10]).style(rounded_action_btn_style);
        if !busy && provider_enabled_count < provider_model_count {
            enable_provider_btn = enable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), true),
            ));
        }
        let mut disable_provider_btn =
            button(text("全部禁用").size(12)).padding([6, 10]).style(rounded_action_btn_style);
        if !busy && provider_enabled_count > 0 {
            disable_provider_btn = disable_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsSetAll(p.id.clone(), false),
            ));
        }
        let mut reset_provider_btn =
            button(text("恢复默认").size(12)).padding([6, 10]).style(rounded_action_btn_style);
        if !busy && provider_model_count > 0 {
            reset_provider_btn = reset_provider_btn.on_press(Message::Settings(
                message::SettingsMessage::ProviderModelsReset(p.id.clone()),
            ));
        }
        let expand_provider_btn = button(text(if expanded { "收起" } else { "展开" }).size(12))
            .on_press(Message::Settings(message::SettingsMessage::ProviderModelsExpansionToggled(
                p.id.clone(),
            )))
            .padding([6, 10])
            .style(rounded_action_btn_style);

        let mut card_col = column![
            row![
                row![
                    container(provider_logo_svg(&p.id, 16.0))
                        .center_x(Length::Fixed(18.0))
                        .center_y(Length::Fixed(18.0)),
                    column![
                        text(&p.name).size(14),
                        text(&p.id).size(12).style(settings_muted_text_style)
                    ]
                    .spacing(2),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                settings_value_badge(format!(
                    "{} / {} 已启用",
                    provider_enabled_count, provider_model_count
                )),
                row![
                    enable_provider_btn,
                    disable_provider_btn,
                    reset_provider_btn,
                    expand_provider_btn,
                ]
                .spacing(6)
                .wrap(),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .wrap()
        ]
        .spacing(12);

        if expanded {
            let mut first_model = true;
            for m in models {
                let provider_id = p.id.clone();
                let model_id = m.id.clone();
                let model_name = m.name.clone();
                let enabled = m.enabled;
                let title_color_alpha = if enabled { 1.0 } else { 0.65 };
                let id_color_alpha = if enabled { 0.65 } else { 0.5 };
                let meta_alpha = if enabled { 0.65 } else { 0.5 };
                let meta_text = format!(
                    "工具: {} · 图片输入: {} · 上下文限制: {}",
                    bool_support_label(m.toolcall),
                    bool_support_label(m.attachment),
                    format_context_limit(m.context_limit)
                );
                let provider_id_toggle = provider_id.clone();
                let model_id_toggle = model_id.clone();

                let model_toggle = if s.loading || s.saving {
                    toggler(enabled)
                } else {
                    toggler(enabled).on_toggle(move |b| {
                        Message::Settings(message::SettingsMessage::ModelToggle(
                            provider_id_toggle.clone(),
                            model_id_toggle.clone(),
                            b,
                        ))
                    })
                };

                let item = container(
                    row![
                        column![
                            row![
                                text(model_name).size(14).style(move |t: &iced::Theme| {
                                    text::Style {
                                        color: Some(
                                            t.palette().text.scale_alpha(title_color_alpha),
                                        ),
                                    }
                                }),
                                text(model_id.clone()).size(12).style(move |t: &iced::Theme| {
                                    text::Style {
                                        color: Some(t.palette().text.scale_alpha(id_color_alpha)),
                                    }
                                }),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .wrap(),
                            text(meta_text).size(12).style(move |t: &iced::Theme| {
                                text::Style {
                                    color: Some(t.palette().text.scale_alpha(meta_alpha)),
                                }
                            }),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        // 打开模型详情弹窗的按钮
                        icon_btn(
                            Icon::QuestionCircle,
                            "更多",
                            Some(Message::Settings(message::SettingsMessage::ModelDetailOpen(
                                provider_id.clone(),
                                model_id.clone(),
                            ))),
                        ),
                        model_toggle,
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding([10, 0])
                .width(Length::Fill);

                if !first_model {
                    card_col = card_col.push(settings_divider());
                }
                first_model = false;

                card_col = card_col.push(item);
            }
        }

        let card = container(card_col).padding(16).width(Length::Fill).style(settings_panel_style);

        list = list.push(card);
    }

    if !any {
        let empty_title = if s.loading {
            "正在刷新模型目录…"
        } else {
            "暂无已连接的提供商或模型"
        };
        let empty_description = if s.loading {
            "稍后会展示当前已发现的 provider 与模型。"
        } else {
            "请先完成 provider 连接，或调整搜索关键字后重试。"
        };

        list = list.push(settings_panel(
            column![
                text(empty_title).size(14),
                text(empty_description).size(12).style(settings_muted_text_style),
            ]
            .spacing(6),
        ));
    }

    let query_input = text_input("搜索模型", &s.query)
        .on_input(|v| Message::Settings(message::SettingsMessage::ModelQueryChanged(v)))
        .padding([10, 12])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let refresh_btn = icon_btn(
        Icon::ArrowCounterClockwise,
        if s.loading {
            "刷新中…"
        } else if s.saving {
            "保存中…"
        } else {
            "刷新"
        },
        if s.loading || s.saving {
            None
        } else {
            Some(Message::Settings(message::SettingsMessage::ModelsRefresh))
        },
    );

    let filter_content = settings_section_content(
        column![
            query_input,
            row![
                settings_value_badge(format!("{} 个提供商", matched_provider_count)),
                settings_value_badge(format!("{} 个模型", matched_model_count)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    );

    let mut content = column![
        row![
            container(settings_page_intro(
                "模型配置",
                "浏览已连接 provider 的模型目录，搜索、筛选并切换模型可用状态。"
            ))
            .width(Length::Fill),
            refresh_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Start),
        settings_section_card(
            "搜索与过滤",
            "按 provider 名称、provider ID、模型名称或模型 ID 过滤结果。"
        ),
        filter_content,
    ]
    .spacing(16)
    .width(Length::Fill);

    let mut enable_all_btn =
        button(text("全部启用").size(12)).padding([7, 12]).style(rounded_action_btn_style);
    if !s.loading && !s.saving && enabled_model_count < total_model_count {
        enable_all_btn = enable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(true)));
    }

    let mut disable_all_btn =
        button(text("全部禁用").size(12)).padding([7, 12]).style(rounded_action_btn_style);
    if !s.loading && !s.saving && enabled_model_count > 0 {
        disable_all_btn = disable_all_btn
            .on_press(Message::Settings(message::SettingsMessage::ModelsSetAll(false)));
    }

    content = content.push(
        row![
            settings_section_card(
                "已连接模型",
                if s.saving {
                    "正在保存模型状态，完成后即可继续操作。"
                } else {
                    "按 provider 分组展示可用模型，可单独或一次性切换启用状态。"
                }
            ),
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
/// - 标题栏：弹窗标题、切换按钮（字段列表/原始 JSON）、关闭按钮
/// - 模型标识：提供商名称/模型名称、提供商 ID/模型 ID
/// - 内容区：根据 `show_raw` 标志显示字段列表或原始 JSON
///
/// # UI 交互
///
/// - 关闭按钮/点击遮罩：触发 `SettingsMessage::ModelDetailClose` 消息
/// - 切换按钮：触发 `SettingsMessage::ModelDetailToggleRaw` 消息
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.model_settings;
    let mut base = dialog;

    // 检查是否需要显示模型详情弹窗
    if let Some(mm) = &s.detail_modal {
        let close_message = Message::Settings(message::SettingsMessage::ModelDetailClose);
        let close_btn = settings_close_button(close_message.clone());

        // 切换显示模式的按钮（字段列表 / 原始 JSON）
        let toggle_raw_btn = button(text(if mm.show_raw { "字段列表" } else { "原始 JSON" }))
            .on_press(Message::Settings(message::SettingsMessage::ModelDetailToggleRaw))
            .padding([6, 10])
            .style(rounded_action_btn_style);

        let header = row![
            column![
                text("模型详情").size(16),
                text(format!("{} / {}", mm.provider_name, mm.model_name))
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            toggle_raw_btn,
            close_btn
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let meta_row = row![
            settings_value_badge(format!("Provider {}", mm.provider_id)),
            settings_value_badge(format!("Model {}", mm.model_id)),
        ]
        .spacing(8)
        .wrap();

        // 根据 show_raw 标志决定显示字段列表还是原始 JSON
        let body: Element<'_, Message> = if mm.show_raw {
            // 显示原始 JSON 文本
            scrollable(container(text(&mm.raw_json).size(12)).padding([0, 12]).width(Length::Fill))
                .height(Length::Fill)
                .into()
        } else {
            // 显示字段列表（标签-值对）
            let mut rows_col = column![].spacing(10);
            for r in &mm.rows {
                rows_col = rows_col.push(
                    row![
                        text(&r.label)
                            .size(13)
                            .style(settings_muted_text_style)
                            .width(Length::Fixed(240.0)),
                        text(&r.value).size(13).width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
            scrollable(container(rows_col).padding([0, 12]).width(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        let modal_col =
            column![header, meta_row, settings_divider(), body].spacing(14).height(Length::Fill);

        let card =
            settings_modal_card(modal_col).width(Length::Fixed(780.0)).height(Length::Fixed(520.0));
        base = settings_modal_overlay(Some(base), close_message, card);
    }

    base
}
#[cfg(test)]
#[path = "system_settings_models_tests.rs"]
mod system_settings_models_tests;
