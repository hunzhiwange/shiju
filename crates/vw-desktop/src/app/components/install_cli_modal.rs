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

use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_svg, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_pick_list_menu_style, settings_pick_list_style,
    settings_value_badge,
};
use crate::app::{Message, message};
use iced::alignment::Horizontal;
use iced::widget::{Space, button, column, container, image, pick_list, progress_bar, row, scrollable, text};
use iced::{Alignment, Background, ContentFit, Element, Length, Theme};
use std::hash::{Hash, Hasher};

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
/// use crate::app::components::install_cli_modal;
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
pub fn view<'a>(title: &'a str, content: &'a str) -> Element<'a, Message> {
    view_inner(
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
    update_progress: Option<&'a vw_shared::update::UpdateProgress>,
    last_copied_code_hash: Option<u64>,
) -> Element<'a, Message> {
    view_inner(
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
    )
}

pub fn view_app_update(app: &crate::app::App) -> Element<'_, Message> {
    view_inner(
        &app.cli_install_modal_title,
        &app.cli_install_modal_message,
        Some(&app.cli_install_modal_current_version),
        Some(&app.cli_install_modal_server_version),
        app.cli_install_modal_is_checking_update,
        app.cli_install_modal_show_install_action,
        app.cli_install_modal_use_app_update_action,
        false,
        app.app_update_progress.as_ref(),
        app.last_copied_code_hash,
        Some(&app.desktop_update),
        (!app.cli_install_modal_use_app_update_action).then(|| {
            super::cli_command_help::view(app.cli_commands_tab, app.last_copied_code_hash)
        }),
    )
}

