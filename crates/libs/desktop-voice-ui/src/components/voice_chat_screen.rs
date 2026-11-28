use super::{CONTROL_SIZE, Event, ScreenEvent, voice_icon, voice_tooltip};
use desktop_i18n::I18n;
use desktop_ui_controls::{
    overlays::SideOverlay,
    system_settings_common::{
        primary_action_btn_style, round_icon_btn_style, rounded_action_btn_style,
        settings_modal_card, settings_muted_text_style, settings_panel_style, settings_toggle,
    },
};
use desktop_voice::screen_context::{State, Target};
use iced::{
    Alignment, Background, Element, Length,
    widget::{button, column, container, mouse_area, row, svg, text, tooltip},
};

const SETTINGS_ICON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../assets/icons/feather/settings.svg"
));

pub(super) fn settings_control<Message: Clone + 'static>(
    i18n: I18n,
    map: fn(Event) -> Message,
    screen: &State,
    active: bool,
) -> Element<'static, Message> {
    let enabled = screen.enabled;
    let gear = voice_icon(SETTINGS_ICON, 17.0).style(move |theme: &iced::Theme, _| svg::Style {
        color: Some(if enabled {
            theme.extended_palette().primary.base.color
        } else {
            theme.palette().text
        }),
    });
    let trigger =
        mouse_area(container(gear).center(CONTROL_SIZE).id("voice-chat-settings").style(|theme| {
            let button = round_icon_btn_style(theme, button::Status::Active);
            container::Style {
                background: button.background,
                border: button.border,
                ..Default::default()
            }
        }))
        .interaction(iced::mouse::Interaction::Pointer);
    let trigger = voice_tooltip(trigger, i18n.__("通话设置"), tooltip::Position::Bottom);
    let toggle = settings_toggle(enabled)
        .label(i18n.__("允许单帧屏幕分享"))
        .text_size(13)
        .width(Length::Fill)
        .on_toggle(move |enabled| map(Event::Screen(ScreenEvent::Enable(enabled))));
    let mut options = column![
        voice_tooltip(
            toggle,
            i18n.__("只分享确认窗口的一帧，最大 1024 像素并按大小缩小；不持续采集，采集端不另存截图。已发送内容可能被网关和模型保留。"),
            tooltip::Position::Top,
        ),
        screen_action(i18n, "屏幕录制授权")
            .on_press(map(Event::Screen(ScreenEvent::Permission))),
    ]
    .spacing(6);
    if enabled {
        options = options
            .push(
                screen_action(i18n, "选择前台窗口")
                    .on_press_maybe(active.then(|| map(Event::Screen(ScreenEvent::Offer)))),
            )
            .push(
                screen_action(i18n, "改用当前聊天附件分享")
                    .on_press(map(Event::Screen(ScreenEvent::AttachmentChat))),
            );
    }
    let panel = container(options).width(228).padding(10).style(|theme| {
        let mut style = settings_panel_style(theme);
        if let Some(Background::Color(color)) = &mut style.background {
            color.a = 1.0;
        }
        style.border.radius = 10.0.into();
        style
    });
    let panel = mouse_area(panel).interaction(iced::mouse::Interaction::Idle);
    SideOverlay::new(trigger, panel).hover_and_click().gap(6.0).into()
}

pub(super) fn activity<Message: Clone + 'static>(
    i18n: I18n,
    map: fn(Event) -> Message,
    screen: &State,
) -> Option<Element<'static, Message>> {
    if let Some(target) = &screen.offered {
        let label = target_label(i18n, target);
        let confirmation = i18n
            .format("即将分享：%s。请确认窗口内容。", &[label.as_str().into()])
            .expect("valid share target");
        return Some(
            settings_modal_card(
                column![
                    text(confirmation).size(13).wrapping(text::Wrapping::WordOrGlyph),
                    row![
                        screen_action(i18n, "取消分享")
                            .on_press(map(Event::Screen(ScreenEvent::Cancel))),
                        screen_action(i18n, "确认采集并分享这一帧")
                            .style(primary_action_btn_style)
                            .on_press(map(Event::Screen(ScreenEvent::Confirm))),
                    ]
                    .spacing(8)
                    .wrap(),
                ]
                .spacing(8),
            )
            .max_width(460)
            .into(),
        );
    }

    let label = if screen.selecting {
        "正在选择：三秒内切换到目标应用，随后返回确认。"
    } else if screen.busy() {
        "正在采集已确认窗口的单帧；可以取消。"
    } else {
        return None;
    };
    Some(
        row![
            text(i18n.__(label)).size(12).style(settings_muted_text_style),
            screen_action(i18n, "取消分享").on_press(map(Event::Screen(ScreenEvent::Cancel))),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap()
        .into(),
    )
}

fn screen_action<Message: Clone + 'static>(
    i18n: I18n,
    label: &'static str,
) -> button::Button<'static, Message> {
    button(text(i18n.__(label)).size(13)).padding([6, 10]).style(rounded_action_btn_style)
}

fn target_label(i18n: I18n, target: &Target) -> String {
    let Some(application) = &target.application else {
        return target.label.clone();
    };
    i18n.format(
        "%s · %s · 窗口 %u",
        &[
            application.as_str().into(),
            target.title.as_deref().unwrap_or(i18n.__("无标题")).into(),
            target.window.into(),
        ],
    )
    .expect("valid window label")
}
