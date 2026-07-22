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
    settings_panel, settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

use self::common::{LABEL_WIDTH, enabled_channels, number_row};

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

    let mut content = column![
        settings_page_intro("通道配置", "集中配置 CLI 与外部消息通道的接入参数。"),
        settings_section_card("已启用通道", "快速查看当前已打开的入口，便于核对多通道部署状态。"),
        settings_panel(column![text(enabled_text).size(12).style(settings_muted_text_style)]),
        settings_section_card(
            "全局参数",
            "这些参数对所有消息通道生效，包括固定项目目录和消息超时预算。"
        ),
        global_settings_panel(app),
    ]
    .spacing(16)
    .width(Length::Fill);

    content = content.push(basic_channels::telegram_panel(app));
    content = content.push(basic_channels::discord_panel(app));
    content = content.push(basic_channels::slack_panel(app));
    content = content.push(basic_channels::mattermost_panel(app));
    content = content.push(basic_channels::webhook_panel(app));
    content = content.push(basic_channels::imessage_panel(app));
    content = content.push(basic_channels::matrix_panel(app));
    content = content.push(basic_channels::signal_panel(app));
    content = content.push(extended_channels::whatsapp_panel(app));
    content = content.push(extended_channels::linq_panel(app));
    content = content.push(extended_channels::wati_panel(app));
    content = content.push(extended_channels::nextcloud_talk_panel(app));
    #[cfg(not(target_arch = "wasm32"))]
    {
        content = content.push(extended_channels::email_panel(app));
    }
    content = content.push(extended_channels::irc_panel(app));
    content = content.push(enterprise_channels::lark_panel(app));
    content = content.push(enterprise_channels::feishu_panel(app));
    content = content.push(enterprise_channels::dingtalk_panel(app));
    content = content.push(enterprise_channels::qq_panel(app));
    content = content.push(enterprise_channels::nostr_panel(app));
    content = content.push(enterprise_channels::clawdtalk_panel(app));

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_channels_tests.rs"]
mod system_settings_channels_tests;
