//! 独立 Web 安装弹窗，复用设置弹窗与主题控件。
use super::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_field_label, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_select,
};
use crate::app::i18n::I18n;
use crate::app::message::ViewMessage;
use crate::app::web_install::Event;
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.web_install;
    if !state.visible {
        return base;
    }
    let i18n = I18n::new(app.desktop_locale);
    let message = |event| Message::View(ViewMessage::WebInstall(event));
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("安装 Web 版本")).size(20),
        settings_close_button(close.clone()),
    );
    let label = match app.desktop_update.source {
        Some(vw_shared::update::UpdateSource::Shiju) => Some(i18n.__("shiju（默认）")),
        Some(vw_shared::update::UpdateSource::Github) => Some("github.com"),
        None => None,
    };
    let source_control: Element<'a, Message> =
        if state.busy || app.desktop_update.busy || app.desktop_update.saving {
            text(label.unwrap_or(i18n.__("请选择来源"))).size(SETTINGS_CONTROL_TEXT_SIZE).into()
        } else {
            settings_select([i18n.__("shiju（默认）"), "github.com"], label, |label| {
                Message::View(ViewMessage::AppUpdateSourceSelected(if label == "github.com" {
                    vw_shared::update::UpdateSource::Github
                } else {
                    vw_shared::update::UpdateSource::Shiju
                }))
            })
            .placeholder(i18n.__("请选择来源"))
            .width(Length::Fill)
            .into()
        };
    let mut body = column![
        text(i18n.__("Web 版本不随应用打包，将从所选来源下载最新正式版并校验后安装。")).size(14),
        text(i18n.__("需要 Node.js 22 或以上版本。服务仅供本机访问，业务功能使用 ShiJu 网关。"))
            .size(13)
            .style(settings_muted_text_style),
        text(i18n.__("关闭弹窗后服务继续运行；退出应用时自动停止。"))
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(14);
    body = body.push(column![settings_field_label(i18n.__("升级来源")), source_control].spacing(6));
    if let Some(path) = &state.path {
        body = body
            .push(text(i18n.__("安装目录")).size(13))
            .push(text(path.display().to_string()).size(13));
    } else {
        body = body.push(text(i18n.__("尚未安装 Web 版本")).size(13));
    }
    if let Some(url) = &state.url {
        body = body.push(text(i18n.__("访问地址")).size(13)).push(text(url).size(14));
    }
    if !state.status.is_empty() {
        body = body.push(text(i18n.__(state.status.as_str()).to_owned()).size(13));
    }
    let install = button(text(i18n.__("安装并启动")).size(13))
        .style(primary_action_btn_style)
        .padding([9, 16])
        .on_press_maybe(
            (!state.busy
                && state.url.is_none()
                && app.desktop_update.source.is_some()
                && !app.desktop_update.saving)
                .then(|| message(Event::InstallAndStart)),
        );
    let start = button(text(i18n.__("启动 / 打开 Web")).size(13))
        .style(rounded_action_btn_style)
        .padding([9, 16])
        .on_press_maybe((!state.busy && state.path.is_some()).then(|| message(Event::Start)));
    let stop = button(text(i18n.__("停止")).size(13))
        .style(rounded_action_btn_style)
        .padding([9, 16])
        .on_press_maybe((!state.busy && state.url.is_some()).then(|| message(Event::Stop)));
    let footer = row![install, start, stop].spacing(8).wrap();
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fixed(620.0))
        .height(Length::Fixed(460.0));
    settings_modal_overlay(Some(base), close, card)
}
