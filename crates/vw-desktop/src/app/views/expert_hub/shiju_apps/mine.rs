//! 展示已安装识矩应用的运行就绪状态与启动入口。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::{ShijuAppListDto, ShijuInstalledAppDto};

use super::{app_title, card_grid, styles};
use crate::app::Message;
use crate::app::message::{AccountMessage, ShijuAppsMessage};
use crate::app::shiju_apps::{ShijuAppDetailTarget, ShijuAppsLoadState};

/// 只有协调状态与运行时状态都为 `ready` 时判定应用可启动。
pub(crate) fn runtime_readiness(app: &ShijuInstalledAppDto) -> Result<&'static str, String> {
    if app.reconcile_status == "ready" && app.runtime_status == "ready" {
        Ok("运行就绪")
    } else {
        Err(format!("协调状态：{}；运行状态：{}", app.reconcile_status, app.runtime_status))
    }
}

/// 为已就绪应用生成启动消息，未就绪时不暴露启动操作。
pub(crate) fn open_action_message(app: &ShijuInstalledAppDto) -> Option<Message> {
    runtime_readiness(app)
        .is_ok()
        .then(|| Message::ShijuApps(ShijuAppsMessage::OpenInstalled(app.installation_id.clone())))
}

fn installed_card(
    i18n: crate::app::i18n::I18n,
    index: usize,
    app: &ShijuInstalledAppDto,
) -> Element<'static, Message> {
    let ready = runtime_readiness(app).is_ok();
    let readiness = if ready {
        i18n.__("运行就绪").to_string()
    } else {
        i18n.format(
            "尚未就绪 · 协调状态：%s；运行状态：%s",
            &[app.reconcile_status.clone().into(), app.runtime_status.clone().into()],
        )
        .expect("valid readiness")
    };
    let initial = app
        .name
        .chars()
        .next()
        .unwrap_or_else(|| i18n.__("应用").chars().next().unwrap_or('A'))
        .to_string();
    let title = row![
        container(text(initial).size(18))
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(44.0))
            .center(Length::Fixed(44.0))
            .style(styles::badge_style),
        column![
            text(app_title(&app.name))
                .size(16)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::None),
            text(app.installation_id.clone()).size(11).font(if app.installation_id.is_ascii() {
                Font::MONOSPACE
            } else {
                Font::default()
            }),
        ]
        .spacing(3)
        .width(Length::Fill),
        container(text(i18n.__(if ready { i18n.__("就绪") } else { i18n.__("未就绪") })).size(11))
            .padding([4, 8])
            .style(move |theme| styles::status_style(theme, ready)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let mut actions = row![
        text(readiness).size(11).style(styles::muted_text_style),
        iced::widget::Space::new().width(Length::Fill),
        button(text(i18n.__("查看")).size(12))
            .padding([6, 12])
            .style(styles::quiet_button_style)
            .on_press(Message::ShijuApps(ShijuAppsMessage::OpenDetail(
                ShijuAppDetailTarget::Mine(index),
            ))),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(message) = open_action_message(app) {
        actions = actions.push(
            button(text(i18n.__("打开")).size(12))
                .padding([6, 12])
                .style(styles::quiet_button_style)
                .on_press(message),
        );
    }
    let card = column![
        title,
        text(app.description.clone().unwrap_or_else(|| i18n.__("暂无应用说明").to_string()))
            .size(13)
            .height(Length::Fixed(42.0)),
        text(
            i18n.format(
                "来源 %s · 版本 %s",
                &[app.source.clone().into(), app.version.clone().into()]
            )
            .expect("valid app metadata")
        )
        .size(11)
        .style(styles::muted_text_style),
        actions,
    ]
    .spacing(10);
    container(card)
        .padding(16)
        .width(Length::Fixed(310.0))
        .height(Length::Fixed(194.0))
        .style(styles::card_style)
        .into()
}

fn feedback(
    i18n: crate::app::i18n::I18n,
    message: String,
    action: Option<(&'static str, Message)>,
) -> Element<'static, Message> {
    let mut content = column![text(message).size(14)].spacing(12).align_x(Alignment::Center);
    if let Some((label, message)) = action {
        content = content.push(
            button(text(i18n.__(label)).size(13))
                .padding([7, 12])
                .style(styles::quiet_button_style)
                .on_press(message),
        );
    }
    container(content).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
}

/// 按登录、加载、错误或成功状态展示“我的应用”。
pub(super) fn view(
    i18n: crate::app::i18n::I18n,
    state: &ShijuAppsLoadState<ShijuAppListDto>,
) -> Element<'static, Message> {
    match state {
        ShijuAppsLoadState::Idle => feedback(i18n, i18n.__("我的应用尚未加载").to_string(), None),
        ShijuAppsLoadState::NeedsLogin => feedback(
            i18n,
            i18n.__("登录识矩账户后可查看我的应用").to_string(),
            Some((i18n.__("登录"), Message::Account(AccountMessage::StartBrowserLogin))),
        ),
        ShijuAppsLoadState::Loading { .. } => {
            feedback(i18n, i18n.__("正在加载我的应用…").to_string(), None)
        }
        ShijuAppsLoadState::Error(error) => feedback(
            i18n,
            i18n.format("我的应用加载失败：%s", &[error.clone().into()]).expect("valid apps error"),
            Some((i18n.__("重试"), Message::ShijuApps(ShijuAppsMessage::Retry))),
        ),
        ShijuAppsLoadState::Loaded(apps) if apps.apps.is_empty() => {
            feedback(i18n, i18n.__("当前账户尚未安装应用").to_string(), None)
        }
        ShijuAppsLoadState::Loaded(apps) => {
            let cards = apps
                .apps
                .iter()
                .enumerate()
                .map(|(index, app)| installed_card(i18n, index, app))
                .collect::<Vec<_>>();
            scrollable(
                container(card_grid(cards))
                    .padding([0, styles::SCROLLABLE_SIDE_PADDING])
                    .width(Length::Fill),
            )
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
