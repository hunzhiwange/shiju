//! Dictation controls use the same theme-aware button style as the composer toolbar.
use super::surface_styles::{
    BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, round_icon_button_style,
    square_icon_button_style, tooltip_dark_style,
};
use super::composer_icons::ComposerIcon;
use desktop_i18n::I18n;
use desktop_voice_ui::message::voice_draft::Phase;
#[derive(Debug, Clone)]
pub enum Event { Start, Finish, Cancel, Apply }
#[derive(Clone, Copy)]
pub struct Input { pub phase: Phase, pub pending: bool, pub ready: bool }

use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Element, Length};

pub fn action(phase: Phase, pending: bool) -> (Event, &'static str) {
    if pending {
        return (Event::Apply, "应用听写");
    }
    match phase {
        Phase::Recording => (Event::Finish, "停止听写"),
        Phase::Transcribing => (Event::Cancel, "正在转写，点击取消"),
        Phase::Idle | Phase::Error => (Event::Start, "听写"),
    }
}
pub fn view(i18n: I18n, input: Input) -> Element<'static, Event> {
    let phase = input.phase;
    if matches!(phase, Phase::Recording | Phase::Transcribing) {
        return iced::widget::Space::new().into();
    }
    let pending = input.pending;
    let (event, label) = if phase == Phase::Recording && !input.ready {
        (Event::Cancel, "正在准备麦克风，点击取消")
    } else {
        action(phase, pending)
    };
    let icon = ComposerIcon::Microphone.svg(BOTTOM_BAR_ICON_SIZE);
    let content: Element<'static, Event> =
        if pending { text(i18n.__(label)).size(12).into() } else { icon.into() };
    let control = button(container(content).center_x(Length::Fill).center_y(Length::Fill))
        .height(BOTTOM_BAR_ICON_BUTTON_SIZE)
        .width(if pending { Length::Shrink } else { Length::Fixed(BOTTOM_BAR_ICON_BUTTON_SIZE) })
        .padding([4, 6])
        .style(|theme, status| round_icon_button_style(theme, status, true))
        .on_press(event);
    let tip = container(text(dictation_tooltip(i18n, label)).size(12))
        .style(tooltip_dark_style)
        .padding([6, 8]);
    let mut controls = row![tooltip(control, tip, tooltip::Position::Top)]
        .spacing(3)
        .align_y(Alignment::Center);
    if phase == Phase::Recording || pending {
        controls = controls.push(
            button(text(i18n.__("取消")).size(12))
                .padding([4, 6])
                .style(|theme, status| square_icon_button_style(theme, status, true))
                .on_press(Event::Cancel),
        );
    }
    controls.into()
}

pub fn dictation_tooltip(i18n: I18n, label: &str) -> String {
    i18n.__(label).to_owned()
}

/// Recording replaces the editor; all completion controls share the existing Finish event.
pub fn recording_bar(i18n: I18n, input: Input, waveform: Element<'static, Event>) -> Element<'static, Event> {
    fn control(
        symbol: &'static str,
        label: &'static str,
        event: Event,
    ) -> Element<'static, Event> {
        tooltip(
            button(container(text(symbol).size(18)).center_x(Length::Fill).center_y(Length::Fill))
                .width(32)
                .height(32)
                .padding(4)
                .style(|theme, status| square_icon_button_style(theme, status, true))
                .on_press(event),
            container(text(label).size(12))
                .style(tooltip_dark_style)
                .padding([6, 8]),
            tooltip::Position::Top,
        )
        .into()
    }
    let (status, cancel) = recording_labels(i18n, input);
    let mut bar = row![control("×", cancel, Event::Cancel)]
        .spacing(6).align_y(Alignment::Center);
    if let Some(status) = status {
        bar = bar.push(text(status).width(Length::Fill));
    } else {
        bar = bar.push(waveform)
            .push(control("■", i18n.__("停止听写"), Event::Finish))
            .push(control("↑", i18n.__("完成听写"), Event::Finish));
    }
    container(bar)
        .padding(8)
        .width(Length::Fill)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border { radius: 18.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

/// Resolve semantic recording state on every draw so an active recording follows locale changes.
pub fn recording_labels(i18n: I18n, input: Input) -> (Option<&'static str>, &'static str) {
    let status = if input.phase == Phase::Transcribing {
        Some(i18n.__("正在转写…"))
    } else if !input.ready {
        Some(i18n.__("正在准备麦克风…"))
    } else { None };
    (status, i18n.__("取消听写"))
}
