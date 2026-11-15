//! Main-tab selection and per-project preview restoration.
use std::collections::HashMap;
use iced::{Task, Point};
use iced::widget::Id;
use crate::ui_types::Screen;
use crate::components::top_bar::types::AppTab;
use desktop_editor::preview::PreviewTab;
#[derive(Clone)]
pub enum Message {
    GoHome, HomeAppsBarScrollChanged(f32), HomeAppsBarPrev, HomeAppsBarNext,
    TabSelected(String), TabClosed(String), TabHovered(Option<String>),
}
pub struct State {
    pub project_path: Option<String>,
    pub project_preview_tabs: HashMap<String, Vec<PreviewTab>>,
    pub preview_tabs: Vec<PreviewTab>,
    pub project_preview_active_path: HashMap<String, Option<String>>,
    pub active_preview_path: Option<String>,
    pub screen: Screen,
    pub active_tab_id: Option<String>,
    pub preview_tab_menu_path: Option<String>,
    pub preview_tab_menu_pos: Option<Point>,
    pub home_apps_bar_scroll_x: f32,
    pub home_apps_bar_scroll_id: Id,
    pub code_projects_menu_open: bool,
    pub open_tabs: Vec<AppTab>,
    pub hovered_tab_id: Option<String>,
}
pub fn update(app: &mut State, message: Message) -> Task<()> {
    match message {
        Message::GoHome => {
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            if let Some(prev_path) = previous_project_path.clone() {
                app.project_preview_tabs
                    .insert(prev_path.clone(), std::mem::take(&mut app.preview_tabs));
                app.project_preview_active_path.insert(prev_path, app.active_preview_path.take());
            }
            app.screen = Screen::Home;
            app.active_tab_id = Some("home".to_string());
            app.project_path = None;
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
            app.code_projects_menu_open = false;
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            if let Some(prev_path) = previous_project_path.clone() {
                app.project_preview_tabs
                    .insert(prev_path.clone(), std::mem::take(&mut app.preview_tabs));
                app.project_preview_active_path.insert(prev_path, app.active_preview_path.take());
            }
            app.active_tab_id = Some(id.clone());
            if let Some(tab) = app.open_tabs.iter().find(|t| t.id == id) {
                app.screen = tab.screen;
                if let Some(path) = &tab.project_path {
                    app.project_path = Some(path.clone());
                    let tabs = app.project_preview_tabs.remove(path).unwrap_or_default();
                    let active = app.project_preview_active_path.remove(path).unwrap_or_default();
                    app.preview_tabs = tabs;
                    app.active_preview_path = active;
                } else if id == "home" {
                    app.project_path = None;
                    app.preview_tabs.clear();
                    app.active_preview_path = None;
                }
            } else if let Some(ref path) = app.project_path {
                let tabs = app.project_preview_tabs.remove(path).unwrap_or_default();
                let active = app.project_preview_active_path.remove(path).unwrap_or_default();
                app.preview_tabs = tabs;
                app.active_preview_path = active;
            }
            if previous_project_path != app.project_path {
                app.preview_tab_menu_path = None;
                app.preview_tab_menu_pos = None;
            }
            Task::none()
        }
        Message::TabClosed(id) => {
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            if let Some(index) = app.open_tabs.iter().position(|t| t.id == id) {
                app.open_tabs.remove(index);

                // If we closed the active tab, switch to another one
                if app.active_tab_id.as_ref() == Some(&id) {
                    if let Some(last_tab) = app.open_tabs.last() {
                        app.active_tab_id = Some(last_tab.id.clone());
                        app.screen = last_tab.screen;
                        if let Some(path) = &last_tab.project_path {
                            if previous_project_path.as_ref() != Some(path) {
                                if let Some(prev_path) = previous_project_path.clone() {
                                    app.project_preview_tabs.insert(
                                        prev_path.clone(),
                                        std::mem::take(&mut app.preview_tabs),
                                    );
                                    app.project_preview_active_path
                                        .insert(prev_path, app.active_preview_path.take());
                                }
                                app.project_path = Some(path.clone());
                                let tabs =
                                    app.project_preview_tabs.remove(path).unwrap_or_default();
                                let active = app
                                    .project_preview_active_path
                                    .remove(path)
                                    .unwrap_or_default();
                                app.preview_tabs = tabs;
                                app.active_preview_path = active;
                            } else {
                                app.project_path = Some(path.clone());
                            }
                        } else if last_tab.id == "home" {
                            if let Some(prev_path) = previous_project_path.clone() {
                                app.project_preview_tabs.insert(
                                    prev_path.clone(),
                                    std::mem::take(&mut app.preview_tabs),
                                );
                                app.project_preview_active_path
                                    .insert(prev_path, app.active_preview_path.take());
                            }
                            app.project_path = None;
                            app.preview_tab_menu_path = None;
                            app.preview_tab_menu_pos = None;
                        }
                    } else {
                        // Should not happen if home tab is always there, but just in case
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
    }
}
