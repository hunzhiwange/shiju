//! Current gateway archived metadata, with one list scroll and anchored menus.

use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_divider, settings_error_banner,
    settings_modal_card_style, settings_muted_text_style, settings_panel_style,
    settings_text_input,
};
use crate::app::i18n::{I18n, settings_extensions};
use crate::app::message::settings::{SettingsMessage, archived_chats::ArchivedChatsMessage};
use crate::app::state::{ArchivedChatsMenu, ArchivedChatsSort, ArchivedChatsState};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Length};
use vw_config_types::ui::DesktopLocale;

mod delete_modal;

fn event(message: ArchivedChatsMessage) -> Message {
    Message::Settings(SettingsMessage::ArchivedChats(message))
}

fn icon(icon: Icon, size: f32) -> iced::widget::svg::Svg<'static> {
    icon_svg(icon, size).style(|theme: &iced::Theme, _| iced::widget::svg::Style {
        color: Some(theme.palette().text.scale_alpha(0.8)),
    })
}

fn action(label: &str, message: ArchivedChatsMessage) -> Element<'_, Message> {
    optional_action(label, Some(message))
}

fn optional_action(label: &str, message: Option<ArchivedChatsMessage>) -> Element<'_, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(message.map(event))
        .into()
}

fn choice<'a>(
    label: impl Into<String>,
    selected: bool,
    message: Option<ArchivedChatsMessage>,
) -> Element<'a, Message> {
    let mut content = row![text(label.into()).size(13).wrapping(text::Wrapping::None)]
        .spacing(8)
        .align_y(Alignment::Center)
        .push(Space::new().width(Length::Fill));
    if selected {
        content = content.push(icon(Icon::CheckSquare, 14.0));
    }
    button(content)
        .width(Length::Fill)
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(message.map(event))
        .into()
}

fn type_menu(state: &ArchivedChatsState, i18n: I18n) -> Element<'_, Message> {
    let mut choices = column![
        text(i18n.__("类型")).size(12).style(settings_muted_text_style),
        choice(i18n.__("全部聊天"), true, Some(ArchivedChatsMessage::MenuClosed)),
        choice(i18n.__("本地"), false, None),
        choice(i18n.__("云端"), false, None),
        text(i18n.__("当前服务没有来源分类，暂不支持本地或云端筛选。"))
            .size(12)
            .style(settings_muted_text_style),
        settings_divider(),
        text(i18n.__("排序方式")).size(12).style(settings_muted_text_style),
    ]
    .spacing(4);
    for (sort, label) in [
        (ArchivedChatsSort::Updated, "更新时间"),
        (ArchivedChatsSort::Created, "创建时间"),
        (ArchivedChatsSort::Title, "按字母顺序"),
    ] {
        choices = choices.push(choice(
            i18n.__(label),
            state.sort == sort,
            Some(ArchivedChatsMessage::SortSelected(sort)),
        ));
    }
    container(choices).padding(12).width(240).style(settings_modal_card_style).into()
}

fn project_menu(state: &ArchivedChatsState, i18n: I18n) -> Element<'_, Message> {
    let mut choices = column![choice(
        i18n.__("所有项目"),
        state.selected_project_id.is_none(),
        Some(ArchivedChatsMessage::ProjectSelected(None)),
    )]
    .spacing(4);
    for project in state.project_options() {
        choices = choices.push(choice(
            project.name,
            state.selected_project_id.as_deref() == Some(project.id.as_str()),
            Some(ArchivedChatsMessage::ProjectSelected(Some(project.id))),
        ));
    }
    container(
        scrollable(container(choices).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Shrink),
    )
    .max_height(280)
    .padding([12, 5])
    .width(240)
    .style(settings_modal_card_style)
    .into()
}

