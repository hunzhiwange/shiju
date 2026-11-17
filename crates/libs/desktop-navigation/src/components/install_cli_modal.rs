//! CLI 安装提示模态框组件
//!
//! 本模块提供 CLI（命令行工具）安装提示的模态框界面组件。
//! 当检测到 CLI 未安装或需要用户安装时，使用此模态框向用户
//! 展示安装相关信息并引导用户完成安装流程。
//!
//! # 组件特性
//!
//! - 居中显示的卡片式模态框
//! - 带有品牌 Logo 标识
//! - 可自定义标题和内容文本
//! - 半透明背景遮罩层
//! - 点击遮罩或关闭按钮可关闭模态框

use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_field_label, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_muted_text_style, settings_select,
};
use super::message::{Message, ViewMessage, HelpRenderer, settings_hover_help_label};
use iced::alignment::Horizontal;
use iced::widget::{
    Space, button, column, container, image, row, text, text::Wrapping,
};
use iced::{Alignment, ContentFit, Element, Length, Theme};
use super::install_cli_modal_progress::update_value;
pub(crate) use super::install_cli_modal_progress::{update_progress_block_localized, logo_badge_style, hash_copy_text};
#[cfg(test)] pub(crate) use super::install_cli_modal_progress::{update_progress_card_style, format_bytes, format_rate, compact_url};
#[cfg(test)] pub(crate) use super::install_cli_modal_progress::update_progress_block;

const HEADER_LOGO_IMAGE_SIZE: f32 = 76.0;
const HEADER_LOGO_BADGE_SIZE: f32 = 88.0;
const HEADER_LOGO_BADGE_RADIUS: f32 = 20.0;
const UPDATE_LOGO_IMAGE_SIZE: f32 = 144.0;
const UPDATE_LOGO_BADGE_SIZE: f32 = 160.0;
const UPDATE_LOGO_BADGE_RADIUS: f32 = 26.0;

/// 渲染 CLI 安装提示模态框
///
/// 创建一个居中显示的模态框，用于提示用户安装 CLI 工具。
/// 模态框包含品牌 Logo、标题、内容文本和关闭按钮，
/// 并带有半透明的背景遮罩层。
///
/// # 参数
///
/// * `title` - 模态框标题文本，通常为"安装 CLI"或类似的提示文字
/// * `content` - 模态框内容文本，详细说明安装步骤或相关说明
///
/// # 返回值
///
/// 返回一个 `Element<Message>` 类型的 UI 元素，可直接嵌入到
/// 应用程序的视图层级中。
///
/// # 示例
///
/// ```ignore
/// use desktop_navigation::components::install_cli_modal;
///
/// let modal = install_cli_modal::view(
///     "安装 VibeWindow CLI",
///     "请按照以下步骤安装 CLI 工具..."
/// );
/// ```
///
/// # UI 结构
///
/// 模态框采用层叠布局：
/// 1. 底层：半透明黑色遮罩（点击可关闭）
/// 2. 顶层：居中的白色卡片容器
///    - 品牌 Logo 图标
///    - 标题文本
///    - 内容描述文本
///    - 关闭按钮
pub fn view<'a>(title: &'a str, content: &'a str, help: HelpRenderer) -> Element<'a, Message> {
    view_inner(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        title,
        content,
        None,
        None,
        false,
        false,
        false,
        title == "CLI 安装完成",
        None,
        None,
        None,
        None,
        0,
        help,
    )
}

pub fn view_localized<'a>(
    i18n: desktop_i18n::I18n,
    title: &'a str,
    content: &'a str,
    help: HelpRenderer,
) -> Element<'a, Message> {
    view_inner(
        i18n,
        title,
        content,
        None,
        None,
        false,
        false,
        false,
        title == "CLI 安装完成",
        None,
        None,
        None,
        None,
        0,
        help,
    )
}

pub fn view_update_check<'a>(
    title: &'a str,
    content: &'a str,
    current_version: &'a str,
    server_version: &'a str,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
    help: HelpRenderer,
) -> Element<'a, Message> {
    view_update_check_with_progress(
        title,
        content,
        current_version,
        server_version,
        is_checking,
        show_install_action,
        use_app_update_action,
        None,
        None,
        help,
    )
}

pub fn view_update_check_with_progress<'a>(
    title: &'a str,
    content: &'a str,
    current_version: &'a str,
    server_version: &'a str,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
    update_progress: Option<&'a update_types::UpdateProgress>,
    last_copied_code_hash: Option<u64>,
    help: HelpRenderer,
) -> Element<'a, Message> {
    view_inner(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        title,
        content,
        Some(current_version),
        Some(server_version),
        is_checking,
        show_install_action,
        use_app_update_action,
        false,
        update_progress,
        last_copied_code_hash,
        None,
        None,
        0,
        help,
    )
}

