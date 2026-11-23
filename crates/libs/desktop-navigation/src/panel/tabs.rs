//! Main-tab selection and preview restoration.
use crate::components::top_bar::types::AppTab;
use crate::ui_types::Screen;
use desktop_editor::preview::PreviewTab;
use iced::widget::Id;
use iced::{Point, Task};
use std::collections::HashMap;

pub const EDITOR_TAB_ID: &str = "editor";

#[derive(Clone)]
pub enum Message {
    GoHome,
    HomeAppsBarScrollChanged(f32),
    HomeAppsBarPrev,
    HomeAppsBarNext,
    TabSelected(String),
    TabClosed(String),
    TabHovered(Option<String>),
}
pub struct State {
    pub apps_search_query: String,
    pub project_path: Option<String>,
    pub project_preview_tabs: HashMap<String, Vec<PreviewTab>>,
    pub preview_tabs: Vec<PreviewTab>,
    /// Standalone editors parked while the current project owns `preview_tabs`.
    pub standalone_preview_tabs: Vec<PreviewTab>,
    pub(crate) standalone_preview_active_path: Option<String>,
    pub project_preview_active_path: HashMap<String, Option<String>>,
    pub active_preview_path: Option<String>,
    pub screen: Screen,
    pub active_tab_id: Option<String>,
    pub preview_tab_menu_path: Option<String>,
    pub preview_tab_menu_pos: Option<Point>,
    pub home_apps_bar_scroll_x: f32,
    pub home_apps_bar_scroll_id: Id,
    pub open_tabs: Vec<AppTab>,
    pub hovered_tab_id: Option<String>,
}
pub enum Effect {
    GoHome { previous_project_path: Option<String> },
    TabSelected { id: String, previous_project_path: Option<String> },
    TabClosed { previous_project_path: Option<String> },
}
pub struct Update {
    pub task: Task<()>,
    pub effect: Option<Effect>,
}
pub fn update(
    app: &mut State,
    code_projects_menu_open: &mut bool,
    panels: &mut super::settings::State,
    message: Message,
) -> Update {
    let previous_project_path = app.project_path.clone();
    let effect = match &message {
        Message::GoHome => Some(Effect::GoHome { previous_project_path }),
        Message::TabSelected(id) => {
            Some(Effect::TabSelected { id: id.clone(), previous_project_path })
        }
        Message::TabClosed(_) => Some(Effect::TabClosed { previous_project_path }),
        _ => None,
    };
    let task = match message {
        Message::GoHome => {
            panels.close_menu();
            app.switch_preview_project(None);
            app.screen = Screen::Home;
            app.active_tab_id = Some("home".to_string());
            app.remember_standalone_selection();
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            Task::none()
        }
        Message::HomeAppsBarScrollChanged(x) => {
            app.home_apps_bar_scroll_x = x;
            Task::none()
        }
        Message::HomeAppsBarPrev => {
            let x = (app.home_apps_bar_scroll_x - 0.25).max(0.0);
            app.home_apps_bar_scroll_x = x;
            iced::widget::operation::snap_to(
                app.home_apps_bar_scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: Some(x), y: None },
            )
        }
        Message::HomeAppsBarNext => {
            let x = (app.home_apps_bar_scroll_x + 0.25).min(1.0);
            app.home_apps_bar_scroll_x = x;
            iced::widget::operation::snap_to(
                app.home_apps_bar_scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: Some(x), y: None },
            )
        }
        Message::TabSelected(id) => {
            *code_projects_menu_open = false;
            app.select_tab(id);
            Task::none()
        }
        Message::TabClosed(id) => {
            if let Some(index) = app.open_tabs.iter().position(|tab| tab.id == id) {
                app.open_tabs.remove(index);
                if app.active_tab_id.as_ref() == Some(&id) {
                    if let Some(last_tab) = app.open_tabs.last() {
                        app.select_tab(last_tab.id.clone());
                    } else {
                        app.active_tab_id = None;
                        app.screen = Screen::Home;
                    }
                }
            }
            Task::none()
        }
        Message::TabHovered(id) => {
            app.hovered_tab_id = id;
            Task::none()
        }
    };
    Update { task, effect }
}

impl State {
    fn select_tab(&mut self, id: String) {
        if let Some(tab) = self.open_tabs.iter().find(|tab| tab.id == id) {
            let screen = tab.screen;
            let project = tab.project_path.clone();
            if project.is_some() || id == EDITOR_TAB_ID || id == "home" {
                self.switch_preview_project(project);
            }
            self.screen = screen;
            if id == EDITOR_TAB_ID {
                let selected = self.active_preview_path.as_ref()
                    .or(self.standalone_preview_active_path.as_ref());
                self.active_preview_path = selected
                    .and_then(|path| self.preview_tabs.iter().find(|tab| &tab.path == path))
                    .or_else(|| self.preview_tabs.last())
                    .map(|tab| tab.path.clone());
                self.preview_tab_menu_path = None;
                self.preview_tab_menu_pos = None;
            } else if id == "home" {
                self.remember_standalone_selection();
                self.preview_tab_menu_path = None;
                self.preview_tab_menu_pos = None;
            }
        }
        self.active_tab_id = Some(id);
    }

    fn remember_standalone_selection(&mut self) {
        if let Some(path) = self.active_preview_path.take() {
            self.standalone_preview_active_path = Some(path);
        }
    }

    pub fn show_project(&mut self) {
        self.screen = Screen::Project;
    }
}