fn menus(state: &ArchivedChatsState, i18n: I18n) -> Element<'_, Message> {
    let type_button = button(
        row![
            icon(Icon::Sliders, 14.0),
            text(i18n.__("全部聊天")).size(13),
            icon(Icon::ChevronDown, 12.0)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .style(rounded_action_btn_style)
    .on_press(event(ArchivedChatsMessage::TypeMenuToggled));
    let project_label = state
        .project_options()
        .into_iter()
        .find(|project| Some(&project.id) == state.selected_project_id.as_ref())
        .map(|project| project.name)
        .unwrap_or_else(|| i18n.__("所有项目").to_owned());
    let project_button = button(
        row![
            icon(Icon::FolderOpen, 14.0),
            container(text(project_label).size(13).wrapping(text::Wrapping::None))
                .width(Length::Fill)
                .clip(true),
            icon(Icon::ChevronDown, 12.0),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8, 10])
    .style(rounded_action_btn_style)
    .on_press(event(ArchivedChatsMessage::ProjectMenuToggled));
    row![
        Element::from(
            BelowOverlay::new(type_button, type_menu(state, i18n))
                .show(state.menu == Some(ArchivedChatsMenu::Type))
                .gap(5.0)
                .on_close(event(ArchivedChatsMessage::MenuClosed))
        ),
        container(
            BelowOverlay::new(project_button, project_menu(state, i18n))
                .show(state.menu == Some(ArchivedChatsMenu::Project))
                .gap(5.0)
                .on_close(event(ArchivedChatsMessage::MenuClosed))
        )
        .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    page(&app.archived_chats, app.desktop_locale)
}

pub(crate) fn page(state: &ArchivedChatsState, locale: DesktopLocale) -> Element<'_, Message> {
    let i18n = I18n::new(locale);
    let heading = row![
        text(i18n.__("已归档的聊天")).size(24).width(Length::Fill),
        optional_action(
            i18n.__("刷新"),
            state.operating_ids.is_empty().then_some(ArchivedChatsMessage::Refresh),
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let toolbar = responsive(move |size| {
        let search = settings_text_input(i18n.__("搜索已归档的聊天"), &state.search)
            .on_input(|search| event(ArchivedChatsMessage::SearchChanged(search)))
            .width(Length::Fill);
        if size.width >= 700.0 {
            row![search, container(menus(state, i18n)).width(330)]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        } else {
            column![search, menus(state, i18n)].spacing(8).into()
        }
    })
    .height(Length::Shrink);
    let mut list = column![].spacing(18).width(Length::Fill);
    let status = if state.loading {
        Some(i18n.__("正在加载已归档的聊天…"))
    } else if state.error.is_some() {
        None
    } else if state.project_options().is_empty() {
        Some(i18n.__("没有已归档的聊天"))
    } else if state.groups().is_empty() {
        Some(i18n.__("没有匹配的已归档聊天"))
    } else {
        None
    };
    if let Some(status) = status {
        list = list.push(
            container(text(status).size(14).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding([32, 0]),
        );
    } else if state.error.is_some() {
        // Raw transport errors may contain addresses; keep user-facing feedback localized.
        list = list
            .push(settings_error_banner(i18n.__("无法加载已归档的聊天，请重试。")))
            .push(action(i18n.__("重试"), ArchivedChatsMessage::Refresh));
    } else {
        for group in state.groups() {
            let header = row![
                icon(Icon::FolderOpen, 16.0),
                container(text(group.project.name).size(16).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .clip(true),
                text(settings_extensions::archived_chat_count(locale, group.rows.len()))
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            let mut rows = column![].spacing(10);
            for (index, session) in group.rows.into_iter().enumerate() {
                if index != 0 {
                    rows = rows.push(settings_divider());
                }
                let title = if session.title.trim().is_empty() {
                    i18n.__("未命名聊天")
                } else {
                    &session.title
                };
                let title_view = container(text(title).size(14).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .clip(true);
                let operating = state.operating_ids.contains(&session.id);
                rows = rows.push(
                    row![
                        column![
                            tooltip(
                                title_view,
                                container(text(title).size(13))
                                    .padding(8)
                                    .max_width(420)
                                    .style(settings_modal_card_style),
                                tooltip::Position::Top
                            ),
                            text(settings_extensions::archived_chat_updated(
                                locale,
                                session.time.updated
                            ))
                            .size(12)
                            .style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        button(icon(Icon::Trash, 14.0))
                            .padding([8, 10])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!operating && state.delete_confirmation.is_none()).then(|| event(
                                    ArchivedChatsMessage::DeleteRequested(session.id.clone())
                                ))
                            ),
                        optional_action(
                            i18n.__(if operating { "正在恢复会话" } else { "取消归档" }),
                            (!operating && state.delete_confirmation.is_none())
                                .then(|| ArchivedChatsMessage::Restore(session.id.clone())),
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            list = list.push(
                column![
                    header,
                    container(rows).padding(16).width(Length::Fill).style(settings_panel_style),
                ]
                .spacing(10),
            );
        }
    }
    let base = column![
        heading,
        toolbar,
        scrollable(container(list).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    ]
    .spacing(20)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
    delete_modal::overlay(base, state, i18n)
}
