//! 系统设置 - 定时任务配置界面组件
//!
//! 本模块提供 Cron 全局配置、任务列表管理与任务添加界面。

use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_divider, settings_error_banner, settings_field_label,
    settings_group_card,
    settings_help_button_with_locale as settings_help_button,
    settings_modal_card, settings_modal_card_style, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style,
    settings_segment_button_style, settings_select, settings_success_banner, settings_tab_button,
    settings_tab_row, settings_text_editor, settings_text_editor_style, settings_text_input,
    settings_value_badge,
};

use desktop_i18n::I18n;
use crate::state::{CronAddJobType, CronSettingsTab};
use crate::message::{self, Message};

use chrono::{DateTime, Local};
use iced::widget::{button, checkbox, column, container, row, slider, text, text_editor};
use iced::{Alignment, Background, Element, Length, Theme};
use gateway_cron::CronJobDto;

pub struct ProviderOptions<'a> {
    pub id: &'a str,
    pub models: Vec<&'a str>,
}
#[derive(Clone, Copy, Default)]
pub struct DeliveryAvailability {
    pub bot: bool,
    pub webhook: bool,
    pub telegram: bool,
    pub discord: bool,
    pub slack: bool,
    pub mattermost: bool,
    pub qq: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub email: bool,
}
/// Borrowed inputs read by the Cron UI; the host retains navigation and other settings.
pub struct ViewData<'a> {
    pub cron_settings: &'a crate::state::CronSettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a Theme,
    pub project_path: Option<&'a str>,
    pub recent_projects: &'a [String],
    pub agents: Vec<&'a str>,
    pub providers: Vec<ProviderOptions<'a>>,
    pub delivery: DeliveryAvailability,
    pub scheduler_error: Option<&'a str>,
    pub last_copied_code_hash: Option<u64>,
    pub help_content: for<'b> fn(
        &'b str,
        std::borrow::Cow<'b, str>,
        Option<iced::Color>,
        I18n,
    ) -> Element<'b, Message>,
}
fn hover_help_label<'a>(app: &ViewData<'_>, title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a,str>>) -> iced::widget::Row<'a,Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, (app.help_content)(title, description.into(), Some(app.app_theme.palette().text), I18n::new(app.desktop_locale)))
}
fn settings_page_heading_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>) -> Element<'a,Message> {
    hover_help_label(app, title, text(title).size(28), description).wrap().into()
}
fn settings_group_heading_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>) -> Element<'a,Message> {
    hover_help_label(app, title, text(title).size(17), description).wrap().into()
}
fn settings_flexible_row_with_help<'a>(app: &ViewData<'_>, title: &'a str, description: impl Into<std::borrow::Cow<'a,str>>, width: f32, control: impl Into<Element<'a,Message>>) -> iced::widget::Container<'a,Message> {
    let label = hover_help_label(app, title, settings_field_label(title), description).width(Length::Fill).wrap();
    desktop_ui_controls::system_settings_common::settings_flexible_row_with_label(label,width,control)
}

pub mod date_time;
mod delivery;
mod editor_modal;
pub mod form;
pub use editor_modal::with_edit_modal;
pub mod history;
pub mod list;
mod schedule;
pub mod templates;

use list::jobs_tab;

const RUN_HISTORY_PREVIEW_LINES: usize = 3;

