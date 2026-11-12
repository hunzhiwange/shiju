//! 系统设置 - 定时任务配置界面组件
//!
//! 本模块提供 Cron 全局配置、任务列表管理与任务添加界面。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_divider, settings_error_banner, settings_field_label,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_help_button_with_locale as settings_help_button, settings_hover_help_label,
    settings_modal_card, settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_page_heading_with_help,
    settings_segment_button_style, settings_select, settings_success_banner, settings_tab_button,
    settings_tab_row, settings_text_editor, settings_text_editor_style, settings_text_input,
    settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::I18n;
#[cfg(test)]
use crate::app::state::CronAddScheduleKind;
use crate::app::state::{CronAddJobType, CronSettingsTab};
use crate::app::{App, Message, message};
use chrono::{DateTime, Local};
use iced::widget::{button, checkbox, column, container, row, slider, text, text_editor};
use iced::{Alignment, Background, Element, Length, Theme};
use vw_gateway_client::CronJobDto;

pub(crate) mod date_time;
mod form;
mod list;
mod schedule;
#[cfg(test)]
use list::job_row;
use list::jobs_tab;

const RUN_HISTORY_PREVIEW_LINES: usize = 3;

pub(super) const CRON_TABS: &[SettingsTabDescriptor<CronSettingsTab>] = &[
    SettingsTabDescriptor::new(
        CronSettingsTab::Jobs,
        "任务",
        &["cron", "schedule", "任务列表", "启用"],
    ),
    SettingsTabDescriptor::new(
        CronSettingsTab::Config,
        "配置",
        &["history retention", "执行历史", "历史留存"],
    ),
    SettingsTabDescriptor::new(
        CronSettingsTab::Scheduler,
        "调度器",
        &[
            "scheduler",
            "执行能力",
            "最大任务",
            "并发上限",
            "最大并发",
            "max_tasks",
            "max_concurrent",
        ],
    ),
    SettingsTabDescriptor::new(CronSettingsTab::Add, "添加", &["新增任务", "shell", "agent"]),
];

fn field_row<'a>(
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

fn field_row_with_help<'a>(
    app: &App,
    label: &'static str,
    description: impl Into<std::borrow::Cow<'a, str>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let label = settings_hover_help_label(
        i18n,
        &app.app_theme,
        i18n.__(label),
        settings_field_label(i18n.__(label)),
        description,
    );
    container(column![label, control.into()].spacing(6)).padding([14, 0]).width(Length::Fill).into()
}

fn at_field<'a>(app: &App, value: &'a str, editing: bool) -> Element<'a, Message> {
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

fn cron_segment_style(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let mut style = settings_segment_button_style(theme, status, is_active);
    if is_active {
        style.text_color = theme.palette().text;
    }
    style
}

fn tab_button(
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

fn segment_button<'a>(
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

fn small_button<'a>(
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

fn primary_button<'a>(label: impl Into<String>, on_press: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(primary_action_btn_style)
        .on_press(on_press)
        .into()
}

fn secondary_button<'a>(label: impl Into<String>, on_press: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(rounded_action_btn_style)
        .on_press(on_press)
        .into()
}

fn text_field<'a>(
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_text_input(placeholder, value).on_input(on_input).into()
}

fn with_selected_option(mut options: Vec<String>, selected: &str) -> Vec<String> {
    let selected = selected.trim();
    if !selected.is_empty() && !options.iter().any(|option| option == selected) {
        options.push(selected.to_string());
    }
    options.sort();
    options.dedup();
    options
}

fn models_for_provider(app: &App, provider_id: &str) -> Vec<String> {
    app.agents_settings
        .provider_models
        .iter()
        .find(|provider| provider.id == provider_id)
        .map(|provider| provider.models.iter().map(|model| model.id.clone()).collect())
        .unwrap_or_default()
}

