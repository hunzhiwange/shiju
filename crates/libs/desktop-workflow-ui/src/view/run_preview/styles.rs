//! Internal styles helpers.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowPreviewStatusKind {
    Running,
    Succeeded,
    Failed,
    Canceled,
    Waiting,
    Unknown,
}

pub fn workflow_run_status_label(
    i18n: desktop_i18n::I18n,
    status: WorkflowRunStatus,
) -> &'static str {
    match status {
        WorkflowRunStatus::Running => i18n.__("运行中"),
        WorkflowRunStatus::Succeeded => i18n.__("成功"),
        WorkflowRunStatus::Failed => i18n.__("失败"),
        WorkflowRunStatus::Canceled => i18n.__("已取消"),
    }
}

pub fn workflow_run_node_status_label(
    i18n: desktop_i18n::I18n,
    status: Option<&str>,
) -> &'static str {
    match status {
        Some("running") | Some("started") => i18n.__("运行中"),
        Some("succeeded") | Some("success") | Some("finished") => i18n.__("成功"),
        Some("failed") | Some("error") => i18n.__("失败"),
        Some("canceled") | Some("cancelled") => i18n.__("已取消"),
        Some(_) => i18n.__("未知"),
        None => i18n.__("等待中"),
    }
}

pub(super) fn workflow_run_node_status_kind(status: Option<&str>) -> WorkflowPreviewStatusKind {
    match status {
        Some("running") | Some("started") => WorkflowPreviewStatusKind::Running,
        Some("succeeded") | Some("success") | Some("finished") => {
            WorkflowPreviewStatusKind::Succeeded
        }
        Some("failed") | Some("error") => WorkflowPreviewStatusKind::Failed,
        Some("canceled") | Some("cancelled") => WorkflowPreviewStatusKind::Canceled,
        Some(_) => WorkflowPreviewStatusKind::Unknown,
        None => WorkflowPreviewStatusKind::Waiting,
    }
}

pub fn workflow_run_preview_node_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.70
            } else {
                0.46
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.42
            } else {
                0.16
            }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn workflow_run_preview_output_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.58
            } else {
                0.34
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.34
            } else {
                0.14
            }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn workflow_run_preview_output_empty_style(
    theme: &Theme,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    // The empty state stays lighter than a populated report: a dashed-feel low-contrast
    // surface so "no output" reads as placeholder rather than real content.
    iced::widget::container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark { 0.32 } else { 0.20 }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.24 } else { 0.10 }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn workflow_run_preview_error_heading_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
}

pub fn workflow_run_preview_error_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    // Keep the body text on the base palette text so the multi-line debug report stays
    // legible against a translucent danger tint in both themes; the danger accent is
    // reserved for the heading and border so the surface still reads as an error panel.
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(if is_dark {
            0.30
        } else {
            0.42
        }))),
        border: Border {
            width: 1.0,
            color: palette.danger.base.color.scale_alpha(if is_dark { 0.52 } else { 0.42 }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn workflow_run_preview_chip_style(
    theme: &Theme,
    status: WorkflowPreviewStatusKind,
) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    let color = workflow_run_status_kind_color(theme, status);
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(if is_dark { 0.20 } else { 0.10 }))),
        border: Border {
            width: 1.0,
            color: color.scale_alpha(if is_dark { 0.48 } else { 0.28 }),
            radius: 4.0.into(),
        },
        // Keep chip text fully opaque so muted-accent kinds stay readable on dark backgrounds.
        text_color: Some(opaque_color(color)),
        ..Default::default()
    }
}

/// Drops a color's translucency so it renders at full strength as chip/label text.
pub(super) fn opaque_color(color: Color) -> Color {
    Color { a: 1.0, ..color }
}

pub(super) fn workflow_run_status_badge_style(
    theme: &Theme,
    status: WorkflowRunStatus,
) -> iced::widget::container::Style {
    let color = workflow_run_status_color(theme, status);

    iced::widget::container::Style {
        text_color: Some(color),
        background: Some(Background::Color(color.scale_alpha(if is_dark_theme(theme) {
            0.16
        } else {
            0.10
        }))),
        border: Border { width: 1.0, color: color.scale_alpha(0.34), radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn workflow_status_dot<'a>(color_fn: impl Fn(&Theme) -> Color + 'a) -> Element<'a, Message> {
    container(Space::new().width(8).height(8))
        .style(move |theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(color_fn(theme))),
            border: Border { radius: 999.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub(super) fn workflow_run_status_color(theme: &Theme, status: WorkflowRunStatus) -> Color {
    match status {
        WorkflowRunStatus::Running => theme.palette().primary,
        WorkflowRunStatus::Succeeded => theme.extended_palette().success.base.color,
        WorkflowRunStatus::Failed => theme.extended_palette().danger.base.color,
        WorkflowRunStatus::Canceled => theme.extended_palette().secondary.base.color,
    }
}

pub(super) fn workflow_run_status_kind_color(theme: &Theme, status: WorkflowPreviewStatusKind) -> Color {
    let palette = theme.extended_palette();
    match status {
        WorkflowPreviewStatusKind::Running => theme.palette().primary,
        WorkflowPreviewStatusKind::Succeeded => palette.success.base.color,
        WorkflowPreviewStatusKind::Failed => palette.danger.base.color,
        WorkflowPreviewStatusKind::Canceled => palette.secondary.base.color,
        WorkflowPreviewStatusKind::Waiting => theme.palette().text.scale_alpha(0.56),
        WorkflowPreviewStatusKind::Unknown => palette.warning.base.color,
    }
}
