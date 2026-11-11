//! The archive sits in the existing settings scroll area (4 px gutter).
use super::system_settings_common::*;
use crate::app::message::voice_recordings::{Event, State};
use crate::app::{App, Message};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, row, text},
};
fn action(label: &str, event: Event) -> Element<'_, Message> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press(Message::Voice(crate::app::message::voice::Event::Recordings(event)))
        .into()
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    content(&app.voice.recordings)
}
pub(super) fn content(state: &State) -> Element<'_, Message> {
    let mut items=column![
        settings_field_label("最近本机录音"),
        text("主动开启后仅在本机保存最近二十段成功听写（总计最多 64 MiB）；关闭后停止新增，已有录音可手动删除。").style(settings_muted_text_style),
        text(crate::app::voice::recordings::Store::path().map(|p|p.display().to_string()).unwrap_or_else(|e|e)).size(12),
        row![action("刷新",Event::Refresh),action("删除全部",Event::Clear)].spacing(8),
    ].spacing(10);
    if let Some(problem) = &state.problem {
        items = items.push(text(problem).style(settings_muted_text_style));
    }
    if state.entries.is_empty() {
        items = items.push(text("尚无本机录音。"));
    }
    for (index, entry) in state.entries.iter().enumerate().rev() {
        let seconds = entry.bytes.saturating_sub(44) / 48_000;
        let date = chrono::DateTime::<chrono::Local>::from(entry.modified)
            .format("%m-%d %H:%M:%S")
            .to_string();
        items = items.push(
            row![
                text(format!("录音 {} · {} · {} 秒", index + 1, date, seconds)).width(Length::Fill),
                action("播放", Event::Play(entry.id.clone())),
                action("重新识别", Event::Retry(entry.id.clone())),
                action("删除", Event::Delete(entry.id.clone())),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
        );
    }
    if state.replay.is_some() {
        items = items.push(action("停止回放", Event::Stop));
    }
    if state.busy {
        items = items.push(row![text("正在重新识别…"), action("取消", Event::Stop)].spacing(8));
    }
    if let Some(result) = &state.result {
        items = items.push(text(result)).push(
            row![action("回填到当前输入框", Event::Apply), action("放弃结果", Event::Dismiss)]
                .spacing(8),
        );
    }
    items.into()
}
#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