fn project_options(app: &App, selected: &str) -> Vec<String> {
    let mut options = app.recent_projects.clone();
    if let Some(path) = app.project_path.as_deref().filter(|value| !value.trim().is_empty()) {
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

fn project_field<'a>(
    app: &App,
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

fn pick_field<'a>(
    options: Vec<String>,
    selected: Option<String>,
    on_selected: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_select(options, selected, on_selected).width(Length::Fill).into()
}

fn job_title(job: &CronJobDto) -> String {
    job.name
        .as_ref()
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| job.id.clone())
}

fn job_project_label(app: &App, job: &CronJobDto) -> String {
    if let Some(path) = job.project_path.as_deref().map(str::trim).filter(|path| !path.is_empty()) {
        return path.to_string();
    }

    app.project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| {
            crate::__!(crate::app::i18n::I18n::new(app.desktop_locale), "当前项目: %s", path)
                .expect("valid cron display template")
        })
        .unwrap_or_else(|| {
            crate::app::i18n::I18n::new(app.desktop_locale).__("默认工作区").to_string()
        })
}

fn job_is_agent(job: &CronJobDto) -> bool {
    if job.job_type.trim().eq_ignore_ascii_case("agent") {
        return true;
    }

    let has_prompt = job.prompt.as_deref().is_some_and(|value| !value.trim().is_empty());
    let has_command = !job.command.trim().is_empty();
    has_prompt && !has_command
}

fn format_cron_datetime(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| value.to_string())
}

fn format_optional_cron_datetime(value: Option<&str>) -> String {
    value.map(format_cron_datetime).unwrap_or_else(|| "无".to_string())
}

fn preview_lines(value: &str, max_lines: usize) -> String {
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

fn edit_form(app: &App) -> Element<'_, Message> {
    form::view(app, true)
}

