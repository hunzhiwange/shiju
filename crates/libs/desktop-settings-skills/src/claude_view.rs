//! Credential-free Claude configuration controls using the shared modal layout.
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Border, Element, Length, Theme,
    widget::{button, column, container, row, text},
};
use vw_api_app_skills::claude_api::*;

pub fn view<'a, M: Clone + 'static>(
    s: ViewState<'a>,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let (status, marker_hint) = match s.status {
        None => ("尚未读取", "刷新后可查看受管标记的状态。"),
        Some(ClaudeConfigStatus::Missing) => ("无配置文件", "启用时将写入受管标记。"),
        Some(ClaudeConfigStatus::Invalid) => ("配置无效", "修复 Claude 配置文件后刷新。"),
        Some(ClaudeConfigStatus::Valid { managed: true, .. }) => {
            ("已启用", "Claude 插件的安装和启用状态单独管理。")
        }
        Some(ClaudeConfigStatus::Valid { primary_api_key_present: true, .. }) => {
            ("存在外部凭据", "请先在原配置来源处理凭据冲突，再刷新。")
        }
        Some(ClaudeConfigStatus::Valid { .. }) => ("未启用", "此标记用于识别受管配置。"),
    };
    let editable = !s.busy
        && matches!(
            s.status,
            Some(ClaudeConfigStatus::Missing)
                | Some(ClaudeConfigStatus::Valid { primary_api_key_present: false, .. })
                | Some(ClaudeConfigStatus::Valid { managed: true, .. })
        );
    let control = |label, event| {
        let enabled = if matches!(event, Event::Onboarding(_)) { !s.busy } else { editable };
        button(text(label).size(13))
            .padding([7, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let status = container(text(status).size(12)).padding([4, 9]).style(move |theme: &Theme| {
        let palette = theme.extended_palette();
        let pair = match s.status {
            Some(ClaudeConfigStatus::Valid { managed: true, .. }) => palette.success.weak,
            Some(ClaudeConfigStatus::Valid { primary_api_key_present: true, .. }) => {
                palette.warning.weak
            }
            Some(ClaudeConfigStatus::Invalid) => palette.danger.weak,
            _ => palette.background.weak,
        };
        container::Style {
            background: Some(pair.color.into()),
            text_color: Some(pair.text),
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..container::Style::default()
        }
    });
    let section = |content| {
        container(content).width(Length::Fill).padding(14).style(settings_section_card_style)
    };
    let mut body = column![].spacing(12);
    if let Some(error) = s.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if let Some(notice) = s.notice {
        body = body.push(settings_success_banner(notice.as_str()));
    }
    body = body
        .push(section(
            column![
                row![settings_field_label("受管标记"), status]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .wrap(),
                text(marker_hint).size(13).style(settings_muted_text_style),
                row![
                    control("启用受管标记", Event::Marker(ClaudeConfigAction::Enable)),
                    control("清除受管标记", Event::Marker(ClaudeConfigAction::Disable)),
                ]
                .spacing(8)
                .wrap(),
            ]
            .spacing(10),
        ))
        .push(section(
            column![
                settings_field_label("首次启动确认"),
                text("选择 Claude 首次启动时是否显示确认。此处暂无法查看当前设置。")
                    .size(13)
                    .style(settings_muted_text_style),
                row![
                    control("跳过首次确认", Event::Onboarding(ClaudeConfigAction::Enable)),
                    control("恢复首次确认", Event::Onboarding(ClaudeConfigAction::Disable)),
                ]
                .spacing(8)
                .wrap(),
            ]
            .spacing(10),
        ));
    let close = message(Event::Close);
    let header = settings_modal_header(
        row![
            container(icon_svg(Icon::AppClaudeCode, 24.0))
                .center_x(40)
                .center_y(40)
                .style(settings_icon_tile_style),
            text("Claude 配置").size(22),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        settings_close_button(close.clone()),
    );
    let actions = row![
        button(text("刷新状态").size(13))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button(text("关闭").size(13))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ]
    .spacing(8);
    let mut footer = row![].align_y(Alignment::Center).spacing(8);
    if s.busy {
        footer = footer.push(text("正在同步配置…").size(13).style(settings_muted_text_style));
    }
    let footer = footer.push(container(actions).align_right(Length::Fill));
    let card = settings_modal_layout(
        container(header).padding([0, 7]),
        body,
        Some(container(footer).padding([0, 7]).into()),
    )
    .style(super::modal_card_style)
    .width(580)
    .height(460);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
#[derive(Clone, Copy)]
pub struct ViewState<'a> {
    pub open: bool,
    pub busy: bool,
    pub status: &'a Option<vw_api_app_skills::claude_api::ClaudeConfigStatus>,
    pub notice: &'a Option<String>,
    pub error: &'a Option<String>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Refresh,
    Marker(vw_api_app_skills::claude_api::ClaudeConfigAction),
    Onboarding(vw_api_app_skills::claude_api::ClaudeConfigAction),
}
