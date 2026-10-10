//! 系统设置 - 定时任务配置界面组件
//!
//! 本模块提供 Cron 全局配置、任务列表管理与任务添加界面。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SETTINGS_LABEL_WIDTH,
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_checkbox_style, settings_close_button, settings_divider, settings_error_banner,
    settings_help_button, settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_section_content, settings_segment_button_style,
    settings_success_banner, settings_text_editor_style, settings_text_input_style,
    settings_value_badge,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::{CronAddJobType, CronAddScheduleKind, CronSettingsTab};
use crate::app::{App, Message, message};
use chrono::{DateTime, Local};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_editor,
    text_input,
};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::{CronJobDto, CronRunDto};

const NO_ACP_AGENT_LABEL: &str = "不使用 ACP";
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
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn tab_button(
    tab: CronSettingsTab,
    active_tab: CronSettingsTab,
    label: &'static str,
) -> Element<'static, Message> {
    let is_active = tab == active_tab;
    button(text(label).size(13))
        .padding([8, 14])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .on_press(Message::Settings(message::SettingsMessage::CronTabSelected(tab)))
        .into()
}

fn segment_button<'a>(
    label: &'static str,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label).size(13))
        .padding([8, 14])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
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
    text_input(placeholder, value)
        .on_input(on_input)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .padding(SETTINGS_CONTROL_PADDING)
        .style(settings_text_input_style)
        .into()
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

fn acp_agent_options(app: &App, selected: &str) -> Vec<String> {
    let mut options = Vec::with_capacity(app.acp_agents.len() + 1);
    options.push(NO_ACP_AGENT_LABEL.to_string());
    options.extend(app.acp_agents.iter().cloned());
    with_selected_option(options, selected)
}

fn selected_acp_agent_label(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() { NO_ACP_AGENT_LABEL.to_string() } else { value.to_string() }
}

fn acp_agent_value_from_label(value: String) -> String {
    if value == NO_ACP_AGENT_LABEL { String::new() } else { value }
}

fn pick_field<'a>(
    options: Vec<String>,
    selected: Option<String>,
    on_selected: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    pick_list(options, selected, on_selected)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill)
        .into()
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
        .map(|path| format!("当前项目: {path}"))
        .unwrap_or_else(|| "默认工作区".to_string())
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

