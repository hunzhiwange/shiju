//! 系统设置 — 提供商已连接视图
//!
//! 本模块提供“模型提供商”设置中已连接与热门提供商的管理视图。
//!
//! # 功能
//! - 渲染页面头部：标题、同步按钮与刷新按钮
//! - 已连接提供商列表：展示卡片并支持断开连接（含二次确认）
//! - 热门提供商列表：根据模式匹配目录或现有提供商，展示连接/已连接/未发现状态
//! - 自定义提供商：提供添加入口

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, danger_action_btn_style, primary_action_btn_style, provider_logo_svg,
    rounded_action_btn_style, settings_divider, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_section_card, settings_value_badge,
};
use crate::app::state::{
    ModelCatalogEntry, ProviderSummary, SHIJU_OFFICIAL_PROVIDER_ID, SHIJU_OFFICIAL_PROVIDER_NAME,
    is_shiju_official_provider_id,
};
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, progress_bar, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

const PROVIDER_LABEL_WIDTH: f32 = SETTINGS_LABEL_WIDTH + 40.0;

pub(super) fn provider_action_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.18))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.72),
            radius: 16.0.into(),
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

pub(super) fn provider_item_row<'a>(
    logo_id: impl Into<String>,
    title: impl Into<String>,
    description: impl Into<String>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let logo_id = logo_id.into();
    let title = title.into();
    let description = description.into();

    container(
        row![
            row![
                container(provider_logo_svg(&logo_id, 18.0))
                    .center_x(Length::Fixed(22.0))
                    .center_y(Length::Fixed(22.0)),
                column![
                    text(title).size(13),
                    text(description).size(11).style(settings_muted_text_style),
                ]
                .spacing(4)
            ]
            .spacing(10)
            .width(Length::Fixed(PROVIDER_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) struct PopularProviderMatch {
    pub(super) display: String,
    pub(super) provider_id: Option<String>,
    pub(super) is_connected: bool,
}

pub(super) fn popular_provider_match(
    pattern: &str,
    catalog_items: &[ModelCatalogEntry],
    providers: &[ProviderSummary],
) -> PopularProviderMatch {
    if is_shiju_official_provider_id(pattern) {
        return PopularProviderMatch {
            display: SHIJU_OFFICIAL_PROVIDER_NAME.to_string(),
            provider_id: Some(SHIJU_OFFICIAL_PROVIDER_ID.to_string()),
            is_connected: providers
                .iter()
                .any(|provider| is_shiju_official_provider_id(&provider.id) && provider.connected),
        };
    }

    let pat_lower = pattern.to_ascii_lowercase();
    let found_catalog = catalog_items.iter().find(|p| {
        let id_lower = p.provider_id.to_ascii_lowercase();
        let name_lower = p.provider_name.to_ascii_lowercase();
        id_lower == pat_lower
            || id_lower.contains(&pat_lower)
            || name_lower == pat_lower
            || name_lower.contains(&pat_lower)
    });

    if let Some(p) = found_catalog {
        let is_connected = providers.iter().any(|pp| pp.id == p.provider_id && pp.connected);
        return PopularProviderMatch {
            display: p.provider_name.clone(),
            provider_id: Some(p.provider_id.clone()),
            is_connected,
        };
    }

    if let Some(p) = providers.iter().find(|p| {
        let id_lower = p.id.to_ascii_lowercase();
        let name_lower = p.name.to_ascii_lowercase();
        id_lower == pat_lower
            || id_lower.contains(&pat_lower)
            || name_lower == pat_lower
            || name_lower.contains(&pat_lower)
    }) {
        return PopularProviderMatch {
            display: p.name.clone(),
            provider_id: Some(p.id.clone()),
            is_connected: p.connected,
        };
    }

    PopularProviderMatch { display: pattern.to_string(), provider_id: None, is_connected: false }
}

/// 渲染“模型提供商”设置页面的已连接/热门/自定义提供商视图。
///
/// # 参数
/// - `app`: 应用状态引用，从中读取 `provider_settings`（加载状态、已连接列表、目录、热门模式等）。
///
/// # 返回
/// - 返回一个 `iced::Element`，可直接作为子视图插入到设置页布局中。
///
/// # 示例
/// ```ignore
/// let element = view(&app);
/// // 在布局中使用
/// container(element).into()
/// ```
///
/// # 行为
/// - 头部显示标题、同步按钮与刷新按钮；同步中显示顶部进度条并禁用重复点击。
/// - 已连接列表：若无已连接项，显示“暂无”；否则逐条渲染卡片并支持断开连接（二次确认）。
/// - 热门列表：根据模式匹配目录或现有提供商，显示对应状态与操作；提供“更多模型提供商”按钮打开目录。
/// - 自定义区块：引导添加与 OpenAI 兼容的自定义提供商。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.provider_settings;
    let header_busy = s.loading || s.models_syncing;

    let header =
        row![
            container(settings_page_intro(
                "模型提供商",
                "统一管理已连接、热门和自定义的模型提供商入口。",
            ))
            .width(Length::Fill),
            button(text(if s.models_syncing { "同步中…" } else { "从 models.dev 同步" }))
                .on_press_maybe((!header_busy).then_some(Message::Settings(
                    message::SettingsMessage::ProviderModelsSyncRemote,
                )))
                .padding([7, 12])
                .style(if s.models_syncing {
                    rounded_action_btn_style
                } else {
                    primary_action_btn_style
                }),
            button(text(if s.loading { "刷新中…" } else { "刷新" }))
                .on_press_maybe(
                    (!header_busy)
                        .then_some(Message::Settings(message::SettingsMessage::ProvidersRefresh,))
                )
                .padding([7, 12])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

    let mut root = column![header].spacing(16);

    if s.models_syncing {
        root = root.push(settings_panel(
            column![
                row![
                    text(&s.models_sync_label).size(13),
                    container(text(" ")).width(Length::Fill),
                    text(format!("{:.0}%", s.models_sync_progress * 100.0))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .align_y(Alignment::Center),
                container(progress_bar(0.0..=1.0, s.models_sync_progress)).height(8),
            ]
            .spacing(10),
        ));
    }

    root = root.push(settings_section_card(
        "已连接的提供商",
        "查看当前可用的 provider，并管理密钥、编辑与断开连接。",
    ));

    let connected = s.providers.iter().filter(|provider| provider.connected).collect::<Vec<_>>();
    if connected.is_empty() {
        root = root.push(settings_panel(
            text("暂无已连接的提供商。").size(13).style(settings_muted_text_style),
        ));
    } else {
        let mut connected_panel = column![];

        for (index, p) in connected.iter().enumerate() {
            let source_badge = settings_value_badge(p.source_label.as_str());
            let is_confirming_disconnect =
                s.disconnect_confirm_provider_id.as_deref() == Some(p.id.as_str());
            let controls: Element<'_, Message> = if is_confirming_disconnect {
                row![
                    source_badge,
                    button(text("取消"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::ProviderDisconnectCanceled,
                        ))
                        .padding([7, 12])
                        .style(rounded_action_btn_style),
                    button(text("确认断开"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::ProviderDisconnectConfirmed(p.id.clone()),
                        ))
                        .padding([7, 12])
                        .style(danger_action_btn_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else {
                row![
                    source_badge,
                    button(text("修改密钥"))
                        .on_press(Message::Settings(message::SettingsMessage::ProviderConnectOpen(
                            p.id.clone()
                        )))
                        .padding([7, 12])
                        .style(rounded_action_btn_style),
                    button(text("编辑"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::CustomProviderEditOpen(p.id.clone()),
                        ))
                        .padding([7, 12])
                        .style(rounded_action_btn_style),
                    button(text("断开连接"))
                        .on_press(Message::Settings(
                            message::SettingsMessage::ProviderDisconnectRequested(p.id.clone()),
                        ))
                        .padding([7, 12])
                        .style(rounded_action_btn_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            };

            connected_panel = connected_panel.push(provider_item_row(
                &p.id,
                p.name.clone(),
                p.id.clone(),
                controls,
            ));

            if index + 1 != connected.len() {
                connected_panel = connected_panel.push(settings_divider());
            }
        }

        root = root.push(settings_panel(connected_panel.spacing(0)));
    }

    root = root.push(settings_section_card(
        "热门提供商",
        "根据热门模式匹配目录中的 provider，快速建立常用连接。",
    ));

    let official =
        popular_provider_match(SHIJU_OFFICIAL_PROVIDER_ID, &s.catalog_items, &s.providers);
    let mut official_controls =
        row![settings_value_badge("官方推荐")].spacing(8).align_y(Alignment::Center);
    if official.is_connected {
        official_controls = official_controls.push(settings_value_badge("已连接"));
    } else {
        official_controls = official_controls.push(
            button(text("连接"))
                .on_press(Message::Settings(message::SettingsMessage::ProviderConnectOpen(
                    SHIJU_OFFICIAL_PROVIDER_ID.to_string(),
                )))
                .padding([7, 12])
                .style(primary_action_btn_style),
        );
    }
    let mut popular_panel = column![provider_item_row(
        SHIJU_OFFICIAL_PROVIDER_ID,
        official.display,
        "识矩官方模型服务，连接后可自动发现可用模型。",
        official_controls,
    )];

    for (original_index, pat) in s
        .popular_patterns
        .iter()
        .enumerate()
        .filter(|(_, pattern)| !is_shiju_official_provider_id(pattern))
    {
        let PopularProviderMatch { display, provider_id, is_connected } =
            popular_provider_match(pat, &s.catalog_items, &s.providers);

        let logo_id = provider_id.clone().unwrap_or_else(|| "agent".to_owned());
        let mut controls = row![].spacing(8).align_y(Alignment::Center);

        if is_connected {
            controls = controls.push(settings_value_badge("已连接"));
        } else if provider_id.is_none() {
            controls = controls.push(settings_value_badge("未发现"));
        }

        if let Some(pid) = provider_id.clone().filter(|_| !is_connected) {
            controls = controls.push(
                button(text("连接"))
                    .on_press(Message::Settings(message::SettingsMessage::ProviderConnectOpen(pid)))
                    .padding([7, 12])
                    .style(primary_action_btn_style),
            );
        }

        controls = controls.push(
            button(text("移除"))
                .on_press(Message::Settings(message::SettingsMessage::PopularProviderRemove(
                    original_index,
                )))
                .padding([7, 12])
                .style(rounded_action_btn_style),
        );

        let description = provider_id
            .map(|pid| format!("provider id: {}", pid))
            .unwrap_or_else(|| format!("匹配模式：{}", pat));

        popular_panel = popular_panel.push(settings_divider()).push(provider_item_row(
            logo_id,
            display,
            description,
            controls,
        ));
    }

    let popular_action = container(
        row![
            column![
                text("发现更多提供商").size(13),
                text("从模型目录中浏览更多 provider，并快速加入热门或直接连接。")
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            button(text("更多模型提供商"))
                .on_press(Message::Settings(message::SettingsMessage::ProviderCatalogOpen))
                .padding([8, 14])
                .style(primary_action_btn_style),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding([14, 16])
    .style(provider_action_panel_style);

    root = root.push(settings_panel(popular_panel.spacing(0)));
    root = root.push(popular_action);

    root =
        root.push(settings_section_card("自定义提供商", "按 base URL 接入 OpenAI 兼容 provider。"));
    root = root.push(settings_panel(provider_item_row(
        "agent",
        "OpenAI 兼容 provider",
        "添加一个自定义 provider，并在后续编辑连接参数。",
        row![
            container(text(" ")).width(Length::Fill),
            button(text("添加提供商"))
                .on_press(Message::Settings(message::SettingsMessage::CustomProviderOpen))
                .padding([7, 12])
                .style(primary_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )));

    root.into()
}
