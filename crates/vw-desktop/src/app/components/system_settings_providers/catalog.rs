//! 系统设置中模型提供商配置的目录、表单与弹窗能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, provider_logo_svg, rounded_action_btn_style, settings_modal_card,
    settings_modal_overlay, settings_muted_text_style, settings_text_input_style,
};
use crate::app::state::{ModelCatalogEntry, ProviderSummary};
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CatalogProviderItem {
    pub(super) provider_id: String,
    pub(super) provider_name: String,
    pub(super) model_count: usize,
    pub(super) is_connected: bool,
    pub(super) in_popular: bool,
}

pub(super) fn visible_catalog_providers(
    catalog_items: &[ModelCatalogEntry],
    configured_providers: &[ProviderSummary],
    popular_patterns: &[String],
    query: &str,
) -> Vec<CatalogProviderItem> {
    let mut providers = BTreeMap::<String, (String, usize)>::new();
    for item in catalog_items {
        providers
            .entry(item.provider_id.clone())
            .and_modify(|entry| entry.1 += 1)
            .or_insert_with(|| (item.provider_name.clone(), 1));
    }

    let query = query.trim().to_ascii_lowercase();
    providers
        .into_iter()
        .filter_map(|(provider_id, (provider_name, model_count))| {
            if !query.is_empty()
                && !provider_id.to_ascii_lowercase().contains(&query)
                && !provider_name.to_ascii_lowercase().contains(&query)
            {
                return None;
            }

            Some(CatalogProviderItem {
                is_connected: configured_providers
                    .iter()
                    .any(|provider| provider.id == provider_id && provider.connected),
                in_popular: popular_patterns.iter().any(|pattern| pattern == &provider_id),
                provider_id,
                provider_name,
                model_count,
            })
        })
        .collect()
}

pub(super) fn model_count_label(model_count: usize) -> String {
    if model_count == 1 { "1 个模型".to_string() } else { format!("{} 个模型", model_count) }
}

pub(super) fn catalog_surface_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.16))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.76),
            radius: 18.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn catalog_list_frame_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color.scale_alpha(0.84))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.92),
            radius: 20.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.04),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        snap: false,
        ..Default::default()
    }
}

pub(super) fn catalog_item_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color.scale_alpha(0.72))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.82),
            radius: 14.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.05),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        },
        snap: false,
        ..Default::default()
    }
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
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
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.provider_settings;
    let mut base = dialog;

    if s.catalog_open {
        let close_btn = button(text("×").size(20))
            .on_press(Message::Settings(message::SettingsMessage::ProviderCatalogClose))
            .padding([4, 10])
            .style(rounded_action_btn_style);

        let catalog_items = s.catalog_items.clone();

        let visible_providers = visible_catalog_providers(
            &catalog_items,
            &s.providers,
            &s.popular_patterns,
            &s.catalog_query,
        );

        let mut list = column![].spacing(8);
        let visible_count = visible_providers.len();
        for provider in visible_providers {
            let CatalogProviderItem {
                provider_id,
                provider_name,
                model_count,
                is_connected,
                in_popular,
            } = provider;
            let connect_btn = if is_connected {
                button(text("已连接")).padding([6, 10]).style(rounded_action_btn_style)
            } else {
                button(text("连接"))
                    .on_press(Message::Settings(message::SettingsMessage::ProviderConnectOpen(
                        provider_id.clone(),
                    )))
                    .padding([6, 10])
                    .style(primary_action_btn_style)
            };
            let add_btn = if in_popular {
                button(text("已在热门")).padding([6, 10]).style(rounded_action_btn_style)
            } else {
                button(text("添加到热门"))
                    .on_press(Message::Settings(
                        message::SettingsMessage::ProviderCatalogAddToPopular(provider_id.clone()),
                    ))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
            };

            let subtitle = model_count_label(model_count);
            let row_item = container(
                row![
                    container(provider_logo_svg(&provider_id, 16.0))
                        .center_x(Length::Fixed(36.0))
                        .center_y(Length::Fixed(36.0))
                        .style(catalog_surface_style),
                    column![
                        text(provider_name.clone()).size(14),
                        text(subtitle).size(12).style(|t: &iced::Theme| text::Style {
                            color: Some(t.palette().text.scale_alpha(0.65))
                        })
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    container(text(provider_id.clone()).size(12).style(|t: &iced::Theme| {
                        text::Style { color: Some(t.palette().text.scale_alpha(0.72)) }
                    }))
                    .padding([5, 10])
                    .style(catalog_surface_style),
                    connect_btn,
                    add_btn
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([14, 16])
            .width(Length::Fill)
            .style(catalog_item_style);

            list = list.push(row_item);
        }

        let body_content: Element<'_, Message> = if s.catalog_loading {
            container(column![text("加载中…").size(14)].spacing(8))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else if visible_count == 0 {
            let empty_text = if catalog_items.is_empty() {
                "暂无可用模型提供商"
            } else {
                "未找到匹配的模型提供商"
            };
            container(text(empty_text).size(14).style(|t: &iced::Theme| text::Style {
                color: Some(t.palette().text.scale_alpha(0.7)),
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            scrollable(container(list).padding(iced::Padding {
                top: 4.0,
                right: 12.0,
                bottom: 12.0,
                left: 12.0,
            }))
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .into()
        };

        let body = container(body_content)
            .padding(iced::Padding { top: 8.0, right: 0.0, bottom: 4.0, left: 0.0 })
            .height(Length::Fill)
            .style(catalog_list_frame_style);

        let search_block = container(
            column![
                text("搜索与筛选").size(13),
                text("按 provider 名称或 provider ID 快速定位可用目录。")
                    .size(12)
                    .style(settings_muted_text_style),
                text_input("搜索模型提供商", &s.catalog_query)
                    .on_input(|v| {
                        Message::Settings(message::SettingsMessage::ProviderCatalogQueryChanged(v))
                    })
                    .width(Length::Fill)
                    .style(settings_text_input_style),
            ]
            .spacing(8),
        )
        .padding([14, 16])
        .style(catalog_surface_style);

        let modal_col = column![
            row![
                column![
                    text("更多模型提供商").size(18),
                    text("浏览模型目录中的 provider，并快速连接或加入热门。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                container(text("")).width(Length::Fill),
                close_btn,
            ]
            .align_y(Alignment::Center),
            search_block,
            body
        ]
        .spacing(16)
        .height(Length::Fill);

        let card =
            settings_modal_card(modal_col).width(Length::Fixed(900.0)).height(Length::Fixed(520.0));

        base = settings_modal_overlay(
            Some(base),
            Message::Settings(message::SettingsMessage::ProviderCatalogClose),
            card,
        );
    }

    base
}
