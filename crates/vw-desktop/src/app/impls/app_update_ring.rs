//! Status-bar control for the shared desktop updater.

use std::f32::consts::TAU;

use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::message::view::ViewMessage;
use crate::app::{App, Message};
use iced::widget::{button, container, stack, svg, text, tooltip};
use iced::{Element, Length, Theme};
use update_types::UpdateProgress;

const SIZE: f32 = 22.0;

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
        RingState::Available => ("↑", i18n.__("下载安装包").to_owned()),
        RingState::Downloading(Some(percent)) => {
            ("↓", format!("{} {percent}%", i18n.__("下载进度")))
        }
        RingState::Downloading(None) => ("↓", i18n.__("正在下载并校验安装包...").to_owned()),
        RingState::Verifying => ("…", i18n.__("正在校验安装包…").to_owned()),
        RingState::Ready => ("✓", i18n.__("安装更新").to_owned()),
        RingState::Installing => ("…", i18n.__("更新中…").to_owned()),
    };

    // The software renderer supports SVG arcs, including the same clockwise origin as the quota ring.
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
    let icon = stack![arc, container(glyph).center(Length::Fill)].width(SIZE).height(SIZE);
    let trigger: Element<'_, Message> = match state.action() {
        Some(action) => button(icon)
            .padding(2)
            .style(|theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border.radius = ((SIZE + 4.0) / 2.0).into();
                style
            })
            .on_press(Message::View(action))
            .into(),
        None => container(icon).padding(2).into(),
    };
    Some(
        tooltip::Tooltip::new(
            trigger,
            text(format!("{label} · v{version}")).size(13),
            tooltip::Position::Top,
        )
        .gap(5.0)
        .padding(8.0)
        .style(crate::app::components::system_settings_common::settings_panel_style)
        .into(),
    )
}

#[cfg(test)]
#[path = "app_update_ring_tests.rs"]
mod tests;
