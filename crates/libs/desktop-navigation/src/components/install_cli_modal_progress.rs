use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{icon_svg, rounded_action_btn_style, settings_muted_text_style};
use super::message::Message;
use iced::widget::{Space, button, column, container, progress_bar, row, text};
use iced::{Alignment, Background, Element, Length, Theme};
use std::hash::{Hash, Hasher};
pub(super) fn update_value(label: String) -> iced::widget::Text<'static> {
    text(label).size(12).style(settings_muted_text_style)
}

pub(crate) fn update_progress_block_localized<'a>(
    i18n: desktop_i18n::I18n,
    progress: &'a update_types::UpdateProgress,
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
        .unwrap_or_else(|| i18n.__("下载中").to_string());
    let total_label =
        progress.total_bytes.map(format_bytes).unwrap_or_else(|| i18n.__("未知").to_string());
    let copy_hash = hash_copy_text(&progress.download_url);
    let copied = last_copied_code_hash == Some(copy_hash);
    let copy_label = if copied { "✓" } else { "复制" };

    container(
        column![
            row![
                text(i18n.__("下载进度")).size(12).style(settings_muted_text_style),
                Space::new().width(Length::Fill),
                text(percent_label).size(12),
            ]
            .align_y(Alignment::Center),
            container(progress_bar(0.0..=1.0, progress_value)).height(Length::Fixed(8.0)),
            row![
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "已下载 %s",
                    &[format_bytes(progress.downloaded_bytes).into()]
                )),
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "总大小 %s",
                    &[total_label.into()]
                )),
            ]
            .spacing(8),
            row![
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "速度 %s",
                    &[format_rate(progress.bytes_per_second).into()]
                )),
                update_value(desktop_i18n::modals::render(
                    i18n,
                    "平均 %s",
                    &[format_rate(progress.average_bytes_per_second).into()]
                )),
            ]
            .spacing(8),
            row![
                column![
                    text(i18n.__("下载 URL")).size(11).style(settings_muted_text_style),
                    text(compact_url(&progress.download_url)).size(11),
                ]
                .spacing(3)
                .width(Length::Fill),
                button(
                    row![
                        icon_svg(if copied { Icon::Check } else { Icon::Copy }, 13.0),
                        text(i18n.__(copy_label)).size(12)
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

pub(crate) fn logo_badge_style(
    theme: &Theme,
    logo_badge_radius: f32,
) -> iced::widget::container::Style {
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
        border: iced::Border { radius: logo_badge_radius.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(crate) fn update_progress_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.55))),
        border: iced::Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}

pub(crate) fn format_rate(bytes_per_second: f64) -> String {
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

pub(crate) fn compact_url(url: &str) -> String {
    const MAX_LEN: usize = 72;
    let chars = url.chars().collect::<Vec<_>>();
    if chars.len() <= MAX_LEN {
        return url.to_string();
    }
    let head = chars.iter().take(42).collect::<String>();
    let tail = chars.iter().skip(chars.len().saturating_sub(24)).collect::<String>();
    format!("{head}...{tail}")
}

pub(crate) fn hash_copy_text(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
pub(crate) fn update_progress_block<'a>(
    progress: &'a update_types::UpdateProgress,
    last_copied_code_hash: Option<u64>,
) -> Element<'a, Message> {
    update_progress_block_localized(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        progress,
        last_copied_code_hash,
    )
}
