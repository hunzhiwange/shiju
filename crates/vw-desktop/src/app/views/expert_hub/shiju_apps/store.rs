//! 展示识矩应用目录及应用安装状态。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::{ShijuAppCatalogDto, ShijuCatalogAppDto};

use super::{app_title, card_grid, styles};
use crate::app::Message;
use crate::app::message::{AccountMessage, ShijuAppsMessage};
use crate::app::shiju_apps::{ShijuAppDetailTarget, ShijuAppsLoadState};

fn catalog_card(index: usize, app: &ShijuCatalogAppDto) -> Element<'static, Message> {
    let installed = app.installed;
    let initial = app.name.chars().next().unwrap_or('应').to_string();
    let badge = container(text(initial).size(18))
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .center(Length::Fixed(44.0))
        .style(styles::badge_style);
    let status = if installed { "已安装" } else { "未安装" };
    let title = row![
        badge,
        column![
            text(app_title(&app.name))
                .size(16)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::None),
            text(app.app_id.clone()).size(11).font(Font::MONOSPACE),
        ]
        .spacing(3)
        .width(Length::Fill),
        container(text(status).size(11))
            .padding([4, 8])
            .style(move |theme| styles::status_style(theme, installed)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let description = app.description.clone().unwrap_or_else(|| "暂无应用说明".to_string());
    let metadata = format!(
        "版本 {} · 权限 {}",
        app.version,
        if app.permissions.is_empty() {
            "无".to_string()
        } else {
            app.permissions.len().to_string()
        }
    );
    let card = column![
        title,
        text(description).size(13).height(Length::Fixed(42.0)),
        text(metadata).size(11).style(styles::muted_text_style),
        row![
            iced::widget::Space::new().width(Length::Fill),
            button(text("查看").size(12))
                .padding([6, 12])
                .style(styles::quiet_button_style)
                .on_press(Message::ShijuApps(ShijuAppsMessage::OpenDetail(
                    ShijuAppDetailTarget::Store(index),
                ))),
        ],
    ]
    .spacing(10);
    container(card)
        .padding(16)
        .width(Length::Fixed(310.0))
        .height(Length::Fixed(184.0))
        .style(styles::card_style)
        .into()
}

fn feedback(message: String, action: Option<(&'static str, Message)>) -> Element<'static, Message> {
    let mut content = column![text(message).size(14)].spacing(12).align_x(Alignment::Center);
    if let Some((label, message)) = action {
        content = content.push(
            button(text(label).size(13))
                .padding([7, 12])
                .style(styles::quiet_button_style)
                .on_press(message),
        );
    }
    container(content).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
}

/// 按登录、加载、错误或成功状态展示应用商店。
pub(super) fn view(state: &ShijuAppsLoadState<ShijuAppCatalogDto>) -> Element<'static, Message> {
    match state {
        ShijuAppsLoadState::Idle => feedback("应用目录尚未加载".to_string(), None),
        ShijuAppsLoadState::NeedsLogin => feedback(
            "登录识矩账户后可浏览应用商店".to_string(),
            Some(("登录", Message::Account(AccountMessage::StartBrowserLogin))),
        ),
        ShijuAppsLoadState::Loading { .. } => feedback("正在加载应用商店…".to_string(), None),
        ShijuAppsLoadState::Error(error) => feedback(
            format!("应用商店加载失败：{error}"),
            Some(("重试", Message::ShijuApps(ShijuAppsMessage::Retry))),
        ),
        ShijuAppsLoadState::Loaded(catalog) if catalog.apps.is_empty() => {
            feedback("应用商店暂时为空".to_string(), None)
        }
        ShijuAppsLoadState::Loaded(catalog) => {
            let cards = catalog
                .apps
                .iter()
                .enumerate()
                .map(|(index, app)| catalog_card(index, app))
                .collect::<Vec<_>>();
            scrollable(card_grid(cards))
                .direction(Direction::Vertical(
                    Scrollbar::new()
                        .width(styles::SCROLLBAR_WIDTH)
                        .scroller_width(styles::SCROLLBAR_WIDTH),
                ))
                .height(Length::Fill)
                .into()
        }
    }
}
