//! The archive sits in the existing settings scroll area (4 px gutter).
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, row, text},
};
fn action<'a, Message: Clone + 'a>(label: &'a str, event: Event, map: fn(Event) -> Message) -> Element<'a, Message> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press(map(event))
        .into()
}
pub fn content<'a>(state: View<'a>, archive_path: Result<std::path::PathBuf, String>) -> Element<'a, Event> {
    let map = std::convert::identity::<Event>;
    let mut items=column![
        settings_field_label("最近本机录音"),
        text("主动开启后仅在本机保存最近二十段成功听写（总计最多 64 MiB）；关闭后停止新增，已有录音可手动删除。").style(settings_muted_text_style),
        text(archive_path.map(|p|p.display().to_string()).unwrap_or_else(|e|e)).size(12),
        row![action("刷新", Event::Refresh, map),action("删除全部", Event::Clear, map)].spacing(8),
    ].spacing(10);
    if let Some(problem) = state.problem {
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
                action("播放", Event::Play(entry.id.clone()), map),
                action("重新识别", Event::Retry(entry.id.clone()), map),
                action("删除", Event::Delete(entry.id.clone()), map),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
        );
    }
    if state.replaying {
        items = items.push(action("停止回放", Event::Stop, map));
    }
    if state.busy {
        items = items.push(row![text("正在重新识别…"), action("取消", Event::Stop, map)].spacing(8));
    }
    if let Some(result) = state.result {
        items = items.push(text(result)).push(
            row![action("回填到当前输入框", Event::Apply, map), action("放弃结果", Event::Dismiss, map)]
                .spacing(8),
        );
    }
    items.into()
}

pub struct View<'a> {
    pub entries: &'a [voice_recordings::Entry],
    pub result: Option<&'a str>,
    pub problem: Option<&'a str>,
    pub replaying: bool,
    pub busy: bool,
}
#[derive(Clone)]
pub enum Event { Refresh, Clear, Play(String), Retry(String), Delete(String), Stop, Apply, Dismiss }

#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
