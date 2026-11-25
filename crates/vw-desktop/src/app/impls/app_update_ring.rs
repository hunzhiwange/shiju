//! Status-bar control for the shared desktop updater.

use std::f32::consts::TAU;

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_modal_card,
    settings_optional_modal_overlay, settings_panel_style,
};
use crate::app::message::view::ViewMessage;
use crate::app::{App, Message};
use desktop_ui_controls::system_settings_common::with_settings_modal_escape;
use desktop_update::panel::PanelMessage;
use iced::widget::{button, column, container, row, stack, svg, text, tooltip};
use iced::{Element, Font, Length, Theme};
use update_types::UpdateProgress;

const SIZE: f32 = 22.0;
const BADGE_SIZE: f32 = 18.0;
const DOWNLOAD_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
    <path d="M8 2v8m-3-3 3 3 3-3M2.5 9.5v2a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2v-2"
        fill="none" stroke="black" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#;
const READY_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
    <path d="m3.5 8 3 3 6-6" fill="none" stroke="black" stroke-width="1.6"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#;
const WARNING_ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 56 56">
    <path d="M23.8 7.7a4.8 4.8 0 0 1 8.4 0l21 36.5a4.8 4.8 0 0 1-4.2 7.2H7a4.8 4.8 0 0 1-4.2-7.2Z"
        fill="#ffc52e" stroke="#fff4cf" stroke-width="2.6" stroke-linejoin="round"/>
    <path d="M28 19v13" stroke="#593700" stroke-width="4" stroke-linecap="round"/>
    <circle cx="28" cy="40" r="2.2" fill="#593700"/>
</svg>"##;

#[derive(Debug, PartialEq, Eq)]
enum RingState {
    Available,
    Downloading(Option<u8>),
    Verifying,
    Ready,
    Installing,
}

impl RingState {
    fn downloading(progress: Option<&UpdateProgress>) -> Self {
        let Some((downloaded, total)) = progress.and_then(|progress| {
            progress
                .total_bytes
                .filter(|total| *total > 0)
                .map(|total| (progress.downloaded_bytes, total))
        }) else {
            return Self::Downloading(None);
        };
        if downloaded >= total {
            Self::Verifying
        } else {
            Self::Downloading(Some((u128::from(downloaded) * 100 / u128::from(total)) as u8))
        }
    }

    fn action(&self) -> Option<ViewMessage> {
        match self {
            Self::Available => Some(ViewMessage::RunAppUpdate),
            Self::Ready => Some(ViewMessage::OpenAppInstaller),
            Self::Downloading(_) | Self::Verifying | Self::Installing => None,
        }
    }

    fn percent(&self) -> u8 {
        match self {
            Self::Available => 100,
            Self::Downloading(percent) => percent.unwrap_or(0),
            Self::Verifying | Self::Ready | Self::Installing => 100,
        }
    }
}

fn ring_state(app: &App) -> Option<(&str, RingState)> {
    let update = &app.desktop_update;
    let release = if update.busy || update.ready.is_some() {
        update.release.as_ref()?
    } else {
        update.notification_release()?
    };
    let state = if update.busy {
        if update.download_abort.is_some() {
            RingState::downloading(app.update_panel.app_update_progress.as_ref())
        } else {
            RingState::Installing
        }
    } else if update.ready.is_some() {
        RingState::Ready
    } else if update.checking {
        return None;
    } else {
        RingState::Available
    };
    Some((&release.version, state))
}

