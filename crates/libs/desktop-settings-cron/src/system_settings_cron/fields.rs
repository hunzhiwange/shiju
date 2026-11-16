use super::*;
use iced::widget::column;

pub(super) fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            settings_field_label(label),
            control.into(),
            text(description).size(12).style(settings_muted_text_style)
        ]
        .spacing(6),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub(super) fn field_row_with_help<'a>(
    app: &ViewData<'_>,
    label: &'static str,
    description: impl Into<std::borrow::Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let label = hover_help_label(
        app,
        i18n.__(label),
        settings_field_label(i18n.__(label)),
        description,
    );
    container(column![label, control.into()].spacing(6)).padding([14, 0]).width(Length::Fill).into()
}

pub(super) fn at_field<'a>(app: &ViewData<'_>, value: &'a str, editing: bool) -> Element<'a, Message> {
    let label =
        if value.is_empty() { "选择日期和时间".to_string() } else { value.to_string() };
    field_row_with_help(
        app,
        "指定时间",
        "选择日期及小时、分钟；确认后预览校验。",
        secondary_button(
            label,
            Message::Settings(message::SettingsMessage::CronPickerOpen { editing }),
        ),
    )
}

pub(super) fn cron_segment_style(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let mut style = settings_segment_button_style(theme, status, is_active);
    if is_active {
        style.text_color = theme.palette().text;
    }
    style
}

pub(super) fn tab_button(
    tab: CronSettingsTab,
    active_tab: CronSettingsTab,
    label: &'static str,
) -> Element<'static, Message> {
    settings_tab_button(
        label,
        tab == active_tab,
        Some(Message::Settings(message::SettingsMessage::CronTabSelected(tab))),
    )
    .into()
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

pub(super) fn small_button<'a>(
    label: impl Into<String>,
    on_press: Message,
    danger: bool,
) -> Element<'a, Message> {
    let mut btn = button(text(label.into()).size(12)).padding([7, 10]);
    btn = if danger {
        btn.style(danger_action_btn_style)
    } else {
        btn.style(rounded_action_btn_style)
    };
    btn.on_press(on_press).into()
}

pub(super) fn primary_button<'a>(label: impl Into<String>, on_press: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(primary_action_btn_style)
        .on_press(on_press)
        .into()
}

pub(super) fn secondary_button<'a>(label: impl Into<String>, on_press: Message) -> Element<'a, Message> {
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

pub(super) fn with_selected_option(mut options: Vec<String>, selected: &str) -> Vec<String> {
    let selected = selected.trim();
    if !selected.is_empty() && !options.iter().any(|option| option == selected) {
        options.push(selected.to_string());
    }
    options.sort();
    options.dedup();
    options
}

pub(super) fn models_for_provider(app: &ViewData<'_>, provider_id: &str) -> Vec<String> {
    app.providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .map(|provider| provider.models.iter().map(|model| (*model).to_string()).collect())
        .unwrap_or_default()
}

pub(super) fn project_options(app: &ViewData<'_>, selected: &str) -> Vec<String> {
    let mut options = app.recent_projects.to_vec();
    if let Some(path) = app.project_path.filter(|value| !value.trim().is_empty()) {
        options.push(path.to_string());
    }
    with_selected_option(options, selected)
}

#[derive(Clone, PartialEq)]
enum ProjectChoice {
    None(&'static str),
    Path(String),
}

impl std::fmt::Display for ProjectChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None(label) => f.write_str(label),
            Self::Path(path) => f.write_str(path),
        }
    }
}

pub(super) fn project_field<'a>(
    app: &ViewData<'_>,
    path: &str,
    on_selected: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let empty = ProjectChoice::None(I18n::new(app.desktop_locale).__("选择项目"));
    let path = path.trim();
    let selected = if path.is_empty() { empty.clone() } else { ProjectChoice::Path(path.into()) };
    let options = std::iter::once(empty).chain(
        project_options(app, path)
            .into_iter()
            .filter(|path| !path.trim().is_empty())
            .map(ProjectChoice::Path),
    );
    settings_select(options, Some(selected), move |choice| {
        on_selected(match choice {
            ProjectChoice::None(_) => String::new(),
            ProjectChoice::Path(path) => path,
        })
    })
    .width(Length::Fill)
    .into()
}

pub(super) fn pick_field<'a>(
    options: Vec<String>,
    selected: Option<String>,
    on_selected: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_select(options, selected, on_selected).width(Length::Fill).into()
}

pub(super) fn job_title(job: &CronJobDto) -> String {
    job.name
        .as_ref()
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| job.id.clone())
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

pub(super) fn format_cron_datetime(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub(super) fn format_optional_cron_datetime(value: Option<&str>) -> String {
    value.map(format_cron_datetime).unwrap_or_else(|| "无".to_string())
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
                    "%u 条",
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
        settings_group_card(field_row(
            "定时任务群通知",
            "独立配置企业微信群机器人的 Webhook，用于接收任务结果。请在需要通知的任务中开启结果投递。",
            secondary_button(
                if app.delivery.webhook {
                    "修改通知 Webhook"
                } else {
                    "配置通知 Webhook"
                },
                Message::Settings(message::SettingsMessage::OpenDelivery(crate::message::DeliveryMode::Webhook)),
            ),
        )),
    ]
    .spacing(14)
    .into()
}

pub fn add_tab<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    form::view(app, false)
}
