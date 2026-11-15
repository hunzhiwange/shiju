use crate::Event as Message;
use crate::Event;
use crate::catalog::{icon_svg, section_card_style};
use crate::types::SkillsDirectoryScope;
use config_ui::DesktopLocale;
use desktop_assets::assets::Icon;
use desktop_i18n::settings_extensions::text as tr;
use desktop_ui_controls::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_panel_style, settings_segment_button_style,
    settings_success_banner,
};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length};

#[derive(Clone, Copy)]
pub enum DetailActionStyle {
    Primary,
    Secondary,
    Danger,
}

pub fn header_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    settings_panel_style(theme)
}

pub fn catalog_panel_style(theme: &iced::Theme) -> iced::widget::container::Style {
    section_card_style(theme)
}

pub fn status_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
    is_error: bool,
) -> Element<'a, Message> {
    if is_error { settings_error_banner(message) } else { settings_success_banner(message) }
}

pub fn scope_button(
    label: &'static str,
    scope: SkillsDirectoryScope,
    active_scope: SkillsDirectoryScope,
    enabled: bool,
) -> Element<'static, Message> {
    let is_active = scope == active_scope;
    let mut button = button(text(label).size(12)).padding([8, 12]);

    if enabled {
        button = button.on_press(Event::SkillsDirectoryScopeChanged(scope));
    }

    button
        .style(move |theme: &iced::Theme, status| {
            let mut style = settings_segment_button_style(theme, status, is_active);
            style.border.width = 0.0;
            style.text_color = theme.palette().text.scale_alpha(if enabled { 1.0 } else { 0.5 });
            style
        })
        .into()
}

pub fn refresh_button(locale: DesktopLocale, loading: bool) -> Element<'static, Message> {
    detail_action_button(
        locale,
        if loading { tr(locale, "刷新中") } else { tr(locale, "刷新") },
        Icon::ArrowRepeat,
        DetailActionStyle::Secondary,
        (!loading).then_some(Event::SkillsRefresh),
    )
}

pub fn loading_banner<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(12))
        .padding([10, 12])
        .width(Length::Fill)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                text_color: Some(theme.palette().text.scale_alpha(0.72)),
                background: Some(Background::Color(
                    palette.background.weak.color.scale_alpha(0.18),
                )),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.45),
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub fn empty_state<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    container(
        column![
            container(icon_svg(Icon::Search, 18.0)).padding([12, 12]).style(
                |theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        text_color: Some(theme.palette().text.scale_alpha(0.7)),
                        background: Some(Background::Color(
                            palette.background.weak.color.scale_alpha(0.22),
                        )),
                        border: Border { radius: 14.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                }
            ),
            text(title).size(14).wrapping(iced::widget::text::Wrapping::Glyph),
            text(description).size(12).style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.68)),
            }),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .padding([18, 14])
    .width(Length::Fill)
    .center_x(Length::Fill)
    .style(section_card_style)
    .into()
}

pub fn detail_source_note(locale: DesktopLocale, path: Option<&str>) -> Element<'static, Message> {
    if let Some(path) = path {
        return container(
            text(path.to_string())
                .size(11)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::Glyph)
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.6)),
                }),
        )
        .width(Length::Fill)
        .into();
    }

    container(text(tr(locale, "当前为内置技能文档，未关联本地目录。")).size(11).style(
        |theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.6)),
        },
    ))
    .width(Length::Fill)
    .into()
}

pub fn detail_action_button<M: Clone + 'static>(
    locale: DesktopLocale,
    label: &'static str,
    icon: Icon,
    style: DetailActionStyle,
    on_press: Option<M>,
) -> Element<'static, M> {
    let on_press_enabled = on_press.is_some();
    let icon = icon_svg(icon, 14.0).style(move |theme: &iced::Theme, _| {
        let color = match (on_press_enabled, style) {
            (false, _) => theme.palette().text.scale_alpha(0.45),
            (true, DetailActionStyle::Primary) => theme.extended_palette().primary.base.text,
            (true, DetailActionStyle::Danger) => iced::Color::WHITE,
            (true, DetailActionStyle::Secondary) => theme.palette().text,
        };
        iced::widget::svg::Style { color: Some(color) }
    });
    let content =
        row![icon, text(tr(locale, label)).size(13)].spacing(8).align_y(Alignment::Center);

    let mut btn = button(content).padding([8, 14]);
    if let Some(message) = on_press {
        btn = btn.on_press(message);
    }

    match style {
        DetailActionStyle::Primary => btn.style(primary_action_btn_style).into(),
        DetailActionStyle::Secondary => btn.style(rounded_action_btn_style).into(),
        DetailActionStyle::Danger => btn.style(danger_action_btn_style).into(),
    }
}
