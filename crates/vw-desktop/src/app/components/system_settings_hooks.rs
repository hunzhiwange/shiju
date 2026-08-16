//! 钩子配置设置界面组件
//!
//! 本模块提供 钩子配置的图形化设置入口，用于管理运行时 hooks 总开关、
//! 当前内置钩子、插件 hook 信任状态与安全审计记录。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_control_with_hint,
    settings_error_banner, settings_muted_text_style, settings_page_intro, settings_panel,
    settings_section_card, settings_segment_button_style, settings_value_badge,
};
use crate::app::message::settings::HooksMessage;
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn enabled_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::EnabledToggled(value)))
}

fn command_logger_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(value)))
}

fn plugin_hooks_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::PluginHooksToggled(value)))
}

fn catalog_refresh_requested() -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::CatalogRefresh))
}

fn audit_refresh_requested() -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::AuditRefresh))
}

fn trust_requested(entry: &vw_gateway_client::HookCatalogEntryDto) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::TrustRequested {
        plugin_id: entry.plugin_id.clone(),
        hook_hash: entry.hook_hash.clone(),
    }))
}

fn revoke_requested(entry: &vw_gateway_client::HookCatalogEntryDto) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::RevokeRequested {
        plugin_id: entry.plugin_id.clone(),
        hook_hash: entry.hook_hash.clone(),
    }))
}

fn refresh_button<'a>(label: &'static str, loading: bool, message: Message) -> Element<'a, Message> {
    let mut control = button(text(if loading { "加载中…" } else { label }).size(12))
        .padding([7, 12])
        .style(|theme, status| settings_segment_button_style(theme, status, false));
    if !loading {
        control = control.on_press(message);
    }
    control.into()
}

fn hook_catalog_card<'a>(
    entry: &'a vw_gateway_client::HookCatalogEntryDto,
    mutation_loading: bool,
) -> Element<'a, Message> {
    let status = if entry.trusted { "已信任" } else { "待审核" };
    let args = if entry.args.is_empty() { "（无）".to_string() } else { entry.args.join(" ") };
    let action_message = if entry.trusted { revoke_requested(entry) } else { trust_requested(entry) };
    let action_label = if entry.trusted { "撤销信任" } else { "信任" };
    let mut action = button(text(action_label).size(12))
        .padding([7, 12])
        .style(|theme, status| settings_segment_button_style(theme, status, false));
    if !mutation_loading {
        action = action.on_press(action_message);
    }

    settings_panel(
        column![
            row![
                text(&entry.plugin_id).size(14).width(Length::Fill),
                settings_value_badge(status),
                action,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("事件").size(11).style(settings_muted_text_style),
                text(&entry.event).size(12),
                text("模式").size(11).style(settings_muted_text_style),
                text(&entry.mode).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("命令").size(11).style(settings_muted_text_style),
                text(&entry.command).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("参数").size(11).style(settings_muted_text_style),
                text(args).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("工作目录").size(11).style(settings_muted_text_style),
                text(&entry.working_dir).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(9)
        .width(Length::Fill),
    )
    .into()
}

fn hook_catalog_panel(app: &App) -> Element<'_, Message> {
    let s = &app.hooks_settings;
    let mut rows = column![].spacing(10).width(Length::Fill);
    if s.catalog.is_empty() {
        let message = if s.catalog_loading { "正在加载插件 hooks…" } else { "未发现插件 hooks" };
        rows = rows.push(
            container(text(message).size(12).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding(18),
        );
    } else {
        for entry in &s.catalog {
            rows = rows.push(hook_catalog_card(entry, s.mutation_loading));
        }
    }

    let list = scrollable(container(rows).padding(iced::Padding::default().right(8.0)))
        .height(Length::Shrink)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    container(list).max_height(360).width(Length::Fill).into()
}

fn audit_panel(app: &App) -> Element<'_, Message> {
    let s = &app.hooks_settings;
    let mut rows = column![].spacing(8).width(Length::Fill);
    if s.audit.is_empty() {
        let message = if s.audit_loading { "正在加载审计记录…" } else { "暂无审计记录" };
        rows = rows.push(
            container(text(message).size(12).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding(18),
        );
    } else {
        for entry in &s.audit {
            let status = if entry.success { "成功" } else { "失败" };
            rows = rows.push(settings_panel(
                row![
                    text(&entry.timestamp).size(12),
                    text(&entry.tool_name).size(12).width(Length::Fill),
                    text(format!("{} ms", entry.duration_ms)).size(12),
                    settings_value_badge(status),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ));
        }
    }

    let list = scrollable(container(rows).padding(iced::Padding::default().right(8.0)))
        .height(Length::Shrink)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    container(list).max_height(260).width(Length::Fill).into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.hooks_settings;

    let enabled_row = field_row(
        "启用",
        "控制是否开启运行时 hooks 执行。",
        settings_control_with_hint(
            checkbox(s.enabled)
                .label("开启运行时 hooks 执行")
                .on_toggle(enabled_toggled)
                .style(settings_checkbox_style),
            "关闭总开关后将暂停 hooks 执行，但会保留内置钩子的具体启用状态。",
        ),
    );

    let builtin_header = settings_section_card(
        "内置钩子",
        "当前提供审计类内置 hooks。后续新增内置项时会继续在这里扩展。",
    );

    let command_logger_row = field_row(
        "command_logger",
        "记录命令调用信息，便于审计与问题回溯。",
        checkbox(s.command_logger).on_toggle(command_logger_toggled).style(settings_checkbox_style),
    );

    let plugin_hooks_row = field_row(
        "插件 hooks",
        "仅加载已发现且已信任的插件 hook 声明。",
        settings_control_with_hint(
            checkbox(s.plugins_enabled)
                .label("允许执行可信插件 hooks")
                .on_toggle(plugin_hooks_toggled)
                .style(settings_checkbox_style),
            "待审核声明不会执行；信任与撤销均由服务端按当前稳定哈希校验。",
        ),
    );

    let catalog_header = row![
        settings_section_card("插件 hook 审核", "查看插件声明并管理当前哈希对应的信任状态。")
            .width(Length::Fill),
        refresh_button("刷新清单", s.catalog_loading, catalog_refresh_requested()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let audit_header = row![
        settings_section_card(
            "审计记录",
            "仅显示时间、工具名、耗时和成功状态，不包含参数或结果正文。",
        )
        .width(Length::Fill),
        refresh_button("刷新审计", s.audit_loading, audit_refresh_requested()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut content = column![
        settings_page_intro("钩子配置", "配置运行时 hooks 总开关与内置钩子的启用状态。"),
        settings_section_card("基础行为", "关闭总开关后将暂停 hooks 执行。"),
        settings_panel(column![enabled_row].spacing(0)),
        builtin_header,
        settings_panel(column![command_logger_row].spacing(0)),
        settings_section_card("插件 hooks", "控制可信插件 hooks 的执行并审核当前声明。"),
        settings_panel(column![plugin_hooks_row].spacing(0)),
        catalog_header,
        hook_catalog_panel(app),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = &s.catalog_error {
        content = content.push(settings_error_banner(error));
    }
    if let Some(error) = &s.mutation_error {
        content = content.push(settings_error_banner(error));
    }
    content = content.push(audit_header).push(audit_panel(app));
    if let Some(error) = &s.audit_error {
        content = content.push(settings_error_banner(error));
    }

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_hooks_tests.rs"]
mod system_settings_hooks_tests;
