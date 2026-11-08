//! Shared settings modal controls; only the explicit confirmation message activates a token.
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_control_with_hint, settings_error_banner, settings_field_label, settings_group_card,
    settings_group_heading_with_help, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_text_input,
};
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::{
    App, Message,
    message::SettingsMessage,
    state::{PluginInstallForm, PluginInstallPhase},
};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element};
fn event(message: SettingsMessage) -> Message {
    Message::Settings(message)
}
pub(super) fn view<'a>(
    app: &App,
    base: Element<'a, Message>,
    form: &'a PluginInstallForm,
) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let close = event(SettingsMessage::PluginsInstallClose);
    let title = if form.update_id.is_some() {
        tr(locale, "更新插件")
    } else {
        tr(locale, "安装插件")
    };
    let input = settings_text_input("", &form.source).on_input_maybe(
        (!form.busy())
            .then_some(|value| event(SettingsMessage::PluginsInstallSourceChanged(value))),
    );
    let hint = if form.update_id.is_some() {
        tr(locale, "更新来源可留空以使用已登记市场；本地插件请提供新包路径。")
    } else {
        tr(locale, "只通过网关审计和安装，不在界面中拷贝文件。")
    };
    let mut body =
        column![settings_field_label(tr(locale, "来源")), settings_control_with_hint(input, hint),]
            .spacing(8);
    if let Some(preview) = form.preview() {
        let mut review = column![].spacing(10);
        review = review.push(
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
        review = review.push(
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
        review = review.push(
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
            review = review.push(
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
        review = review.push(
            text(tr(locale, "权限变化：安装/升级不授予新的执行权限；MCP、钩子和引导技能不会由安装动作自动运行。声明变化仍受宿主审批，内容信任须重新核对。"))
                .size(13)
                .style(settings_muted_text_style),
        );
        review = review.push(
            text(tr(locale, super::plugin_dependency_summary(&preview.dependency_checks))).size(13),
        );
        for check in &preview.dependency_checks {
            review = review.push(
                text(format!("{} · {}：{}", check.component, check.dependency, check.detail))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Glyph),
            );
        }
        body = body.push(settings_group_card(container(review).padding([14, 0])));
    }
    match &form.phase {
        PluginInstallPhase::Preparing => {
            body = body.push(
                text(tr(locale, "正在获取与审计原包…")).size(13).style(settings_muted_text_style),
            )
        }
        PluginInstallPhase::Committing(_) => {
            body = body.push(
                text(tr(locale, "正在提交；请等待完成…")).size(13).style(settings_muted_text_style),
            )
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
    .padding([8, 10])
    .style(rounded_action_btn_style)
    .on_press_maybe((!form.busy()).then_some(event(SettingsMessage::PluginsInstallPrepare)));
    let cancel = button(text(tr(locale, "取消")))
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            (!matches!(form.phase, PluginInstallPhase::Committing(_))).then_some(close.clone()),
        );
    let confirm = button(text(if form.update_id.is_some() {
        tr(locale, "确认更新")
    } else {
        tr(locale, "确认安装")
    }))
    .padding([8, 10])
    .style(primary_action_btn_style)
    .on_press_maybe(form.can_confirm().then_some(event(SettingsMessage::PluginsInstallConfirm)));
    let header = settings_modal_header(
        settings_group_heading_with_help(
            I18n::new(locale),
            &app.app_theme,
            title,
            tr(locale, "本地路径、git+URL#版本、npm:包@版本、插件@市场"),
        ),
        settings_close_button(close.clone()),
    );
    let actions = row![prepare, cancel, confirm].spacing(8).align_y(Alignment::Center).wrap();
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(super::modal_card_style)
        .width(820)
        .height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
