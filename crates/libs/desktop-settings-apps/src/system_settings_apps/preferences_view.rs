use super::preferences::{PreferencesMessage};

use desktop_ui_controls::system_settings_common::*;
use super::preferences::PreferencesState;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, container, row, text},
};

const ACTION_PADDING: [u16; 2] = [8, 12];

pub fn overlay<'a, C: Clone + 'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &'a PreferencesState,
    _host_id: &'a str,
    rows: &'a [super::catalog::CatalogRow],
    base: Element<'a, M>,
    heading: impl Fn(&'static str, &'static str, u16) -> Element<'a, M>,
    event: impl Fn(PreferencesMessage<C>) -> M + Copy + 'static,
) -> Element<'a, M> {
    if !state.open {
        return base;
    }
    let close = event(PreferencesMessage::Close);
    let editable = !state.busy;
    let mut body = column![].spacing(14);
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    if state.busy {
        body = body.push(text(i18n.__("正在等待网关确认…")).size(13));
    }
    if let Some(draft) = &state.draft {
        let group_heading = |title, description| {
            heading(title, description, 17)
        };
        let mut visibility =
            column![group_heading(i18n.__("可见应用与顺序"), i18n.__("显示或隐藏应用，并调整它们在目录中的顺序。"),)]
                .spacing(10);
        for (index, id) in draft.user.app_order.iter().enumerate() {
            if let Some(app) = rows.iter().find(|r| {
                serde_json::to_value(r.entry.id)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .as_ref()
                    == Some(id)
            }) {
                let app_id = app.entry.id;
                visibility =
                    visibility.push(
                        row![
                            checkbox(draft.user.visible_apps.contains(&app_id))
                                .label(app.entry.name.clone())
                                .size(16)
                                .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                                .width(Length::Fill)
                                .style(settings_checkbox_style)
                                .on_toggle_maybe(editable.then_some(move |v| event(
                                    PreferencesMessage::Visible(app_id, v)
                                ))),
                            button(text(i18n.__("上移")).size(SETTINGS_CONTROL_TEXT_SIZE))
                                .padding(ACTION_PADDING)
                                .style(rounded_action_btn_style)
                                .on_press_maybe(
                                    (editable && index > 0)
                                        .then_some(event(PreferencesMessage::MoveUp(id.clone())))
                                ),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(8),
                    );
            }
        }
        body = body.push(visibility);
    }
    let header = settings_modal_header(
        heading(i18n.__("应用设置"), i18n.__("设置应用可见性与顺序。"), 28),
        settings_close_button(close.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(ACTION_PADDING)
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(close.clone())),
        button(text(i18n.__("保存可见性/顺序")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(ACTION_PADDING)
            .style(primary_action_btn_style)
            .on_press_maybe(
                (editable && state.draft.is_some()).then_some(event(PreferencesMessage::SaveUser))
            ),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(actions.into()),
        Some("apps-preferences-body"),
    )
    .width(Length::Fill)
    .max_width(560.0)
    .max_height(560.0);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
