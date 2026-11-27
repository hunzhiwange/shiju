//! 新增和编辑共用的任务表单，保存边界位于固定弹窗页脚。
use super::*;
use crate::message::SettingsMessage as S;
use crate::state::{CronEditorTab, CronJobDraft};
use desktop_ui_controls::system_settings_common::settings_toggle;
use iced::widget::column;

pub fn view<'a>(app: &ViewData<'a>, editing: bool) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let draft = if editing { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
    let form: Element<'_, Message> = match draft.editor_tab {
        CronEditorTab::Basic => column![
            text_field(i18n.__("输入任务名称"), &draft.name, |name| Message::Settings(
                S::CronJobEditNameChanged(name)
            )),
            composer::view(app, draft, editing),
            schedule::frequency_form(app, draft, editing),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into(),
        CronEditorTab::Advanced => advanced_fields(app, draft, editing),
        CronEditorTab::Delivery => delivery_fields(app, draft),
        CronEditorTab::Preview => schedule::preview(i18n, draft, editing),
    };

    let submitting = draft.submitting;
    form.map(move |message| {
        if submitting {
            Message::None
        } else if editing {
            message
        } else {
            for_add(message)
        }
    })
}

fn delivery_fields<'a>(app: &ViewData<'_>, draft: &'a CronJobDraft) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut fields = column![
        settings_toggle(draft.delivery_enabled)
            .label(i18n.__("开启结果投递"))
            .on_toggle(|enabled| Message::Settings(S::CronJobEditDeliveryEnabledToggled(enabled))),
    ]
    .spacing(12)
    .width(Length::Fill);
    if draft.delivery_enabled {
        fields = fields.push(super::delivery::view(app, draft));
    }
    if let Err(error) = crate::delivery::validate(draft, app.delivery) {
        fields = fields.push(settings_error_banner(desktop_i18n::settings::message(i18n, &error)));
    }
    fields.into()
}

fn advanced_fields<'a>(
    app: &ViewData<'_>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut fields = column![
        field_row_with_help(
            app,
            "任务类型",
            "Shell 执行命令，Agent 执行提示词任务。",
            row![
                segment_button(
                    "Agent",
                    draft.job_type == CronAddJobType::Agent,
                    Message::Settings(S::CronJobEditJobTypeChanged(CronAddJobType::Agent))
                ),
                segment_button(
                    "Shell",
                    draft.job_type == CronAddJobType::Shell,
                    Message::Settings(S::CronJobEditJobTypeChanged(CronAddJobType::Shell))
                ),
            ]
            .spacing(8),
        ),
        schedule::timezone_field(i18n, draft, editing),
        field_row_with_help(
            app,
            "项目路径",
            "可手工输入项目目录；留空使用默认工作区。",
            text_field(i18n.__("项目目录路径"), &draft.project_path, |v| {
                Message::Settings(S::CronJobEditProjectPathChanged(v))
            }),
        ),
    ];
    if draft.job_type == CronAddJobType::Agent {
        fields = fields
            .push(field_row_with_help(
                app,
                "ACP 代理",
                "可选，保留已有 ACP 执行代理设置。",
                text_field("ACP agent", &draft.acp_agent, |v| {
                    Message::Settings(S::CronJobEditAcpAgentChanged(v))
                }),
            ))
            .push(field_row_with_help(
                app,
                "回退模型",
                "主模型失败后依次尝试。",
                fallback_models(app, draft),
            ))
            .push(field_row(
                i18n.__("任务池"),
                i18n.__("开启后触发时投递到项目任务池，由任务看板接管执行。"),
                checkbox(draft.task_pool)
                    .label(i18n.__("投递到任务池"))
                    .on_toggle(|v| Message::Settings(S::CronJobEditTaskPoolToggled(v)))
                    .style(settings_checkbox_style),
            ))
            .push(field_row_with_help(
                app,
                "唤醒",
                "保留唤醒标记，供桌面侧识别需要关注的任务。",
                checkbox(draft.wake)
                    .label(i18n.__("任务运行时标记为需要关注"))
                    .on_toggle(|v| Message::Settings(S::CronJobEditWakeToggled(v)))
                    .style(settings_checkbox_style),
            ));
    }
    fields = fields.push(field_row_with_help(
        app,
        "执行后删除",
        "适合指定时间的一次性任务。",
        checkbox(draft.delete_after_run)
            .label(i18n.__("运行成功后删除任务"))
            .on_toggle(|v| Message::Settings(S::CronJobEditDeleteAfterRunToggled(v)))
            .style(settings_checkbox_style),
    ));
    settings_group_card(fields).into()
}