pub const CRON_TABS: &[(CronSettingsTab, &str, &[&str])] = &[
    (
        CronSettingsTab::Jobs,
        "定时任务",
        &["cron", "schedule", "任务列表", "启用"],
    ),
    (
        CronSettingsTab::History,
        "运行记录",
        &["history", "运行中", "成功", "失败"],
    ),
    (
        CronSettingsTab::Config,
        "配置",
        &["history retention", "执行历史", "历史留存"],
    ),
    (
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
    (CronSettingsTab::Add, "添加", &["新增任务", "shell", "agent"]),
];

mod fields;
use fields::*;
pub use fields::{job_is_agent, edit_form, add_tab, config_tab};

pub fn view<'a, M: Clone + 'static>(app: &ViewData<'a>, scheduler_content: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let s = app.cron_settings;
    let help_btn = settings_help_button(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::CronHelpOpen),
    );
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let tabs = settings_tab_row(
        CRON_TABS
            .iter()
            .filter(|descriptor| descriptor.0 != CronSettingsTab::Add)
            .map(|descriptor| tab_button(descriptor.0, s.active_tab, i18n.__(descriptor.1))),
    )
    .wrap();

    let tab_content = match s.active_tab {
        CronSettingsTab::Jobs => jobs_tab(app).map(map),
        CronSettingsTab::History => history_tab(app).map(map),
        CronSettingsTab::Config => config_tab(app).map(map),
        CronSettingsTab::Scheduler => {
            scheduler_content
        }
        CronSettingsTab::Add => jobs_tab(app).map(map),
    };

    let heading: Element<'_, Message> = row![
            container(settings_page_heading_with_help(app,
                i18n.__("定时任务配置"),
                i18n.__("配置 Cron 子系统的开关与历史保留策略。")
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start).into();
    let mut col = column![heading.map(map), Element::<Message>::from(tabs).map(map)].spacing(16).width(Length::Fill);
    if let Some(error) = save_error_banner(app) {
        col = col.push(error.map(map));
    }
    col = col.push(tab_content);

    if let Some(status) = &s.action_status {
        col = col.push(settings_success_banner(
            desktop_i18n::I18n::new(app.desktop_locale).__(status).to_string(),
        ).map(map));
    }

    if matches!(s.active_tab, CronSettingsTab::Jobs | CronSettingsTab::History) {
        col = col.height(Length::Fill);
    }
    col.into()
}

fn active_save_error<'a>(app: &ViewData<'a>) -> Option<&'a str> {
    app.cron_settings.save_error.as_deref().or(app.scheduler_error)
}

fn save_error_banner<'a>(app: &ViewData<'a>) -> Option<Element<'a, Message>> {
    active_save_error(app).map(|error| {
        settings_error_banner(
            app.cron_settings
                .save_error_translation
                .as_ref()
                .map(|message| {
                    message
                        .render(desktop_i18n::I18n::new(app.desktop_locale))
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

fn with_access_confirmation<'a, M: Clone + 'static>(
    app: &ViewData<'a>,
    dialog: Element<'a, M>, map: fn(Message) -> M,
) -> Element<'a, M> {
    let state = app.cron_settings;
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
    settings_modal_overlay(Some(dialog), map(cancel), Element::<Message>::from(container(card).padding([10, 12])).map(map))
}

fn with_runs_modal<'a, M: Clone + 'static>(app: &ViewData<'a>, dialog: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let s = app.cron_settings;
    let Some(job_id) = s.runs_modal_job_id.as_deref() else {
        return dialog;
    };
    let job_label = s
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(job_title)
        .unwrap_or_else(|| job_id.to_string());
    let tr = desktop_i18n::I18n::new(app.desktop_locale);
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
    settings_modal_overlay(Some(dialog), map(close), Element::<Message>::from(container(card).padding([10, 12])).map(map))
}

pub fn view_overlays<'a, M: Clone + 'static>(app: &ViewData<'a>, dialog: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let s = app.cron_settings;
    let dialog = with_edit_modal(app, dialog, map);
    let dialog = with_runs_modal(app, dialog, map);
    let dialog = history::with_details(app, dialog, map);

    if !s.show_help_modal {
        return with_access_confirmation(app, picker_overlay(app, dialog, map), map);
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

    let help_text = I18n::new(app.desktop_locale).__(help_text).to_string();
    let (copy_icon, copy_tip) = desktop_ui_controls::system_settings_common::settings_help_copy_state(app.last_copied_code_hash, &help_text);
    desktop_ui_controls::system_settings_common::with_settings_help_modal(
        dialog,
        I18n::new(app.desktop_locale).__("定时任务配置帮助").to_string(),
        help_text.clone(),
        copy_icon,
        desktop_i18n::settings_runtime::text(app.desktop_locale, copy_tip),
        map(Message::CopyCode(help_text)),
        map(Message::Settings(message::SettingsMessage::CronHelpClose)),
    )
}

/// 运行记录的筛选入口与当前加载状态。
fn history_tab<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    history::view(app)
}

fn picker_overlay<'a, M: Clone + 'static>(app: &ViewData<'a>, base: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let Some(picker) = &app.cron_settings.date_time_picker else { return base; };
    let draft = if picker.editing { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
    let timezone = draft.schedule_draft.as_ref().map(|d| d.timezone.as_str()).unwrap_or("UTC");
    date_time::overlay(picker, timezone, base, map)
}

#[cfg(test)]
mod helpers_tests;
