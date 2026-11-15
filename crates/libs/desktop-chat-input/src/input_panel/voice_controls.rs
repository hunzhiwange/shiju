//! Dictation controls use the same theme-aware button style as the composer toolbar.
use super::surface_styles::{BOTTOM_BAR_ICON_BUTTON_SIZE, BOTTOM_BAR_ICON_SIZE, square_icon_button_style};
use desktop_voice_ui::message::voice_draft::Phase;
#[derive(Debug, Clone)]
pub enum Event { Start, Finish, Cancel, Apply }
#[derive(Clone, Copy)]
pub struct Input { pub phase: Phase, pub pending: bool, pub ready: bool }

use iced::widget::{button, container, row, svg, text, tooltip};
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
pub fn view(input: Input) -> Element<'static, Event> {
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
    let data = match phase {
        Phase::Recording => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6 6h12v12H6z"/></svg>"#
        }
        Phase::Transcribing => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="2" d="M6 6l12 12M6 18L18 6"/></svg>"#
        }
        _ => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10v2a7 7 0 0014 0v-2M12 19v3M8 22h8"/></g></svg>"#
        }
    };
    let icon = svg(svg::Handle::from_memory(data.as_bytes()))
        .width(BOTTOM_BAR_ICON_SIZE)
        .height(BOTTOM_BAR_ICON_SIZE)
        .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().text) });
    let content: Element<'static, Event> =
        if pending { text(label).size(12).into() } else { icon.into() };
    let control = button(container(content).center_x(Length::Fill).center_y(Length::Fill))
        .height(BOTTOM_BAR_ICON_BUTTON_SIZE)
        .width(if pending { Length::Shrink } else { Length::Fixed(BOTTOM_BAR_ICON_BUTTON_SIZE) })
        .padding([4, 6])
        .style(|theme, status| square_icon_button_style(theme, status, true))
        .on_press(event);
    let mut controls =
        row![tooltip(control, text(dictation_tooltip(label)).width(320), tooltip::Position::Top)]
            .spacing(3)
            .align_y(Alignment::Center);
    if phase == Phase::Recording || pending {
        controls = controls.push(
            button(text("取消").size(12))
                .padding([4, 6])
                .style(|theme, status| square_icon_button_style(theme, status, true))
                .on_press(Event::Cancel),
        );
    }
    controls.into()
}

/// 系统快捷键仅作提示，不触发系统听写或权限请求。
pub fn dictation_tooltip(label: &str) -> String {
    #[cfg(target_os = "macos")]
    if label == "听写" {
        return format!(
            "{label}\n也可使用 macOS 系统听写：将光标放入输入框，按系统设置的听写快捷键（如连按两次 Fn/地球仪键）。可在‘系统设置 → 键盘 → 听写’中开启或修改。\n系统听写可用于支持文本输入的可编辑输入框。"
        );
    }
    label.to_owned()
}

/// Recording replaces the editor; all completion controls share the existing Finish event.
pub fn recording_bar(input: Input, waveform: Element<'static, Event>) -> Element<'static, Event> {
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
            text(label),
            tooltip::Position::Top,
        )
        .into()
    }
    let mut bar =
        row![control("×", "取消听写", Event::Cancel)].spacing(6).align_y(Alignment::Center);
    if input.phase == Phase::Transcribing {
        bar = bar.push(text("正在转写…").width(Length::Fill));
    } else if !input.ready {
        bar = bar.push(text("正在准备麦克风…").width(Length::Fill));
    } else {
        bar = bar
            .push(waveform)
            .push(control("■", "停止听写", Event::Finish))
            .push(control("↑", "完成听写", Event::Finish));
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
