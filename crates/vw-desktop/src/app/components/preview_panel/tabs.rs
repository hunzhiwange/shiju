use crate::app::{Message, message};
use iced::widget::{Space, button, column, container, text};
use iced::{Background, Color, Element, Length, Theme, Vector};
use iced_code_editor::i18n::Language;

pub(super) fn preview_tab_menu_messages(path: &str) -> [Message; 4] {
    [
        Message::Preview(message::PreviewMessage::Close(path.to_string())),
        Message::Preview(message::PreviewMessage::TabMenuCloseLeft(path.to_string())),
        Message::Preview(message::PreviewMessage::TabMenuCloseRight(path.to_string())),
        Message::Preview(message::PreviewMessage::TabMenuCloseAll),
    ]
}

pub(super) fn preview_tab_menu_button_style(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => p.background.weak.color,
        button::Status::Pressed => p.background.strong.color,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        ..Default::default()
    }
}

pub(super) fn preview_tab_menu_is_dark(theme: &Theme) -> bool {
    let bg = theme.palette().background;
    bg.r + bg.g + bg.b < 1.5
}

pub(super) fn preview_tab_menu_separator_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let separator_color = if preview_tab_menu_is_dark(theme) {
        let text = theme.palette().text;
        Color::from_rgba(text.r, text.g, text.b, 0.15)
    } else {
        p.background.strong.color
    };

    container::Style { background: Some(separator_color.into()), ..Default::default() }
}

pub(super) fn preview_tab_menu_container_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let is_dark = preview_tab_menu_is_dark(theme);

    container::Style {
        background: Some(
            if is_dark { p.background.weak.color } else { p.background.base.color }.into(),
        ),
        border: iced::Border {
            color: if is_dark {
                p.background.weak.color.scale_alpha(0.65)
            } else {
                p.background.strong.color
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.30 } else { 0.10 }),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// 构建预览标签页右键菜单。
pub fn build_preview_tab_menu<'a>(path: &str) -> Element<'a, Message> {
    let [close, close_left, close_right, close_all] = preview_tab_menu_messages(path);

    let menu_btn = |label: String, msg: Message| -> Element<'a, Message> {
        button(container(text(label).size(13)).width(Length::Fill).padding([2, 8]))
            .on_press(msg)
            .style(preview_tab_menu_button_style)
            .width(Length::Fill)
            .into()
    };

    let separator = || -> Element<'a, Message> {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(preview_tab_menu_separator_style)
            .into()
    };

    let content = column![
        menu_btn("关闭".to_string(), close),
        separator(),
        menu_btn("关闭左侧".to_string(), close_left),
        menu_btn("关闭右侧".to_string(), close_right),
        separator(),
        menu_btn("关闭所有".to_string(), close_all),
    ]
    .width(Length::Fixed(140.0))
    .padding(2)
    .spacing(1);

    container(content).style(preview_tab_menu_container_style).into()
}

/// 编辑器界面语言包装类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DisplayLanguage(pub(super) Language);

/// 返回可选界面语言列表。
pub(super) fn all_languages() -> &'static [DisplayLanguage] {
    const LANGUAGES: [DisplayLanguage; 8] = [
        DisplayLanguage(Language::English),
        DisplayLanguage(Language::ChineseSimplified),
        DisplayLanguage(Language::French),
        DisplayLanguage(Language::Spanish),
        DisplayLanguage(Language::German),
        DisplayLanguage(Language::Italian),
        DisplayLanguage(Language::PortugueseBR),
        DisplayLanguage(Language::PortuguesePT),
    ];
    &LANGUAGES
}

impl std::fmt::Display for DisplayLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Language::English => write!(f, "English"),
            Language::ChineseSimplified => write!(f, "简体中文"),
            Language::French => write!(f, "Français"),
            Language::Spanish => write!(f, "Español"),
            Language::German => write!(f, "Deutsch"),
            Language::Italian => write!(f, "Italiano"),
            Language::PortugueseBR => write!(f, "Português (BR)"),
            Language::PortuguesePT => write!(f, "Português (PT)"),
        }
    }
}
