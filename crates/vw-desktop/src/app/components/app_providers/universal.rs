use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::universal::{Event, message},
    state::app_providers::universal::Form,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::universal_sync::{
        UniversalCatalogResult, UniversalLiveResult, UniversalSyncError,
    },
};

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

fn reason_label(reason: UniversalSyncError) -> &'static str {
    match reason {
        UniversalSyncError::UnsupportedApp => "此应用不支持统一投影",
        UniversalSyncError::AppDisabled => "统一记录未启用此应用",
        UniversalSyncError::InvalidTargetIdentity => "生成的档案标识无效",
        UniversalSyncError::InvalidProfile => "供应商档案无效",
        UniversalSyncError::TargetConflict => "档案标识与已有供应商冲突",
        UniversalSyncError::StorageUnavailable => "目录存储暂不可用",
        UniversalSyncError::ChangedDuringUpdate => "档案已被其他操作修改，请重试",
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

pub(super) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let mut fields = column![].spacing(12);
    if let Some(records) = &form.records {
        if records.is_empty() {
            fields = fields.push(text("没有已保存的统一供应商记录。"));
        } else {
            let options = records.iter().map(|record| RecordOption {
                id: record.id.clone(),
                name: record.config.name.clone(),
            });
            let selected = form.selected_record();
            fields = fields.push(text("统一供应商记录").size(16)).push(
                settings_select(
                    options,
                    selected.map(|record| RecordOption {
                        id: record.id.clone(),
                        name: record.config.name.clone(),
                    }),
                    |option| message(Event::Select(option.id)),
                )
                .placeholder("选择已保存的记录")
                .width(Length::Fill)
                .disabled(form.busy),
            );
            if let Some(record) = selected {
                let config = &record.config;
                let mut details =
                    column![text("记录详情").size(16), text(&config.base_url).size(13)].spacing(6);
                if let Some(notes) = &config.notes {
                    details = details.push(text(notes).size(13).style(settings_muted_text_style));
                }
                let mut apps = column![text("选择目标应用").size(16)].spacing(10);
                for (app, enabled) in [
                    (AppId::Claude, config.apps.claude),
                    (AppId::Codex, config.apps.codex),
                    (AppId::Gemini, config.apps.gemini),
                ] {
                    apps = apps.push(
                        checkbox(form.apps.contains(&app))
                            .label(format!(
                                "{} · 记录{}",
                                app_label(app),
                                if enabled { "已启用" } else { "未启用" }
                            ))
                            .on_toggle_maybe(
                                (!form.busy)
                                    .then_some(move |checked| message(Event::Toggle(app, checked))),
                            )
                            .style(settings_checkbox_style),
                    );
                }
                fields = fields.push(settings_panel(details)).push(settings_panel(apps));
            }
        }
    } else if form.busy {
        fields = fields.push(text("正在读取统一供应商记录…"));
    } else {
        fields = fields.push(action("重试读取", Event::RetryList, true, false));
    }
    if let Some(error) = &form.error {
        fields = fields.push(settings_panel(text(format!("请求失败：{error}")).size(13)));
    }
    if !form.results.is_empty() {
        fields = fields.push(text("逐应用结果").size(16));
    }
    for item in &form.results {
        let catalog = match item.catalog {
            UniversalCatalogResult::Updated => "目录已更新".to_owned(),
            UniversalCatalogResult::Failed { code } => {
                format!("目录更新失败：{}", reason_label(code))
            }
        };
        let live = match item.live {
            UniversalLiveResult::NotAttempted => "原生配置未写入",
        };
        let mut result = column![
            text(app_label(item.app_id)).size(16),
            text(catalog).size(13),
            text(live).size(13)
        ]
        .spacing(6);
        if let Some(id) = &item.provider_id {
            result = result.push(text(format!("档案标识：{id}")).size(13));
        }
        fields = fields.push(settings_panel(result));
    }
    let close = if form.busy { Message::None } else { message(Event::Close) };
    let card = settings_modal_card(
        column![
            row![
                text("统一供应商投影").size(22).width(Length::Fill),
                settings_close_button(close.clone())
            ]
            .align_y(Alignment::Center),
            text("将已保存的记录投影到所选应用目录。").size(13).style(settings_muted_text_style),
            scrollable(container(fields).padding([0, 7]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            row![
                action("返回", Event::Close, !form.busy, false),
                action(
                    if form.busy && form.records.is_some() {
                        "投影中…"
                    } else {
                        "保存投影"
                    },
                    Event::Submit,
                    !form.busy && form.selected.is_some() && !form.apps.is_empty(),
                    true
                ),
            ]
            .spacing(10),
        ]
        .spacing(12),
    )
    .width(Length::Fixed(600.))
    .height(Length::Fixed(610.));
    settings_modal_overlay(Some(base), close, card)
}