fn job_row<'a>(app: &'a App, job: &'a CronJobDto) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let selected = s.selected_job_ids.iter().any(|id| id == &job.id);
    let details_expanded = s.expanded_job_ids.iter().any(|id| id == &job.id);
    let is_editing = s.editing_job_id.as_deref() == Some(job.id.as_str());
    let id_for_select = job.id.clone();
    let id_for_details = job.id.clone();
    let id_for_toggle = job.id.clone();
    let id_for_edit = job.id.clone();
    let id_for_runs = job.id.clone();
    let id_for_delete = job.id.clone();

    let status_label = if job.enabled { "启用" } else { "禁用" };
    let is_agent = job_is_agent(job);
    let type_label = if is_agent { "Agent" } else { "Shell" };
    let last_status = job.last_status.as_deref().unwrap_or("未运行");
    let next_run = format_cron_datetime(&job.next_run);
    let last_run = format_optional_cron_datetime(job.last_run.as_deref());
    let delivery_label = if job.delivery_mode == "announce" { "投递" } else { "不投递" };

    let header = row![
        checkbox(selected)
            .label("")
            .on_toggle(move |v| Message::Settings(
                message::SettingsMessage::CronJobSelectionToggled(id_for_select.clone(), v)
            ))
            .style(settings_checkbox_style),
        button(
            row![
                text(job_title(job)).size(14).width(Length::Fill),
                text(if details_expanded { "收起" } else { "展开" })
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(rounded_action_btn_style)
        .on_press(Message::Settings(message::SettingsMessage::CronJobDetailsToggled(
            id_for_details,
            !details_expanded,
        ))),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut meta = column![
        row![
            text("ID").size(12).style(settings_muted_text_style),
            text(job.id.clone()).size(12),
        ]
        .spacing(10),
        row![
            text("概况").size(12).style(settings_muted_text_style),
            settings_value_badge(type_label),
            settings_value_badge(status_label),
            settings_value_badge(delivery_label),
            settings_value_badge(last_status),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("表达式").size(12).style(settings_muted_text_style),
            text(job.expression.clone()).size(12),
        ]
        .spacing(10),
        row![text("下次运行").size(12).style(settings_muted_text_style), text(next_run).size(12),]
            .spacing(10),
        row![text("上次运行").size(12).style(settings_muted_text_style), text(last_run).size(12),]
            .spacing(10),
        row![
            text("项目").size(12).style(settings_muted_text_style),
            text(job_project_label(app, job)).size(12).wrapping(iced::widget::text::Wrapping::Word),
        ]
        .spacing(10),
        row![
            text("执行选项").size(12).style(settings_muted_text_style),
            text(format!(
                "agent: {} / acp: {} / model: {} / fallbacks: {} / full-access: {} / task-pool: {} / wake: {} / delete-after-run: {}",
                job.agent.as_deref().unwrap_or("默认"),
                job.acp_agent
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("未使用"),
                job.model.as_deref().unwrap_or("默认"),
                job.fallbacks.len(),
                if job.full_access { "是" } else { "否" },
                if job.task_pool { "是" } else { "否" },
                if job.wake { "是" } else { "否" },
                if job.delete_after_run { "是" } else { "否" },
            ))
            .size(12),
        ]
        .spacing(10),
        text(if is_agent { job.prompt.clone().unwrap_or_default() } else { job.command.clone() })
            .size(12),
    ]
    .spacing(7);

    if let Some(last_output) =
        job.last_output.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        meta = meta.push(
            row![
                text("最近历史").size(12).style(settings_muted_text_style),
                text(preview_lines(last_output, RUN_HISTORY_PREVIEW_LINES))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            ]
            .spacing(10),
        );
    }

    let toggle_label = if job.enabled { "禁用" } else { "启用" };
    let actions = row![
        small_button(
            "编辑",
            Message::Settings(message::SettingsMessage::CronJobEditStarted(id_for_edit)),
            false
        ),
        small_button(
            "历史",
            Message::Settings(message::SettingsMessage::CronJobRunsOpen(id_for_runs)),
            false
        ),
        small_button(
            toggle_label,
            Message::Settings(message::SettingsMessage::CronJobEnabledChanged(
                id_for_toggle,
                !job.enabled
            )),
            false
        ),
        small_button(
            "删除",
            Message::Settings(message::SettingsMessage::CronJobDelete(id_for_delete)),
            true
        ),
    ]
    .spacing(8);

    let mut content = column![header].spacing(12).width(Length::Fill);
    if details_expanded {
        content = content.push(meta).push(actions);
        if is_editing {
            content = content.push(settings_divider()).push(edit_form(app));
        }
    }

    container(content).padding([14, 0]).width(Length::Fill).into()
}

fn run_row(run: &CronRunDto) -> Element<'_, Message> {
    let duration =
        run.duration_ms.map(|value| format!("{value} ms")).unwrap_or_else(|| "无".to_string());
    let output = run.output.as_deref().unwrap_or("无输出");
    let started_at = format_cron_datetime(&run.started_at);
    let finished_at = format_cron_datetime(&run.finished_at);

    container(
        column![
            row![
                settings_value_badge(run.status.clone()),
                text(format!("开始 {started_at}")).size(12),
                text(format!("结束 {finished_at}")).size(12).style(settings_muted_text_style),
                text(format!("耗时 {duration}")).size(12).style(settings_muted_text_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            container(
                text(output)
                    .size(11)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill)
            )
            .width(Length::Fill)
            .padding(iced::Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        ]
        .spacing(8),
    )
    .padding([12, 0])
    .width(Length::Fill)
    .into()
}

fn edit_form(app: &App) -> Element<'_, Message> {
    let draft = &app.cron_settings.edit_draft;
    let agent_options = with_selected_option(
        app.agents_settings
            .entries
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.key.clone())
            .collect(),
        &draft.agent,
    );
    let acp_agent_options = acp_agent_options(app, &draft.acp_agent);
    let provider_options = with_selected_option(
        app.agents_settings.provider_models.iter().map(|provider| provider.id.clone()).collect(),
        &draft.model_provider,
    );
    let selected_provider =
        (!draft.model_provider.trim().is_empty()).then_some(draft.model_provider.clone());
    let selected_model_for_provider = draft
        .model
        .strip_prefix(&format!("{}/", draft.model_provider))
        .unwrap_or(draft.model.as_str())
        .trim()
        .to_string();
    let model_options = with_selected_option(
        models_for_provider(app, &draft.model_provider),
        &selected_model_for_provider,
    );
    let project_options = project_options(app, &draft.project_path);

    let schedule_row = match draft.schedule_kind {
        CronAddScheduleKind::Cron => field_row(
            "Cron 表达式",
            "支持标准 5/6 字段表达式。",
            text_field("例如 0 9 * * 1-5", &draft.schedule, |v| {
                Message::Settings(message::SettingsMessage::CronJobEditScheduleChanged(v))
            }),
        ),
        CronAddScheduleKind::At => field_row(
            "指定时间",
            "RFC3339 时间，例如 2026-06-04T09:00:00Z。",
            text_field("RFC3339 时间", &draft.at, |v| {
                Message::Settings(message::SettingsMessage::CronJobEditAtChanged(v))
            }),
        ),
        CronAddScheduleKind::Every => field_row(
            "固定间隔",
            "按毫秒数循环触发。",
            text_field("例如 60000", &draft.every_ms, |v| {
                Message::Settings(message::SettingsMessage::CronJobEditEveryMsChanged(v))
            }),
        ),
    };

    let mut form = column![
        field_row(
            "任务名称",
            "用于在列表中识别任务。",
            text_field("可选名称", &draft.name, |v| Message::Settings(
                message::SettingsMessage::CronJobEditNameChanged(v)
            ))
        ),
        field_row(
            "任务类型",
            "Shell 执行命令，Agent 执行提示词任务。",
            row![
                segment_button(
                    "Shell",
                    draft.job_type == CronAddJobType::Shell,
                    Message::Settings(message::SettingsMessage::CronJobEditJobTypeChanged(
                        CronAddJobType::Shell
                    ))
                ),
                segment_button(
                    "Agent",
                    draft.job_type == CronAddJobType::Agent,
                    Message::Settings(message::SettingsMessage::CronJobEditJobTypeChanged(
                        CronAddJobType::Agent
                    ))
                )
            ]
            .spacing(8),
        ),
        field_row(
            "调度类型",
            "选择周期、指定时间或固定间隔。",
            row![
                segment_button(
                    "Cron",
                    draft.schedule_kind == CronAddScheduleKind::Cron,
                    Message::Settings(message::SettingsMessage::CronJobEditScheduleKindChanged(
                        CronAddScheduleKind::Cron
                    ))
                ),
                segment_button(
                    "指定时间",
                    draft.schedule_kind == CronAddScheduleKind::At,
                    Message::Settings(message::SettingsMessage::CronJobEditScheduleKindChanged(
                        CronAddScheduleKind::At
                    ))
                ),
                segment_button(
                    "固定间隔",
                    draft.schedule_kind == CronAddScheduleKind::Every,
                    Message::Settings(message::SettingsMessage::CronJobEditScheduleKindChanged(
                        CronAddScheduleKind::Every
                    ))
                )
            ]
            .spacing(8),
        ),
        schedule_row,
        field_row(
            "项目",
            "选择任务执行所在项目，Shell 和 Agent 都会切到该目录。",
            pick_field(
                project_options,
                (!draft.project_path.trim().is_empty()).then_some(draft.project_path.clone()),
                |v| Message::Settings(message::SettingsMessage::CronJobEditProjectPathChanged(v)),
            ),
        ),
        field_row(
            "项目路径",
            "可手工输入项目目录；留空使用默认工作区。",
            text_field("项目目录路径", &draft.project_path, |v| {
                Message::Settings(message::SettingsMessage::CronJobEditProjectPathChanged(v))
            }),
        ),
    ]
    .spacing(0);

    form = match draft.job_type {
        CronAddJobType::Shell => form.push(field_row(
            "执行命令",
            "按安全策略执行的 Shell 命令。",
            text_editor(&draft.command_editor)
                .placeholder("Shell 命令")
                .on_action(|action| {
                    Message::Settings(message::SettingsMessage::CronJobEditCommandEditorAction(
                        action,
                    ))
                })
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .padding(SETTINGS_CONTROL_PADDING)
                .height(Length::Fixed(120.0))
                .style(settings_text_editor_style),
        )),
        CronAddJobType::Agent => form
            .push(field_row(
                "委托代理",
                "默认使用 Main，套用代理配置中的 provider、模型和温度。",
                pick_field(
                    agent_options,
                    (!draft.agent.trim().is_empty()).then_some(draft.agent.clone()),
                    |v| Message::Settings(message::SettingsMessage::CronJobEditAgentChanged(v)),
                ),
            ))
            .push(field_row(
                "ACP 智能体",
                "默认不使用 ACP；选择后通过对应 ACP 智能体执行。",
                pick_field(
                    acp_agent_options,
                    Some(selected_acp_agent_label(&draft.acp_agent)),
                    |v| {
                        Message::Settings(message::SettingsMessage::CronJobEditAcpAgentChanged(
                            acp_agent_value_from_label(v),
                        ))
                    },
                ),
            ))
            .push(field_row(
                "Agent 提示词",
                "创建 Agent 类型定时任务。",
                text_editor(&draft.prompt_editor)
                    .placeholder("要让 Agent 执行的任务")
                    .on_action(|action| {
                        Message::Settings(message::SettingsMessage::CronJobEditPromptEditorAction(
                            action,
                        ))
                    })
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .height(Length::Fixed(140.0))
                    .style(settings_text_editor_style),
            ))
            .push(field_row(
                "模型提供商",
                "与智能体配置中的模型选择一致。",
                pick_field(provider_options, selected_provider, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditModelProviderChanged(v))
                }),
            ))
            .push(field_row(
                "模型下拉",
                "从当前提供商的模型列表选择，会写入 provider/model。",
                pick_field(
                    model_options,
                    (!selected_model_for_provider.is_empty())
                        .then_some(selected_model_for_provider),
                    {
                        let provider = draft.model_provider.clone();
                        move |model| {
                            let value = if provider.trim().is_empty() {
                                model
                            } else {
                                format!("{provider}/{model}")
                            };
                            Message::Settings(message::SettingsMessage::CronJobEditModelChanged(
                                value,
                            ))
                        }
                    },
                ),
            ))
            .push(field_row(
                "模型手工输入",
                "可选，留空使用代理或全局默认模型。",
                text_field("provider/model", &draft.model, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditModelChanged(v))
                }),
            ))
            .push(field_row(
                "回退模型",
                "主模型失败后依次尝试，使用逗号或换行分隔。",
                text_field("provider/model, provider/model", &draft.fallbacks, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditFallbacksChanged(v))
                }),
            ))
            .push(field_row(
                "完全访问",
                "开启后该 Agent 定时任务按发送面板的完全访问权限执行。",
                checkbox(draft.full_access)
                    .label("获取完全访问权限")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditFullAccessToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row(
                "任务池",
                "开启后触发时投递到项目任务池，由任务看板接管执行。",
                checkbox(draft.task_pool)
                    .label("投递到任务池")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditTaskPoolToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row(
                "唤醒",
                "保留唤醒标记，供桌面侧识别需要关注的任务。",
                checkbox(draft.wake)
                    .label("任务运行时标记为需要关注")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditWakeToggled(v))
                    })
                    .style(settings_checkbox_style),
            )),
    };

    form = form
        .push(field_row(
            "执行后删除",
            "适合指定时间的一次性任务，成功后自动清理。",
            checkbox(draft.delete_after_run)
                .label("运行成功后删除任务")
                .on_toggle(|v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeleteAfterRunToggled(v))
                })
                .style(settings_checkbox_style),
        ))
        .push(field_row(
            "结果投递",
            "将任务执行结果投递到配置好的通道。",
            checkbox(draft.delivery_enabled)
                .label("开启结果投递")
                .on_toggle(|v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryEnabledToggled(
                        v,
                    ))
                })
                .style(settings_checkbox_style),
        ));

    if draft.delivery_enabled {
        form = form
            .push(field_row(
                "投递通道",
                "例如 telegram、discord、slack、mattermost、email。",
                text_field("channel", &draft.delivery_channel, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryChannelChanged(
                        v,
                    ))
                }),
            ))
            .push(field_row(
                "投递目标",
                "通道内的目标 ID、地址或频道。",
                text_field("target", &draft.delivery_to, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryToChanged(v))
                }),
            ))
            .push(field_row(
                "尽力投递",
                "开启后投递失败不会把任务标记为失败。",
                checkbox(draft.delivery_best_effort)
                    .label("投递失败不影响任务状态")
                    .on_toggle(|v| {
                        Message::Settings(
                            message::SettingsMessage::CronJobEditDeliveryBestEffortToggled(v),
                        )
                    })
                    .style(settings_checkbox_style),
            ));
    }

    form.push(
        row![
            container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            row![
                primary_button(
                    "保存",
                    Message::Settings(message::SettingsMessage::CronJobEditSave)
                ),
                secondary_button(
                    "取消",
                    Message::Settings(message::SettingsMessage::CronJobEditCanceled),
                )
            ]
            .spacing(10)
        ]
        .spacing(22),
    )
    .into()
}

