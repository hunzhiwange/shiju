use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::i18n::I18n;
use crate::app::message::chat::media_jobs::{self, AssetAction, Event, JobEntry, SessionKey};
use crate::app::{App, Message};
use iced::widget::{Column, button, column, container, image, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_gateway_client::media::MediaJobState;

pub(in crate::app::components::chat_panel) fn view(app: &App) -> Option<Element<'_, Message>> {
    let (key, jobs) = media_jobs::active_jobs(app)?;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut content = column![].spacing(8).width(Length::Fill);
    for entry in jobs {
        content = content.push(job_view(i18n, &key, entry));
    }
    let scrollbar = scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(7);
    Some(
        container(scrollable(content).direction(scrollable::Direction::Vertical(scrollbar)))
            .width(Length::Fill)
            .max_height(260)
            .into(),
    )
}

fn job_view<'a>(i18n: I18n, key: &SessionKey, entry: &'a JobEntry) -> Element<'a, Message> {
    let id = entry.reference.id;
    let mut header = row![
        text(i18n.__(media_jobs::operation_label(entry.reference.operation)).to_owned()).size(13),
        text(status(i18n, entry)).size(12),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if entry.can_cancel() {
        header = header.push(action(
            i18n,
            "取消",
            Some(media_jobs::message(Event::Cancel(key.clone(), id))),
        ));
    }
    if !entry.refreshing
        && !entry.canceling
        && (entry.error.is_some()
            || entry
                .snapshot
                .as_ref()
                .is_some_and(|job| matches!(job.state, MediaJobState::SubmissionUnknown)))
    {
        header = header.push(action(
            i18n,
            "刷新",
            Some(media_jobs::message(Event::Refresh(key.clone(), id))),
        ));
    }

    let mut content: Column<'a, Message> = column![header.wrap()].spacing(6).width(Length::Fill);
    if let Some(result) = entry.result() {
        if let Some(value) = &result.text {
            content = content.push(text(value.as_str()).size(14)).push(action(
                i18n,
                "复制",
                Some(Message::CopyCode(value.clone())),
            ));
        }
        for asset in &result.assets {
            if let Some(handle) = entry.previews.get(&asset.id) {
                content = content.push(image(handle.clone()).height(Length::Fixed(128.0)));
            }
            let mut actions =
                row![text(asset.mime.as_str()).size(12)].spacing(7).align_y(Alignment::Center);
            for operation in media_jobs::asset_actions(asset) {
                let label = match operation {
                    AssetAction::Preview if entry.previews.contains_key(&asset.id) => continue,
                    AssetAction::Preview => "预览图片",
                    AssetAction::Play => "播放",
                    AssetAction::Save => "保存",
                };
                let event =
                    (!entry.asset_busy.contains(&(asset.id.clone(), operation))).then(|| {
                        media_jobs::message(Event::Asset(
                            key.clone(),
                            id,
                            asset.id.clone(),
                            operation,
                        ))
                    });
                actions = actions.push(action(i18n, label, event));
            }
            content = content.push(actions.wrap());
        }
    }
    content.into()
}

fn action<'a>(
    i18n: I18n,
    label: &str,
    message: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    button(text(i18n.__(label).to_owned()).size(12))
        .padding([4, 8])
        .style(rounded_action_btn_style)
        .on_press_maybe(message)
}

fn status(i18n: I18n, entry: &JobEntry) -> String {
    if let Some(error) = entry.error {
        return i18n.__(error).to_owned();
    }
    if entry.canceling {
        return i18n.__("正在取消媒体任务…").to_owned();
    }
    let label = match entry.snapshot.as_ref().map(|job| &job.state) {
        None => "正在查询媒体任务…",
        Some(MediaJobState::Submitted) => "媒体任务已提交",
        Some(MediaJobState::Running { progress_percent: None }) => "媒体任务处理中",
        Some(MediaJobState::Running { progress_percent: Some(progress) }) => {
            return format!("{} {}%", i18n.__("媒体任务处理中"), progress);
        }
        Some(MediaJobState::Succeeded { .. }) => "媒体任务已完成",
        Some(MediaJobState::Failed { code }) => media_jobs::service_error(*code),
        Some(MediaJobState::Canceled) => "媒体任务已取消",
        Some(MediaJobState::SubmissionUnknown) => "媒体任务提交结果未知",
    };
    i18n.__(label).to_owned()
}