fn config_tab(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = &app.cron_settings;
    let enabled_row = settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__("启用"),
        i18n.__("控制是否启用 Cron 子系统。"),
        320.0,
        checkbox(s.enabled)
            .label(crate::app::i18n::I18n::new(app.desktop_locale).__("开启 Cron 子系统"))
            .on_toggle(|v| Message::Settings(message::SettingsMessage::CronEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let history_slider = slider(1.0..=500.0, s.max_run_history as f32, |v| {
        Message::Settings(message::SettingsMessage::CronMaxRunHistoryChanged(v.round() as u32))
    })
    .width(Length::Fill);

    let history_row = settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__("历史保留"),
        i18n.__("每个任务保留的执行历史记录数量，建议 20-200。"),
        320.0,
        row![
            history_slider,
            container(settings_value_badge(
                crate::__!(
                    crate::app::i18n::I18n::new(app.desktop_locale),
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
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("基础行为"),
            i18n.__("控制 Cron 子系统是否运行与历史留存数量。")
        ),
        settings_group_card(column![enabled_row, settings_divider(), history_row]),
    ]
    .spacing(14)
    .into()
}

fn add_tab(app: &App) -> Element<'_, Message> {
    form::view(app, false)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.cron_settings;
    let help_btn = settings_help_button(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::CronHelpOpen),
    );
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let tabs = settings_tab_row(
        CRON_TABS
            .iter()
            .map(|descriptor| tab_button(descriptor.tab, s.active_tab, i18n.__(descriptor.label))),
    )
    .wrap();

    let tab_content = match s.active_tab {
        CronSettingsTab::Jobs => jobs_tab(app),
        CronSettingsTab::Config => config_tab(app),
        CronSettingsTab::Scheduler => {
            crate::app::components::system_settings_scheduler::embedded_view(app)
        }
        CronSettingsTab::Add => jobs_tab(app),
    };

    let mut col = column![
        row![
            container(settings_page_heading_with_help(
                i18n,
                &app.app_theme,
                i18n.__("定时任务配置"),
                i18n.__("配置 Cron 子系统的开关与历史保留策略。")
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = save_error_banner(app) {
        col = col.push(error);
    }
    col = col.push(tab_content);

    if let Some(status) = &s.action_status {
        col = col.push(settings_success_banner(
            crate::app::i18n::I18n::new(app.desktop_locale).__(status).to_string(),
        ));
    }

    if s.active_tab == CronSettingsTab::Jobs {
        col = col.height(Length::Fill);
    }
    col.into()
}

fn active_save_error(app: &App) -> Option<&str> {
    app.cron_settings.save_error.as_deref().or(app.scheduler_settings.save_error.as_deref())
}

fn save_error_banner(app: &App) -> Option<Element<'_, Message>> {
    active_save_error(app).map(|error| {
        settings_error_banner(
            app.cron_settings
                .save_error_translation
                .as_ref()
                .map(|message| {
                    message
                        .render(crate::app::i18n::I18n::new(app.desktop_locale))
                        .expect("valid cron error template")
                })
                .unwrap_or_else(|| error.to_string()),
        )
        .into()
    })
}

fn cron_modal_style(theme: &Theme) -> container::Style {
    let mut style = settings_modal_card_style(theme);
    // Keep the task list from showing through either dialog.
    if let Some(Background::Color(color)) = &mut style.background {
        color.a = 1.0;
    }
    style
}

fn with_edit_modal<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let editing = s.editing_job_id.is_some();
    if !editing && !s.show_add_modal {
        return dialog;
    }
    let draft = if editing { &s.edit_draft } else { &s.add_draft };
    let tr = crate::app::i18n::I18n::new(app.desktop_locale);
    let close = if draft.submitting {
        Message::None
    } else {
        Message::Settings(message::SettingsMessage::CronJobEditCanceled)
    };
    let header = settings_modal_header(
        column![
            text(if editing { tr.__("编辑任务") } else { tr.__("添加任务") }).size(20),
            text("配置执行内容、时间规则与有效期").size(13).style(settings_muted_text_style)
        ]
        .spacing(4),
        settings_close_button(close.clone()),
    );
    let actions = row![
        secondary_button(tr.__("取消"), close.clone()),
        schedule::save_button(draft, editing),
    ]
    .spacing(10);
    let mut footer = column![schedule::preview(draft, editing)].spacing(8).width(Length::Fill);
    if let Some(error) = save_error_banner(app) {
        footer = footer.push(error);
    }
    let footer = footer.push(container(actions).align_right(Length::Fill));
    let card = settings_modal_layout(
        header,
        if editing { edit_form(app) } else { add_tab(app) },
        Some(footer.into()),
    )
    .style(cron_modal_style)
    .width(740)
    .height(620);
    settings_modal_overlay(Some(dialog), close, container(card).padding([10, 12]))
}

fn with_access_confirmation<'a>(
    app: &'a App,
    dialog: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &app.cron_settings;
    let editing = state.editing_job_id.is_some();
    if !editing && !state.show_add_modal {
        return dialog;
    }
    let draft = if editing { &state.edit_draft } else { &state.add_draft };
    if !draft.full_access_confirmation {
        return dialog;
    }
    let cancel = Message::Settings(message::SettingsMessage::CronFullAccessCancel { editing });
    let header = settings_modal_header(
        text("确认完全访问权限").size(20),
        settings_close_button(cancel.clone()),
    );
    let body = column![
        text("仅为当前 Agent 定时任务启用完全访问。触发后可按完全访问执行工具，包括读写文件、运行命令和网络访问。请只授权可信的提示词与项目。").size(14),
        text("ACP 额外未授权请求仍可能被非交互后台拒绝。取消会保持受限权限；确认后仍需保存任务才会生效。").size(13).style(settings_muted_text_style),
    ].spacing(12);
    let actions = row![
        secondary_button("取消授权", cancel.clone()),
        button(text("确认授权").size(13))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press(Message::Settings(message::SettingsMessage::CronFullAccessConfirm {
                editing
            })),
    ]
    .spacing(10)
    .wrap();
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(cron_modal_style)
        .width(560)
        .height(360);
    settings_modal_overlay(Some(dialog), cancel, container(card).padding([10, 12]))
}

fn with_runs_modal<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let Some(job_id) = s.runs_modal_job_id.as_deref() else {
        return dialog;
    };
    let job_label = s
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(job_title)
        .unwrap_or_else(|| job_id.to_string());
    let tr = crate::app::i18n::I18n::new(app.desktop_locale);
    let close = Message::Settings(message::SettingsMessage::CronJobRunsClose);
    let header = settings_modal_header(
        column![
            text(tr.__("执行历史")).size(20),
            text(job_label).size(13).style(settings_muted_text_style)
        ]
        .spacing(4),
        settings_close_button(close.clone()),
    );

    let status: Option<Element<'_, Message>> = if s.runs_modal_loading {
        Some(text(tr.__("正在加载历史记录...")).size(13).style(settings_muted_text_style).into())
    } else if let Some(error) = &s.runs_modal_error {
        Some(settings_error_banner(error).into())
    } else if s.runs_modal.is_empty() {
        Some(text(tr.__("暂无执行历史")).size(13).style(settings_muted_text_style).into())
    } else {
        None
    };
    let card = if let Some(status) = status {
        settings_modal_layout(header, status, None)
    } else {
        let editor = text_editor(&s.runs_modal_editor)
            .on_action(|action| {
                Message::Settings(message::SettingsMessage::CronJobRunsEditorAction(action))
            })
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .padding(SETTINGS_CONTROL_PADDING)
            .height(Length::Fill)
            .style(settings_text_editor_style);
        settings_modal_card(column![header, editor].spacing(16).height(Length::Fill))
    }
    .style(cron_modal_style)
    .width(780)
    .height(620);
    settings_modal_overlay(Some(dialog), close, container(card).padding([10, 12]))
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let dialog = with_edit_modal(app, dialog);
    let dialog = with_runs_modal(app, dialog);

    if !s.show_help_modal {
        return with_access_confirmation(app, date_time::view_overlay(app, dialog));
    }

    let help_text = r#"定时任务配置说明

一、作用
- cron 是定时任务子系统的全局开关与历史保留策略。
- 你创建的定时任务（cron_add / cron_update 等）都依赖这里的全局配置。
- 本节不定义具体任务表达式，只控制"是否运行"与"历史留存"。

二、字段含义
1) enabled
- 类型：布尔（true / false）
- 含义：是否启用 Cron 子系统。
- true：调度器会持续扫描并触发到期任务。
- false：Cron 调度器不启动，已有任务不会按计划执行（但任务定义仍保留在工作区）。

2) max_run_history
- 类型：整数
- 含义：每个任务最多保留多少条执行历史记录。
- 默认：50。
- 当历史条数超过上限时，会按"旧记录优先清理"的方式裁剪，避免记录无限增长。

