#[derive(Clone)]
pub enum Event { Close, Select(String), Toggle(vw_api_app_identity::AppId, bool), RetryList, Submit }
fn message(event: Event) -> Message { Message::Universal(event) }
use desktop_ui_controls::system_settings_common::*;
use super::help::Help;
use crate::{Message, state::universal::*};
use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, row, text},
};
use vw_api_types::agent_apps::AppId;
use vw_api_app_provider_edit::universal_sync::UniversalCatalogResult;
use vw_api_app_provider_edit::universal_sync::UniversalLiveResult;
use vw_api_app_provider_edit::universal_sync::UniversalSyncError;

#[derive(Clone, PartialEq, Eq)]
struct RecordOption {
    id: String,
    name: String,
}

impl std::fmt::Display for RecordOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

fn app_label(app: AppId) -> &'static str {
    match app {
        AppId::Claude => "Claude Code",
        AppId::Codex => "Codex",
        AppId::Gemini => "Gemini CLI",
        _ => "其他应用",
    }
}

fn reason_label(i18n: desktop_i18n::I18n, reason: UniversalSyncError) -> &'static str {
    match reason {
        UniversalSyncError::UnsupportedApp => i18n.__("此智能体不支持添加该配置"),
        UniversalSyncError::AppDisabled => i18n.__("此配置未对该智能体开放"),
        UniversalSyncError::InvalidTargetIdentity => i18n.__("生成的档案标识无效"),
        UniversalSyncError::InvalidProfile => i18n.__("供应商档案无效"),
        UniversalSyncError::TargetConflict => i18n.__("档案标识与已有供应商冲突"),
        UniversalSyncError::StorageUnavailable => i18n.__("目录存储暂不可用"),
        UniversalSyncError::ChangedDuringUpdate => i18n.__("档案已被其他操作修改，请重试"),
    }
}

fn action(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    button(text(label))
        .padding([8, 14])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}

pub fn card<'a>(
    form: &'a Form,
    help: Help<'_>,
) -> Element<'a, Message> {
    let i18n = help.i18n;
    let mut fields = column![].spacing(12);
    if let Some(records) = &form.records {
        if records.is_empty() {
            fields = fields.push(text(i18n.__("没有可添加的供应商配置。")));
        } else {
            let options = records.iter().map(|record| RecordOption {
                id: record.id.clone(),
                name: record.config.name.clone(),
            });
            let selected = form.selected_record();
            fields = fields.push(text(i18n.__("已保存的供应商配置")).size(16)).push(
                settings_select(
                    options,
                    selected.map(|record| RecordOption {
                        id: record.id.clone(),
                        name: record.config.name.clone(),
                    }),
                    |option| message(Event::Select(option.id)),
                )
                .placeholder(i18n.__("选择供应商配置"))
                .width(Length::Fill)
                .disabled(form.busy),
            );
            if let Some(record) = selected {
                let config = &record.config;
                let mut details =
                    column![text(i18n.__("供应商配置详情")).size(16), text(&config.base_url).size(13)].spacing(6);
                if let Some(notes) = &config.notes {
                    details = details.push(text(notes).size(13).style(settings_muted_text_style));
                }
                let mut apps = column![text(i18n.__("选择要添加到的智能体")).size(16)].spacing(10);
                for (app, enabled) in [
                    (AppId::Claude, config.apps.claude),
                    (AppId::Codex, config.apps.codex),
                    (AppId::Gemini, config.apps.gemini),
                ] {
                    apps = apps.push(
                        checkbox(form.apps.contains(&app))
                            .label(
                                i18n.format(
                                    if enabled { "%s · 配置已开放" } else { "%s · 配置未开放" },
                                    &[(app_label(app).to_string()).into()],
                                )
                                .expect("valid display template"),
                            )
                            .on_toggle_maybe(
                                (!form.busy)
                                    .then_some(move |checked| message(Event::Toggle(app, checked))),
                            )
                            .style(settings_checkbox_style),
                    );
                }
                fields = fields
                    .push(settings_divider())
                    .push(settings_section_content(details))
                    .push(settings_divider())
                    .push(settings_section_content(apps));
            }
        }
    } else if form.busy {
        fields = fields.push(text(i18n.__("正在读取供应商配置…")));
    } else {
        fields = fields.push(action(i18n.__("重试读取"), Event::RetryList, true, false));
    }
    if let Some(error) = &form.error {
        fields = fields.push(settings_panel(text(i18n.format("请求失败：%s", &[(error.to_string()).into()]).expect("valid display template")).size(13)));
    }
    if !form.results.is_empty() {
        fields = fields.push(text(i18n.__("添加结果")).size(16));
    }
    for item in &form.results {
        let catalog = match item.catalog {
            UniversalCatalogResult::Updated => i18n.__("已添加到供应商列表").to_owned(),
            UniversalCatalogResult::Failed { code } => {
                i18n.format("添加失败：%s", &[(reason_label(i18n, code).to_string()).into()]).expect("valid display template")
            }
        };
        let live = match item.live {
            UniversalLiveResult::NotAttempted => i18n.__("原生配置未写入"),
        };
        let mut result = column![
            text(app_label(item.app_id)).size(16),
            text(catalog).size(13),
            text(live).size(13)
        ]
        .spacing(6);
        if let Some(id) = &item.provider_id {
            result = result.push(text(i18n.format("档案标识：%s", &[(id.to_string()).into()]).expect("valid display template")).size(13));
        }
        fields = fields.push(settings_panel(result));
    }
    let close = if form.busy { Message::None } else { message(Event::Close) };
    let header = settings_modal_header(
        help.heading(
            i18n.__("添加到其他智能体"),
            i18n.__("把已保存的供应商配置添加到所选智能体的供应商列表。添加后需手动启用。"),
            22,
        ),
        settings_close_button(close),
    );
    let footer = row![
        action(i18n.__("返回"), Event::Close, !form.busy, false),
        action(
            if form.busy && form.records.is_some() {
                i18n.__("正在添加供应商配置…")
            } else {
                i18n.__("添加")
            },
            Event::Submit,
            !form.busy && form.selected.is_some() && !form.apps.is_empty(),
            true
        ),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, fields, Some(footer.into())).width(600).height(610);
    card.into()
}
