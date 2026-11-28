//! The archive sits in the existing settings scroll area.
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, row, text},
};

fn action(label: &str, event: Event) -> iced::widget::Button<'_, Event> {
    button(text(label).size(14)).padding([8, 14]).style(rounded_action_btn_style).on_press(event)
}

pub fn content<'a>(
    i18n: I18n,
    state: View<'a>,
    archive_path: Result<std::path::PathBuf, String>,
) -> Element<'a, Event> {
    let mut actions = row![action(i18n.__("刷新"), Event::Refresh)].spacing(8);
    if !state.entries.is_empty() {
        actions = actions.push(action(i18n.__("删除全部"), Event::Clear));
    }
    let (title, description) = if state.problem.is_some() || !state.entries.is_empty() {
        (i18n.__("听写录音"), "")
    } else {
        (i18n.__("尚无本机录音。"), i18n.__("在语音设置中开启保存听写录音。"))
    };
    let mut items = column![
        settings_flexible_row(title, description, 240.0, actions.wrap().align_x(Alignment::End))
            .padding([12, 0])
    ];
    if let Some(problem) = state.problem {
        items = items.push(settings_error_banner(crate::feedback::render(i18n, problem)));
    }
    for (index, entry) in state.entries.iter().enumerate().rev() {
        let seconds = entry.bytes.saturating_sub(44) / 48_000;
        let date = chrono::DateTime::<chrono::Local>::from(entry.modified)
            .format("%m-%d %H:%M:%S")
            .to_string();
        let label = i18n
            .format(
                "录音 %u · %s · %u 秒",
                &[(index + 1).into(), date.as_str().into(), seconds.into()],
            )
            .expect("valid recording label");
        items = items.push(settings_divider()).push(
            iced::widget::container(
                row![
                    text(label).size(14).width(Length::Fill),
                    action(i18n.__("播放"), Event::Play(entry.id.clone())),
                    action(i18n.__("重新识别"), Event::Retry(entry.id.clone())),
                    action(i18n.__("删除"), Event::Delete(entry.id.clone())),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .wrap(),
            )
            .padding([10, 0])
            .width(Length::Fill),
        );
    }
    if state.replaying {
        items = items.push(action(i18n.__("停止回放"), Event::Stop));
    }
    if state.busy {
        items = items.push(
            row![text(i18n.__("正在重新识别…")), action(i18n.__("取消"), Event::Stop)]
                .align_y(Alignment::Center)
                .spacing(8),
        );
    }
    if let Some(result) = state.result {
        items = items.push(settings_panel(
            column![
                text(result).width(Length::Fill),
                row![
                    action(i18n.__("回填到当前输入框"), Event::Apply),
                    action(i18n.__("放弃结果"), Event::Dismiss),
                ]
                .spacing(8)
                .wrap(),
            ]
            .spacing(10),
        ));
    }
    let mut page = column![text(i18n.__("最近本机录音")).size(17), settings_group_card(items),]
        .spacing(8)
        .width(Length::Fill);
    if !state.entries.is_empty() {
        match archive_path {
            Ok(path) => {
                page = page.push(
                    text(path.display().to_string())
                        .width(Length::Fill)
                        .size(12)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .style(settings_muted_text_style),
                );
            }
            Err(error) if state.problem.is_none() => {
                page = page.push(settings_error_banner(crate::feedback::render(i18n, &error)));
            }
            Err(_) => {}
        }
    }
    page.into()
}

pub struct View<'a> {
    pub entries: &'a [voice_recordings::Entry],
    pub result: Option<&'a str>,
    pub problem: Option<&'a str>,
    pub replaying: bool,
    pub busy: bool,
}

#[derive(Clone)]
pub enum Event {
    Refresh,
    Clear,
    Play(String),
    Retry(String),
    Delete(String),
    Stop,
    Apply,
    Dismiss,
}

#[cfg(test)]
#[path = "voice_recordings_tests.rs"]
mod tests;