三、典型示例

示例 A：默认推荐（大多数场景）
{
  "cron": {
    "enabled": true,
    "max_run_history": 50
  }
}

示例 B：高审计需求（保留更多历史）
{
  "cron": {
    "enabled": true,
    "max_run_history": 200
  }
}

示例 C：临时停用调度器（任务定义不删除）
{
  "cron": {
    "enabled": false,
    "max_run_history": 50
  }
}

四、配置建议
1) 开发/测试环境
- max_run_history 可设为 20-50，便于快速迭代并减少噪音。

2) 生产环境
- 建议保持 enabled=true，避免遗漏计划任务。
- max_run_history 可按审计需求设为 50-200。

3) 存储与追溯平衡
- 数值越大，历史追溯越充分，但磁盘占用与查询成本会增加。
- 如果只关心近期失败，可使用较小值（如 30-50）。

五、排查建议（任务没按时跑）
1) 先确认 cron.enabled=true。
2) 检查任务本身是否 enabled，表达式是否有效。
3) 检查主进程是否在运行，避免误以为后台仍在调度。
4) 查看最近 run history，确认是"未触发"还是"触发后执行失败"。
5) 确认 Gateway 可用且系统设置保存成功，再重启应用观察日志。
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "定时任务配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::CronHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_cron_tests.rs"]
mod system_settings_cron_tests;

#[cfg(test)]
#[path = "system_settings_cron_ui_refresh_27_tests.rs"]
mod ui_refresh_27_tests;