fn view_inner<'a>(
    title: &'a str,
    content: &'a str,
    current_version: Option<&'a str>,
    server_version: Option<&'a str>,
    is_checking: bool,
    show_install_action: bool,
    use_app_update_action: bool,
    show_logo_above_content: bool,
    update_progress: Option<&'a vw_shared::update::UpdateProgress>,
    last_copied_code_hash: Option<u64>,
    desktop_update: Option<&crate::app::desktop_update::DesktopUpdate>,
    cli_commands: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let close_message = Message::View(message::ViewMessage::CloseInstallCliModal);
    let has_version_info = current_version.is_some() && server_version.is_some();
    let helper_text = if use_app_update_action {
        "更新前会先检测当前版本与服务端版本，确认后再执行安装流程。"
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

    // 构建模态框卡片主体
    // 包含 Logo、标题、内容和关闭按钮的垂直布局
    let version_block: Element<'a, Message> =
        if let (Some(current_version), Some(server_version)) = (current_version, server_version) {
            container(
                row![
                    settings_value_badge(format!("当前版本 {current_version}")),
                    settings_value_badge(format!("服务器版本 {server_version}")),
                ]
                .spacing(8),
            )
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else {
            Space::new().height(0.0).into()
        };

    let action_buttons: Element<'a, Message> = if has_version_info {
        let detect_label = if is_checking { "检测中..." } else { "检测更新" };
        let detect_message = if use_app_update_action {
            message::ViewMessage::CheckAppUpdate
        } else {
            message::ViewMessage::CheckCliToolUpdate
        };
        let action_row = row![
            button(detect_label)
                .on_press_maybe((!is_checking).then_some(Message::View(detect_message)))
                .padding([8, 16])
                .style(rounded_action_btn_style)
        ]
        .spacing(12);
        #[cfg(not(target_arch = "wasm32"))]
        let action_row = if use_app_update_action && desktop_update.is_some_and(|state| state.ready.is_some()) {
            action_row.push(
                button("自动更新并退出 / 重试")
                    .on_press_maybe(
                        (!is_checking)
                            .then_some(Message::View(message::ViewMessage::OpenAppInstaller)),
                    )
                    .padding([8, 16])
                    .style(primary_action_btn_style),
            )
        } else {
            action_row
        };
        let action_row = if show_install_action {
            let install_label =
                if use_app_update_action { "下载安装包" } else { "安装 CLI" };
            let install_message = if use_app_update_action {
                message::ViewMessage::RunAppUpdate
            } else {
                message::ViewMessage::RunInstallCliTool
            };
            action_row.push(
                button(install_label)
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
                button("跳过此版本")
                    .on_press_maybe((!is_checking && !state.saving_skip)
                        .then_some(Message::View(message::ViewMessage::SkipAppUpdate)))
                    .padding([8, 16])
                    .style(rounded_action_btn_style),
            )
        } else {
            action_row
        };
        container(
            action_row.push(
                button("关闭")
                    .on_press(close_message.clone())
                    .padding([8, 16])
                    .style(rounded_action_btn_style),
            ),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    } else {
        container(
            button("关闭")
                .on_press(close_message.clone())
                .padding([8, 16])
                .style(primary_action_btn_style),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
    };

    let progress_block: Element<'a, Message> =
        if use_app_update_action && let Some(progress) = update_progress {
            update_progress_block(progress, last_copied_code_hash)
        } else {
            Space::new().height(0.0).into()
        };

    let source_control: Element<'a, Message> = if let Some(state) = desktop_update {
        let label = match state.source {
            Some(vw_shared::update::UpdateSource::Shiju) => Some("shiju（默认）"),
            Some(vw_shared::update::UpdateSource::Github) => Some("github.com"),
            None => None,
        };
        let control: Element<'a, Message> = if state.busy || state.saving {
            text(label.unwrap_or("请选择来源")).size(SETTINGS_CONTROL_TEXT_SIZE).into()
        } else {
            pick_list(["shiju（默认）", "github.com"], label, |label| {
                Message::View(message::ViewMessage::AppUpdateSourceSelected(
                    if label == "github.com" {
                        vw_shared::update::UpdateSource::Github
                    } else {
                        vw_shared::update::UpdateSource::Shiju
                    },
                ))
            })
            .placeholder("请选择来源")
            .padding(SETTINGS_CONTROL_PADDING)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .into()
        };
        row![text("升级来源"), control].spacing(12).align_y(Alignment::Center).into()
    } else {
        Space::new().height(0.0).into()
    };

    let card_content: Element<'a, Message> = if let Some(commands) = cli_commands {
        let body = column![
            text(content).size(12).style(settings_muted_text_style),
            source_control,
            version_block,
            text(helper_text).size(12).style(settings_muted_text_style),
            commands,
        ]
        .spacing(16);
        let body = scrollable(container(body).padding([0, 7]))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fixed(380.0));
        column![
            row![
                text(title).size(24).width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .align_y(Alignment::Center),
            body,
            action_buttons,
        ]
        .spacing(16)
        .into()
    } else if has_version_info {
        column![
            container(logo_badge(true)).width(Length::Fill).center_x(Length::Fill),
            column![
                text(title).size(24),
                text(content)
                    .size(12)
                    .align_x(Horizontal::Center)
                    .style(settings_muted_text_style)
                    .line_height(iced::widget::text::LineHeight::Relative(1.5)),
            ]
            .spacing(6)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
            source_control,
            version_block,
            progress_block,
            container(
                text(helper_text)
                    .size(12)
                    .align_x(Horizontal::Center)
                    .style(settings_muted_text_style),
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
            action_buttons,
        ]
        .spacing(16)
        .align_x(Horizontal::Center)
        .into()
    } else if show_logo_above_content {
        column![
            row![Space::new().width(Length::Fill), settings_close_button(close_message.clone()),]
                .width(Length::Fill)
                .align_y(Alignment::Start),
            container(logo_badge(true)).width(Length::Fill).center_x(Length::Fill),
            column![
                text(title).size(24).align_x(Horizontal::Center),
                text(content)
                    .size(12)
                    .align_x(Horizontal::Center)
                    .style(settings_muted_text_style)
                    .line_height(iced::widget::text::LineHeight::Relative(1.5)),
            ]
            .spacing(6)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
        ]
        .spacing(16)
        .align_x(Horizontal::Center)
        .into()
    } else {
        column![
            row![
                row![
                    logo_badge(false),
                    column![
                        text(title).size(24),
                        text(content)
                            .size(12)
                            .style(settings_muted_text_style)
                            .line_height(iced::widget::text::LineHeight::Relative(1.5)),
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                ]
                .spacing(14)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .align_y(Alignment::Start),
            version_block,
            container(
                text(helper_text)
                    .size(12)
                    .align_x(Horizontal::Left)
                    .style(settings_muted_text_style),
            )
            .width(Length::Fill),
            action_buttons,
        ]
        .spacing(16)
        .into()
    };

    let card = settings_modal_card(card_content).width(Length::Fixed(540.0)).max_width(620.0);

    settings_modal_overlay(None, close_message, card)
}

pub(super) fn update_progress_block<'a>(
    progress: &'a vw_shared::update::UpdateProgress,
    last_copied_code_hash: Option<u64>,
) -> Element<'a, Message> {
    let progress_value = progress
        .total_bytes
        .filter(|total| *total > 0)
        .map(|total| (progress.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let percent_label = progress
        .total_bytes
        .filter(|total| *total > 0)
        .map(|_| format!("{:.0}%", progress_value * 100.0))
        .unwrap_or_else(|| "下载中".to_string());
    let total_label = progress.total_bytes.map(format_bytes).unwrap_or_else(|| "未知".to_string());
    let copy_hash = hash_copy_text(&progress.download_url);
    let copied = last_copied_code_hash == Some(copy_hash);
    let copy_label = if copied { "✓" } else { "复制" };

    container(
        column![
            row![
                text("下载进度").size(12).style(settings_muted_text_style),
                Space::new().width(Length::Fill),
                text(percent_label).size(12),
            ]
            .align_y(Alignment::Center),
            container(progress_bar(0.0..=1.0, progress_value)).height(Length::Fixed(8.0)),
            row![
                settings_value_badge(format!("已下载 {}", format_bytes(progress.downloaded_bytes))),
                settings_value_badge(format!("总大小 {total_label}")),
            ]
            .spacing(8),
            row![
                settings_value_badge(format!("速度 {}", format_rate(progress.bytes_per_second))),
                settings_value_badge(format!(
                    "平均 {}",
                    format_rate(progress.average_bytes_per_second)
                )),
            ]
            .spacing(8),
            row![
                column![
                    text("下载 URL").size(11).style(settings_muted_text_style),
                    text(compact_url(&progress.download_url)).size(11),
                ]
                .spacing(3)
                .width(Length::Fill),
                button(
                    row![
                        icon_svg(if copied { Icon::Check } else { Icon::Copy }, 13.0),
                        text(copy_label).size(12)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                )
                .on_press(Message::CopyCode(progress.download_url.clone()))
                .padding([7, 12])
                .width(Length::Fixed(78.0))
                .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(12)
    .style(update_progress_card_style)
    .into()
}

pub(super) fn logo_badge_style(
    theme: &Theme,
    logo_badge_radius: f32,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette().primary.scale_alpha(
            if theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5
            {
                0.20
            } else {
                0.08
            },
        ))),
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.78),
            radius: logo_badge_radius.into(),
        },
        ..Default::default()
    }
}

pub(super) fn update_progress_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.55))),
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.65),
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}

pub(super) fn format_rate(bytes_per_second: f64) -> String {
    if bytes_per_second <= 0.0 {
        return "0 KB/s".to_string();
    }
    const MB: f64 = 1024.0 * 1024.0;
    if bytes_per_second < MB {
        format!("{:.1} KB/s", bytes_per_second / 1024.0)
    } else {
        format!("{:.1} MB/s", bytes_per_second / MB)
    }
}

pub(super) fn compact_url(url: &str) -> String {
    const MAX_LEN: usize = 72;
    let chars = url.chars().collect::<Vec<_>>();
    if chars.len() <= MAX_LEN {
        return url.to_string();
    }
    let head = chars.iter().take(42).collect::<String>();
    let tail = chars.iter().skip(chars.len().saturating_sub(24)).collect::<String>();
    format!("{head}...{tail}")
}

pub(super) fn hash_copy_text(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
