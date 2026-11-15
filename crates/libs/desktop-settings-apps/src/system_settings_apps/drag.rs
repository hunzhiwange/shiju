use super::{preferences, presentation::{self, Presence}};
use iced::Subscription;
use vw_api_app_identity::AppId;
#[derive(Clone, Debug, Default)]
pub struct DragState {
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

/// Move only installed, visible slots. Hidden and uninstalled entries keep their positions.
pub fn reordered(
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
pub fn can_start(state: &super::ViewState<'_>, id: AppId) -> bool {
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


pub struct DragUpdate {
    pub drag: DragState, pub expanded: Option<AppId>, pub hovered: Option<AppId>, pub order: Option<Vec<String>>,
}
pub fn update(state: &super::ViewState<'_>, value: DragMessage) -> DragUpdate {
    let mut result = DragUpdate { drag: state.drag.clone(), expanded: state.expanded, hovered: state.hovered, order: None };
    match value {
        DragMessage::Start(id) if can_start(state, id) => {
            result.expanded = None;
            result.drag = DragState { source: Some(id), target: Some(id) };
        }
        DragMessage::Hover(id) => {
            result.hovered = Some(id);
            if result.drag.source.is_some() && can_start(state, id) {
                result.drag.target = Some(id);
            }
        }
        DragMessage::Leave(id) => {
            if result.hovered == Some(id) {
                result.hovered = None;
            }
        }
        DragMessage::Cancel => result.drag = Default::default(),
        DragMessage::Drop => {
            let drag = std::mem::take(&mut result.drag);
            let (Some(source), Some(target)) = (drag.source, drag.target) else {
                return result;
            };
            if !can_start(state, source) || !can_start(state, target) {
                return result;
            }
            let movable: Vec<_> = preferences::visible_rows(state)
                .iter()
                .filter(|a| presentation::presence(a) == Presence::Installed)
                .map(|a| presentation::key(a.entry.id))
                .collect();
            let Some(saved) = &state.preferences.saved else {
                return result;
            };
            if let Some(order) = reordered(
                &saved.user.app_order,
                &movable,
                &presentation::key(source),
                &presentation::key(target),
            ) {
                result.order = Some(order);
            }
        }
        _ => {}
    }
    result
}

/// Observe release even if another widget captures it; cancel outside the originating window.
pub fn subscription(
    state: &DragState,
    main: Option<iced::window::Id>,
) -> Subscription<DragMessage> {
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
    .map(|(main, (id, value))| if main == Some(id) { value } else { DragMessage::Cancel })
}
