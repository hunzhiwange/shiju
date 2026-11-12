//! Current gateway archived metadata, with shared selects and one list scroll.

use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, icon_svg, rounded_action_btn_style, settings_divider,
    settings_error_banner, settings_modal_card_style, settings_muted_text_style,
    settings_panel_style, settings_select, settings_text_input,
};
use crate::app::i18n::{I18n, settings_extensions};
use crate::app::message::settings::{SettingsMessage, archived_chats::ArchivedChatsMessage};
use crate::app::state::{ArchivedChatsSort, ArchivedChatsState};
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

#[derive(Clone, PartialEq)]
struct FilterOption<T> {
    value: T,
    label: String,
}

impl<T> std::fmt::Display for FilterOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn menus(state: &ArchivedChatsState, i18n: I18n) -> Element<'_, Message> {
    let sorts: Vec<_> = [
        (ArchivedChatsSort::Updated, "更新时间"),
        (ArchivedChatsSort::Created, "创建时间"),
        (ArchivedChatsSort::Title, "按字母顺序"),
    ]
    .into_iter()
    .map(|(value, label)| FilterOption { value, label: i18n.__(label).to_owned() })
    .collect();
    let selected_sort = sorts.iter().find(|option| option.value == state.sort).cloned();
    let mut projects =
        vec![FilterOption { value: None, label: i18n.__("所有项目").to_owned() }];
    projects.extend(
        state
            .project_options()
            .into_iter()
            .map(|project| FilterOption { value: Some(project.id), label: project.name }),
    );
    let selected_project =
        projects.iter().find(|option| option.value == state.selected_project_id).cloned();
    row![
        settings_select(sorts, selected_sort, |option| {
            event(ArchivedChatsMessage::SortSelected(option.value))
        })
        .width(Length::FillPortion(1)),
        settings_select(projects, selected_project, |option| {
            event(ArchivedChatsMessage::ProjectSelected(option.value))
        })
        .width(Length::FillPortion(1)),
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
            let project_id = group.project.id.clone();
            let more = button(text("…").size(14).center())
                .width(26)
                .height(26)
                .padding([2, 6])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (state.operating_ids.is_empty() && state.delete_confirmation.is_none()).then(
                        || event(ArchivedChatsMessage::ProjectActionsToggled(project_id.clone())),
                    ),
                );
            let delete = button(text(i18n.__("删除项目中的全部已归档聊天")).size(13))
                .padding([8, 10])
                .style(danger_action_btn_style)
                .on_press(event(ArchivedChatsMessage::ProjectDeleteRequested(project_id.clone())));
            let more: Element<'_, Message> = BelowOverlay::new(
                more,
                container(delete).padding(8).style(settings_modal_card_style),
            )
            .show(state.project_action_menu.as_ref() == Some(&project_id))
            .gap(5.0)
            .on_close(event(ArchivedChatsMessage::ProjectActionsToggled(project_id)))
            .into();
            let header = row![
                icon(Icon::FolderOpen, 16.0),
                container(text(group.project.name).size(16).wrapping(text::Wrapping::None))
                    .width(Length::Fill)
                    .clip(true),
                more,
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
                        button(icon(Icon::Trash, 12.0))
                            .width(26)
                            .height(26)
                            .padding(7)
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!operating && state.delete_confirmation.is_none()).then(|| event(
                                    ArchivedChatsMessage::DeleteRequested(session.id.clone())
                                ))
                            ),
                        button(
                            text(i18n.__(if operating {
                                "正在恢复会话"
                            } else {
                                "取消归档"
                            }))
                            .size(12),
                        )
                        .height(26)
                        .padding([4, 8])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            (!operating && state.delete_confirmation.is_none()).then(|| {
                                event(ArchivedChatsMessage::Restore(session.id.clone()))
                            }),
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
    if state.project_delete.as_ref().is_some_and(|batch| !batch.failed.is_empty())
        && state.delete_confirmation.is_none()
    {
        list = list.push(optional_action(
            i18n.__("重试失败项"),
            (!state.loading && state.operating_ids.is_empty())
                .then_some(ArchivedChatsMessage::ProjectDeleteRetry),
        ));
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