pub(super) fn view(app: &App) -> Option<Element<'_, Message>> {
    let (version, state) = ring_state(app)?;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let (glyph, label) = match &state {
        RingState::Available => ("", i18n.__("有可用更新").to_owned()),
        RingState::Downloading(Some(percent)) => {
            ("↓", format!("{} {percent}%", i18n.__("下载进度")))
        }
        RingState::Downloading(None) => ("↓", i18n.__("正在下载并校验安装包...").to_owned()),
        RingState::Verifying => ("…", i18n.__("正在校验安装包…").to_owned()),
        RingState::Ready => ("", i18n.__("安装更新").to_owned()),
        RingState::Installing => ("…", i18n.__("更新中…").to_owned()),
    };

    let trigger: Element<'_, Message> = if let Some(action) = state.action() {
        let icon = if state == RingState::Available { DOWNLOAD_ICON } else { READY_ICON };
        let icon = svg::Svg::new(svg::Handle::from_memory(icon))
            .width(12)
            .height(12)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().background) });
        let badge = container(icon).center(BADGE_SIZE).style(|theme: &Theme| container::Style {
            background: Some(theme.palette().text.into()),
            border: iced::border::rounded(BADGE_SIZE / 2.0),
            ..Default::default()
        });
        button(badge)
            .padding(4)
            .style(|theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border = iced::border::rounded(8);
                if status == button::Status::Active {
                    style.background = Some(theme.palette().text.scale_alpha(0.10).into());
                }
                style
            })
            .on_press(Message::View(action))
            .into()
    } else {
        container(progress_ring(&state, glyph)).padding(2).into()
    };
    Some(
        tooltip::Tooltip::new(
            trigger,
            text(format!("{label} · v{version}")).size(12),
            tooltip::Position::Top,
        )
        .gap(5.0)
        .padding(7.0)
        .style(|theme| {
            let mut style = settings_panel_style(theme);
            style.border.radius = 14.0.into();
            style
        })
        .into(),
    )
}

fn progress_ring(state: &RingState, glyph: &'static str) -> Element<'static, Message> {
    // Match the clockwise origin of the quota ring.
    let circumference = TAU * 9.0;
    let filled = circumference * f32::from(state.percent()) / 100.0;
    let ring = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 22 22">
            <circle cx="11" cy="11" r="9" fill="none" stroke="white" stroke-width="2" opacity=".25"/>
            <circle cx="11" cy="11" r="9" fill="none" stroke="white" stroke-width="2"
                stroke-dasharray="{filled:.4} {circumference:.4}" transform="rotate(-90 11 11)"/>
        </svg>"#
    );
    let arc = svg::Svg::new(svg::Handle::from_memory(ring.into_bytes()))
        .width(SIZE)
        .height(SIZE)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.extended_palette().primary.strong.color),
        });
    let glyph = text(glyph).size(11).style(|theme: &Theme| text::Style {
        color: Some(theme.palette().text),
    });
    stack![arc, container(glyph).center(Length::Fill)].width(SIZE).height(SIZE).into()
}

pub(in crate::app::impls) fn with_install_confirmation<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let show = app.update_panel.app_installer_confirmation_request == Some(app.desktop_update.request)
        && !app.desktop_update.busy
        && app.desktop_update.ready.is_some();
    let modal = show.then(|| {
        let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
        let resolve = |install| {
            Message::View(ViewMessage::AppUpdateAutomation(
                PanelMessage::ResolveAppInstallerConfirmation { install },
            ))
        };
        let cancel = resolve(false);
        let actions = row![
            button(text(i18n.__("取消")).center())
                .width(Length::FillPortion(1))
                .padding([8, 12])
                .style(|theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    style.border.radius = 18.0.into();
                    style
                })
                .on_press(cancel.clone()),
            button(text(i18n.__("更新")).center())
                .width(Length::FillPortion(1))
                .padding([8, 12])
                .style(|theme, status| {
                    let mut style = primary_action_btn_style(theme, status);
                    style.border.radius = 18.0.into();
                    style
                })
                .on_press(resolve(true)),
        ]
        .spacing(10);
        let card = settings_modal_card(
            column![
                svg::Svg::new(svg::Handle::from_memory(WARNING_ICON)).width(52).height(52),
                column![
                    text(i18n.__("现在更新识矩？"))
                        .size(17)
                        .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }),
                    text(i18n.__("识矩将退出以安装更新，当前活动的本地会话会中断。")).size(14),
                ]
                .spacing(8),
                actions,
            ]
            .spacing(17),
        )
        .width(Length::Fill)
        .max_width(320);
        (cancel.clone(), with_settings_modal_escape(card.into(), cancel))
    });
    settings_optional_modal_overlay(base, modal)
}

#[cfg(test)]
#[path = "app_update_ring_tests.rs"]
mod tests;
