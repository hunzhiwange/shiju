//! Session menu and rename transitions, independent of desktop resources.
use session_info::Info;
use std::collections::{HashMap, HashSet};

pub struct State<'a> {
    pub menu: &'a mut Option<String>,
    pub anchor: &'a mut Option<iced::Point>,
    pub actions: &'a mut bool,
    pub project_menu: &'a mut Option<String>,
    pub expanded: &'a mut HashSet<String>,
    pub rename_id: &'a mut Option<String>,
    pub rename_value: &'a mut String,
    pub sessions: &'a mut Vec<Info>,
    pub projects: &'a mut HashMap<String, Vec<Info>>,
}
pub enum Event {
    RightClick(String, f32, f32),
    Close,
    Toggle(String),
    Title(String, Option<u128>),
    Rename(String),
    Change(String),
    Cancel,
    Save,
}
pub struct Rename {
    pub id: String,
    pub title: String,
}

pub fn update(state: &mut State<'_>, event: Event) -> Option<Rename> {
    match event {
        Event::RightClick(id, x, y) => {
            *state.project_menu = None;
            *state.menu = Some(id);
            *state.anchor = Some(iced::Point::new(x, y));
        }
        Event::Close => {
            *state.menu = None;
            *state.anchor = None;
        }
        Event::Toggle(id) => {
            if !state.expanded.remove(&id) {
                state.expanded.insert(id);
            }
        }
        Event::Title(id, elapsed_ms) => {
            let double = elapsed_ms.is_some_and(|elapsed| elapsed < 400);
            if double {
                *state.menu = None;
                *state.anchor = None;
                *state.actions = false;
                *state.rename_id = Some(id.clone());
                if let Some(info) = state.sessions.iter().find(|info| info.id == id) {
                    *state.rename_value = info.title.clone();
                }
            }
        }
        Event::Rename(id) => {
            *state.menu = None;
            *state.anchor = None;
            *state.actions = false;
            *state.rename_id = Some(id.clone());
            if let Some(info) = state.sessions.iter().find(|info| info.id == id).or_else(|| {
                state
                    .projects
                    .values()
                    .find_map(|sessions| sessions.iter().find(|info| info.id == id))
            }) {
                *state.rename_value = info.title.clone();
            }
        }
        Event::Change(value) => *state.rename_value = value,
        Event::Cancel => {
            *state.rename_id = None;
            state.rename_value.clear();
        }
        Event::Save => {
            let id = state.rename_id.clone()?;
            let title = state.rename_value.trim().to_owned();
            if title.is_empty() {
                return None;
            }
            *state.rename_id = None;
            state.rename_value.clear();
            if let Some(info) = state.sessions.iter_mut().find(|info| info.id == id) {
                info.title = title.clone();
            }
            for sessions in state.projects.values_mut() {
                if let Some(info) = sessions.iter_mut().find(|info| info.id == id) {
                    info.title = title.clone();
                }
            }
            return Some(Rename { id, title });
        }
    }
    None
}
