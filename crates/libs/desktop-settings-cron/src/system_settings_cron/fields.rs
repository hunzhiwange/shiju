use super::*;
use iced::widget::column;

pub(super) fn schedule_popup_style(theme: &Theme) -> container::Style {
    let mut style = cron_modal_style(theme);
    let palette = theme.palette();
    if let Some(Background::Color(color)) = &mut style.background {
        color.r += (palette.text.r - color.r) * 0.03;
        color.g += (palette.text.g - color.g) * 0.03;
        color.b += (palette.text.b - color.b) * 0.03;
    }
    style.border.radius = 16.0.into();
    style.border.color = palette.text.scale_alpha(0.14);
    style.shadow.offset = iced::Vector::new(0.0, 6.0);
    style.shadow.blur_radius = 18.0;
    style
}

pub(super) fn field_row_with_help<'a>(
    app: &ViewData<'_>,
    label: &'static str,
    description: &'static str,
    control: impl Fn() -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let label = i18n.__(label);
    let description = i18n.__(description);
    let help = app.help_content;
    let color = app.app_theme.palette().text;
    iced::widget::responsive(move |size| {
        let label = desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
            settings_field_label(label),
            help(label, description.into(), Some(color), i18n),
        );
        if size.width >= 600.0 {
            desktop_ui_controls::system_settings_common::settings_flexible_row_with_label(
                label.width(Length::Fill).wrap(),
                360.0,
                control(),
            )
            .padding([16, 0])
            .into()
        } else {
            container(column![label.wrap(), control()].spacing(6))
                .padding([14, 0])
                .width(Length::Fill)
                .into()
        }
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn cron_segment_style(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let mut style = settings_segment_button_style(theme, status, is_active);
    if is_active {
        style.text_color = theme.palette().text;
    }
    style
}

pub(super) fn segment_button<'a>(
    label: &'static str,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label).size(13))
        .padding([8, 14])
        .style(move |theme: &Theme, status| cron_segment_style(theme, status, is_active))
        .on_press(on_press)
        .into()
}

pub(super) fn compact_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
) -> iced::widget::Button<'a, M> {
    button(text(label.into()).size(12)).padding([7, 10])
}

pub(super) fn small_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    on_press: M,
    danger: bool,
) -> Element<'a, M> {
    let mut btn = compact_button(label);
    btn = if danger {
        btn.style(danger_action_btn_style)
    } else {
        btn.style(rounded_action_btn_style)
    };
    btn.on_press(on_press).into()
}

pub(super) fn primary_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    on_press: M,
) -> Element<'a, M> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(primary_action_btn_style)
        .on_press(on_press)
        .into()
}

pub(super) fn secondary_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    on_press: M,
) -> Element<'a, M> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(rounded_action_btn_style)
        .on_press(on_press)
        .into()
}

pub(super) fn text_field<'a>(
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_text_input(placeholder, value).on_input(on_input).into()
}

#[cfg(test)]
pub(crate) fn pick_field<'a>(
    options: Vec<String>,
    selected: Option<String>,
    on_selected: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_select(options, selected, on_selected).width(Length::Fill).into()
}

pub(super) fn job_title(job: &CronJobDto) -> &str {
    crate::state::task_title(job.name.as_deref())
}

pub(super) fn job_project_label(app: &ViewData<'_>, job: &CronJobDto) -> String {
    if let Some(path) = job.project_path.as_deref().map(str::trim).filter(|path| !path.is_empty()) {
        return path.to_string();
    }

    app.project_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| {
            desktop_i18n::__!(desktop_i18n::I18n::new(app.desktop_locale), "当前项目: %s", path)
                .expect("valid cron display template")
        })
        .unwrap_or_else(|| {
            desktop_i18n::I18n::new(app.desktop_locale).__("默认工作区").to_string()
        })
}

pub fn job_is_agent(job: &CronJobDto) -> bool {
    if job.job_type.trim().eq_ignore_ascii_case("agent") {
        return true;
    }

    let has_prompt = job.prompt.as_deref().is_some_and(|value| !value.trim().is_empty());
    let has_command = !job.command.trim().is_empty();
    has_prompt && !has_command
}

pub(crate) fn format_cron_datetime(value: &str, timezone: NamedTimeZone) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&timezone).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub(super) fn format_optional_cron_datetime(value: Option<&str>, timezone: NamedTimeZone) -> String {
    value.map(|value| format_cron_datetime(value, timezone)).unwrap_or_else(|| "无".to_string())
}

pub(super) fn preview_lines(value: &str, max_lines: usize) -> String {
    let mut lines = value.lines();
    let mut preview = lines.by_ref().take(max_lines).collect::<Vec<_>>().join("\n");

    if lines.next().is_some() {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str("...");
    }

    preview
}

pub fn edit_form<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    form::view(app, true)
}

pub fn config_tab<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = app.cron_settings;
    let enabled_row = settings_flexible_row_with_help(app,
        i18n.__("启用"),
        i18n.__("控制是否启用 Cron 子系统。"),
        320.0,
        checkbox(s.enabled)
            .label(desktop_i18n::I18n::new(app.desktop_locale).__("开启 Cron 子系统"))
            .on_toggle(|v| Message::Settings(message::SettingsMessage::CronEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let history_slider = slider(1.0..=500.0, s.max_run_history as f32, |v| {
        Message::Settings(message::SettingsMessage::CronMaxRunHistoryChanged(v.round() as u32))
    })
    .width(Length::Fill);

    let history_row = settings_flexible_row_with_help(app,
        i18n.__("历史保留"),
        i18n.__("每个任务保留的执行历史记录数量，建议 20-200。"),
        320.0,
        row![
            history_slider,
            container(settings_value_badge(
                desktop_i18n::__!(
                    desktop_i18n::I18n::new(app.desktop_locale),
                    i18n.__("%u 条"),
                    s.max_run_history
                )
                .expect("valid cron display template")
            ))
            .align_right(84.0)
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    );

    column![
        settings_group_heading_with_help(app,
            i18n.__("基础行为"),
            i18n.__("控制 Cron 子系统是否运行与历史留存数量。")
        ),
        settings_group_card(column![enabled_row, settings_divider(), history_row]),
    ]
    .spacing(14)
    .into()
}

pub fn add_tab<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    form::view(app, false)
}
