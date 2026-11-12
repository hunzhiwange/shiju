//! 新增和编辑共用的任务表单，保存边界位于固定弹窗页脚。
use super::*;
use iced::widget::column;

pub(super) fn view(app: &App, editing: bool) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let draft = if editing { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
    let agent_options = with_selected_option(
        app.agents_settings
            .entries
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.key.clone())
            .collect(),
        &draft.agent,
    );
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
    let schedule_row = schedule::frequency_form(app, draft, true);

    let mut form = column![
        field_row_with_help(
            app,
            "任务名称",
            i18n.__("可选，便于在列表中识别。"),
            text_field(i18n.__("可选名称"), &draft.name, |v| Message::Settings(
                message::SettingsMessage::CronJobEditNameChanged(v)
            ))
        ),
        field_row_with_help(
            app,
            "任务类型",
            i18n.__("Shell 执行命令，Agent 执行提示词任务。"),
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
            .spacing(8)
            .wrap(),
        ),
        schedule_row,
        field_row(
            i18n.__("项目"),
            i18n.__("选择任务执行所在项目，Shell 和 Agent 都会切到该目录。"),
            project_field(app, &draft.project_path, |v| Message::Settings(
                message::SettingsMessage::CronJobEditProjectPathChanged(v)
            ),),
        ),
        field_row_with_help(
            app,
            "项目路径",
            i18n.__("可手工输入项目目录；留空使用默认工作区。"),
            text_field(i18n.__("项目目录路径"), &draft.project_path, |v| {
                Message::Settings(message::SettingsMessage::CronJobEditProjectPathChanged(v))
            }),
        ),
    ]
    .spacing(0);

    form = match draft.job_type {
        CronAddJobType::Shell => form.push(field_row_with_help(
            app,
            "执行命令",
            i18n.__("按安全策略执行的 Shell 命令。"),
            text_editor(&draft.command_editor)
                .placeholder(i18n.__("Shell 命令"))
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
            .push(field_row_with_help(
                app,
                "智能体",
                i18n.__("默认使用 Main，套用代理配置中的 provider、模型和温度。"),
                pick_field(
                    agent_options,
                    (!draft.agent.trim().is_empty()).then_some(draft.agent.clone()),
                    |v| Message::Settings(message::SettingsMessage::CronJobEditAgentChanged(v)),
                ),
            ))
            .push(field_row_with_help(
                app,
                "模型提供商",
                i18n.__("与智能体配置中的模型选择一致。"),
                pick_field(provider_options, selected_provider, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditModelProviderChanged(v))
                }),
            ))
            .push(field_row_with_help(
                app,
                "模型下拉",
                i18n.__("从当前提供商的模型列表选择，会写入 provider/model。"),
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
            .push(field_row_with_help(
                app,
                "模型手工输入",
                i18n.__("可选，留空使用代理或全局默认模型。"),
                text_field("provider/model", &draft.model, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditModelChanged(v))
                }),
            ))
            .push(field_row_with_help(
                app,
                "Agent 提示词",
                i18n.__("创建 Agent 类型定时任务。"),
                settings_text_editor(&draft.prompt_editor)
                    .placeholder(i18n.__("要让 Agent 执行的任务"))
                    .on_action(|action| {
                        Message::Settings(message::SettingsMessage::CronJobEditPromptEditorAction(
                            action,
                        ))
                    })
                    .height(Length::Fixed(140.0)),
            ))
            .push(field_row_with_help(
                app,
                "ACP 代理",
                "可选，保留已有 ACP 执行代理设置。",
                text_field("ACP agent", &draft.acp_agent, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditAcpAgentChanged(v))
                }),
            ))
            .push(field_row_with_help(
                app,
                "回退模型",
                i18n.__("主模型失败后依次尝试，使用逗号或换行分隔。"),
                text_field("provider/model, provider/model", &draft.fallbacks, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditFallbacksChanged(v))
                }),
            ))
            .push(field_row(
                i18n.__("完全访问"),
                i18n.__("开启后该 Agent 定时任务按发送面板的完全访问权限执行。"),
                checkbox(draft.full_access)
                    .label(i18n.__("获取完全访问权限"))
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditFullAccessToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row(
                i18n.__("任务池"),
                i18n.__("开启后触发时投递到项目任务池，由任务看板接管执行。"),
                checkbox(draft.task_pool)
                    .label(i18n.__("投递到任务池"))
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditTaskPoolToggled(v))
                    })
                    .style(settings_checkbox_style),
            ))
            .push(field_row_with_help(
                app,
                "唤醒",
                i18n.__("保留唤醒标记，供桌面侧识别需要关注的任务。"),
                checkbox(draft.wake)
                    .label(i18n.__("任务运行时标记为需要关注"))
                    .on_toggle(|v| {
                        Message::Settings(message::SettingsMessage::CronJobEditWakeToggled(v))
                    })
                    .style(settings_checkbox_style),
            )),
    };

    form = form
        .push(field_row_with_help(
            app,
            "执行后删除",
            i18n.__("适合指定时间的一次性任务。"),
            checkbox(draft.delete_after_run)
                .label(i18n.__("运行成功后删除任务"))
                .on_toggle(|v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeleteAfterRunToggled(v))
                })
                .style(settings_checkbox_style),
        ))
        .push(field_row_with_help(
            app,
            "结果投递",
            i18n.__("将任务执行结果投递到配置好的通道。"),
            checkbox(draft.delivery_enabled)
                .label(i18n.__("开启结果投递"))
                .on_toggle(|v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryEnabledToggled(
                        v,
                    ))
                })
                .style(settings_checkbox_style),
        ));

    if draft.delivery_enabled {
        form = form
            .push(field_row_with_help(
                app,
                "投递通道",
                i18n.__("例如 telegram、discord、slack、mattermost、email。"),
                text_field("channel", &draft.delivery_channel, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryChannelChanged(
                        v,
                    ))
                }),
            ))
            .push(field_row(
                i18n.__("投递目标"),
                i18n.__("通道内的目标 ID、地址或频道。"),
                text_field("target", &draft.delivery_to, |v| {
                    Message::Settings(message::SettingsMessage::CronJobEditDeliveryToChanged(v))
                }),
            ))
            .push(field_row(
                i18n.__("尽力投递"),
                "",
                checkbox(draft.delivery_best_effort)
                    .label(i18n.__("投递失败不影响任务状态"))
                    .on_toggle(|v| {
                        Message::Settings(
                            message::SettingsMessage::CronJobEditDeliveryBestEffortToggled(v),
                        )
                    })
                    .style(settings_checkbox_style),
            ));
    }

    let element: Element<'_, Message> = settings_group_card(form).into();
    let submitting = draft.submitting;
    element.map(move |message| {
        if submitting {
            Message::None
        } else if editing {
            message
        } else {
            for_add(message)
        }
    })
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
        Message::Settings(S::CronJobEditModelChanged(v)) => {
            Message::Settings(S::CronAddModelChanged(v))
        }
        Message::Settings(S::CronJobEditModelProviderChanged(v)) => {
            Message::Settings(S::CronAddModelProviderChanged(v))
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
        Message::Settings(S::CronScheduleDraftChanged { draft, .. }) => {
            Message::Settings(S::CronScheduleDraftChanged { editing: false, draft })
        }
        Message::Settings(S::CronSchedulePreview { .. }) => {
            Message::Settings(S::CronSchedulePreview { editing: false })
        }
        Message::Settings(S::CronPickerOpen { .. }) => {
            Message::Settings(S::CronPickerOpen { editing: false })
        }
        Message::Settings(S::CronJobEditAcpAgentChanged(v)) => {
            Message::Settings(S::CronAddAcpAgentChanged(v))
        }
        other => other,
    }
}
