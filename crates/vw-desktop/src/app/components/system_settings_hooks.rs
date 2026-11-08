//! 钩子配置设置界面组件
//!
//! 本模块提供 钩子配置的图形化设置入口，用于管理运行时 hooks 总开关、
//! 当前内置钩子、插件 hook 信任状态与安全审计记录。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_control_with_hint, settings_error_banner, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading, settings_group_heading_with_help,
    settings_muted_text_style, settings_page_heading_with_help, settings_panel,
    settings_row_with_flexible_control, settings_value_badge,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::HooksMessage;
use crate::app::{App, Message, i18n::I18n, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text::Wrapping};
use iced::{Alignment, Element, Length};
use vw_config_types::ui::DesktopLocale;

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 320.0, control)
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

fn refresh_button<'a>(
    locale: DesktopLocale,
    label: &'static str,
    loading: bool,
    message: Message,
) -> Element<'a, Message> {
    let mut control = button(text(if loading { tr(locale, "加载中…") } else { label }).size(12))
        .padding([7, 12])
        .style(rounded_action_btn_style);
    if !loading {
        control = control.on_press(message);
    }
    control.into()
}

fn hook_catalog_card<'a>(
    locale: DesktopLocale,
    entry: &'a vw_gateway_client::HookCatalogEntryDto,
    mutation_loading: bool,
) -> Element<'a, Message> {
    let status = if entry.trusted { tr(locale, "已信任") } else { tr(locale, "待审核") };
    let args = if entry.args.is_empty() {
        tr(locale, "（无）").to_string()
    } else {
        entry.args.join(" ")
    };
    let action_message =
        if entry.trusted { revoke_requested(entry) } else { trust_requested(entry) };
    let action_label =
        if entry.trusted { tr(locale, "撤销信任") } else { tr(locale, "信任") };
    let mut action = button(text(action_label).size(12)).padding([7, 12]).style(if entry.trusted {
        rounded_action_btn_style
    } else {
        primary_action_btn_style
    });
    if !mutation_loading {
        action = action.on_press(action_message);
    }

    settings_panel(
        column![
            row![
                text(&entry.plugin_id).size(14).width(Length::Fill).wrapping(Wrapping::Glyph),
                settings_value_badge(status),
                action,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text(tr(locale, "事件")).size(11).style(settings_muted_text_style),
                text(&entry.event).size(12),
                text(tr(locale, "模式")).size(11).style(settings_muted_text_style),
                text(&entry.mode).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            catalog_detail(tr(locale, "命令"), &entry.command),
            catalog_detail(tr(locale, "参数"), args),
            catalog_detail(tr(locale, "工作目录"), &entry.working_dir),
        ]
        .spacing(9)
        .width(Length::Fill),
    )
    .into()
}

fn catalog_detail<'a>(
    label: &'static str,
    value: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    row![
        text(label).size(12).style(settings_muted_text_style),
        text(value).size(13).width(Length::Fill).wrapping(Wrapping::Glyph),
    ]
    .spacing(8)
    .into()
}

fn hook_catalog_panel(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.hooks_settings;
    let mut rows = column![].spacing(10).width(Length::Fill);
    if s.catalog.is_empty() {
        let message = if s.catalog_loading {
            tr(locale, "正在加载插件 hooks…")
        } else {
            tr(locale, "未发现插件 hooks")
        };
        rows = rows.push(
            container(text(message).size(12).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding(18),
        );
    } else {
        for entry in &s.catalog {
            rows = rows.push(hook_catalog_card(locale, entry, s.mutation_loading));
        }
    }

    let list = scrollable(container(rows).padding([0, 7]))
        .height(Length::Shrink)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    container(list).max_height(360).width(Length::Fill).into()
}

fn audit_panel(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.hooks_settings;
    let mut rows = column![].spacing(8).width(Length::Fill);
    if s.audit.is_empty() {
        let message = if s.audit_loading {
            tr(locale, "正在加载审计记录…")
        } else {
            tr(locale, "暂无审计记录")
        };
        rows = rows.push(
            container(text(message).size(12).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding(18),
        );
    } else {
        for entry in &s.audit {
            let status = if entry.success { tr(locale, "成功") } else { tr(locale, "失败") };
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

    let list = scrollable(container(rows).padding([0, 7]))
        .height(Length::Shrink)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
    container(list).max_height(260).width(Length::Fill).into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let i18n = I18n::new(locale);
    let theme = &app.app_theme;
    let s = &app.hooks_settings;

    let enabled_row = settings_flexible_row_with_help(
        i18n,
        theme,
        tr(locale, "启用"),
        tr(locale, "控制是否开启运行时 hooks 执行。"),
        320.0,
        settings_control_with_hint(
            checkbox(s.enabled)
                .label(tr(locale, "开启运行时 hooks 执行"))
                .on_toggle(enabled_toggled)
                .style(settings_checkbox_style),
            tr(locale, "关闭总开关后将暂停 hooks 执行，但会保留内置钩子的具体启用状态。"),
        ),
    );

    let builtin_header = settings_group_heading_with_help(
        i18n,
        theme,
        tr(locale, "内置钩子"),
        tr(locale, "当前提供审计类内置 hooks。后续新增内置项时会继续在这里扩展。"),
    );

    let command_logger_row = settings_flexible_row_with_help(
        i18n,
        theme,
        "command_logger",
        tr(locale, "记录命令调用信息，便于审计与问题回溯。"),
        320.0,
        checkbox(s.command_logger).on_toggle(command_logger_toggled).style(settings_checkbox_style),
    );

    let plugin_hooks_row = field_row(
        tr(locale, "插件 hooks"),
        tr(locale, "仅加载已发现且已信任的插件 hook 声明。"),
        settings_control_with_hint(
            checkbox(s.plugins_enabled)
                .label(tr(locale, "允许执行可信插件 hooks"))
                .on_toggle(plugin_hooks_toggled)
                .style(settings_checkbox_style),
            tr(locale, "待审核声明不会执行；信任与撤销均由服务端按当前稳定哈希校验。"),
        ),
    );

    let catalog_header = row![
        container(settings_group_heading_with_help(
            i18n,
            theme,
            tr(locale, "插件 hook 审核"),
            tr(locale, "查看插件声明并管理当前哈希对应的信任状态。")
        ))
        .width(Length::Fill),
        refresh_button(
            locale,
            tr(locale, "刷新清单"),
            s.catalog_loading,
            catalog_refresh_requested()
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let audit_header = row![
        container(settings_group_heading_with_help(
            i18n,
            theme,
            tr(locale, "审计记录"),
            tr(locale, "仅显示时间、工具名、耗时和成功状态，不包含参数或结果正文。"),
        ))
        .width(Length::Fill),
        refresh_button(locale, tr(locale, "刷新审计"), s.audit_loading, audit_refresh_requested()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut content = column![
        settings_page_heading_with_help(
            i18n,
            theme,
            tr(locale, "钩子配置"),
            tr(locale, "配置运行时 hooks 总开关与内置钩子的启用状态。")
        ),
        settings_group_heading(
            tr(locale, "基础行为"),
            tr(locale, "关闭总开关后将暂停 hooks 执行。")
        ),
        settings_group_card(enabled_row),
        builtin_header,
        settings_group_card(command_logger_row),
        settings_group_heading_with_help(
            i18n,
            theme,
            tr(locale, "插件 hooks"),
            tr(locale, "控制可信插件 hooks 的执行并审核当前声明。")
        ),
        settings_group_card(plugin_hooks_row),
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

#[cfg(test)]
#[path = "system_settings_hooks_ui_refresh_28_tests.rs"]
mod ui_refresh_28_tests;
