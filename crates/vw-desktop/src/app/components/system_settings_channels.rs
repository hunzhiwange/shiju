//! 系统设置中 channels 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

mod basic_channels;
mod common;
mod enterprise_channels;
mod extended_channels;

#[cfg(test)]
mod basic_channels_tests;
#[cfg(test)]
mod common_tests;
#[cfg(test)]
mod enterprise_channels_tests;
#[cfg(test)]
mod extended_channels_tests;

use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_muted_text_style, settings_page_intro,
    settings_error_banner, settings_panel, settings_section_card, settings_text_input_style,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::state::ChannelsSettingsTab;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

use self::common::{LABEL_WIDTH, enabled_channels, number_row};

const CHANNEL_TAB_COLUMNS: usize = 5;

pub(super) fn channel_tab_descriptors() -> Vec<SettingsTabDescriptor<ChannelsSettingsTab>> {
    let mut tabs = vec![
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::Global,
            "全局",
            &["CLI 通道", "项目目录", "workspace", "消息超时"],
        ),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::Telegram,
            "Telegram",
            &["bot token", "allowed users"],
        ),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::Discord,
            "Discord",
            &["bot token", "guild"],
        ),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::Slack,
            "Slack",
            &["外部消息", "bot token", "app token"],
        ),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::Mattermost,
            "Mattermost",
            &["server url", "token"],
        ),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Webhook, "Webhook", &["webhook", "secret"]),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::IMessage,
            "iMessage",
            &["imessage", "联系人"],
        ),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Matrix, "Matrix", &["homeserver", "room"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Signal, "Signal", &["signal cli"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::WhatsApp, "WhatsApp", &["whatsapp"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Linq, "Linq", &["linq"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Wati, "WATI", &["wati"]),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::NextcloudTalk,
            "Nextcloud Talk",
            &["nextcloud", "talk"],
        ),
    ];
    #[cfg(not(target_arch = "wasm32"))]
    tabs.push(SettingsTabDescriptor::new(
        ChannelsSettingsTab::Email,
        "Email",
        &["email", "imap", "smtp"],
    ));
    tabs.extend([
        SettingsTabDescriptor::new(ChannelsSettingsTab::Irc, "IRC", &["irc"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Lark, "Lark", &["lark", "飞书"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Feishu, "Feishu", &["feishu", "飞书"]),
        SettingsTabDescriptor::new(
            ChannelsSettingsTab::DingTalk,
            "DingTalk",
            &["dingtalk", "钉钉"],
        ),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Qq, "QQ", &["qq"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::Nostr, "Nostr", &["nostr"]),
        SettingsTabDescriptor::new(ChannelsSettingsTab::ClawdTalk, "ClawdTalk", &["clawdtalk"]),
    ]);
    tabs
}

fn global_settings_panel(app: &App) -> Element<'_, Message> {
    let s = &app.channels_settings;

    settings_panel(
        column![
            row![
                text("CLI 通道").size(13).width(Length::Fixed(LABEL_WIDTH)),
                checkbox(s.cli)
                    .label("启用 CLI 通道")
                    .on_toggle(|next| {
                        Message::Settings(SettingsMessage::Channels(ChannelsMessage::CliToggled(
                            next,
                        )))
                    })
                    .style(settings_checkbox_style),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
            settings_divider(),
            row![
                text("项目目录").size(13).width(Length::Fixed(LABEL_WIDTH)),
                text_input("留空时按 workspace 推断", &s.project_dir_input)
                    .on_input(|next| {
                        Message::Settings(SettingsMessage::Channels(
                            ChannelsMessage::ProjectDirChanged(next),
                        ))
                    })
                    .padding([10, 12])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
            settings_divider(),
            number_row(
                "消息超时",
                s.message_timeout_secs.max(1),
                1,
                86_400,
                "秒",
                "global.message_timeout_secs",
            ),
        ]
        .spacing(12),
    )
    .into()
}

fn tab_button(
    tab: ChannelsSettingsTab,
    active_tab: ChannelsSettingsTab,
    label: &'static str,
) -> Element<'static, Message> {
    let active = tab == active_tab;
    debug_assert_eq!(label, tab.label());

    button(text(label).size(12))
        .padding([7, 12])
        .width(Length::Fill)
        .on_press(Message::Settings(SettingsMessage::Channels(ChannelsMessage::TabSelected(tab))))
        .style(move |theme: &iced::Theme, status| {
            let palette = theme.extended_palette();
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let background = if active {
                Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark {
                    0.24
                } else {
                    0.14
                })))
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                Some(Background::Color(palette.background.weak.color.scale_alpha(if is_dark {
                    0.7
                } else {
                    0.9
                })))
            } else {
                None
            };

            iced::widget::button::Style {
                background,
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.palette().text.scale_alpha(0.9)
                },
                border: Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: if active {
                        theme.palette().primary.scale_alpha(0.42)
                    } else if is_dark {
                        palette.background.strong.color.scale_alpha(0.8)
                    } else {
                        Color::from_rgba8(15, 23, 42, 0.1)
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

fn channel_tabs(active_tab: ChannelsSettingsTab) -> Element<'static, Message> {
    let tabs = channel_tab_descriptors();

    let mut rows = column![].spacing(8).width(Length::Fill);
    for tabs_in_row in tabs.chunks(CHANNEL_TAB_COLUMNS) {
        let mut tab_row = row![].spacing(8).width(Length::Fill);
        for descriptor in tabs_in_row {
            tab_row = tab_row.push(
                container(tab_button(descriptor.tab, active_tab, descriptor.label))
                    .width(Length::FillPortion(1)),
            );
        }
        for _ in tabs_in_row.len()..CHANNEL_TAB_COLUMNS {
            tab_row = tab_row.push(container(text("")).width(Length::FillPortion(1)));
        }
        rows = rows.push(tab_row);
    }

    rows.into()
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
    let enabled = enabled_channels(app);
    let enabled_text = if enabled.is_empty() {
        "当前未启用任何通道".to_string()
    } else {
        format!("已启用：{}", enabled.join("、"))
    };

    let active_tab = app.channels_settings.active_tab;
    let selected_panel: Element<'_, Message> = match active_tab {
        ChannelsSettingsTab::Global => global_settings_panel(app),
        ChannelsSettingsTab::Telegram => basic_channels::telegram_panel(app),
        ChannelsSettingsTab::Discord => basic_channels::discord_panel(app),
        ChannelsSettingsTab::Slack => basic_channels::slack_panel(app),
        ChannelsSettingsTab::Mattermost => basic_channels::mattermost_panel(app),
        ChannelsSettingsTab::Webhook => basic_channels::webhook_panel(app),
        ChannelsSettingsTab::IMessage => basic_channels::imessage_panel(app),
        ChannelsSettingsTab::Matrix => basic_channels::matrix_panel(app),
        ChannelsSettingsTab::Signal => basic_channels::signal_panel(app),
        ChannelsSettingsTab::WhatsApp => extended_channels::whatsapp_panel(app),
        ChannelsSettingsTab::Linq => extended_channels::linq_panel(app),
        ChannelsSettingsTab::Wati => extended_channels::wati_panel(app),
        ChannelsSettingsTab::NextcloudTalk => extended_channels::nextcloud_talk_panel(app),
        #[cfg(not(target_arch = "wasm32"))]
        ChannelsSettingsTab::Email => extended_channels::email_panel(app),
        #[cfg(target_arch = "wasm32")]
        ChannelsSettingsTab::Email => global_settings_panel(app),
        ChannelsSettingsTab::Irc => extended_channels::irc_panel(app),
        ChannelsSettingsTab::Lark => enterprise_channels::lark_panel(app),
        ChannelsSettingsTab::Feishu => enterprise_channels::feishu_panel(app),
        ChannelsSettingsTab::DingTalk => enterprise_channels::dingtalk_panel(app),
        ChannelsSettingsTab::Qq => enterprise_channels::qq_panel(app),
        ChannelsSettingsTab::Nostr => enterprise_channels::nostr_panel(app),
        ChannelsSettingsTab::ClawdTalk => enterprise_channels::clawdtalk_panel(app),
    };

    let mut content = column![
        settings_page_intro("通道配置", "集中配置 CLI 与外部消息通道的接入参数。"),
        settings_section_card("已启用通道", "快速查看当前已打开的入口，便于核对多通道部署状态。"),
        settings_panel(column![text(enabled_text).size(12).style(settings_muted_text_style)]),
        settings_section_card("通道", "选择一个通道进行配置。"),
        channel_tabs(active_tab),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = &app.channels_settings.save_error {
        content = content.push(settings_error_banner(error));
    }
    content = content.push(selected_panel);

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_channels_tests.rs"]
mod system_settings_channels_tests;