pub fn view_inner<'a>(
    i18n: desktop_i18n::I18n,
    title: &'a str,
    content: &'a str,
    current_version: Option<&'a str>,
    server_version: Option<&'a str>,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
    show_logo_above_content: bool,
    update_progress: Option<&'a update_types::UpdateProgress>,
    last_copied_code_hash: Option<u64>,
    desktop_update: Option<&desktop_update::app::desktop_update::DesktopUpdate>,
    cli_commands: Option<Element<'a, Message>>,
    animation_frame: usize,
    help: HelpRenderer,
) -> Element<'a, Message> {
    let close_message = Message::View(ViewMessage::CloseInstallCliModal);
    let has_version_info = current_version.is_some() && server_version.is_some();
    #[cfg(target_arch = "wasm32")]
    let installing = false;
    #[cfg(not(target_arch = "wasm32"))]
    let installing = use_app_update_action
        && desktop_update.is_some_and(|state| state.busy && state.ready.is_some());
    let helper_text = if use_app_update_action {
        if cfg!(target_os = "linux") {
            "自动选择 DEB/RPM，下载校验后通过系统授权安装；成功后提示启动新版。"
        } else {
            "更新前会先检测当前版本与服务端版本，确认后再执行安装流程。"
        }
    } else {
        "通常无需手动安装，正式版应用启动时会自动安装 CLI；需要时可在此重新安装。"
    };

    let logo_badge = |expanded: bool| {
        let (logo_image_size, logo_badge_size, logo_badge_radius) = if expanded {
            (UPDATE_LOGO_IMAGE_SIZE, UPDATE_LOGO_BADGE_SIZE, UPDATE_LOGO_BADGE_RADIUS)
        } else {
            (HEADER_LOGO_IMAGE_SIZE, HEADER_LOGO_BADGE_SIZE, HEADER_LOGO_BADGE_RADIUS)
        };

        container(
            image(assets::get_image(Icon::Logo))
                .width(Length::Fixed(logo_image_size))
                .height(Length::Fixed(logo_image_size))
                .content_fit(ContentFit::Contain),
        )
        .width(Length::Fixed(logo_badge_size))
        .height(Length::Fixed(logo_badge_size))
        .align_x(Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |theme: &Theme| logo_badge_style(theme, logo_badge_radius))
    };

    let version_block: Element<'a, Message> =
        if let (Some(current_version), Some(server_version)) = (current_version, server_version) {
            row![
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "当前版本 %s",
                    &[current_version.into()]
                )),
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "服务器版本 %s",
                    &[server_version.into()]
                )),
            ]
            .spacing(8)
            .wrap()
            .into()
        } else {
            Space::new().height(0.0).into()
        };

    let actions: Option<Element<'a, Message>> = if has_version_info {
        let detect_label = if is_checking && !installing { "检测中..." } else { "检测更新" };
        let detect_message = if use_app_update_action {
            ViewMessage::CheckAppUpdate
        } else {
            ViewMessage::CheckCliToolUpdate
        };
        let action_row = row![
            button(text(i18n.__(detect_label)).size(13))
                .on_press_maybe((!is_checking).then_some(Message::View(detect_message)))
                .padding([8, 16])
                .style(rounded_action_btn_style)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        #[cfg(not(target_arch = "wasm32"))]
        let action_row = if use_app_update_action
            && desktop_update.is_some_and(|state| state.ready.is_some())
        {
            let label = if installing { "更新中…" } else { "自动更新并退出 / 重试" };
            let mut content = row![].spacing(8).align_y(Alignment::Center);
            if installing {
                content = content.push(
                    text(desktop_ui_controls::status_animation::spinner_frame(animation_frame))
                        .width(Length::Fixed(16.0)),
                );
            }
            let content = content.push(text(i18n.__(label)).size(13));
            action_row
                .push(
                    button(content)
                        .on_press_maybe(
                            (!is_checking)
                                .then_some(Message::View(ViewMessage::OpenAppInstaller)),
                        )
                        .padding([8, 16])
                        .style(primary_action_btn_style),
                )
                .push(
                    button(text(i18n.__("打开安装包目录")).size(13))
                        .on_press_maybe((!is_checking).then_some(Message::View(
                            ViewMessage::OpenAppInstallerDirectory,
                        )))
                        .padding([8, 16])
                        .style(rounded_action_btn_style),
                )
        } else {
            action_row
        };
        let action_row = if use_app_update_action
            && desktop_update.is_some_and(|state| state.download_abort.is_some())
        {
            action_row.push(
                button(text(i18n.__("取消下载")).size(13))
                    .on_press(Message::View(ViewMessage::CancelAppUpdate))
                    .padding([8, 16])
                    .style(rounded_action_btn_style),
            )
        } else {
            action_row
        };
        let action_row = if show_install_action {
            let install_label =
                if use_app_update_action { "下载安装包" } else { "安装 CLI" };
            let install_message = if use_app_update_action {
                ViewMessage::RunAppUpdate
            } else {
                ViewMessage::RunInstallCliTool
            };
            action_row.push(
                button(text(i18n.__(install_label)).size(13))
                    .on_press_maybe((!is_checking).then_some(Message::View(install_message)))
                    .padding([8, 16])
                    .style(primary_action_btn_style),
            )
        } else {
            action_row
        };
        let action_row = if use_app_update_action
            && show_install_action
            && let Some(state) = desktop_update
            && state.notification_release().is_some()
        {
            action_row.push(
                button(text(i18n.__("跳过此版本")).size(13))
                    .on_press_maybe(
                        (!is_checking && !state.saving_skip)
                            .then_some(Message::View(ViewMessage::SkipAppUpdate)),
                    )
                    .padding([8, 16])
                    .style(rounded_action_btn_style),
            )
        } else {
            action_row
        };
        Some(
            container(
                action_row
                    .push(
                        button(text(i18n.__("关闭")).size(13))
                            .on_press(close_message.clone())
                            .padding([8, 16])
                            .style(rounded_action_btn_style),
                    )
                    .wrap(),
            )
            .align_right(Length::Fill)
            .into(),
        )
    } else if show_logo_above_content {
        None
    } else {
        Some(
            container(
                button(text(i18n.__("关闭")).size(13))
                    .on_press(close_message.clone())
                    .padding([8, 16])
                    .style(primary_action_btn_style),
            )
            .align_right(Length::Fill)
            .into(),
        )
    };

    let progress_block: Element<'a, Message> =
        if use_app_update_action && let Some(progress) = update_progress {
            update_progress_block_localized(i18n, progress, last_copied_code_hash)
        } else {
            Space::new().height(0.0).into()
        };

    let source_control: Element<'a, Message> = if let Some(state) = desktop_update {
        let label = match state.source {
            Some(desktop_release::UpdateSource::Shiju) => Some(i18n.__("shiju（默认）")),
            Some(desktop_release::UpdateSource::Github) => Some("github.com"),
            None => None,
        };
        let control: Element<'a, Message> = if state.busy || state.saving {
            text(label.unwrap_or(i18n.__("请选择来源"))).size(SETTINGS_CONTROL_TEXT_SIZE).into()
        } else {
            settings_select([i18n.__("shiju（默认）"), "github.com"], label, |label| {
                Message::View(ViewMessage::AppUpdateSourceSelected(
                    if label == "github.com" {
                        desktop_release::UpdateSource::Github
                    } else {
                        desktop_release::UpdateSource::Shiju
                    },
                ))
            })
            .placeholder(i18n.__("请选择来源"))
            .width(Length::Fill)
            .into()
        };
        column![settings_field_label(i18n.__("升级来源")), control].spacing(6).into()
    } else {
        Space::new().height(0.0).into()
    };

    let header = settings_modal_header(
        settings_hover_help_label(
            help,
            i18n,
            None,
            i18n.__(title),
            text(i18n.__(title)).size(20).wrapping(Wrapping::WordOrGlyph),
            i18n.__(helper_text),
        )
        .wrap(),
        settings_close_button(close_message.clone()),
    );
    let description = text(i18n.__(content))
        .size(13)
        .width(Length::Fill)
        .wrapping(Wrapping::WordOrGlyph)
        .style(settings_muted_text_style)
        .line_height(iced::widget::text::LineHeight::Relative(1.5));
    let helper = text(i18n.__(helper_text))
        .size(12)
        .width(Length::Fill)
        .wrapping(Wrapping::WordOrGlyph)
        .style(settings_muted_text_style);
    let (body, height): (Element<'_, Message>, _) = if let Some(commands) = cli_commands {
        (
            column![description, column![source_control, version_block].spacing(12), commands,]
                .spacing(14)
                .into(),
            540,
        )
    } else if has_version_info {
        (
            column![
                container(logo_badge(true)).center_x(Length::Fill),
                description,
                column![source_control, version_block].spacing(12),
                progress_block,
            ]
            .spacing(14)
            .into(),
            640,
        )
    } else if show_logo_above_content {
        (
            column![
                container(logo_badge(true)).center_x(Length::Fill),
                description.align_x(Horizontal::Center),
            ]
            .spacing(14)
            .into(),
            390,
        )
    } else {
        (
            column![
                row![logo_badge(false), description].spacing(14).align_y(Alignment::Center),
                helper,
            ]
            .spacing(14)
            .into(),
            330,
        )
    };
    let card = settings_modal_layout(header, body, actions).width(540).height(height);
    settings_modal_overlay(None, close_message, container(card).padding(12))
}

#[cfg(test)]
use iced::Background;