fn jobs_tab(app: &App) -> Element<'_, Message> {
    let s = &app.cron_settings;
    let all_selected = !s.jobs.is_empty() && s.selected_job_ids.len() == s.jobs.len();
    let selected_count = s.selected_job_ids.len();
    let mut list = column![
        row![
            checkbox(all_selected)
                .label(format!("全选 {} 个任务", s.jobs.len()))
                .on_toggle(|v| Message::Settings(
                    message::SettingsMessage::CronJobsSelectAllToggled(v)
                ))
                .style(settings_checkbox_style),
            container(text("")).width(Length::Fill),
            small_button(
                "刷新",
                Message::Settings(message::SettingsMessage::CronJobsRefresh),
                false
            ),
            small_button(
                format!("启用已选({selected_count})"),
                Message::Settings(message::SettingsMessage::CronSelectedJobsEnable),
                false
            ),
            small_button(
                format!("禁用已选({selected_count})"),
                Message::Settings(message::SettingsMessage::CronSelectedJobsDisable),
                false
            ),
            small_button(
                format!("删除已选({selected_count})"),
                Message::Settings(message::SettingsMessage::CronSelectedJobsDelete),
                true
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        settings_divider(),
    ]
    .spacing(10);

    if s.jobs_loading {
        list = list.push(text("正在加载定时任务...").size(12).style(settings_muted_text_style));
    } else if s.jobs.is_empty() {
        list = list.push(text("当前没有定时任务。").size(12).style(settings_muted_text_style));
    } else {
        for (index, job) in s.jobs.iter().enumerate() {
            if index > 0 {
                list = list.push(settings_divider());
            }
            list = list.push(job_row(app, job));
        }
    }

    column![
        settings_section_card("当前任务", "默认展示已配置的定时任务，并支持单项或批量管理。"),
        settings_panel(list),
    ]
    .spacing(14)
    .into()
}

fn config_tab(app: &App) -> Element<'_, Message> {
    let s = &app.cron_settings;
    let enabled_row = field_row(
        "启用",
        "控制是否启用 Cron 子系统。",
        checkbox(s.enabled)
            .label("开启 Cron 子系统")
            .on_toggle(|v| Message::Settings(message::SettingsMessage::CronEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let history_slider = slider(1.0..=500.0, s.max_run_history as f32, |v| {
        Message::Settings(message::SettingsMessage::CronMaxRunHistoryChanged(v.round() as u32))
    })
    .width(Length::Fill);

    let history_row = field_row(
        "历史保留",
        "每个任务保留的执行历史记录数量，建议 20-200。",
        row![history_slider, settings_value_badge(format!("{} 条", s.max_run_history))]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    column![
        settings_section_card("基础行为", "控制 Cron 子系统是否运行与历史留存数量。"),
        settings_section_content(column![enabled_row, history_row].spacing(0)),
    ]
    .spacing(14)
    .into()
}

fn add_tab(app: &App) -> Element<'_, Message> {
    let draft = &app.cron_settings.add_draft;
    let agent_options = with_selected_option(
        app.agents_settings
            .entries
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.key.clone())
            .collect(),
        &draft.agent,
    );
    let acp_agent_options = acp_agent_options(app, &draft.acp_agent);
    let provider_options = with_selected_option(
        app.agents_settings.provider_models.iter().map(|provider| provider.id.clone()).collect(),
        &draft.model_provider,
    );
    let selected_provider =
        (!draft.model_provider.trim().is_empty()).then_some(draft.model_provider.clone());
    let selected_model_for_provider = draft
        .model
        .strip_prefix(&format!("{}/", draft.model_provider))
        .unwrap_or(draft.model.as_str())
        .trim()
        .to_string();
    let model_options = with_selected_option(
        models_for_provider(app, &draft.model_provider),
        &selected_model_for_provider,
    );
    let project_options = project_options(app, &draft.project_path);

    let name_row = field_row(
        "任务名称",
        "可选，便于在列表中识别。",
        text_field("例如 每日备份", &draft.name, |v| {
            Message::Settings(message::SettingsMessage::CronAddNameChanged(v))
        }),
    );
    let job_type_row = field_row(
        "任务类型",
        "Shell 执行命令，Agent 执行提示词任务。",
        row![
            segment_button(
                "Shell",
                draft.job_type == CronAddJobType::Shell,
                Message::Settings(message::SettingsMessage::CronAddJobTypeChanged(
                    CronAddJobType::Shell
                ))
            ),
            segment_button(
                "Agent",
                draft.job_type == CronAddJobType::Agent,
                Message::Settings(message::SettingsMessage::CronAddJobTypeChanged(
                    CronAddJobType::Agent
                ))
            )
        ]
        .spacing(8),
    );
    let schedule_kind_row = field_row(
        "调度类型",
        "选择周期、指定时间或固定间隔。",
        row![
            segment_button(
                "Cron",
                draft.schedule_kind == CronAddScheduleKind::Cron,
                Message::Settings(message::SettingsMessage::CronAddScheduleKindChanged(
                    CronAddScheduleKind::Cron
                ))
            ),
            segment_button(
                "指定时间",
                draft.schedule_kind == CronAddScheduleKind::At,
                Message::Settings(message::SettingsMessage::CronAddScheduleKindChanged(
                    CronAddScheduleKind::At
                ))
            ),
            segment_button(
                "固定间隔",
                draft.schedule_kind == CronAddScheduleKind::Every,
                Message::Settings(message::SettingsMessage::CronAddScheduleKindChanged(
                    CronAddScheduleKind::Every
                ))
            )
        ]
        .spacing(8),
    );
    let schedule_row = match draft.schedule_kind {
        CronAddScheduleKind::Cron => field_row(
            "Cron 表达式",
            "支持标准 5/6 字段表达式。",
            text_field("例如 0 9 * * 1-5", &draft.schedule, |v| {
                Message::Settings(message::SettingsMessage::CronAddScheduleChanged(v))
            }),
        ),
        CronAddScheduleKind::At => field_row(
            "指定时间",
            "RFC3339 时间，例如 2026-06-04T09:00:00Z。",
            text_field("RFC3339 时间", &draft.at, |v| {
                Message::Settings(message::SettingsMessage::CronAddAtChanged(v))
            }),
        ),
        CronAddScheduleKind::Every => field_row(
            "固定间隔",
            "按毫秒数循环触发。",
            text_field("例如 60000", &draft.every_ms, |v| {
                Message::Settings(message::SettingsMessage::CronAddEveryMsChanged(v))
            }),
        ),
    };

    let project_row = field_row(
        "项目",
        "选择任务执行所在项目，Shell 和 Agent 都会切到该目录。",
        pick_field(
            project_options,
            (!draft.project_path.trim().is_empty()).then_some(draft.project_path.clone()),
            |v| Message::Settings(message::SettingsMessage::CronAddProjectPathChanged(v)),
        ),
    );
    let project_path_row = field_row(
        "项目路径",
        "可手工输入项目目录；留空使用默认工作区。",
        text_field("项目目录路径", &draft.project_path, |v| {
            Message::Settings(message::SettingsMessage::CronAddProjectPathChanged(v))
        }),
    );

    let mut form = column![
        name_row,
        job_type_row,
        schedule_kind_row,
        schedule_row,
        project_row,
        project_path_row
    ]
    .spacing(0);
    form = match draft.job_type {
        CronAddJobType::Shell => form.push(field_row(
            "执行命令",
            "创建 Shell 类型定时任务。",
            text_editor(&draft.command_editor)
                .placeholder("Shell 命令")
                .on_action(|action| {
                    Message::Settings(message::SettingsMessage::CronAddCommandEditorAction(action))
                })
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .padding(SETTINGS_CONTROL_PADDING)
                .height(Length::Fixed(120.0))
                .style(settings_text_editor_style),
        )),
        CronAddJobType::Agent => form
            .push(field_row(
                "委托代理",
                "默认使用 Main，套用代理配置中的 provider、模型和温度。",
                pick_field(
                    agent_options,
                    (!draft.agent.trim().is_empty()).then_some(draft.agent.clone()),
                    |v| Message::Settings(message::SettingsMessage::CronAddAgentChanged(v)),
                ),
            ))
            .push(field_row(
                "ACP 智能体",
                "默认不使用 ACP；选择后通过对应 ACP 智能体执行。",
                pick_field(
                    acp_agent_options,
                    Some(selected_acp_agent_label(&draft.acp_agent)),
                    |v| {
                        Message::Settings(message::SettingsMessage::CronAddAcpAgentChanged(
                            acp_agent_value_from_label(v),
                        ))
                    },
                ),
            ))
            .push(field_row(
                "Agent 提示词",
                "创建 Agent 类型定时任务。",
                text_editor(&draft.prompt_editor)
                    .placeholder("要让 Agent 执行的任务")
                    .on_action(|action| {
                        Message::Settings(message::SettingsMessage::CronAddPromptEditorAction(
                            action,
                        ))
                    })
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .height(Length::Fixed(140.0))
                    .style(settings_text_editor_style),
            ))
            .push(field_row(
                "模型提供商",
                "与智能体配置中的模型选择一致。",
                pick_field(provider_options, selected_provider, |v| {
                    Message::Settings(message::SettingsMessage::CronAddModelProviderChanged(v))
                }),
            ))
            .push(field_row(
                "模型下拉",
                "从当前提供商的模型列表选择，会写入 provider/model。",
                pick_field(
                    model_options,
                    (!selected_model_for_provider.is_empty())
                        .then_some(selected_model_for_provider),
                    {
                        let provider = draft.model_provider.clone();
                        move |model| {
                            let value = if provider.trim().is_empty() {
                                model
                            } else {
                                format!("{provider}/{model}")
                            };
                            Message::Settings(message::SettingsMessage::CronAddModelChanged(value))
                        }
                    },
                ),
            ))
            .push(field_row(
                "模型手工输入",
                "可选，留空使用代理或全局默认模型。",
                text_field("provider/model", &draft.model, |v| {
                    Message::Settings(message::SettingsMessage::CronAddModelChanged(v))
                }),
            ))
            .push(field_row(
                "回退模型",
                "主模型失败后依次尝试，使用逗号或换行分隔。",
                text_field("provider/model, provider/model", &draft.fallbacks, |v| {
                    Message::Settings(message::SettingsMessage::CronAddFallbacksChanged(v))
                }),
            ))
            .push(field_row(
                "完全访问",
                "开启后该 Agent 定时任务按发送面板的完全访问权限执行。",
                checkbox(draft.full_access)
                    .label("获取完全访问权限")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronAddFullAccessToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row(
                "任务池",
                "开启后触发时投递到项目任务池，由任务看板接管执行。",
                checkbox(draft.task_pool)
                    .label("投递到任务池")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronAddTaskPoolToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row(
                "唤醒",
                "保留唤醒标记，供桌面侧识别需要关注的任务。",
                checkbox(draft.wake)
                    .label("任务运行时标记为需要关注")
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronAddWakeToggled(v))
                    })
                    .style(settings_checkbox_style),
            )),
    };

    form = form.push(field_row(
        "执行后删除",
        "适合指定时间的一次性任务，成功后自动清理。",
        checkbox(draft.delete_after_run)
            .label("运行成功后删除任务")
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::CronAddDeleteAfterRunToggled(v))
            })
            .style(settings_checkbox_style),
    ));

    form = form.push(field_row(
        "结果投递",
        "将任务执行结果投递到配置好的通道。",
        checkbox(draft.delivery_enabled)
            .label("开启结果投递")
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::CronAddDeliveryEnabledToggled(v))
            })
            .style(settings_checkbox_style),
    ));

    if draft.delivery_enabled {
        form = form
            .push(field_row(
                "投递通道",
                "例如 telegram、discord、slack、mattermost、email。",
                text_field("channel", &draft.delivery_channel, |v| {
                    Message::Settings(message::SettingsMessage::CronAddDeliveryChannelChanged(v))
                }),
            ))
            .push(field_row(
                "投递目标",
                "通道内的目标 ID、地址或频道。",
                text_field("target", &draft.delivery_to, |v| {
                    Message::Settings(message::SettingsMessage::CronAddDeliveryToChanged(v))
                }),
            ))
            .push(field_row(
                "尽力投递",
                "开启后投递失败不会把任务标记为失败。",
                checkbox(draft.delivery_best_effort)
                    .label("投递失败不影响任务状态")
                    .on_toggle(|v| {
                        Message::Settings(
                            message::SettingsMessage::CronAddDeliveryBestEffortToggled(v),
                        )
                    })
                    .style(settings_checkbox_style),
            ));
    }

    let form = form.push(
        row![
            container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            primary_button("新增任务", Message::Settings(message::SettingsMessage::CronAddSubmit))
        ]
        .spacing(22),
    );

    column![
        settings_section_card(
            "添加任务",
            "创建 Shell 或 Agent 定时任务，支持 Cron、指定时间和固定间隔。"
        ),
        settings_section_content(form),
    ]
    .spacing(14)
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.cron_settings;
    let help_btn = settings_help_button(Message::Settings(message::SettingsMessage::CronHelpOpen));
    let tabs = CRON_TABS.iter().fold(row![].spacing(8), |row, descriptor| {
        row.push(tab_button(descriptor.tab, s.active_tab, descriptor.label))
    });

    let tab_content = match s.active_tab {
        CronSettingsTab::Jobs => jobs_tab(app),
        CronSettingsTab::Config => config_tab(app),
        CronSettingsTab::Scheduler => {
            crate::app::components::system_settings_scheduler::embedded_view(app)
        }
        CronSettingsTab::Add => add_tab(app),
    };

    let mut col = column![
        row![
            container(settings_page_intro(
                "定时任务配置",
                "配置 Cron 子系统的开关与历史保留策略。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(app) {
        col = col.push(settings_error_banner(error));
    }
    col = col.push(tab_content);

    if let Some(status) = &s.action_status {
        col = col.push(settings_success_banner(status));
    }

    col.into()
}

fn active_save_error(app: &App) -> Option<&str> {
    app.cron_settings.save_error.as_deref().or(app.scheduler_settings.save_error.as_deref())
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

    let header = row![
        column![
            text("执行历史").size(18),
            text(job_label).size(12).style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        settings_close_button(Message::Settings(message::SettingsMessage::CronJobRunsClose)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut rows = column![].spacing(0).width(Length::Fill);
    if s.runs_modal_loading {
        rows = rows.push(text("正在加载历史记录...").size(13).style(settings_muted_text_style));
    } else if let Some(err) = &s.runs_modal_error {
        rows = rows.push(settings_error_banner(err));
    } else if s.runs_modal.is_empty() {
        rows = rows.push(text("暂无执行历史").size(13).style(settings_muted_text_style));
    } else {
        for run in &s.runs_modal {
            rows = rows.push(run_row(run)).push(settings_divider());
        }
    }

    let body: Element<'_, Message> =
        if s.runs_modal_loading || s.runs_modal_error.is_some() || s.runs_modal.is_empty() {
            scrollable(container(rows).width(Length::Fill).padding(iced::Padding {
                top: 0.0,
                right: 8.0,
                bottom: 0.0,
                left: 0.0,
            }))
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .into()
        } else {
            text_editor(&s.runs_modal_editor)
                .placeholder("暂无执行历史")
                .on_action(|action| {
                    Message::Settings(message::SettingsMessage::CronJobRunsEditorAction(action))
                })
                .size(12)
                .padding(SETTINGS_CONTROL_PADDING)
                .height(Length::Fill)
                .style(settings_text_editor_style)
                .into()
        };

    let card = settings_modal_card(
        column![header, settings_divider(), body].spacing(14).height(Length::Fill),
    )
    .width(Length::Fixed(780.0))
    .height(Length::Fixed(620.0));

    settings_modal_overlay(
        Some(dialog),
        Message::Settings(message::SettingsMessage::CronJobRunsClose),
        card,
    )
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let dialog = with_runs_modal(app, dialog);

    if !s.show_help_modal {
        return dialog;
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
