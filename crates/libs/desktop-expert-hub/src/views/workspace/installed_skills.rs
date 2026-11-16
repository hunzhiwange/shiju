//! 桌面只注入领域状态、主题和语言。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{ViewInput, installed_skills};
use iced::Element;
fn input<'a>(app: &WorkspaceInput<'a>) -> ViewInput<'a> {
    ViewInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
    }
}
fn map(message: crate::views::Message) -> Message {
    match message {
        crate::views::Message::SkillHub(value) => Message::SkillHub(value),
        crate::views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        crate::views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        crate::views::Message::None => Message::None,
    }
}
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    {
        let i18n = desktop_i18n::I18n::new(app.desktop_locale);
        let title = i18n.__("我安装的技能");
        let theme = app.app_theme;
        let header = installed_skills::header(
            i18n,
            move || {
                desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
                    iced::widget::text(title).size(25).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
                    desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
                        title,
                        i18n.__("管理当前目录范围内可启用、关闭或卸载的本地技能"),
                        Some(theme.palette().text),
                        i18n,
                    ),
                )
            },
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Public(
                crate::message::skill_hub::PublicSkillMessage::CloseInstalled,
            )),
        );
        installed_skills::view(&input(app), header, map)
    }
}
pub(super) fn with_delete_confirmation<'a>(
    app: &WorkspaceInput<'a>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match installed_skills::delete_confirmation(&input(app)) {
        Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(base),
            map(close),
            card.map(map),
        ),
        None => base,
    }
}