fn fallback_models<'a>(app: &ViewData<'_>, draft: &CronJobDraft) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let selected = desktop_config::config_cron_jobs::parse_fallback_models(&draft.fallbacks);
    let mut controls = column![].spacing(6);
    for (index, id) in selected.iter().enumerate() {
        let remaining = selected[..index]
            .iter()
            .chain(&selected[index + 1..])
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        controls = controls.push(
            row![
                text(id.clone()).size(SETTINGS_CONTROL_TEXT_SIZE).width(Length::Fill),
                small_button(
                    i18n.__("移除"),
                    Message::Settings(S::CronJobEditFallbacksChanged(remaining)),
                    false,
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    let candidates: Vec<_> = app
        .models
        .iter()
        .flat_map(|group| &group.models)
        .filter(|model| model.id != draft.model && !selected.contains(&model.id))
        .map(|model| model.id.clone())
        .collect();
    let picker = settings_select(candidates, None, move |id| {
        let mut fallbacks = selected.clone();
        fallbacks.push(id);
        Message::Settings(S::CronJobEditFallbacksChanged(fallbacks.join("\n")))
    })
    .placeholder(i18n.__("添加回退模型"))
    .width(Length::Fill);
    controls.push(picker).into()
}

fn for_add(message: Message) -> Message {
    use message::SettingsMessage as S;
    match message {
        Message::Settings(S::CronJobEditAgentChanged(v)) => {
            Message::Settings(S::CronAddAgentChanged(v))
        }
        Message::Settings(S::CronJobEditCommandEditorAction(v)) => {
            Message::Settings(S::CronAddCommandEditorAction(v))
        }
        Message::Settings(S::CronJobEditDeleteAfterRunToggled(v)) => {
            Message::Settings(S::CronAddDeleteAfterRunToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryBestEffortToggled(v)) => {
            Message::Settings(S::CronAddDeliveryBestEffortToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryChannelChanged(v)) => {
            Message::Settings(S::CronAddDeliveryChannelChanged(v))
        }
        Message::Settings(S::CronJobEditDeliveryEnabledToggled(v)) => {
            Message::Settings(S::CronAddDeliveryEnabledToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryToChanged(v)) => {
            Message::Settings(S::CronAddDeliveryToChanged(v))
        }
        Message::Settings(S::CronJobEditFallbacksChanged(v)) => {
            Message::Settings(S::CronAddFallbacksChanged(v))
        }
        Message::Settings(S::CronJobEditFullAccessToggled(v)) => {
            Message::Settings(S::CronAddFullAccessToggled(v))
        }
        Message::Settings(S::CronJobEditJobTypeChanged(v)) => {
            Message::Settings(S::CronAddJobTypeChanged(v))
        }
        Message::Settings(S::CronJobEditNameChanged(v)) => {
            Message::Settings(S::CronAddNameChanged(v))
        }
        Message::Settings(S::CronJobEditProjectPathChanged(v)) => {
            Message::Settings(S::CronAddProjectPathChanged(v))
        }
        Message::Settings(S::CronJobEditPromptEditorAction(v)) => {
            Message::Settings(S::CronAddPromptEditorAction(v))
        }
        Message::Settings(S::CronJobEditTaskPoolToggled(v)) => {
            Message::Settings(S::CronAddTaskPoolToggled(v))
        }
        Message::Settings(S::CronJobEditWakeToggled(v)) => {
            Message::Settings(S::CronAddWakeToggled(v))
        }
        Message::Settings(S::CronJobEditAcpAgentChanged(v)) => {
            Message::Settings(S::CronAddAcpAgentChanged(v))
        }
        other => other,
    }
}
