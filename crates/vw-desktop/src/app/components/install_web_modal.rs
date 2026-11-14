//! 独立 Web 安装弹窗，复用设置弹窗与主题控件。
use super::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_field_label, settings_hover_help_label, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_select,
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
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            i18n.__("
                "),
            text(i18n.__("安装 Web 版本")).size(20),
            [
                i18n.__("从所选来源下载最新正式版 ZIP 并安装。"),
                i18n.__("本机启动需要 Node.js 22 或以上版本，仅供本机访问。关闭弹窗后继续运行，退出应用时停止。"),
                i18n.__("打开 Web 目录可查看文件和 README.md，也可将整个目录部署到静态服务器，无需 Node.js。"),
                i18n.__("静态部署需使用 HTTPS（本机回环地址除外），正确设置 JS/WASM 类型，并另行配置网关地址、认证及跨域来源。"),
            ].join("\n\n"),
        ),
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
    let mut body =
        column![column![settings_field_label(i18n.__("升级来源")), source_control].spacing(6)]
            .spacing(14);
    if let Some(path) = &state.path {
        body = body
            .push(
                row![
                    text(i18n.__("安装目录")).size(13),
                    button(
                        text(i18n.__(if cfg!(target_os = "macos") {
                            "在 Finder 中显示"
                        } else {
                            "打开 Web 目录"
                        }))
                        .size(13),
                    )
                    .style(rounded_action_btn_style)
                    .padding([6, 12])
                    .on_press_maybe((!state.busy).then(|| message(Event::OpenDirectory))),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            )
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
    if let Some(progress) = &state.progress {
        body = body.push(super::install_cli_modal::update_progress_block_localized(
            i18n,
            progress,
            app.last_copied_code_hash,
        ));
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
