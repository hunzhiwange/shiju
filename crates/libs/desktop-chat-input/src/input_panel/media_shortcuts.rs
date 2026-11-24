//! Media composer shortcuts and their input requirements.

use std::path::Path;

use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style,
};
use iced::widget::{button, row, text};
use iced::{Alignment, Element, Theme};

pub use desktop_session::session::agent::MediaMode;

const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];
const VIDEOS: &[&str] = &["mp4", "mov", "webm", "m4v", "mpeg", "mpg"];
const AUDIO: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentRule {
    None,
    Optional,
    Required,
}

pub trait MediaModeInput {
    fn placeholder(self) -> &'static str;
    fn attachment_rule(self) -> AttachmentRule;
    fn attachment_extensions(self) -> impl Iterator<Item = &'static str>;
    fn accepts(self, path: &Path) -> bool;
}

impl MediaModeInput for MediaMode {
    fn placeholder(self) -> &'static str {
        match self {
            Self::Image => "描述想生成的画面，或添加图片说明如何修改…",
            Self::Video => "描述想生成的视频，也可添加一张首帧参考图…",
            Self::Speech => "输入需要合成语音的文本…",
            Self::Vision => "添加图片或视频，并说明想了解什么…",
            Self::Transcription => "添加录音或视频，并说明如何转写…",
        }
    }

    fn attachment_rule(self) -> AttachmentRule {
        match self {
            Self::Image | Self::Video => AttachmentRule::Optional,
            Self::Speech => AttachmentRule::None,
            Self::Vision | Self::Transcription => AttachmentRule::Required,
        }
    }

    fn attachment_extensions(self) -> impl Iterator<Item = &'static str> {
        let (primary, secondary) = match self {
            Self::Image | Self::Video => (IMAGES, &[][..]),
            Self::Speech => (&[][..], &[][..]),
            Self::Vision => (IMAGES, VIDEOS),
            Self::Transcription => (AUDIO, VIDEOS),
        };
        primary.iter().copied().chain(secondary.iter().copied())
    }

    fn accepts(self, path: &Path) -> bool {
        path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            self.attachment_extensions().any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    AppStudio,
    VideoEditor,
    Media(MediaMode),
}

impl Shortcut {
    pub const ALL: [Self; 7] = [
        Self::AppStudio,
        Self::VideoEditor,
        Self::Media(MediaMode::Image),
        Self::Media(MediaMode::Video),
        Self::Media(MediaMode::Speech),
        Self::Media(MediaMode::Vision),
        Self::Media(MediaMode::Transcription),
    ];

    fn label_and_icon(self) -> (&'static str, Icon) {
        match self {
            Self::AppStudio => ("应用 Studio", Icon::Grid1x2),
            Self::VideoEditor => ("识矩剪辑", Icon::Scissors),
            Self::Media(MediaMode::Image) => ("生图", Icon::Image),
            Self::Media(MediaMode::Video) => ("生视频", Icon::ArrowRight),
            Self::Media(MediaMode::Speech) => ("生语音", Icon::ChatTextFill),
            Self::Media(MediaMode::Vision) => ("识图/视频", Icon::Eye),
            Self::Media(MediaMode::Transcription) => ("识语音", Icon::FileText),
        }
    }
}

pub fn view<Message: Clone + 'static>(
    i18n: I18n,
    active: Option<MediaMode>,
    on_shortcut: impl Fn(Shortcut) -> Message,
    on_close: Message,
) -> Element<'static, Message> {
    let mut shortcuts = row![].spacing(6).align_y(Alignment::Center);
    for shortcut in Shortcut::ALL {
        let (label, icon) = shortcut.label_and_icon();
        let selected = active.is_some_and(|mode| shortcut == Shortcut::Media(mode));
        shortcuts = shortcuts.push(chip(i18n.__(label), icon, selected, on_shortcut(shortcut)));
    }
    if active.is_some() {
        shortcuts = shortcuts.push(chip(i18n.__("退出媒体模式"), Icon::X, false, on_close));
    }
    shortcuts.wrap().into()
}

fn chip<Message: Clone + 'static>(
    label: &str,
    icon: Icon,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let icon = super::icons::icon_svg(icon, 13.0).style(move |theme: &Theme, _| {
        iced::widget::svg::Style {
            color: Some(if selected {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            }),
        }
    });
    button(row![icon, text(label.to_owned()).size(12)].spacing(5).align_y(Alignment::Center))
        .padding([4, 8])
        .style(move |theme, status| {
            let mut style = if selected {
                primary_action_btn_style(theme, status)
            } else {
                rounded_action_btn_style(theme, status)
            };
            style.border.radius = 8.0.into();
            style
        })
        .on_press(message)
        .into()
}

#[cfg(test)]
#[path = "media_shortcuts_tests.rs"]
mod media_shortcuts_tests;
