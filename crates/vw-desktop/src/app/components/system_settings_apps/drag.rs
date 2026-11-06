//! Installed-app drag ordering reuses the authenticated user-preferences PATCH.
use super::{
    preferences,
    presentation::{self, Presence},
};
use crate::app::{App, Message, state::AppsCatalogState};
use iced::{Subscription, Task};
use vw_gateway_client::vw_api_types::agent_apps::AppId;

#[derive(Clone, Debug, Default)]
pub(crate) struct DragState {
    pub source: Option<AppId>,
    pub target: Option<AppId>,
}
#[derive(Clone, Debug)]
pub enum DragMessage {
    Start(AppId),
    Hover(AppId),
    Leave(AppId),
    Drop,
    Cancel,
}

pub(crate) fn event(value: DragMessage) -> Message {
    super::message(crate::app::message::settings::AppsCatalogMessage::Drag(value))
}
pub(super) fn can_start(state: &AppsCatalogState, id: AppId) -> bool {
    !state.loading
        && !state.preferences.busy
        && !state.preferences.open
        && !state.lifecycle.active()
        && state.lifecycle.preview.is_none()
        && state.preferences.saved.is_some()
        && state.query.trim().is_empty()
        && preferences::visible_rows(state)
            .iter()
            .any(|app| app.entry.id == id && presentation::presence(app) == Presence::Installed)
}

/// Move only installed, visible slots. Hidden and uninstalled entries keep their positions.
pub(super) fn reordered(
    order: &[String],
    movable: &[String],
    source: &str,
    target: &str,
) -> Option<Vec<String>> {
    if source == target
        || !movable.iter().any(|id| id == source)
        || !movable.iter().any(|id| id == target)
    {
        return None;
    }
    let mut result = order.to_vec();
    for id in movable {
        if !result.contains(id) {
            result.push(id.clone());
        }
    }
    let slots: Vec<_> = result
        .iter()
        .enumerate()
        .filter(|(_, id)| movable.contains(id))
        .map(|(at, _)| at)
        .collect();
    let mut values: Vec<_> = slots.iter().map(|&at| result[at].clone()).collect();
    let from = values.iter().position(|id| id == source)?;
    let to = values.iter().position(|id| id == target)?;
    let item = values.remove(from);
    values.insert(to, item);
    for (at, value) in slots.into_iter().zip(values) {
        result[at] = value;
    }
    Some(result)
}

pub(crate) fn update(app: &mut App, value: DragMessage) -> Task<Message> {
    let state = &mut app.apps_catalog;
    match value {
        DragMessage::Start(id) if can_start(state, id) => {
            state.expanded = None;
            state.drag = DragState { source: Some(id), target: Some(id) };
        }
        DragMessage::Hover(id) => {
            state.hovered = Some(id);
            if state.drag.source.is_some() && can_start(state, id) {
                state.drag.target = Some(id);
            }
        }
        DragMessage::Leave(id) => {
            if state.hovered == Some(id) {
                state.hovered = None;
            }
        }
        DragMessage::Cancel => state.drag = Default::default(),
        DragMessage::Drop => {
            let drag = std::mem::take(&mut state.drag);
            let (Some(source), Some(target)) = (drag.source, drag.target) else {
                return Task::none();
            };
            if !can_start(state, source) || !can_start(state, target) {
                return Task::none();
            }
            let movable: Vec<_> = preferences::visible_rows(state)
                .iter()
                .filter(|a| presentation::presence(a) == Presence::Installed)
                .map(|a| presentation::key(a.entry.id))
                .collect();
            let Some(saved) = &state.preferences.saved else {
                return Task::none();
            };
            if let Some(order) = reordered(
                &saved.user.app_order,
                &movable,
                &presentation::key(source),
                &presentation::key(target),
            ) {
                return preferences::update(app, preferences::PreferencesMessage::Reorder(order));
            }
        }
        _ => {}
    }
    Task::none()
}

/// Observe release even if another widget captures it; cancel outside the originating window.
pub(crate) fn subscription(
    state: &DragState,
    main: Option<iced::window::Id>,
) -> Subscription<Message> {
    if state.source.is_none() {
        return Subscription::none();
    }
    iced::event::listen_with(|event, _, id| {
        use iced::{Event, keyboard, mouse, touch, window};
        let value = match event {
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => DragMessage::Drop,
            Event::Mouse(mouse::Event::CursorLeft)
            | Event::Window(window::Event::Unfocused)
            | Event::Touch(touch::Event::FingerLost { .. }) => DragMessage::Cancel,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => DragMessage::Cancel,
            _ => return None,
        };
        Some((id, value))
    })
    .with(main)
    .map(|(main, (id, value))| event(if main == Some(id) { value } else { DragMessage::Cancel }))
}
