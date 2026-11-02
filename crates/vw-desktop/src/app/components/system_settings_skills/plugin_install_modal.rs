//! Shared settings modal controls; only the explicit confirmation message activates a token.
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_error_banner, settings_modal_card,
    settings_modal_overlay, settings_text_input_style,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::{
    Message,
    message::SettingsMessage,
    state::{PluginInstallForm, PluginInstallPhase},
};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::ui::DesktopLocale;
fn event(message: SettingsMessage) -> Message {
    Message::Settings(message)
}
pub(super) fn view<'a>(
    locale: DesktopLocale,
    base: Element<'a, Message>,
    form: &'a PluginInstallForm,
) -> Element<'a, Message> {
    let close = event(SettingsMessage::PluginsInstallClose);
    let title = if form.update_id.is_some() {
        tr(locale, "更新插件")
    } else {
        tr(locale, "安装插件")
    };
    let input =
        text_input(tr(locale, "本地路径、git+URL#版本、npm:包@版本、插件@市场"), &form.source)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(
                (!form.busy())
                    .then_some(|value| event(SettingsMessage::PluginsInstallSourceChanged(value))),
            );
    let mut body = column![
        text(tr(locale, "来源")).size(14),
        input,
        text(if form.update_id.is_some() {
            tr(locale, "更新来源可留空以使用已登记市场；本地插件请提供新包路径。")
        } else {
            tr(locale, "只通过网关审计和安装，不在界面中拷贝文件。")
        })
        .size(12)
    ]
    .spacing(12);
    if let Some(preview) = form.preview() {
        body = body.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "插件：%s · %s → %s",
                    (preview.plugin_id).to_string(),
                    (preview.old_version.as_deref().unwrap_or(tr(locale, "未安装"))).to_string(),
                    (preview.version.as_deref().unwrap_or(tr(locale, "未声明"))).to_string()
                )
                .expect("valid skills display template"),
            )
            .size(14),
        );
        body = body.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "审计来源：%s",
                    (preview.source).to_string()
                )
                .expect("valid skills display template"),
            )
            .size(12)
            .wrapping(iced::widget::text::Wrapping::Glyph),
        );
        body = body.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(locale),
                    "本次声明：技能目录 %u · MCP 声明 %u · 钩子声明 %u · 界面 %s",
                    preview.capabilities.skills,
                    preview.capabilities.mcp_servers,
                    preview.capabilities.hooks,
                    (if preview.capabilities.interface {
                        tr(locale, "有")
                    } else {
                        tr(locale, "无")
                    })
                    .to_string()
                )
                .expect("valid skills display template"),
            )
            .size(13),
        );
        if let Some(old) = &preview.previous_capabilities {
            body = body.push(
                text(
                    crate::__!(
                        crate::app::i18n::I18n::new(locale),
                        "原声明：技能目录 %u · MCP 声明组 %u · 钩子声明组 %u · 界面 %s",
                        old.skills,
                        old.mcp_servers,
                        old.hooks,
                        (if old.interface { tr(locale, "有") } else { tr(locale, "无") })
                            .to_string()
                    )
                    .expect("valid skills display template"),
                )
                .size(12),
            );
        }
        body=body.push(text(tr(locale, "权限变化：安装/升级不授予新的执行权限；MCP、钩子和引导技能不会由安装动作自动运行。声明变化仍受宿主审批，内容信任须重新核对。")).size(13));
        body = body.push(
            text(tr(locale, super::plugin_dependency_summary(&preview.dependency_checks))).size(13),
        );
        for check in &preview.dependency_checks {
            body = body.push(
                text(format!("{} · {}：{}", check.component, check.dependency, check.detail))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Glyph),
            );
        }
    }
    match &form.phase {
        PluginInstallPhase::Preparing => body = body.push(text(tr(locale, "正在获取与审计原包…"))),
        PluginInstallPhase::Committing(_) => {
            body = body.push(text(tr(locale, "正在提交；请等待完成…")))
        }
        PluginInstallPhase::Failed(error) => {
            body = body.push(settings_error_banner(crate::app::i18n::I18n::new(locale).__(error)))
        }
        _ => {}
    }
    let prepare = button(text(if matches!(form.phase, PluginInstallPhase::Failed(_)) {
        tr(locale, "重试审计")
    } else {
        tr(locale, "审计预览")
    }))
    .padding([8, 14])
    .style(rounded_action_btn_style)
    .on_press_maybe((!form.busy()).then_some(event(SettingsMessage::PluginsInstallPrepare)));
    let cancel = button(text(tr(locale, "取消")))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            (!matches!(form.phase, PluginInstallPhase::Committing(_))).then_some(close.clone()),
        );
    let confirm = button(text(if form.update_id.is_some() {
        tr(locale, "确认更新")
    } else {
        tr(locale, "确认安装")
    }))
    .padding([8, 14])
    .style(primary_action_btn_style)
    .on_press_maybe(form.can_confirm().then_some(event(SettingsMessage::PluginsInstallConfirm)));
    let content = column![
        row![
            text(title).size(22),
            Space::new().width(Length::Fill),
            settings_close_button(close.clone())
        ]
        .align_y(Alignment::Center),
        scrollable(container(body).padding([0, 7]).width(Length::Fill))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4)
            ))
            .height(Length::Fill),
        row![prepare, Space::new().width(Length::Fill), cancel, confirm]
            .spacing(10)
            .align_y(Alignment::Center)
    ]
    .spacing(16)
    .height(Length::Fill);
    settings_modal_overlay(
        Some(base),
        close,
        settings_modal_card(content).width(Length::Fixed(820.)).height(Length::Fixed(620.)),
    )
}
